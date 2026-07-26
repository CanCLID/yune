import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, realpath, rm, symlink } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  WEB06_COLLECTOR_CONTRACT_SHA256,
  WEB06_BINDING_SCENARIO_ORDER,
  WEB06_PREVIEW_SCENARIOS,
  WEB06_RUNNER_TOOLING_PATHS,
  adaptPrivateProtocolReceipt,
  advanceCadenceDeadline,
  buildCollectorOutput,
  buildIncompleteObserverModeProjection,
  buildSuiteSourceArtifactRoles,
  collectFiveWithinSeven,
  combinedAttemptFacts,
  classifyWeb06HarnessFailure,
  commonEndpointSequenceDigest,
  evaluateArtifactResponseGuardObservations,
  evaluateFinalLaneDisposition,
  expectedCommonSamples,
  installWeb06Sentinel,
  mergeWeb06LearnedProtocolSegments,
  mergeWeb06LearnedSentinelSegments,
  parseWeb06CollectorEnvironment,
  protocolCapabilityBlockers,
  protocolHealthBlockers,
  projectWeb06RenderedInput,
  rawEvidencePacketReference,
  resolveCommonSamples,
  selectWeb06LongTaskEntries,
  sentinelLedgerIntegrityErrors,
  validateArtifactResponseGuard,
  validateObservedWeb06Environment,
  validateWeb06RunArtifactSchema,
  web06DomFingerprintDigest,
  writeCompactEvidenceReceipt,
  writeRawEvidencePacket,
} from "./web06-collector.mjs";
import {
  WEB06_OBSERVER_COUNTERBALANCE,
  evaluateObserverOverhead,
  expandScenarioExpectedTimeline,
} from "./web06-metric-contract.mjs";
import { buildRoundEvidenceSummary } from "./web06-receipt-parser.mjs";
import {
  independentlyEvaluateObserverTriplets,
  independentlyRecomputeRoundSummary,
  independentlyValidateRawSentinelIntegrity,
} from "./web06-independent-verifier.mjs";

const COMMIT = "0123456789abcdef0123456789abcdef01234567";
const TREE = "1234567890abcdef1234567890abcdef12345678";
const HASH = "a".repeat(64);
const ARCHIVE_HASH = "b".repeat(64);
const digestJson = (value) => createHash("sha256").update(JSON.stringify(value)).digest("hex");
const stableJson = (value) => Array.isArray(value) ? value.map(stableJson)
  : value && typeof value === "object"
    ? Object.fromEntries(Object.keys(value).sort().map((key) => [key, stableJson(value[key])]))
    : value;
const refreshSourceRoleBindings = (attestation) => {
  const { bindingsSha256: _bindingsSha256, ...bindings } = attestation.sourceArtifactRoles;
  attestation.sourceArtifactRoles.bindingsSha256 = digestJson(stableJson(bindings));
};

function target(id, protocolMode, selectorPolicy = "explicit", origin = "http://127.0.0.1:4173/") {
  return {
    origin,
    sourceCommit: COMMIT,
    sourceTree: TREE,
    treeState: "clean",
    artifactSha256: HASH,
    archiveSha256: ARCHIVE_HASH,
    buildInfoSha256: HASH,
    protocolMode,
    selectorPolicy,
    artifactResponseGuard: {
      version: "web06-artifact-response-guard-v1",
      rootDocumentPath: "index.html",
      entries: [
        { path: "build-info.json", bytes: 10, sha256: HASH },
        { path: "public-artifact-manifest.json", bytes: 20, sha256: HASH },
        { path: "index.html", bytes: 30, sha256: "c".repeat(64) },
      ],
    },
  };
}

function runnerSource() {
  const tooling = {
    version: "web06-runner-tooling-v1",
    files: WEB06_RUNNER_TOOLING_PATHS.map((filePath) => ({ path: filePath, sha256: HASH })),
  };
  return {
    version: "web06-runner-source-v1",
    sourceCommit: COMMIT,
    sourceTree: TREE,
    sourceTreeState: "clean",
    tooling,
    toolingManifestSha256: createHash("sha256").update(JSON.stringify(tooling), "utf8").digest("hex"),
  };
}

function identityManifest(roles) {
  return {
    version: "web06-target-identities-v1",
    metricContractVersion: "web06-metric-v1",
    scenarioRegistryVersion: "web06-scenarios-v1",
    behaviorPredicateVersion: "web06-behavior-predicates-v1",
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    roles,
  };
}

function identityRole(selectedBranch = "NONE", disposition = selectedBranch === "NONE" ? "DIAGNOSTIC" : "PRODUCTION_REDUCTION", overrides = {}) {
  return {
    sourceCommit: COMMIT,
    sourceTree: TREE,
    sourceTreeState: "clean",
    archiveSha256: ARCHIVE_HASH,
    artifactManifestSha256: HASH,
    buildInfoSha256: HASH,
    selectedBranch,
    disposition,
    ...overrides,
  };
}

function runEnvironment() {
  const manifest = {
    version: "web06-run-environment-v1",
    toolchain: {
      rust: "rustc 1.test",
      emscripten: "emcc 4.test",
      node: "v24.test",
      npm: "11.test",
      playwright: "1.test",
      chromium: "test-revision",
      chromiumExecutableSha256: HASH,
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
    browser: {
      viewport: { width: 1365, height: 900 },
      displayRefreshHz: 60,
      displayLane: "declared-60hz",
      cacheRegime: "fresh-profile",
      locale: "zh-HK",
    },
    runner: { workers: 1, retries: 0 },
    ui: { pageSize: 6, aiEnabled: false, debugUi: "production-default", inspectorUi: "production-default" },
  };
  return {
    ...manifest,
    environmentId: createHash("sha256").update(JSON.stringify(manifest), "utf8").digest("hex"),
  };
}

function previewEnv(evidenceRoot) {
  return {
    YUNE_WEB06_EVIDENCE_ROOT: evidenceRoot,
    YUNE_WEB06_COLLECTOR_OUTPUT_PATH: path.join(evidenceRoot, "collector-output.json"),
    YUNE_WEB06_INDEPENDENT_RECOMPUTE_PATH: path.join(evidenceRoot, "independent-recompute.json"),
    YUNE_WEB06_SUITE_ATTESTATION_PATH: path.join(evidenceRoot, "suite-attestation.json"),
    YUNE_WEB06_EXPECTATION: "PREVIEW",
    YUNE_WEB06_RUN_KIND: "preview-canary",
    YUNE_WEB06_RUN_ID: "preview-1",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(identityManifest({ FINAL: identityRole("A") })),
    YUNE_WEB06_RUNNER_SOURCE_JSON: JSON.stringify(runnerSource()),
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: JSON.stringify(runEnvironment()),
    YUNE_WEB06_TARGETS_JSON: JSON.stringify({
      FINAL_MINIMAL: target("FINAL_MINIMAL", "minimal", "omitted", "https://preview.example/"),
    }),
    YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(["FINAL_MINIMAL"]),
    YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(WEB06_PREVIEW_SCENARIOS),
    YUNE_WEB06_BLOCKED_SCENARIOS_JSON: "[]",
    YUNE_WEB06_SELECTED_BRANCH: "A",
    YUNE_WEB06_DISPOSITION: "PRODUCTION_REDUCTION",
    YUNE_WEB06_PLAYWRIGHT_RETRIES: "0",
    YUNE_WEB06_PLAYWRIGHT_WORKERS: "1",
  };
}

function observerEnv(evidenceRoot) {
  return {
    YUNE_WEB06_EVIDENCE_ROOT: evidenceRoot,
    YUNE_WEB06_COLLECTOR_OUTPUT_PATH: path.join(evidenceRoot, "collector-output.json"),
    YUNE_WEB06_INDEPENDENT_RECOMPUTE_PATH: path.join(evidenceRoot, "independent-recompute.json"),
    YUNE_WEB06_SUITE_ATTESTATION_PATH: path.join(evidenceRoot, "suite-attestation.json"),
    YUNE_WEB06_EXPECTATION: "OBSERVER",
    YUNE_WEB06_RUN_KIND: "observer-overhead",
    YUNE_WEB06_RUN_ID: "observer-1",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(identityManifest({
      PRODUCT: identityRole("NONE"),
      BASE: identityRole("NONE"),
    })),
    YUNE_WEB06_RUNNER_SOURCE_JSON: JSON.stringify(runnerSource()),
    YUNE_WEB06_TARGETS_JSON: JSON.stringify({
      PRODUCT: target("PRODUCT", "off", "omitted"),
      BASE_MINIMAL: target("BASE_MINIMAL", "minimal"),
      BASE_FULL: target("BASE_FULL", "full"),
    }),
    YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(["PRODUCT", "BASE_MINIMAL", "BASE_FULL"]),
    YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(["rapid-long-jyutping"]),
    YUNE_WEB06_BLOCKED_SCENARIOS_JSON: "[]",
    YUNE_WEB06_SELECTED_BRANCH: "NONE",
    YUNE_WEB06_DISPOSITION: "DIAGNOSTIC",
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: JSON.stringify(runEnvironment()),
    YUNE_WEB06_PLAYWRIGHT_RETRIES: "0",
    YUNE_WEB06_PLAYWRIGHT_WORKERS: "1",
  };
}

function mutateRunEnvironment(env, mutate) {
  const manifest = JSON.parse(env.YUNE_WEB06_RUN_ENVIRONMENT_JSON);
  delete manifest.environmentId;
  mutate(manifest);
  manifest.environmentId = createHash("sha256").update(JSON.stringify(manifest), "utf8").digest("hex");
  return { ...env, YUNE_WEB06_RUN_ENVIRONMENT_JSON: JSON.stringify(manifest) };
}

test("collector environment is opt-in, external, source-bound, and mode-fixed", () => {
  const parsed = parseWeb06CollectorEnvironment(previewEnv("/tmp/yune-web06-evidence"), { repoRoot: "/repo/yune" });
  assert.deepEqual(parsed.scenarioIds, WEB06_PREVIEW_SCENARIOS);
  assert.deepEqual(parsed.targetOrder, ["FINAL_MINIMAL"]);
  assert.equal(parsed.targets.FINAL_MINIMAL.protocolMode, "minimal");
  assert.equal(parsed.expectation, "PREVIEW");
  const observerOriginal = observerEnv("/tmp/yune-web06-identity-order");
  const observerIdentity = JSON.parse(observerOriginal.YUNE_WEB06_IDENTITY_MANIFEST_JSON);
  observerIdentity.roles = Object.fromEntries(Object.entries(observerIdentity.roles).reverse()
    .map(([name, role]) => [name, Object.fromEntries(Object.entries(role).reverse())]));
  const observerReordered = {
    ...observerOriginal,
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(
      Object.fromEntries(Object.entries(observerIdentity).reverse()),
    ),
  };
  const canonicalObserver = parseWeb06CollectorEnvironment(observerOriginal, { repoRoot: "/repo/yune" });
  const reorderedObserver = parseWeb06CollectorEnvironment(observerReordered, { repoRoot: "/repo/yune" });
  assert.equal(reorderedObserver.identityManifestSha256, canonicalObserver.identityManifestSha256);
  assert.deepEqual(reorderedObserver.identityManifest, canonicalObserver.identityManifest);
  assert.throws(
    () => parseWeb06CollectorEnvironment(previewEnv("/repo/yune/evidence"), { repoRoot: "/repo/yune" }),
    /WEB06_EVIDENCE_ROOT_INSIDE_REPOSITORY/,
  );
  const wrongMode = previewEnv("/tmp/evidence");
  wrongMode.YUNE_WEB06_TARGETS_JSON = JSON.stringify({
    FINAL_MINIMAL: target("FINAL_MINIMAL", "full", "omitted", "https://preview.example/"),
  });
  assert.throws(
    () => parseWeb06CollectorEnvironment(wrongMode, { repoRoot: "/repo/yune" }),
    /WEB06_TARGET_PROTOCOL_MODE_INVALID/,
  );
  assert.throws(
    () => parseWeb06CollectorEnvironment({ ...previewEnv("/tmp/evidence"), YUNE_WEB06_PLAYWRIGHT_RETRIES: "1" }, { repoRoot: "/repo/yune" }),
    /WEB06_PLAYWRIGHT_RETRIES_MUST_BE_ZERO/,
  );
  const missingExpectation = previewEnv("/tmp/evidence");
  delete missingExpectation.YUNE_WEB06_EXPECTATION;
  assert.throws(
    () => parseWeb06CollectorEnvironment(missingExpectation, { repoRoot: "/repo/yune" }),
    /WEB06_EXPECTATION_INVALID/,
  );
  for (const inherited of ["toString", "__proto__", "constructor"]) {
    assert.throws(
      () => parseWeb06CollectorEnvironment({
        ...previewEnv("/tmp/evidence"),
        YUNE_WEB06_EXPECTATION: inherited,
      }, { repoRoot: "/repo/yune" }),
      /WEB06_EXPECTATION_INVALID/,
      `expectation:${inherited}`,
    );
    assert.throws(
      () => parseWeb06CollectorEnvironment({
        ...previewEnv("/tmp/evidence"),
        YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify([inherited]),
      }, { repoRoot: "/repo/yune" }),
      /WEB06_REQUIRED_TARGET_MISSING/,
      `target-order:${inherited}`,
    );
  }
  const extraRole = previewEnv("/tmp/evidence");
  const extraRoleManifest = JSON.parse(extraRole.YUNE_WEB06_IDENTITY_MANIFEST_JSON);
  extraRoleManifest.roles.EXTRA = structuredClone(extraRoleManifest.roles.FINAL);
  extraRole.YUNE_WEB06_IDENTITY_MANIFEST_JSON = JSON.stringify(extraRoleManifest);
  assert.throws(
    () => parseWeb06CollectorEnvironment(extraRole, { repoRoot: "/repo/yune" }),
    /WEB06_IDENTITY_ROLE_SET_MISMATCH/,
  );
  assert.throws(
    () => parseWeb06CollectorEnvironment({ ...previewEnv("/tmp/evidence"), YUNE_WEB06_EXPECTATION: "FINAL" }, { repoRoot: "/repo/yune" }),
    /WEB06_EXPECTATION_RUN_KIND_MISMATCH/,
  );
  assert.throws(
    () => parseWeb06CollectorEnvironment({
      ...previewEnv("/tmp/evidence"),
      YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(["FINAL_MINIMAL", "BASE_MINIMAL"]),
    }, { repoRoot: "/repo/yune" }),
    /WEB06_REQUIRED_TARGET_MISSING|WEB06_EXPECTATION_TARGET_ORDER_MISMATCH/,
  );
  const unpinned = previewEnv("/tmp/evidence");
  unpinned.YUNE_WEB06_TARGETS_JSON = JSON.stringify({
    FINAL_MINIMAL: {
      ...target("FINAL_MINIMAL", "minimal", "omitted", "https://preview.example/"),
      archiveSha256: "c".repeat(64),
    },
  });
  assert.throws(
    () => parseWeb06CollectorEnvironment(unpinned, { repoRoot: "/repo/yune" }),
    /WEB06_TARGET_ARCHIVE_NOT_PINNED/,
  );

  for (const [label, mutate, pattern] of [
    ["AC", (value) => { value.host.powerState = "battery"; }, /AC_POWER_REQUIRED/],
    ["LPM", (value) => { value.host.lowPowerMode = true; }, /LOW_POWER_MODE_INVALID/],
    ["viewport", (value) => { value.browser.viewport.width = 1366; }, /VIEWPORT_INVALID/],
    ["refresh", (value) => { value.browser.displayRefreshHz = 120; }, /REFRESH_INVALID/],
    ["lane", (value) => { value.browser.displayLane = "adaptive"; }, /DISPLAY_LANE_INVALID/],
    ["cache", (value) => { value.browser.cacheRegime = "warm"; }, /CACHE_REGIME_INVALID/],
    ["locale", (value) => { value.browser.locale = "en-US"; }, /LOCALE_INVALID/],
    ["worker", (value) => { value.runner.workers = 2; }, /RUNNER_CONCURRENCY_INVALID/],
    ["AI", (value) => { value.ui.aiEnabled = true; }, /UI_POSTURE_INVALID/],
  ]) {
    assert.throws(
      () => parseWeb06CollectorEnvironment(
        mutateRunEnvironment(previewEnv("/tmp/evidence"), mutate),
        { repoRoot: "/repo/yune" },
      ),
      pattern,
      label,
    );
  }
  for (const origin of [
    "https://user:pass@preview.example/",
    "https://preview.example/?token=innocent",
    "https://preview.example/#fragment",
    "http://preview.example/",
  ]) {
    const env = previewEnv("/tmp/evidence");
    env.YUNE_WEB06_TARGETS_JSON = JSON.stringify({
      FINAL_MINIMAL: target("FINAL_MINIMAL", "minimal", "omitted", origin),
    });
    assert.throws(() => parseWeb06CollectorEnvironment(env, { repoRoot: "/repo/yune" }),
      /WEB06_(?:TARGET_ORIGIN_INVALID|PREVIEW_TARGET_HTTPS_REQUIRED)/);
  }
  const badRunnerPath = previewEnv("/tmp/evidence");
  const runner = JSON.parse(badRunnerPath.YUNE_WEB06_RUNNER_SOURCE_JSON);
  runner.tooling.files[0].path = "apps/yune-web/e2e/not-the-runner.mjs";
  runner.toolingManifestSha256 = createHash("sha256").update(JSON.stringify(runner.tooling), "utf8").digest("hex");
  badRunnerPath.YUNE_WEB06_RUNNER_SOURCE_JSON = JSON.stringify(runner);
  assert.throws(() => parseWeb06CollectorEnvironment(badRunnerPath, { repoRoot: "/repo/yune" }),
    /WEB06_RUNNER_TOOLING_PATH_SET_MISMATCH/);
});

test("observed host/toolchain values must exactly match the declared environment", () => {
  const manifest = runEnvironment();
  const observed = { toolchain: structuredClone(manifest.toolchain), host: structuredClone(manifest.host) };
  assert.equal(validateObservedWeb06Environment(manifest, observed), true);
  observed.host.powerState = "battery";
  assert.throws(() => validateObservedWeb06Environment(manifest, observed), /WEB06_OBSERVED_HOST_MISMATCH/);
  observed.host = structuredClone(manifest.host);
  observed.toolchain.chromium = "forged";
  assert.throws(() => validateObservedWeb06Environment(manifest, observed), /WEB06_OBSERVED_TOOLCHAIN_MISMATCH/);
});

test("artifact response guard is exact and sticky across wrong, unknown, and missing responses", () => {
  const pinned = target("BASE_FULL", "full");
  const validated = validateArtifactResponseGuard(pinned.artifactResponseGuard, pinned, "BASE_FULL");
  const observations = pinned.artifactResponseGuard.entries.map((entry) => ({
    path: entry.path,
    status: 200,
    bytes: entry.bytes,
    sha256: entry.sha256,
  }));
  const evaluate = (rows) => evaluateArtifactResponseGuardObservations({
    guard: validated.guard,
    guardSha256: validated.sha256,
    observations: rows,
    stage: "test",
  });
  assert.equal(evaluate(observations).pass, true);
  const transientWrong = [
    { ...observations[2], sha256: "d".repeat(64) },
    ...observations,
  ];
  assert.equal(evaluate(transientWrong).pass, false, "a later correct response cannot erase wrong bytes");
  assert.ok(evaluate(transientWrong).failureCodes.includes("RESPONSE_SHA256:index.html"));
  const swapped = structuredClone(observations);
  [swapped[0].sha256, swapped[2].sha256] = [swapped[2].sha256, swapped[0].sha256];
  assert.equal(evaluate(swapped).pass, false, "swapped response bytes cannot satisfy path identity");
  const unknown = [...observations, { path: "unsealed.js", status: 200, bytes: 1, sha256: HASH }];
  assert.equal(evaluate(unknown).pass, false);
  assert.equal(evaluate(unknown).unknownPathCount, 1);
  const missing = observations.filter((row) => row.path !== "index.html");
  assert.equal(evaluate(missing).pass, false);
  assert.ok(evaluate(missing).failureCodes.includes("MISSING_RESPONSE:index.html"));
  assert.throws(() => validateArtifactResponseGuard({
    ...pinned.artifactResponseGuard,
    unexpected: true,
  }, pinned), /WEB06_ARTIFACT_RESPONSE_GUARD_SHAPE/);
  assert.throws(() => validateArtifactResponseGuard({
    ...pinned.artifactResponseGuard,
    entries: [...pinned.artifactResponseGuard.entries].reverse(),
  }, pinned), /WEB06_ARTIFACT_RESPONSE_GUARD_BUILD_INFO_IDENTITY/);
});

test("collector, independent, and attestation artifacts use recursive exact public schemas", () => {
  const config = parseWeb06CollectorEnvironment(previewEnv("/tmp/yune-web06-schema"), { repoRoot: "/repo/yune" });
  const attempt = {
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
    rawPacket: { relativePath: `${COMMIT}/preview-1/normal/attempt-1/FINAL_MINIMAL.raw.json`, bytes: 100, sha256: HASH },
    runnerSummaries: {},
  };
  const scenario = {
    targetId: "FINAL_MINIMAL",
    scenarioRunId: "existing-normal-guard",
    scenarioId: "existing-normal-guard",
    schemaId: "jyut6ping3",
    measuredRoundCount: 0,
    validLatencyFrameRoundCount: 0,
    verdict: "SETUP_INVALID",
    preservedHardRedAttemptIds: [],
    preservedHardRedObserved: false,
    attempts: [attempt],
    runnerFiveRoundSummaries: {},
  };
  const observation = { observationSha256: HASH };
  const collector = buildCollectorOutput({ config, scenarioResults: [scenario, {
    ...structuredClone(scenario),
    scenarioRunId: "rapid-jyutping",
    scenarioId: "rapid-jyutping",
    attempts: [{ ...structuredClone(attempt), rawPacket: { ...attempt.rawPacket,
      relativePath: `${COMMIT}/preview-1/rapid/attempt-1/FINAL_MINIMAL.raw.json` } }],
  }], observerTriplets: [], runnerSourceBefore: observation, runnerSourceAfter: observation });
  assert.deepEqual(validateWeb06RunArtifactSchema("collector-output.json", collector).errors, []);
  const independent = {
    version: "web06-independent-recompute-v1",
    writeMode: "create-new",
    collectorOutputSha256: HASH,
    expectation: "PREVIEW",
    disposition: "PRODUCTION_REDUCTION",
    selectedBranch: "A",
    identityManifestSha256: config.identityManifestSha256,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    scenarioResults: collector.scenarioResults.map((row) => ({
      targetId: row.targetId,
      scenarioRunId: row.scenarioRunId,
      scenarioId: row.scenarioId,
      schemaId: row.schemaId,
      measuredRoundCount: 0,
      validLatencyFrameRoundCount: 0,
      verdict: "SETUP_INVALID",
      preservedHardRedAttemptIds: [],
      preservedHardRedObserved: false,
      attemptResults: [{ attemptId: "attempt-1", rawPacketSha256: HASH, measurementStarted: false,
        measurementCompleted: false, classification: "SETUP_INVALID", retainedMeasured: false,
        retainedLogicalRound: false,
        validForLatencyFrame: false, retainedHardRed: false, retryEligible: true, validRedObserved: false,
        failureCode: "WEB06_SETUP_FAILURE" }],
      fiveRoundSummaries: {},
      fiveRoundDiagnosticBindingPairs: {},
      fiveRoundSummarySha256: digestJson({}),
    })),
    observerTriplets: [],
    verificationStatus: "PASS",
  };
  assert.deepEqual(validateWeb06RunArtifactSchema("independent-recompute.json", independent).errors, []);
  const attestation = {
    version: "web06-suite-attestation-v1",
    writeMode: "create-new",
    expectation: "PREVIEW",
    disposition: "PRODUCTION_REDUCTION",
    selectedBranch: "A",
    identityManifestSha256: config.identityManifestSha256,
    runnerSourceManifestSha256: config.runnerSourceManifestSha256,
    runnerSourceObservationSha256: HASH,
    runnerSourcePostObservationSha256: HASH,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    sourceArtifactRoles: buildSuiteSourceArtifactRoles({
      config,
      runnerSourceBefore: observation,
      runnerSourceAfter: observation,
    }),
    versions: collector.versions,
    scenarios: [...config.scenarioIds],
    execution: collector.execution,
    collectorOutput: { relativePath: "collector-output.json", bytes: 100, sha256: HASH },
    independentRecompute: { relativePath: "independent-recompute.json", bytes: 100, sha256: HASH },
    measurementStarted: false,
    measurementCompleted: true,
    verdict: "SETUP_INVALID",
    scenarioResults: collector.scenarioResults,
    observerTriplets: [],
    privacy: { publicAllowlistVersion: "web06-public-evidence-v1", pass: true },
  };
  assert.deepEqual(validateWeb06RunArtifactSchema("suite-attestation.json", attestation).errors, []);
  for (const [label, mutate] of [
    ["runner commit", (copy) => { copy.sourceArtifactRoles.runnerSource.sourceCommit = "f".repeat(40); }],
    ["runner tree", (copy) => { copy.sourceArtifactRoles.runnerSource.sourceTree = "e".repeat(40); }],
    ["runner observation", (copy) => {
      copy.sourceArtifactRoles.runnerSource.beforeObservationSha256 = "c".repeat(64);
      copy.sourceArtifactRoles.runnerSource.afterObservationSha256 = "c".repeat(64);
    }],
    ["runner inherited identity role", (copy) => {
      copy.sourceArtifactRoles.runnerSource.identityRole = "toString";
    }],
    ["top collector contract", (copy) => { copy.collectorContractSha256 = "c".repeat(64); }],
    ["target archive", (copy) => { copy.sourceArtifactRoles.targetRoles.FINAL.archiveSha256 = "c".repeat(64); }],
    ["target artifact", (copy) => {
      copy.sourceArtifactRoles.targetRoles.FINAL.artifactManifestSha256 = "c".repeat(64);
    }],
    ["target build", (copy) => { copy.sourceArtifactRoles.targetRoles.FINAL.buildInfoSha256 = "c".repeat(64); }],
    ["target branch", (copy) => { copy.sourceArtifactRoles.targetRoles.FINAL.selectedBranch = "B"; }],
    ["target disposition", (copy) => {
      copy.sourceArtifactRoles.targetRoles.FINAL.disposition = "MEASURED_NO_GO";
    }],
    ["target membership", (copy) => {
      copy.sourceArtifactRoles.targetRoles.FINAL.targets.EXTRA = structuredClone(
        copy.sourceArtifactRoles.targetRoles.FINAL.targets.FINAL_MINIMAL,
      );
    }],
    ["target protocol", (copy) => {
      copy.sourceArtifactRoles.targetRoles.FINAL.targets.FINAL_MINIMAL.protocolMode = "full";
    }],
    ["target selector", (copy) => {
      copy.sourceArtifactRoles.targetRoles.FINAL.targets.FINAL_MINIMAL.selectorPolicy = "explicit";
    }],
    ["extra identity role", (copy) => {
      copy.sourceArtifactRoles.targetRoles.EXTRA = structuredClone(copy.sourceArtifactRoles.targetRoles.FINAL);
    }],
  ]) {
    const copy = structuredClone(attestation);
    mutate(copy);
    refreshSourceRoleBindings(copy);
    assert.equal(validateWeb06RunArtifactSchema("suite-attestation.json", copy).pass, false, label);
  }
  for (const [fileName, value, mutate] of [
    ["collector-output.json", collector, (copy) => { copy.unknown = true; }],
    ["collector-output.json", collector, (copy) => { copy.collectorContractSha256 = "c".repeat(64); }],
    ["collector-output.json", collector, (copy) => { copy.scenarioResults[0].attempts[0].rawPacket.unknown = true; }],
    ["independent-recompute.json", independent,
      (copy) => { copy.collectorContractSha256 = "c".repeat(64); }],
    ["independent-recompute.json", independent, (copy) => { copy.scenarioResults[0].attemptResults[0].measurementStarted = "false"; }],
    ["suite-attestation.json", attestation, (copy) => { copy.scenarioResults[0].attempts = []; }],
    ["suite-attestation.json", attestation, (copy) => { copy.verdict = "PASS"; }],
    ["suite-attestation.json", attestation, (copy) => { copy.execution.plannedScenarioCount += 1; }],
    ["suite-attestation.json", attestation, (copy) => { copy.expectation = "toString"; }],
    ["suite-attestation.json", attestation, (copy) => {
      copy.sourceArtifactRoles.targetRoles.FINAL.targets.FINAL_MINIMAL.artifactResponseGuardSha256 = "c".repeat(64);
    }],
  ]) {
    const copy = structuredClone(value);
    mutate(copy);
    assert.equal(validateWeb06RunArtifactSchema(fileName, copy).pass, false, `${fileName} mutation`);
  }
});

test("partial observer packets preserve hard REDs and completed clean validity across every public schema", () => {
  const config = parseWeb06CollectorEnvironment(observerEnv("/tmp/yune-web06-observer-schema"), {
    repoRoot: "/repo/yune",
  });
  const partialMode = (modeName, verdict = "SETUP_INVALID") => ({
    ...buildIncompleteObserverModeProjection({
      rawPacket: {
        relativePath: `${COMMIT}/observer-1/triplet-attempt-1/${modeName}.raw.json`,
        bytes: 100,
        sha256: HASH,
      },
      measurementStarted: verdict === "RED_BEHAVIOR",
      behaviorRedObserved: verdict === "RED_BEHAVIOR",
    }),
  });
  const contextIds = ["context-product", "context-minimal", "context-full"];
  const partialTriplet = {
    attemptId: "triplet-attempt-1",
    valid: false,
    counterbalanceSlot: 1,
    freshContextId: contextIds.join("+"),
    modeContextIds: contextIds,
    modeOrder: WEB06_OBSERVER_COUNTERBALANCE[1],
    modeFixedBeforePageLoad: true,
    product: partialMode("product", "RED_BEHAVIOR"),
    minimal: partialMode("minimal"),
    full: partialMode("full"),
  };
  const observerEvaluation = independentlyEvaluateObserverTriplets([partialTriplet]);
  const observation = { observationSha256: HASH };
  const collector = buildCollectorOutput({
    config,
    observerTriplets: [partialTriplet],
    observerEvaluation,
    runnerSourceBefore: observation,
    runnerSourceAfter: observation,
  });
  assert.deepEqual(validateWeb06RunArtifactSchema("collector-output.json", collector).errors, []);

  const independent = {
    version: "web06-independent-recompute-v1",
    writeMode: "create-new",
    collectorOutputSha256: HASH,
    expectation: "OBSERVER",
    disposition: "DIAGNOSTIC",
    selectedBranch: "NONE",
    identityManifestSha256: config.identityManifestSha256,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    scenarioResults: [],
    observerTriplets: [partialTriplet],
    observerEvaluation,
    verificationStatus: "PASS",
  };
  assert.deepEqual(validateWeb06RunArtifactSchema("independent-recompute.json", independent).errors, []);
  const attestation = {
    version: "web06-suite-attestation-v1",
    writeMode: "create-new",
    expectation: "OBSERVER",
    disposition: "DIAGNOSTIC",
    selectedBranch: "NONE",
    identityManifestSha256: config.identityManifestSha256,
    runnerSourceManifestSha256: config.runnerSourceManifestSha256,
    runnerSourceObservationSha256: HASH,
    runnerSourcePostObservationSha256: HASH,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    sourceArtifactRoles: buildSuiteSourceArtifactRoles({
      config,
      runnerSourceBefore: observation,
      runnerSourceAfter: observation,
    }),
    versions: collector.versions,
    scenarios: [...config.scenarioIds],
    execution: collector.execution,
    collectorOutput: { relativePath: "collector-output.json", bytes: 100, sha256: HASH },
    independentRecompute: { relativePath: "independent-recompute.json", bytes: 100, sha256: HASH },
    measurementStarted: true,
    measurementCompleted: false,
    verdict: observerEvaluation.status,
    scenarioResults: [],
    observerTriplets: [partialTriplet],
    privacy: { publicAllowlistVersion: "web06-public-evidence-v1", pass: true },
  };
  assert.deepEqual(validateWeb06RunArtifactSchema("suite-attestation.json", attestation).errors, []);

  for (const [fileName, artifact] of [
    ["collector-output.json", collector],
    ["independent-recompute.json", independent],
    ["suite-attestation.json", attestation],
  ]) {
    const forged = structuredClone(artifact);
    forged.observerTriplets[0].product.hardRedObserved = false;
    assert.equal(validateWeb06RunArtifactSchema(fileName, forged).pass, false,
      `${fileName} must reject a flipped partial hard RED`);
    const erased = structuredClone(artifact);
    erased.observerTriplets[0].product.behaviorRedObserved = false;
    erased.observerTriplets[0].product.hardRedObserved = false;
    assert.equal(validateWeb06RunArtifactSchema(fileName, erased).pass, false,
      `${fileName} must reject a cleared RED_BEHAVIOR observation`);
  }

  const completedMode = (modeName) => {
    const privateCallback = modeName === "product" ? [] : [{
      callbackId: "web06-main-observer-00000001",
      sequenceId: 1,
      operation: "capture",
      startedAt: 2,
      finishedAt: 2.25,
      durationMs: 0.25,
      sourceClass: modeName === "minimal" ? "minimal-probe" : "full-collector",
    }];
    return {
      rawPacket: { relativePath: `${COMMIT}/observer-1/complete/${modeName}.raw.json`, bytes: 100, sha256: HASH },
      measurementStarted: true,
      measurementCompleted: true,
      measurementValid: true,
      behaviorRedObserved: false,
      hardRedBindingValid: true,
      hardRedObserved: false,
      samples: [10],
      commonEquivalenceDigest: HASH,
      ...(modeName === "product" ? {} : { internalEquivalenceDigest: ARCHIVE_HASH }),
      commonVerdict: "PASS",
      internalVerdict: "PASS",
      commonEventCount: 1,
      environmentManifestSha256: config.environmentManifestSha256,
      environmentId: config.environmentId,
      interactionWindowCount: 1,
      sentinelCallbacksMs: [0.25],
      sentinelTotalPerEventMs: [0.25],
      sentinelTotalPerWindowMs: [0.25],
      collectorCallbacksMs: privateCallback.map((row) => row.durationMs),
      mainObserverCallbacksMs: privateCallback.map((row) => row.durationMs),
      workerCollectorCallbacksMs: [],
      callbackLedgerCount: 1,
      callbackLedgerCapacity: 8,
      sentinelAccountedCallbackCount: 1,
      callbackLedgerOverflowCount: 0,
      callbackAttributionComplete: true,
      ...(modeName === "product" ? {} : {
        mainObserverCallbackCount: 1,
        mainObserverCallbackCapacity: 8,
        mainObserverCallbackOverflowCount: 0,
      }),
      callbackIntervals: [{
        kind: "event",
        pageInstanceId: "page-1",
        callbackId: "sentinel-1",
        sequenceId: 1,
        windowIndex: 0,
        eventSequenceId: 1,
        startedAt: 1,
        finishedAt: 1.25,
        durationMs: 0.25,
        sourceClass: "common-sentinel",
      }, ...privateCallback],
      rawLongTasks: [],
      underlyingLongTasksMs: [],
      instrumentationAddedLongTasksMs: [],
    };
  };
  const completedTriplet = structuredClone(partialTriplet);
  completedTriplet.valid = true;
  completedTriplet.product = completedMode("product");
  completedTriplet.minimal = completedMode("minimal");
  completedTriplet.full = completedMode("full");
  const completeFive = Array.from({ length: 5 }, (_, index) => {
    const triplet = structuredClone(completedTriplet);
    const slot = index + 1;
    triplet.attemptId = `triplet-attempt-${slot}`;
    triplet.counterbalanceSlot = slot;
    triplet.modeOrder = WEB06_OBSERVER_COUNTERBALANCE[slot];
    triplet.modeContextIds = [`context-${slot}-product`, `context-${slot}-minimal`, `context-${slot}-full`];
    triplet.freshContextId = triplet.modeContextIds.join("+");
    return triplet;
  });
  const assertObserverEquality = (triplets, label) => {
    assert.deepEqual(independentlyEvaluateObserverTriplets(structuredClone(triplets)),
      evaluateObserverOverhead(structuredClone(triplets)), label);
  };
  assertObserverEquality(completeFive, "clean five-triplet evaluator equality");
  for (const [label, mutate] of [
    ["sentinel callback ceiling", (rows) => { rows[0].minimal.sentinelCallbacksMs[0] = 2; }],
    ["sample shape", (rows) => { rows[0].full.samples.push(11); }],
    ["callback attribution", (rows) => {
      rows[0].minimal.callbackAttributionComplete = false;
      rows[0].minimal.measurementValid = false;
      rows[0].valid = false;
    }],
    ["context reuse", (rows) => {
      rows[1].modeContextIds = [...rows[0].modeContextIds];
      rows[1].freshContextId = rows[1].modeContextIds.join("+");
    }],
    ["common digest", (rows) => { rows[0].full.commonEquivalenceDigest = ARCHIVE_HASH; }],
    ["environment id", (rows) => { rows[0].full.environmentId = "d".repeat(64); }],
    ["counterbalance", (rows) => { rows[0].modeOrder = [...rows[0].modeOrder].reverse(); }],
  ]) {
    const changed = structuredClone(completeFive);
    mutate(changed);
    assertObserverEquality(changed, label);
  }
  const cleanCollector = structuredClone(collector);
  cleanCollector.observerTriplets = [completedTriplet];
  cleanCollector.observerEvaluation = independentlyEvaluateObserverTriplets([completedTriplet]);
  assert.deepEqual(validateWeb06RunArtifactSchema("collector-output.json", cleanCollector).errors, []);
  const completedAttestation = structuredClone(attestation);
  completedAttestation.observerTriplets = [completedTriplet];
  completedAttestation.measurementStarted = true;
  completedAttestation.verdict = cleanCollector.observerEvaluation.status;
  assert.deepEqual(validateWeb06RunArtifactSchema(
    "suite-attestation.json",
    completedAttestation,
  ).errors, []);
  for (const [label, mutate] of [
    ["top environment manifest", (copy) => { copy.environmentManifestSha256 = "c".repeat(64); }],
    ["top environment id", (copy) => { copy.environmentId = "d".repeat(64); }],
    ["mode environment manifest", (copy) => {
      copy.observerTriplets[0].minimal.environmentManifestSha256 = "c".repeat(64);
    }],
    ["mode environment id", (copy) => {
      copy.observerTriplets[0].minimal.environmentId = "d".repeat(64);
    }],
  ]) {
    const forged = structuredClone(completedAttestation);
    mutate(forged);
    assert.equal(validateWeb06RunArtifactSchema("suite-attestation.json", forged).pass, false, label);
  }
  assert.equal(Object.hasOwn(completedTriplet.product, "internalEquivalenceDigest"), false);
  assert.equal(Object.hasOwn(completedTriplet.product, "mainObserverCallbackCount"), false);
  assert.equal(Object.hasOwn(completedTriplet.product, "mainObserverCallbackCapacity"), false);
  assert.equal(Object.hasOwn(completedTriplet.product, "mainObserverCallbackOverflowCount"), false);
  const flippedClean = structuredClone(cleanCollector);
  flippedClean.observerTriplets[0].minimal.measurementValid = false;
  flippedClean.observerTriplets[0].valid = false;
  assert.equal(validateWeb06RunArtifactSchema("collector-output.json", flippedClean).pass, false,
    "a coordinated validity flip cannot relabel a clean completed mode");

  for (const privateKey of [
    "internalEquivalenceDigest",
    "mainObserverCallbackCount",
    "mainObserverCallbackCapacity",
    "mainObserverCallbackOverflowCount",
  ]) {
    const leaked = structuredClone(cleanCollector);
    leaked.observerTriplets[0].product[privateKey] =
      privateKey === "internalEquivalenceDigest" ? HASH : 0;
    assert.equal(validateWeb06RunArtifactSchema("collector-output.json", leaked).pass, false,
      `PRODUCT rejects private observer key ${privateKey}`);
  }

  const invalidatedMode = (triplet, modeName, mutate) => {
    const changed = structuredClone(triplet);
    mutate(changed[modeName]);
    Object.assign(changed[modeName], {
      measurementValid: false,
      hardRedBindingValid: false,
      hardRedObserved: changed[modeName].behaviorRedObserved === true,
      callbackAttributionComplete: false,
      commonVerdict: changed[modeName].behaviorRedObserved === true ? "RED_BEHAVIOR" : "SETUP_INVALID",
      internalVerdict: changed[modeName].behaviorRedObserved === true ? "RED_BEHAVIOR" : "SETUP_INVALID",
    });
    changed.valid = false;
    return changed;
  };
  const collectorForTriplet = (triplet) => {
    const value = structuredClone(collector);
    value.observerTriplets = [triplet];
    value.observerEvaluation = independentlyEvaluateObserverTriplets([triplet]);
    return value;
  };
  for (const [label, mutate] of [
    ["sentinel count mismatch", (mode) => { mode.callbackLedgerCount += 1; }],
    ["sentinel capacity exceeded", (mode) => { mode.callbackLedgerCapacity = 0; }],
    ["sentinel overflow", (mode) => { mode.callbackLedgerOverflowCount = 1; }],
    ["private count mismatch", (mode) => { mode.mainObserverCallbackCount += 1; }],
    ["private capacity exceeded", (mode) => { mode.mainObserverCallbackCapacity = 0; }],
    ["private overflow", (mode) => { mode.mainObserverCallbackOverflowCount = 1; }],
  ]) {
    const invalid = invalidatedMode(completedTriplet, "minimal", mutate);
    assert.deepEqual(validateWeb06RunArtifactSchema(
      "collector-output.json",
      collectorForTriplet(invalid),
    ).errors, [], `${label}: completed semantic invalidity is preserved with false attribution`);
    const forged = structuredClone(invalid);
    Object.assign(forged.minimal, {
      measurementValid: true,
      hardRedBindingValid: true,
      callbackAttributionComplete: true,
      commonVerdict: "PASS",
      internalVerdict: "PASS",
    });
    forged.valid = true;
    const forgedCollector = collectorForTriplet(forged);
    assert.equal(validateWeb06RunArtifactSchema("collector-output.json", forgedCollector).pass, false,
      `${label}: forged attribution is rejected`);
  }

  for (const [label, mutate] of [
    ["negative sentinel callback", (mode) => {
      Object.assign(mode.callbackIntervals[0], { finishedAt: 0, durationMs: -1 });
      mode.sentinelCallbacksMs = [-1];
      mode.sentinelTotalPerEventMs = [-1];
      mode.sentinelTotalPerWindowMs = [-1];
    }],
    ["negative sentinel event total", (mode) => { mode.sentinelTotalPerEventMs[0] = -1; }],
    ["negative sentinel window total", (mode) => { mode.sentinelTotalPerWindowMs[0] = -1; }],
    ["negative collector duration", (mode) => { mode.collectorCallbacksMs[0] = -1; }],
    ["negative main duration", (mode) => { mode.mainObserverCallbacksMs[0] = -1; }],
    ["negative worker duration", (mode) => {
      mode.workerCollectorCallbacksMs = [-1];
      mode.collectorCallbacksMs.push(-1);
    }],
    ["negative raw Long Task", (mode) => {
      mode.rawLongTasks = [{
        startTime: 10,
        durationMs: -1,
        pageInstanceId: "page-1",
        overlapsInteractionWindow: true,
        overlapsIdleControl: false,
        locus: "0:rapid-1",
      }];
    }],
  ]) {
    const invalid = invalidatedMode(completedTriplet, "minimal", mutate);
    assert.deepEqual(validateWeb06RunArtifactSchema(
      "collector-output.json",
      collectorForTriplet(invalid),
    ).errors, [], `${label}: signed invalid evidence is retained`);
    const forged = structuredClone(invalid);
    Object.assign(forged.minimal, {
      measurementValid: true,
      hardRedBindingValid: true,
      callbackAttributionComplete: true,
      commonVerdict: "PASS",
      internalVerdict: "PASS",
    });
    forged.valid = true;
    assert.equal(validateWeb06RunArtifactSchema(
      "collector-output.json",
      collectorForTriplet(forged),
    ).pass, false, `${label}: signed evidence cannot be forged valid`);
  }
  const signedLongTaskWithBehavior = invalidatedMode(completedTriplet, "minimal", (mode) => {
    mode.behaviorRedObserved = true;
    mode.rawLongTasks = [{
      startTime: 10,
      durationMs: -1,
      pageInstanceId: "page-1",
      overlapsInteractionWindow: true,
      overlapsIdleControl: false,
      locus: "0:rapid-1",
    }];
  });
  assert.equal(signedLongTaskWithBehavior.minimal.commonVerdict, "RED_BEHAVIOR");
  assert.equal(signedLongTaskWithBehavior.minimal.hardRedObserved, true);
  assert.deepEqual(validateWeb06RunArtifactSchema(
    "collector-output.json",
    collectorForTriplet(signedLongTaskWithBehavior),
  ).errors, [], "signed-invalid Long Task cannot erase independent behavior RED evidence");
  const behaviorErasedBySetup = structuredClone(signedLongTaskWithBehavior);
  behaviorErasedBySetup.minimal.commonVerdict = "SETUP_INVALID";
  behaviorErasedBySetup.minimal.internalVerdict = "SETUP_INVALID";
  assert.equal(validateWeb06RunArtifactSchema(
    "collector-output.json",
    collectorForTriplet(behaviorErasedBySetup),
  ).pass, false, "signed-invalid Long Task cannot relabel a retained behavior RED as setup-only");

  const locallyRedWithInvalidPeer = (mutate) => {
    const changed = structuredClone(completedTriplet);
    mutate(changed.minimal);
    changed.minimal.hardRedObserved = true;
    changed.full.callbackLedgerCount += 1;
    Object.assign(changed.full, {
      measurementValid: false,
      hardRedBindingValid: false,
      hardRedObserved: false,
      callbackAttributionComplete: false,
      commonVerdict: "SETUP_INVALID",
      internalVerdict: "SETUP_INVALID",
    });
    changed.valid = false;
    return changed;
  };
  for (const [label, mutate] of [
    ["sentinel callback", (mode) => {
      Object.assign(mode.callbackIntervals[0], { finishedAt: 1.5, durationMs: 0.5 });
      mode.sentinelCallbacksMs = [0.5];
      mode.sentinelTotalPerEventMs = [0.5];
      mode.sentinelTotalPerWindowMs = [0.5];
    }],
    ["sentinel per-event total", (mode) => {
      const first = mode.callbackIntervals[0];
      const privateCallback = mode.callbackIntervals[1];
      mode.callbackIntervals = [
        first,
        { ...first, callbackId: "sentinel-2", sequenceId: 2,
          startedAt: 1.5, finishedAt: 1.875, durationMs: 0.375 },
        { ...first, callbackId: "sentinel-3", sequenceId: 3,
          startedAt: 2, finishedAt: 2.375, durationMs: 0.375 },
        privateCallback,
      ];
      mode.callbackLedgerCount = 3;
      mode.sentinelAccountedCallbackCount = 3;
      mode.sentinelCallbacksMs = [0.25, 0.375, 0.375];
      mode.sentinelTotalPerEventMs = [1];
      mode.sentinelTotalPerWindowMs = [1];
    }],
    ["main collector callback", (mode) => {
      Object.assign(mode.callbackIntervals[1], { finishedAt: 7, durationMs: 5 });
      mode.mainObserverCallbacksMs = [5];
      mode.collectorCallbacksMs = [5];
    }],
    ["worker collector callback", (mode) => {
      mode.workerCollectorCallbacksMs = [5];
      mode.collectorCallbacksMs = [0.25, 5];
    }],
    ["instrumentation-added Long Task", (mode) => {
      mode.rawLongTasks = [{
        startTime: 10,
        durationMs: 50,
        pageInstanceId: "page-1",
        overlapsInteractionWindow: true,
        overlapsIdleControl: false,
        locus: "0:rapid-1",
      }];
      mode.instrumentationAddedLongTasksMs = [50];
    }],
  ]) {
    const changed = locallyRedWithInvalidPeer(mutate);
    const producer = evaluateObserverOverhead(structuredClone([changed]));
    const independentResult = independentlyEvaluateObserverTriplets(structuredClone([changed]));
    assert.deepEqual(independentResult, producer, `${label}: producer/verifier equality`);
    assert.deepEqual(producer.preservedUnpairedReds, ["triplet-attempt-1:minimal"],
      `${label}: local RED survives an invalid peer`);
    assert.ok(producer.violations.includes("triplet-attempt-1:minimal-unpaired-valid-red"));
    assert.deepEqual(validateWeb06RunArtifactSchema(
      "collector-output.json",
      collectorForTriplet(changed),
    ).errors, [], `${label}: preserved local RED artifact`);
    const forged = structuredClone(changed);
    forged.minimal.hardRedObserved = false;
    assert.equal(validateWeb06RunArtifactSchema(
      "collector-output.json",
      collectorForTriplet(forged),
    ).pass, false, `${label}: local RED declaration cannot be cleared`);
  }
});

test("expectation matrix pins exact product/base/final identities and selector policies", () => {
  const observer = {
    YUNE_WEB06_EVIDENCE_ROOT: "/tmp/yune-web06-observer",
    YUNE_WEB06_COLLECTOR_OUTPUT_PATH: "/tmp/yune-web06-observer/collector-output.json",
    YUNE_WEB06_INDEPENDENT_RECOMPUTE_PATH: "/tmp/yune-web06-observer/independent-recompute.json",
    YUNE_WEB06_SUITE_ATTESTATION_PATH: "/tmp/yune-web06-observer/suite-attestation.json",
    YUNE_WEB06_EXPECTATION: "OBSERVER",
    YUNE_WEB06_RUN_KIND: "observer-overhead",
    YUNE_WEB06_RUN_ID: "observer-1",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(identityManifest({
      PRODUCT: identityRole("NONE"),
      BASE: identityRole("NONE"),
    })),
    YUNE_WEB06_RUNNER_SOURCE_JSON: JSON.stringify(runnerSource()),
    YUNE_WEB06_TARGETS_JSON: JSON.stringify({
      PRODUCT: target("PRODUCT", "off", "omitted"),
      BASE_MINIMAL: target("BASE_MINIMAL", "minimal"),
      BASE_FULL: target("BASE_FULL", "full"),
    }),
    YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(["PRODUCT", "BASE_MINIMAL", "BASE_FULL"]),
    YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(["rapid-long-jyutping"]),
    YUNE_WEB06_BLOCKED_SCENARIOS_JSON: "[]",
    YUNE_WEB06_SELECTED_BRANCH: "NONE",
    YUNE_WEB06_DISPOSITION: "DIAGNOSTIC",
    YUNE_WEB06_RUN_ENVIRONMENT_JSON: JSON.stringify(runEnvironment()),
    YUNE_WEB06_PLAYWRIGHT_RETRIES: "0",
    YUNE_WEB06_PLAYWRIGHT_WORKERS: "1",
  };
  const parsed = parseWeb06CollectorEnvironment(observer, { repoRoot: "/repo/yune" });
  assert.deepEqual(parsed.targetOrder, ["PRODUCT", "BASE_MINIMAL", "BASE_FULL"]);
  assert.equal(parsed.targets.BASE_MINIMAL.sourceCommit, parsed.targets.BASE_FULL.sourceCommit);
  assert.equal(parsed.targets.BASE_MINIMAL.artifactSha256, parsed.targets.BASE_FULL.artifactSha256);

  const splitBase = structuredClone(observer);
  const splitTargets = JSON.parse(splitBase.YUNE_WEB06_TARGETS_JSON);
  splitTargets.BASE_FULL.sourceCommit = "f".repeat(40);
  splitBase.YUNE_WEB06_TARGETS_JSON = JSON.stringify(splitTargets);
  assert.throws(
    () => parseWeb06CollectorEnvironment(splitBase, { repoRoot: "/repo/yune" }),
    /WEB06_TARGET_SOURCE_COMMIT_NOT_PINNED/,
  );

  const baseline = {
    ...observer,
    YUNE_WEB06_EXPECTATION: "BASELINE",
    YUNE_WEB06_RUN_KIND: "full",
    YUNE_WEB06_RUN_ID: "baseline-1",
    YUNE_WEB06_TARGETS_JSON: JSON.stringify({ BASE_FULL: target("BASE_FULL", "full") }),
    YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(["BASE_FULL"]),
    YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(WEB06_BINDING_SCENARIO_ORDER),
    YUNE_WEB06_DISPOSITION: "SOURCE_CURRENT_BASELINE",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(identityManifest({ BASE: identityRole("NONE", "SOURCE_CURRENT_BASELINE") })),
  };
  assert.deepEqual(
    parseWeb06CollectorEnvironment(baseline, { repoRoot: "/repo/yune" }).targetOrder,
    ["BASE_FULL"],
  );
  assert.throws(
    () => parseWeb06CollectorEnvironment({
      ...baseline,
      YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(["rapid-jyutping"]),
    }, { repoRoot: "/repo/yune" }),
    /WEB06_BINDING_SCENARIO_ORDER_MISMATCH/,
  );
  assert.throws(
    () => parseWeb06CollectorEnvironment({
      ...baseline,
      YUNE_WEB06_SCENARIOS_JSON: JSON.stringify([...WEB06_BINDING_SCENARIO_ORDER].reverse()),
    }, { repoRoot: "/repo/yune" }),
    /WEB06_BINDING_SCENARIO_ORDER_MISMATCH/,
  );
  const reviewedBlocked = {
    scenarioRunId: "learned-row",
    disposition: "BLOCKED",
    reviewCommit: COMMIT,
    planSha256: HASH,
    reasonCode: "protocol-continuity-unavailable",
  };
  const blockedBaseline = {
    ...baseline,
    YUNE_WEB06_BLOCKED_SCENARIOS_JSON: JSON.stringify([reviewedBlocked]),
    YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(WEB06_BINDING_SCENARIO_ORDER.filter((id) => id !== "learned-row")),
  };
  assert.throws(
    () => parseWeb06CollectorEnvironment(blockedBaseline, { repoRoot: "/repo/yune" }),
    /WEB06_BLOCKED_SCENARIOS_CANNOT_AUTHORIZE_EXECUTION/,
  );
  assert.throws(
    () => parseWeb06CollectorEnvironment({
      ...blockedBaseline,
      YUNE_WEB06_BLOCKED_SCENARIOS_JSON: JSON.stringify([{ ...reviewedBlocked, reviewCommit: "not-reviewed" }]),
    }, { repoRoot: "/repo/yune" }),
    /WEB06_BLOCKED_REVIEW_COMMIT_INVALID/,
  );

  const finalEnv = {
    ...baseline,
    YUNE_WEB06_EXPECTATION: "FINAL",
    YUNE_WEB06_RUN_ID: "final-1",
    YUNE_WEB06_SELECTED_BRANCH: "A",
    YUNE_WEB06_DISPOSITION: "PRODUCTION_REDUCTION",
    YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(identityManifest({ FINAL: identityRole("A") })),
    YUNE_WEB06_TARGETS_JSON: JSON.stringify({ FINAL_FULL: target("FINAL_FULL", "full") }),
    YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(["FINAL_FULL"]),
  };
  assert.deepEqual(
    parseWeb06CollectorEnvironment(finalEnv, { repoRoot: "/repo/yune" }).targetOrder,
    ["FINAL_FULL"],
  );
});

test("raw writer retains source/run/attempt identity and rejects nested symlinks", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "web06-collector-"));
  const repo = path.join(root, "repo");
  const evidence = path.join(root, "external");
  await mkdir(repo);
  try {
    const written = await writeRawEvidencePacket({
      evidenceRoot: evidence,
      repoRoot: repo,
      runId: "run-1",
      sourceCommit: COMMIT,
      scenarioId: "rapid-jyutping",
      attemptId: "attempt-1",
      mode: "BASE_FULL",
      packet: { private: "raw", timestamp: 1 },
    });
    assert.match(written.rawPacketSha256, /^[0-9a-f]{64}$/);
    assert.ok(written.rawPath.startsWith(`${await realpath(evidence)}${path.sep}`));
    for (const scenarioRunId of ["fifo-pressure-barriers@jyut6ping3", "fifo-pressure-barriers@luna_pinyin"]) {
      const multiSchemaRaw = await writeRawEvidencePacket({
        evidenceRoot: evidence,
        repoRoot: repo,
        runId: "run-multi-schema",
        sourceCommit: COMMIT,
        scenarioId: scenarioRunId,
        attemptId: "attempt-1",
        mode: "BASE_FULL",
        packet: { scenarioRunId },
      });
      assert.ok(multiSchemaRaw.rawPath.includes(scenarioRunId));
      const compact = await writeCompactEvidenceReceipt({
        evidenceRoot: evidence,
        repoRoot: repo,
        runId: "run-multi-schema",
        sourceCommit: COMMIT,
        scenarioId: scenarioRunId,
        attemptId: "attempt-1",
        mode: "BASE_FULL",
        receipt: { scenarioRunId },
      });
      assert.ok(compact.publicPath.includes(scenarioRunId));
    }
    await assert.rejects(
      writeRawEvidencePacket({
        evidenceRoot: evidence,
        repoRoot: repo,
        runId: "run-1",
        sourceCommit: COMMIT,
        scenarioId: "rapid-jyutping",
        attemptId: "attempt-1",
        mode: "BASE_FULL",
        packet: { overwritten: true },
      }),
      /EEXIST/,
    );
    await assert.rejects(
      writeCompactEvidenceReceipt({
        evidenceRoot: evidence,
        repoRoot: repo,
        runId: "run-1",
        sourceCommit: COMMIT,
        scenarioId: "rapid-jyutping",
        attemptId: "attempt-1",
        mode: "BASE_FULL",
        receipt: { accessToken: "forbidden" },
      }),
      /WEB06_PUBLIC_EVIDENCE_PRIVACY/,
    );

    const symlinkEvidence = path.join(root, "symlink-external");
    await mkdir(symlinkEvidence);
    await mkdir(path.join(root, "escape"));
    await mkdir(path.join(symlinkEvidence, COMMIT));
    await symlink(path.join(root, "escape"), path.join(symlinkEvidence, COMMIT, "run-2"));
    await assert.rejects(
      writeRawEvidencePacket({
        evidenceRoot: symlinkEvidence,
        repoRoot: repo,
        runId: "run-2",
        sourceCommit: COMMIT,
        scenarioId: "rapid-jyutping",
        attemptId: "attempt-1",
        mode: "BASE_FULL",
        packet: {},
      }),
      /WEB06_EVIDENCE_SYMLINK_OR_NON_DIRECTORY/,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("raw references use the reservation's canonical root across lexical root aliases", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "web06-canonical-root-"));
  const repo = path.join(root, "repo");
  const canonicalEvidence = path.join(root, "canonical-evidence");
  const aliasEvidence = path.join(root, "alias-evidence");
  await mkdir(repo);
  await mkdir(canonicalEvidence);
  await symlink(canonicalEvidence, aliasEvidence);
  try {
    const written = await writeRawEvidencePacket({
      evidenceRoot: aliasEvidence,
      repoRoot: repo,
      runId: "alias-run",
      sourceCommit: COMMIT,
      scenarioId: "rapid-jyutping",
      attemptId: "attempt-1",
      mode: "BASE_FULL",
      packet: { identity: "alias-root" },
    });
    assert.equal(written.evidenceRootCanonicalPath, await realpath(canonicalEvidence));
    assert.deepEqual(rawEvidencePacketReference(written), {
      relativePath: `${COMMIT}/alias-run/rapid-jyutping/attempt-1/BASE_FULL.raw.json`,
      bytes: written.rawPacketBytes,
      sha256: written.rawPacketSha256,
    });
    assert.throws(() => rawEvidencePacketReference({
      ...written,
      rawPath: path.join(root, "outside.raw.json"),
    }), /WEB06_RAW_REFERENCE_OUTSIDE_EVIDENCE_ROOT/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("late host timing rebases and never emits a short catch-up deadline", () => {
  assert.deepEqual(
    advanceCadenceDeadline({ previousActualDispatchAt: 100, nominalGapMs: 60, nowMs: 140 }),
    { requestedDispatchAt: 160, rebasedAfterLateHost: false, phaseDeadline: 160 },
  );
  const late = advanceCadenceDeadline({ previousActualDispatchAt: 100, nominalGapMs: 60, nowMs: 175 });
  assert.deepEqual(late, { requestedDispatchAt: 175, rebasedAfterLateHost: true, phaseDeadline: 160 });
  assert.equal(
    advanceCadenceDeadline({ previousActualDispatchAt: late.requestedDispatchAt, nominalGapMs: 60, nowMs: 176 }).requestedDispatchAt,
    235,
  );
});

test("five-within-seven counts a measured RED and replaces only invalid attempts", async () => {
  const statuses = ["SETUP_INVALID", "RED", "PASS", "NO_VERDICT_INVALID_CADENCE", "PASS", "PASS", "PASS"];
  const result = await collectFiveWithinSeven(async ({ attemptId, attemptNumber }) => {
    const status = statuses[attemptNumber - 1];
    return {
      receipt: { attemptId },
      measurementStarted: true,
      measurementCompleted: true,
      parsed: {
        status,
        cadence: status === "NO_VERDICT_INVALID_CADENCE" ? "TOO_SHORT" : "IN_RANGE",
        behaviorErrors: status === "RED" ? ["MEASURED_RED"] : [],
        thresholdViolations: [],
        frameRed: false,
        longTaskRed: false,
      },
    };
  });
  assert.equal(result.series.status, "COMPLETE_WITH_RED");
  assert.equal(result.series.measuredCount, 5);
  assert.equal(result.attempts.length, 7);
  assert.equal(result.measuredAttempts.length, 5);
  assert.deepEqual(result.measuredReceipts.map((receipt) => receipt.attemptId), [
    "attempt-2", "attempt-3", "attempt-5", "attempt-6", "attempt-7",
  ]);
});

test("final measured no-go preserves a localized RED without requiring every row to fail", () => {
  const green = {
    scenarioRunId: "existing-normal-guard",
    seriesStatus: "COMPLETE_GREEN",
    validLatencyFrameRoundCount: 5,
    internalPoolPass: true,
    commonPoolPass: true,
  };
  const red = {
    scenarioRunId: "rapid-jyutping",
    seriesStatus: "COMPLETE_WITH_RED",
    validLatencyFrameRoundCount: 5,
    internalPoolPass: false,
    commonPoolPass: false,
  };
  assert.deepEqual(evaluateFinalLaneDisposition({
    disposition: "MEASURED_NO_GO",
    scenarios: [green, red],
  }), { pass: true, validRedObserved: true, violations: [] });

  const allGreenNoGo = evaluateFinalLaneDisposition({
    disposition: "MEASURED_NO_GO",
    scenarios: [green, { ...green, scenarioRunId: "rapid-jyutping" }],
  });
  assert.equal(allGreenNoGo.pass, false);
  assert.deepEqual(allGreenNoGo.violations, ["MEASURED_NO_GO_WITHOUT_RED"]);

  const productionRed = evaluateFinalLaneDisposition({
    disposition: "PRODUCTION_REDUCTION",
    scenarios: [green, red],
  });
  assert.equal(productionRed.pass, false);
  assert.ok(productionRed.violations.includes("rapid-jyutping:FINAL_NOT_GREEN"));

  const setupBlocked = evaluateFinalLaneDisposition({
    disposition: "MEASURED_NO_GO",
    scenarios: [green, red, {
      scenarioRunId: "learned-row",
      seriesStatus: "SETUP_NO_GO",
      validLatencyFrameRoundCount: 0,
      internalPoolPass: false,
      commonPoolPass: false,
    }],
  });
  assert.equal(setupBlocked.pass, false);
  assert.ok(setupBlocked.violations.includes("learned-row:SETUP_NO_GO:SETUP_NO_GO"));

  for (const incoherent of [
    { ...green, internalPoolPass: false },
    { ...red, internalPoolPass: true, commonPoolPass: true },
  ]) {
    assert.equal(evaluateFinalLaneDisposition({
      disposition: "MEASURED_NO_GO",
      scenarios: [incoherent],
    }).pass, false);
  }
});

test("cadence-invalid hard reds are preserved but cannot occupy latency/frame slots", async () => {
  const result = await collectFiveWithinSeven(async ({ attemptId, attemptNumber }) => {
    const invalidBehaviorRed = attemptNumber === 1;
    return {
      receipt: { attemptId },
      measurementStarted: true,
      measurementCompleted: true,
      parsed: {
        status: invalidBehaviorRed ? "RED_BEHAVIOR" : "PASS",
        cadence: invalidBehaviorRed ? "TOO_SHORT" : "IN_RANGE",
        behaviorErrors: invalidBehaviorRed ? ["EVENT_REORDERED"] : [],
        thresholdViolations: [],
        setupErrors: [],
        frameRed: false,
        longTaskRed: false,
      },
    };
  });
  assert.equal(result.attempts.length, 5);
  assert.equal(result.series.status, "COMPLETE_WITH_RED");
  assert.equal(result.series.measuredCount, 5);
  assert.equal(result.series.validLatencyFrameCount, 4);
  assert.equal(result.series.preservedHardRedCount, 1);
  assert.deepEqual(result.measuredReceipts.map((receipt) => receipt.attemptId), [
    "attempt-1", "attempt-2", "attempt-3", "attempt-4", "attempt-5",
  ]);
  assert.deepEqual(result.validLatencyFrameReceipts.map((receipt) => receipt.attemptId), [
    "attempt-2", "attempt-3", "attempt-4", "attempt-5",
  ]);
});

test("completed setup-qualified behavior RED is logical but nonnumeric, while incomplete RED stops setup-no-go", async () => {
  const completed = await collectFiveWithinSeven(async ({ attemptId, attemptNumber }) => {
    const red = attemptNumber === 1;
    return {
      receipt: { attemptId },
      measurementStarted: true,
      measurementCompleted: true,
      parsed: {
        status: red ? "RED_BEHAVIOR" : "PASS",
        cadence: "IN_RANGE",
        setupErrors: red ? ["SETUP_INVALID_CLOCK_CALIBRATION:d0"] : [],
        behaviorErrors: red ? ["EVENT_REORDERED:1"] : [],
        thresholdViolations: [],
        frameRed: false,
        longTaskRed: false,
      },
    };
  });
  assert.equal(completed.attempts.length, 5);
  assert.equal(completed.series.status, "COMPLETE_WITH_RED");
  assert.equal(completed.series.measuredCount, 5);
  assert.equal(completed.series.validLatencyFrameCount, 4);
  assert.equal(completed.series.retained[0].retainedLogicalRound, true);
  assert.equal(completed.series.retained[0].validForLatencyFrame, false);

  const incomplete = await collectFiveWithinSeven(async ({ attemptId }) => ({
    receipt: { attemptId },
    measurementStarted: true,
    measurementCompleted: false,
    parsed: {
      status: "RED_BEHAVIOR",
      cadence: "NOT_APPLICABLE",
      setupErrors: ["SETUP_INCOMPLETE_MEASUREMENT"],
      behaviorErrors: ["WEB06_ACTION_COMPLETION_COUNT"],
      thresholdViolations: [],
      frameRed: false,
      longTaskRed: false,
    },
  }));
  assert.equal(incomplete.attempts.length, 1);
  assert.equal(incomplete.series.status,
    "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS_WITH_PRESERVED_RED");
  assert.equal(incomplete.series.measuredCount, 0);
  assert.equal(incomplete.series.validLatencyFrameCount, 0);
  assert.equal(incomplete.series.preservedHardRedCount, 1);
  assert.equal(incomplete.series.retained[0].retainedLogicalRound, false);
  assert.equal(incomplete.series.retained[0].verdict, "RED_INCOMPLETE_BEHAVIOR");
  const disposition = evaluateFinalLaneDisposition({
    disposition: "MEASURED_NO_GO",
    scenarios: [{
      scenarioRunId: "rapid-jyutping",
      seriesStatus: incomplete.series.status,
      validLatencyFrameRoundCount: 0,
    }],
  });
  assert.equal(disposition.pass, false);
  assert.ok(disposition.violations.includes(
    "rapid-jyutping:SETUP_NO_GO:SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS_WITH_PRESERVED_RED",
  ));
  assert.ok(disposition.violations.includes("MEASURED_NO_GO_WITHOUT_RED"));
});

test("attempt classification takes the worst common and internal parser verdict", () => {
  const pass = {
    status: "PASS",
    cadence: "IN_RANGE",
    behaviorErrors: [],
    thresholdViolations: [],
    frameRed: false,
    longTaskRed: false,
  };
  const facts = combinedAttemptFacts({
    internalParsed: pass,
    commonParsed: { ...pass, status: "RED_BEHAVIOR", behaviorErrors: ["COMMON_DOM_ENDPOINT"] },
    attemptId: "attempt-1",
  });
  assert.equal(facts.behaviorRed, true);
  assert.equal(facts.setupInvalid, false);
  const disagreement = combinedAttemptFacts({
    internalParsed: pass,
    commonParsed: { ...pass, cadence: "TOO_SHORT" },
    attemptId: "attempt-2",
  });
  assert.equal(disagreement.setupInvalid, true);
});

test("harness failures use an explicit dimensional table and unknown transport loss is setup-invalid", () => {
  for (const code of [
    "WEB06_ACTION_COMPLETION_COUNT",
    "WEB06_IMPORT_CONTINUATION_MARKER_TIMEOUT",
    "WEB06_LEARNED_PREPARE_CONTINUITY_INVALID",
    "WEB06_LEARNED_PROTOCOL_SEQUENCE_REUSE",
    "WEB06_LEARNED_RELOAD_ARRIVAL_INVALID",
    "WEB06_LEARNED_RELOAD_BIND_INVALID",
    "WEB06_PRESSURE_ACTION_MISSING",
    "WEB06_SAME_TASK_PAIR_REORDERED",
  ]) assert.deepEqual(classifyWeb06HarnessFailure(new Error(`${code}:detail`)), { code, dimension: "behavior" });
  for (const code of [
    "WEB06_RUNNER_SOURCE_CHANGED_DURING_ATTEMPT",
    "WEB06_SETUP_FOREGROUND_POSTURE",
    "WEB06_WINDOW_DURATION_INVALID",
    "WEB06_MEASURED_UI_PAGE_SIZE_NOT_SIX",
  ]) assert.deepEqual(classifyWeb06HarnessFailure(new Error(`${code}:detail`)), { code, dimension: "setup" });
  assert.deepEqual(classifyWeb06HarnessFailure(new Error("Target page, context or browser has been closed")), {
    code: "WEB06_UNCLASSIFIED_FAILURE",
    dimension: "setup",
  });
  assert.deepEqual(classifyWeb06HarnessFailure(new Error("WEB06_POST_MEASUREMENT_PACKAGING_FAILURE:hash")), {
    code: "WEB06_POST_MEASUREMENT_PACKAGING_FAILURE",
    dimension: "fatal",
  });
});

test("protocol preflight fails closed on provenance, pending fanout, and unsupported real UI", () => {
  const status = {
    valid: true,
    queueDepth: 0,
    lastEventSequenceId: 1,
    lastActionSequenceId: 1,
    receiptWindowStartEventSequenceId: 2,
    receiptWindowStartActionSequenceId: 2,
    pendingFanoutActions: 0,
    pendingTerminalActions: 0,
  };
  const protocol = { protocolVersion: "web06-private-v1", mode: "full" };
  assert.deepEqual(protocolCapabilityBlockers({
    mode: "full",
    scenarioId: "rapid-jyutping",
    protocol,
    status,
    uiCapabilities: {},
  }), []);
  assert.deepEqual(protocolCapabilityBlockers({
    mode: "full",
    scenarioId: "fifo-pressure-barriers",
    protocol,
    status: { ...status, pendingFanoutActions: 1 },
    uiCapabilities: { importUserdbSameTask: false },
  }), ["PENDING_FANOUT_ACTIONS", "FIFO_IMPORT_SAME_TASK_UI_UNSUPPORTED"]);
  assert.deepEqual(protocolCapabilityBlockers({
    mode: "full",
    scenarioId: "extended-scheduler-barriers",
    protocol,
    status,
    selectedBranch: "B",
    uiCapabilities: {},
  }), ["PUBLIC_DEPLOY_CONTROL_HIDDEN", "PUBLIC_CUSTOMIZE_VALUE_CONTROL_HIDDEN"]);
  assert.deepEqual(protocolCapabilityBlockers({ mode: "off", scenarioId: "rapid-jyutping", protocol }), [
    "PRODUCT_PRIVATE_PROTOCOL_PRESENT",
  ]);
  assert.deepEqual(protocolCapabilityBlockers({
    mode: "full",
    scenarioId: "learned-row",
    protocol,
    status,
    uiCapabilities: {},
  }), ["BACKGROUND_CAUSALITY_UNPROVED", "BROWSER_LIFECYCLE_PROTOCOL_CONTINUITY_UNPROVED"]);
});

test("post-window protocol health rejects invalidations, observer failures, missing return identity, and slow callbacks", () => {
  const healthy = {
    status: {
      valid: true,
      queueDepth: 0,
      pendingFanoutActions: 0,
      pendingTerminalActions: 0,
      mainObserverCallbackCount: 1,
      mainObserverCallbackCapacity: 8,
      mainObserverCallbackOverflowCount: 0,
    },
    invalidations: [],
    mainObserverCallbacks: [{
      callbackId: "web06-main-observer-00000001",
      sequenceId: 1,
      operation: "capture",
      startedAt: 1,
      finishedAt: 1.25,
      durationMs: 0.25,
    }],
    mainObserverCallbacksMs: [0.25],
    actions: [{
      returnedIdentity: { actionId: "wire" },
      worker: { observerFailures: [], collectorSpans: [{ startedAt: 1, finishedAt: 1.5 }] },
    }],
  };
  assert.deepEqual(protocolHealthBlockers(healthy, { requireCallbackLedger: true }), []);
  assert.deepEqual(protocolHealthBlockers({
    ...healthy,
    status: { ...healthy.status, valid: false },
    invalidations: [{ code: "RETURNED_IDENTITY_MISMATCH" }],
    actions: [{ worker: { observerFailures: ["callback threw"], collectorSpans: [{ startedAt: 1, finishedAt: 6 }] } }],
    mainObserverCallbacks: [{
      callbackId: "web06-main-observer-00000001",
      sequenceId: 1,
      operation: "capture",
      startedAt: 1,
      finishedAt: 6,
      durationMs: 5,
    }],
    mainObserverCallbacksMs: [5],
  }, { requireCallbackLedger: true }), [
    "PRIVATE_PROTOCOL_INVALID",
    "PRIVATE_PROTOCOL_INVALIDATIONS",
    "MAIN_OBSERVER_CALLBACK_CEILING",
    "RETURNED_WIRE_IDENTITY_MISSING",
    "WORKER_OBSERVER_FAILURE",
    "COLLECTOR_CALLBACK_CEILING",
  ]);
  assert.deepEqual(protocolHealthBlockers({ ...healthy, mainObserverCallbacksMs: undefined }, {
    requireCallbackLedger: true,
  }), ["MAIN_OBSERVER_CALLBACK_LEDGER_MISSING"]);
});

test("common endpoint resolution permits only the earliest two-step append supersession", () => {
  const frozen = expectedCommonSamples("fair-peer-short");
  assert.deepEqual(frozen.map(({ stepId, expectedInput }) => [stepId, expectedInput]), [
    ["peer-short-1", "n"],
    ["peer-short-2", "ni"],
    ["peer-short-commit", ""],
  ]);
  const events = [
    { eventSequenceId: 1, normalizedEventAt: 100 },
    { eventSequenceId: 3, normalizedEventAt: 160 },
    { eventSequenceId: 5, normalizedEventAt: 220 },
  ];
  const dom = (stepId, input, observedAt) => {
    const domObserved = {
      input,
      renderedInput: input,
      logicalInputProjection: input.replaceAll(" ", ""),
      candidates: input ? [{ label: "1.", text: "你", comment: "", source: "" }] : [],
      pageShape: {
        previousDisabled: true,
        nextDisabled: true,
        highlightedIndex: input ? 0 : -1,
        visibleCount: input ? 1 : 0,
      },
      textareaValue: "",
      selectionStart: 0,
      selectionEnd: 0,
    };
    return {
      stepId,
      observedAt,
      stableDoubleRaf: true,
      firstDomObserved: structuredClone(domObserved),
      domObserved,
    };
  };
  const resolved = resolveCommonSamples({
    scenarioId: "fair-peer-short",
    events,
    snapshots: [dom("peer-short-2", "ni", 168), dom("peer-short-commit", "", 228)],
  });
  assert.deepEqual(
    { outcome: resolved[0].outcome, target: resolved[0].supersededByStepId, observedAt: resolved[0].observedAt },
    { outcome: "superseded", target: "peer-short-2", observedAt: 168 },
  );
  assert.equal(resolved[1].outcome, "painted");
  assert.equal(resolved[2].outcome, "terminal");

  const unstable = resolveCommonSamples({
    scenarioId: "fair-peer-short",
    events,
    snapshots: [
      { ...dom("peer-short-1", "n", 108), stableDoubleRaf: false },
      dom("stale-other-step", "n", 109),
      dom("peer-short-2", "ni", 168),
      dom("peer-short-commit", "", 228),
    ],
  });
  assert.equal(unstable[0].outcome, "superseded");

  const left = { commonSamples: [{ stepId: "one", sampleKind: "covering", outcome: "painted", domFingerprintSha256: "a".repeat(64) }] };
  const right = { commonSamples: [{ stepId: "one", sampleKind: "covering", outcome: "painted", domFingerprintSha256: "b".repeat(64) }] };
  assert.notEqual(commonEndpointSequenceDigest(left), commonEndpointSequenceDigest(right));
});

test("rendered input bytes remain exact while the reviewed logical projection removes only ASCII spaces", () => {
  assert.equal(projectWeb06RenderedInput("ngo hai g"), "ngohaig");
  assert.equal(projectWeb06RenderedInput("ng ohai g"), "ngohaig");
  assert.notEqual(
    web06DomFingerprintDigest({ renderedInput: "ngo hai g", logicalInputProjection: "ngohaig" }),
    web06DomFingerprintDigest({ renderedInput: "ng ohai g", logicalInputProjection: "ngohaig" }),
  );
  assert.equal(projectWeb06RenderedInput("ngo\thai\tg"), "ngo\thai\tg");
});

test("private adapter preserves non-lossy wire identity and normalizes only public args", () => {
  const eventIdentity = {
    protocolVersion: "web06-private-v1",
    eventId: "web06-event-00000011",
    eventSequenceId: 11,
    type: "keydown",
    key: "n",
    code: "KeyN",
    timeStamp: 100,
    eventDeliveredAt: 101,
    classification: "mapped-action(s)",
    reason: "printable-key",
    mappedActionCount: 1,
    compositionEpochId: 7,
    supersessionSubRunId: 2,
    repeat: false,
    ctrlKey: false,
    metaKey: false,
    altKey: false,
    shiftKey: false,
  };
  const actionIdentity = {
    protocolVersion: "web06-private-v1",
    actionId: "web06-action-00000021",
    sequenceId: 21,
    eventId: eventIdentity.eventId,
    eventSequenceId: 11,
    eventActionIndex: 0,
    compositionEpochId: 7,
    supersessionSubRunId: 2,
    actionClass: "native-key",
    supersedable: true,
    boundary: "none",
    rawInputSequence: ["n"],
    originKind: "dom-event",
    originReason: "printable-key",
    actionEnqueuedAt: 102,
    mainQueueDepthAtEnqueue: 0,
    workerSentAt: 103,
    workerDispatchDepth: 1,
  };
  const fingerprint = {
    input: "n",
    page: 0,
    isLastPage: false,
    highlightedIndex: 0,
    candidates: [{ label: "1.", text: "你", comment: "", source: "table" }],
    status: null,
    textareaValue: "",
    selectionStart: 0,
    selectionEnd: 0,
  };
  const receipt = adaptPrivateProtocolReceipt({
    metadata: {
      scenarioId: "fair-peer-short",
      mode: "BASE_MINIMAL",
      source: { commit: COMMIT, treeState: "clean", artifactSha256: HASH },
      roundId: "round-1",
      attemptId: "attempt-1",
    },
    protocolWindow: { receiptWindowStartEventSequenceId: 11, receiptWindowStartActionSequenceId: 21 },
    wireEvents: [{ identity: eventIdentity, mappedActions: [], linkedActionIds: [actionIdentity.actionId] }],
    wireActions: [{
      identity: actionIdentity,
      returnedIdentity: structuredClone(actionIdentity),
      name: "processKey",
      args: ["{n}"],
      mainResponseReceivedAt: 108,
      responseMappingStartedAt: 108.1,
      responseMappingFinishedAt: 108.2,
      worker: {
        workerMessageReceivedAt: 103.1,
        workerActionStartedAt: 103.2,
        workerFinishedAt: 107,
        runtimeSpans: [], adapterSpans: [], persistenceSpans: [], collectorSpans: [], observerFailures: [],
      },
      presentation: {
        identity: actionIdentity,
        outcome: "painted",
        stateUpdateScheduledAt: 108.3,
        stateCommittedAt: 108.4,
        firstRafAt: 109,
        terminalObservedAt: 110,
        presentationExpected: fingerprint,
        domObserved: fingerprint,
        presentationDigest: "opaque-private-digest",
      },
    }],
    driverEvents: [{ requestedDriverDispatchAt: 99, actualDriverDispatchAt: 99.5 }],
    commonSurface: {
      eventClockProbe: { beforeDispatchAt: 1, eventTimestamp: 1.1, afterDispatchAt: 1.2 },
      calibration: {},
      initialDomObserved: { textareaValue: "" },
    },
  });
  assert.equal(receipt.events[0].eventSequenceId, 1);
  assert.equal(receipt.events[0].wireEventSequenceId, 11);
  assert.deepEqual(receipt.events[0].wireIdentity, eventIdentity);
  assert.equal(receipt.actions[0].sequenceId, 1);
  assert.equal(receipt.actions[0].wireSequenceId, 21);
  assert.deepEqual(receipt.actions[0].wireIdentity, actionIdentity);
  assert.deepEqual(receipt.actions[0].returnedWireIdentity, actionIdentity);
  assert.deepEqual(receipt.actions[0].wireArgs, ["{n}"]);
  assert.deepEqual(receipt.actions[0].args, ["{n}"]);
  assert.equal(receipt.actions[0].engineRaw, undefined);
});

test("private adapter takes persistence stress deadlines only from the frozen action contract", () => {
  const expected = expandScenarioExpectedTimeline("fifo-pressure-barriers");
  const userdbHash = expected.actions.find((action) => action.kind === "importUserdb")?.args[0]
    .replace("sha256:", "");
  const wireActions = expected.actions.map((action) => {
    const identity = {
      protocolVersion: "web06-private-v1",
      actionId: `web06-action-${String(action.sequenceId).padStart(8, "0")}`,
      sequenceId: action.sequenceId,
      ...(action.eventSequenceId === undefined ? {} : {
        eventSequenceId: action.eventSequenceId,
        eventId: `web06-event-${String(action.eventSequenceId).padStart(8, "0")}`,
      }),
      ...(action.causedBySequenceId === undefined ? {} : {
        causedBySequenceId: action.causedBySequenceId,
        causedByActionId: `web06-action-${String(action.causedBySequenceId).padStart(8, "0")}`,
      }),
      ...(action.causedByEventSequenceId === undefined ? {} : {
        causedByEventSequenceId: action.causedByEventSequenceId,
        causedByEventId: `web06-event-${String(action.causedByEventSequenceId).padStart(8, "0")}`,
      }),
      eventActionIndex: 0,
      compositionEpochId: 1,
      supersessionSubRunId: 1,
      actionClass: action.classification,
      supersedable: action.supersedable,
      boundary: action.boundary,
      rawInputSequence: action.rawInputSequence ?? [],
      originKind: action.originKind,
      originReason: action.originReason,
      actionEnqueuedAt: 100 + action.sequenceId,
      mainQueueDepthAtEnqueue: 0,
      workerSentAt: 101 + action.sequenceId,
      workerDispatchDepth: 1,
    };
    return {
      identity,
      returnedIdentity: structuredClone(identity),
      name: action.kind,
      args: action.kind === "importUserdb" ? ["<web06-redacted:userdb-text>"] : structuredClone(action.args),
      worker: {},
    };
  });
  const wireEvents = expected.events.map((event) => ({
    identity: {
      protocolVersion: "web06-private-v1",
      eventId: `web06-event-${String(event.eventSequenceId).padStart(8, "0")}`,
      eventSequenceId: event.eventSequenceId,
      type: event.type,
      key: event.key,
      code: event.code,
      timeStamp: 10 + event.eventSequenceId,
      eventDeliveredAt: 10.1 + event.eventSequenceId,
      classification: event.classification,
      reason: event.reason,
      mappedActionCount: event.mappedActionIds.length,
      compositionEpochId: 1,
      supersessionSubRunId: 1,
      repeat: false,
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      shiftKey: false,
    },
    linkedActionIds: event.mappedActionIds.map((actionId) =>
      `web06-action-${String(Number(actionId.slice(1))).padStart(8, "0")}`),
  }));
  const receipt = adaptPrivateProtocolReceipt({
    metadata: { scenarioId: "fifo-pressure-barriers", mode: "BASE_MINIMAL", source: {},
      roundId: "round-1", attemptId: "attempt-1" },
    protocolWindow: { receiptWindowStartEventSequenceId: 1, receiptWindowStartActionSequenceId: 1 },
    wireEvents,
    wireActions,
    driverEvents: expected.events.map(() => ({})),
    commonSurface: { initialDomObserved: { textareaValue: "" }, eventClockProbe: {}, calibration: {} },
    argumentCommitments: {
      "fifo-userdb-import": { importUserdb: { userdbTextSha256: userdbHash } },
    },
  });
  const persistence = receipt.actions.find((action) => action.stepId === "fifo-userdb-import");
  assert.equal(persistence.stressDeadline, true);
  assert.equal(receipt.actions.find((action) => action.stepId === "fifo-userdb-later-1").stressDeadline, false);
  const forgedWire = structuredClone(wireActions);
  forgedWire.find((action) => action.name === "importUserdb").stressDeadline = false;
  const forgedReceipt = adaptPrivateProtocolReceipt({
    metadata: { scenarioId: "fifo-pressure-barriers", mode: "BASE_MINIMAL", source: {},
      roundId: "round-1", attemptId: "attempt-1" },
    protocolWindow: { receiptWindowStartEventSequenceId: 1, receiptWindowStartActionSequenceId: 1 },
    wireEvents,
    wireActions: forgedWire,
    driverEvents: expected.events.map(() => ({})),
    commonSurface: { initialDomObserved: { textareaValue: "" }, eventClockProbe: {}, calibration: {} },
    argumentCommitments: {
      "fifo-userdb-import": { importUserdb: { userdbTextSha256: userdbHash } },
    },
  });
  assert.equal(forgedReceipt.actions.find((action) => action.stepId === "fifo-userdb-import").stressDeadline, true);
});

test("sentinel is external-only and exposes no debug action invoker", () => {
  const source = String(installWeb06Sentinel);
  assert.doesNotMatch(source, /dispatchAction|invokeAction|__YUNE_WEB06__(?!_SENTINEL)/);
  assert.match(source, /MutationObserver/);
  assert.match(source, /PerformanceObserver/);
  assert.match(source, /requestAnimationFrame/);
  assert.match(source, /takeRecords\(\)/, "final snapshot drains pending Long Task records");
  assert.match(source, /entry\.startTime < state\.captureEpochStartedAt/,
    "late buffered setup entries are excluded by capture epoch");
  assert.match(source, /callback\.eventSequenceId = eventIndex \+ 1/,
    "precursor callbacks are backfilled to their eventual event owner");
  assert.match(source, /sentinelTotalPerEventMs: state\.events\.map[\s\S]*state\.callbackLedger/,
    "published per-event totals are derived from the retained callback ledger");
});

test("Long Task capture epoch includes pending final entries and excludes stale buffered setup entries", () => {
  const drainedAtFinalization = [
    { startTime: 90, duration: 60 },
    { startTime: 101, duration: 55 },
    { startTime: 140, duration: 50 },
  ];
  assert.deepEqual(selectWeb06LongTaskEntries(drainedAtFinalization, 100), [
    { startTime: 101, durationMs: 55 },
    { startTime: 140, durationMs: 50 },
  ]);
});

test("learned reload merge preserves two clock realms even when timestamps overlap", () => {
  const expected = expandScenarioExpectedTimeline("learned-row");
  const boundary = expected.events.find((event) => event.stepId === "learned-reload-boundary");
  const lifecycleMarker = {
    ...boundary,
    pageInstanceId: "page-1",
    eventTimestamp: 200,
    normalizedEventAt: 200,
    sentinelObservedAt: 200,
    originOwner: "harness-browser-lifecycle",
  };
  const sentinelSegment = (pageInstanceId, frozenEvents) => ({
    events: frozenEvents.map((event, index) => ({
      ...event,
      pageInstanceId,
      eventTimestamp: 100 + index,
      normalizedEventAt: 100 + index,
      sentinelObservedAt: 100.1 + index,
    })),
    auxiliaryEvents: [], unmatchedEvents: [], snapshots: [{ pageInstanceId, observedAt: 110 }],
    interactionWindows: [{ pageInstanceId, startedAt: 90, endedAt: 130 }],
    idleControlWindows: [], interactionFrameWindows: [{ pageInstanceId, timestamps: [90, 106, 122] }],
    interactionFrameTimestamps: [90, 106, 122], interactionFrameIntervalsMs: [16, 16],
    longTaskObserver: { pageInstanceId, supported: true, installedAt: 80 },
    longTasks: [{ pageInstanceId, startTime: 100, durationMs: 50,
      overlapsInteractionWindow: true, overlapsIdleControl: false }],
    focusVisibilitySamples: [{ pageInstanceId, recordedAt: 100, focused: true, visibilityState: "visible" }],
    assetsRequestedDuringWindow: [], callbackLedger: [{ pageInstanceId, kind: "event", eventSequenceId: 1,
      startedAt: 100, finishedAt: 100.25, durationMs: 0.25 }], callbackLedgerCapacity: 10,
    callbackLedgerOverflowCount: 0, sentinelOverflowCounts: {}, sentinelCallbacksMs: [0.25],
    unattributedInWindowCallbacksMs: [],
    sentinelTotalPerEventMs: frozenEvents.map((_event, index) => index === 0 ? 0.25 : 0),
    sentinelTotalPerWindowMs: [0.25],
    sentinelAccountedCallbackCount: 1,
  });
  const preEvents = expected.events.slice(0, boundary.eventSequenceId - 1);
  const postEvents = expected.events.slice(boundary.eventSequenceId);
  const sentinel = mergeWeb06LearnedSentinelSegments(
    sentinelSegment("page-1", preEvents),
    sentinelSegment("page-2", postEvents.map((event, index) => ({ ...event, eventSequenceId: index + 1 }))),
    lifecycleMarker,
  );
  assert.equal(sentinel.events.length, 25);
  assert.deepEqual(sentinel.events.slice(17, 21).map((event) =>
    [event.pageInstanceId, event.eventSequenceId, event.stepId]), [
    ["page-1", 18, "learned-commit"],
    ["page-1", 19, "learned-reload-boundary"],
    ["page-2", 20, "learned-probe-1"],
    ["page-2", 21, "learned-probe-1"],
  ]);
  assert.equal(sentinel.sentinelTotalPerEventMs[18], 0);
  assert.deepEqual(sentinel.callbackLedger.map((callback) =>
    [callback.pageInstanceId, callback.eventSequenceId]), [
    ["page-1", 1],
    ["page-2", 20],
  ]);
  for (const callback of sentinel.callbackLedger) {
    assert.equal(sentinel.events.filter((event) =>
      event.pageInstanceId === callback.pageInstanceId
      && event.eventSequenceId === callback.eventSequenceId).length, 1,
    "every learned callback owner resolves to exactly one merged event");
  }
  assert.deepEqual(sentinelLedgerIntegrityErrors(sentinel), []);
  assert.deepEqual(independentlyValidateRawSentinelIntegrity(sentinel), { pass: true, errors: [] });
  const duplicateOwner = structuredClone(sentinel);
  duplicateOwner.events.push(structuredClone(duplicateOwner.events[0]));
  duplicateOwner.sentinelTotalPerEventMs.push(0.25);
  assert.ok(sentinelLedgerIntegrityErrors(duplicateOwner)
    .includes("SENTINEL_EVENT_OWNER_IDENTITY_INVALID"));
  assert.ok(independentlyValidateRawSentinelIntegrity(duplicateOwner).errors
    .includes("raw-sentinel-event-owner-identity"));
  assert.deepEqual(sentinel.longTaskObserver.segments.map((segment) => segment.pageInstanceId), ["page-1", "page-2"]);
  assert.equal(sentinel.callbackLedgerCapacity, 20);
  assert.throws(() => mergeWeb06LearnedSentinelSegments(
    sentinelSegment("page-1", preEvents),
    sentinelSegment("page-1", postEvents.map((event, index) => ({ ...event, eventSequenceId: index + 1 }))),
    lifecycleMarker,
  ),
    /WEB06_LEARNED_SENTINEL_REALM_IDENTITY/);

  const protocolSegment = (pageInstanceId, eventId, actionId) => ({
    header: { pageInstanceId },
    status: { receiptWindowStartEventSequenceId: 1, receiptWindowStartActionSequenceId: 1,
      mainObserverCallbackCount: 1, mainObserverCallbackCapacity: 10,
      mainObserverCallbackOverflowCount: 0 },
    events: [{ identity: { eventSequenceId: eventId } }],
    actions: [{ identity: { sequenceId: actionId } }],
    invalidations: [], mainObserverCallbacksMs: [0.1],
    mainObserverCallbacks: [{ callbackId: "web06-main-observer-00000001", sequenceId: 1,
      operation: "event", startedAt: 100, finishedAt: 100.1, durationMs: 0.1 }],
  });
  const protocol = mergeWeb06LearnedProtocolSegments(
    protocolSegment("page-1", 1, 1), protocolSegment("page-2", 1, 1), lifecycleMarker,
  );
  assert.deepEqual(protocol.events.map((event) => event.web06PageInstanceId), ["page-1", "page-2"]);
  assert.deepEqual(protocol.mainObserverCallbacks.map((callback) => callback.pageInstanceId), ["page-1", "page-2"]);
  assert.equal(protocol.status.mainObserverCallbackCount, 2);
  assert.deepEqual(protocol.protocolWindowSegments.map((segment) => [
    segment.pageInstanceId,
    segment.receiptWindowStartEventSequenceId,
    segment.receiptWindowStartActionSequenceId,
  ]), [["page-1", 1, 1], ["page-2", 1, 1]]);
  const duplicateProtocol = protocolSegment("page-1", 1, 1);
  duplicateProtocol.events.push(structuredClone(duplicateProtocol.events[0]));
  assert.throws(() => mergeWeb06LearnedProtocolSegments(
    duplicateProtocol, protocolSegment("page-2", 1, 1), lifecycleMarker,
  ), /WEB06_LEARNED_PROTOCOL_SEQUENCE_REUSE/);

  const wireSegment = (pageInstanceId, frozenEvents, frozenActions, eventOffset, actionOffset) => {
    const actions = frozenActions.map((action) => {
      const sequenceId = action.sequenceId - actionOffset;
      const eventSequenceId = action.eventSequenceId === undefined ? undefined : action.eventSequenceId - eventOffset;
      const causedBySequenceId = action.causedBySequenceId === undefined
        ? undefined : action.causedBySequenceId - actionOffset;
      const causedByEventSequenceId = action.causedByEventSequenceId === undefined
        ? undefined : action.causedByEventSequenceId - eventOffset;
      const identity = {
        protocolVersion: "web06-private-v1",
        actionId: `web06-action-${String(sequenceId).padStart(8, "0")}`,
        sequenceId,
        ...(eventSequenceId === undefined ? {} : {
          eventSequenceId,
          eventId: `web06-event-${String(eventSequenceId).padStart(8, "0")}`,
        }),
        ...(causedBySequenceId === undefined ? {} : {
          causedBySequenceId,
          causedByActionId: `web06-action-${String(causedBySequenceId).padStart(8, "0")}`,
        }),
        ...(causedByEventSequenceId === undefined ? {} : {
          causedByEventSequenceId,
          causedByEventId: `web06-event-${String(causedByEventSequenceId).padStart(8, "0")}`,
        }),
        eventActionIndex: 0,
        compositionEpochId: 1,
        supersessionSubRunId: 1,
        actionClass: action.classification,
        supersedable: action.supersedable,
        boundary: action.boundary,
        rawInputSequence: action.rawInputSequence ?? [],
        originKind: action.originKind,
        originReason: action.originReason,
        actionEnqueuedAt: 102 + sequenceId,
        mainQueueDepthAtEnqueue: 0,
        workerSentAt: 103 + sequenceId,
        workerDispatchDepth: 1,
      };
      return {
        identity,
        returnedIdentity: structuredClone(identity),
        name: action.kind,
        args: structuredClone(action.args),
        worker: {},
      };
    });
    return {
    header: { pageInstanceId },
    status: { receiptWindowStartEventSequenceId: 1, receiptWindowStartActionSequenceId: 1,
      mainObserverCallbackCount: 0, mainObserverCallbackCapacity: 10,
      mainObserverCallbackOverflowCount: 0 },
    events: frozenEvents.map((event, index) => ({
      identity: {
        protocolVersion: "web06-private-v1",
        eventId: `web06-event-${String(index + 1).padStart(8, "0")}`,
        eventSequenceId: index + 1,
        type: event.type,
        key: event.key,
        code: event.code,
        timeStamp: 100 + index,
        eventDeliveredAt: 100.1 + index,
        classification: event.classification,
        reason: event.reason,
        mappedActionCount: 0,
        compositionEpochId: 1,
        supersessionSubRunId: 1,
        repeat: false,
        ctrlKey: false,
        metaKey: false,
        altKey: false,
        shiftKey: false,
      },
      linkedActionIds: actions.filter((action) => action.identity.eventSequenceId === index + 1)
        .map((action) => action.identity.actionId),
    })),
    actions,
    invalidations: [],
    mainObserverCallbacksMs: [],
    mainObserverCallbacks: [],
    };
  };
  const preActions = expected.actions.filter((action) =>
    (action.eventSequenceId ?? action.causedByEventSequenceId) < boundary.eventSequenceId);
  const postActions = expected.actions.filter((action) =>
    (action.eventSequenceId ?? action.causedByEventSequenceId) > boundary.eventSequenceId);
  const mergedProtocol = mergeWeb06LearnedProtocolSegments(
    wireSegment("page-1", preEvents, preActions, 0, 0),
    wireSegment("page-2", postEvents, postActions, boundary.eventSequenceId, preActions.length),
    lifecycleMarker,
  );
  const idleFrameSegments = [
    { pageInstanceId: "page-1", intervalsMs: Array(120).fill(16) },
    { pageInstanceId: "page-2", intervalsMs: Array(120).fill(16) },
  ];
  const receipt = adaptPrivateProtocolReceipt({
    metadata: {
      scenarioId: "learned-row",
      scenarioRunId: "learned-row",
      schemaId: "jyut6ping3",
      mode: "BASE_MINIMAL",
      source: {
        commit: COMMIT, tree: TREE, treeState: "clean", archiveSha256: ARCHIVE_HASH,
        buildInfoSha256: HASH, artifactSha256: HASH, artifactResponseGuardSha256: HASH,
        artifactResponseGuardSummarySha256: HASH, identityManifestSha256: HASH,
        runnerSourceManifestSha256: HASH, runnerToolingManifestSha256: HASH,
        runnerSourceObservationSha256: HASH, runnerSourcePostObservationSha256: HASH,
        observedEnvironmentSha256: HASH, collectorContractSha256: HASH, scenarioIdsSha256: HASH,
        selectedBranch: "NONE", disposition: "SOURCE_CURRENT_BASELINE",
        environmentManifestSha256: HASH, environmentId: "learned-test",
      },
      roundId: "round-1",
      attemptId: "attempt-1",
      measurementStarted: true,
      measurementCompleted: true,
    },
    protocolWindow: { receiptWindowStartEventSequenceId: 1, receiptWindowStartActionSequenceId: 1 },
    protocolWindowSegments: mergedProtocol.protocolWindowSegments,
    wireEvents: mergedProtocol.events,
    wireActions: mergedProtocol.actions,
    externalLifecycleEvents: mergedProtocol.externalLifecycleEvents,
    driverEvents: expected.events.filter((event) => event.type !== "browser-lifecycle")
      .map(() => ({ requestedDriverDispatchAt: undefined, actualDriverDispatchAt: undefined })),
    commonSurface: {
      ...sentinel,
      initialDomObserved: { textareaValue: "" },
      eventClockProbe: { beforeDispatchAt: 0, eventTimestamp: 0, afterDispatchAt: 0 },
      eventClockSegments: {
        preReload: { beforeDispatchAt: 0, eventTimestamp: 0, afterDispatchAt: 0 },
        postReload: { beforeDispatchAt: 0, eventTimestamp: 0, afterDispatchAt: 0 },
      },
      calibration: { driver: {}, worker: {} },
      calibrationSegments: { preReload: { driver: {}, worker: {} }, postReload: { driver: {}, worker: {} } },
      cadenceGaps: [],
      idleFrameIntervalsMs: idleFrameSegments.flatMap((segment) => segment.intervalsMs),
      idleFrameSegments,
      measurementProtocolBlockers: [],
      burstRecoveries: [],
      pressureProofs: [],
      lifecycleContinuity: {
        browserLifecycleEventCount: 1,
        measurementId: "measurement-1",
        pre: {
          phase: "pre-reload",
          protocolVersion: "web06-private-v1",
          measurementId: "measurement-1",
          continuityNonce: "nonce",
          pageInstanceId: "page-1",
          terminal: { persistenceCompleted: true },
          queueIdle: true,
          allActionsCompleted: true,
          storagePayloadKeys: ["measurementId", "continuityNonce"],
        },
        post: {
          phase: "post-reload-bound",
          protocolVersion: "web06-private-v1",
          measurementId: "measurement-1",
          continuityNonce: "nonce",
          pageInstanceId: "page-2",
          storageRemoved: true,
          oneShot: true,
          requiresFreshDriverPageCalibration: true,
          requiresFreshWorkerCalibration: true,
        },
      },
      sentinelOverflowCounts: {},
    },
  });
  assert.equal(receipt.events.length, 25);
  assert.equal(receipt.actions.length, 13);
  assert.deepEqual(receipt.events[18], lifecycleMarker);
  assert.deepEqual([
    receipt.events[19].eventSequenceId,
    receipt.events[19].wireEventSequenceId,
    receipt.events[19].wireEventId,
    receipt.events[19].pageInstanceId,
  ], [20, 1, "web06-event-00000001", "page-2"]);
  assert.deepEqual([
    receipt.actions[10].sequenceId,
    receipt.actions[10].wireSequenceId,
    receipt.actions[10].wireActionId,
    receipt.actions[10].eventSequenceId,
    receipt.actions[10].pageInstanceId,
  ], [11, 1, "web06-action-00000001", 20, "page-2"]);
  const runnerSummary = buildRoundEvidenceSummary(receipt, { surface: "internal" });
  const independentSummary = independentlyRecomputeRoundSummary(receipt, "internal");
  assert.equal(runnerSummary.counts.events, 25);
  assert.equal(independentSummary.counts.events, 25);
  assert.equal(runnerSummary.behaviorErrorCodes.some((error) => error === "EVENT_REORDERED:19"
    || error === "EVENT_IDENTITY:19" || error === "EVENT_CLASSIFICATION:19"), false);
  assert.equal(independentSummary.behaviorErrorCodes.includes("EVENT_FROZEN_IDENTITY:19"), false);
  assert.deepEqual(runnerSummary.behaviorErrorCodes.filter((error) => error.startsWith("WIRE_")), []);
  assert.deepEqual(independentSummary.behaviorErrorCodes.filter((error) => error.startsWith("WIRE_")), []);
  assert.equal(runnerSummary.setupErrorCodes.some((error) => error.startsWith("SETUP_LEARNED_IDLE_")), false);
  assert.equal(independentSummary.setupErrorCodes.some((error) => error.startsWith("SETUP_LEARNED_IDLE_")), false);

  for (const mutate of [
    (candidate) => { candidate.lifecycleContinuity.post.continuityNonce = "wrong-nonce"; },
    (candidate) => { candidate.lifecycleContinuity.post.measurementId = "wrong-measurement"; },
    (candidate) => { candidate.lifecycleContinuity.post.phase = "wrong-phase"; },
    (candidate) => { candidate.lifecycleContinuity.pre.terminal.persistenceCompleted = false; },
  ]) {
    const brokenContinuity = structuredClone(receipt);
    mutate(brokenContinuity);
    const runnerContinuityErrors = buildRoundEvidenceSummary(
      brokenContinuity,
      { surface: "internal" },
    ).behaviorErrorCodes;
    const independentContinuityErrors = independentlyRecomputeRoundSummary(
      brokenContinuity,
      "internal",
    ).behaviorErrorCodes;
    assert.ok(runnerContinuityErrors.includes("LEARNED_REAL_RELOAD_CONTINUITY_INVALID"));
    assert.ok(independentContinuityErrors.includes("LEARNED_REAL_RELOAD_CONTINUITY_INVALID"));
    assert.deepEqual(
      independentContinuityErrors.filter((error) => error.includes("PERSISTENCE_CONTINUITY")),
      runnerContinuityErrors.filter((error) => error.includes("PERSISTENCE_CONTINUITY")),
    );
  }

  const maskedIdleRegression = structuredClone(receipt);
  maskedIdleRegression.idleFrameSegments[0].intervalsMs = Array(120).fill(14);
  maskedIdleRegression.idleFrameSegments[1].intervalsMs = Array(120).fill(16);
  maskedIdleRegression.idleFrameIntervalsMs = maskedIdleRegression.idleFrameSegments
    .flatMap((segment) => segment.intervalsMs);
  assert.ok(buildRoundEvidenceSummary(maskedIdleRegression, { surface: "internal" }).setupErrorCodes
    .some((error) => error.startsWith("SETUP_LEARNED_IDLE_REFRESH_LANE:page-1:")));
  assert.ok(independentlyRecomputeRoundSummary(maskedIdleRegression, "internal").setupErrorCodes
    .some((error) => error.startsWith("SETUP_LEARNED_IDLE_REFRESH_LANE:page-1:")));
});
