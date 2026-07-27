import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

import {
  WEB06_BINDING_SCENARIO_ORDER,
  WEB06_COLLECTOR_CONTRACT_SHA256,
  WEB06_RUNNER_TOOLING_PATHS,
  buildCollectorOutput,
  buildIncompleteObserverModeProjection,
  parseWeb06CollectorEnvironment,
  writeSuiteAttestation,
} from "./web06-collector.mjs";
import {
  verifyCollectorOutput,
} from "./web06-independent-verifier.mjs";
import {
  WEB06_BEHAVIOR_PREDICATE_VERSION,
  WEB06_METRIC_CONTRACT_VERSION,
  WEB06_OBSERVER_COUNTERBALANCE,
  WEB06_SCENARIO_REGISTRY_VERSION,
  evaluateObserverOverhead,
  resolveScenarioRun,
} from "./web06-metric-contract.mjs";
import {
  createRunnerSourceManifest,
  sha256StableJson,
  verifyWeb06SuiteArtifactSet,
} from "./web06-suite-attestation.mjs";

const execFileAsync = promisify(execFile);
const SOURCE_REPOSITORY_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../../..",
);
const HASH_A = "a".repeat(64);
const HASH_B = "b".repeat(64);
const HASH_C = "c".repeat(64);

export const sha256 = (bytes) =>
  createHash("sha256").update(bytes).digest("hex");

export const digestJson = (value) =>
  sha256(Buffer.from(JSON.stringify(value), "utf8"));

export async function writeCanonicalJson(file, value) {
  const bytes = Buffer.from(`${JSON.stringify(value, null, 2)}\n`, "utf8");
  await writeFile(file, bytes);
  return Object.freeze({
    relativePath: path.basename(file),
    bytes: bytes.length,
    sha256: sha256(bytes),
    path: file,
  });
}

export async function artifactReference(file, relativePath = path.basename(file)) {
  const bytes = await readFile(file);
  return Object.freeze({
    relativePath,
    bytes: bytes.length,
    sha256: sha256(bytes),
  });
}

async function createCleanRunnerRepository(outerRoot) {
  const repositoryRoot = path.join(outerRoot, "runner");
  await mkdir(repositoryRoot);
  for (const relativePath of WEB06_RUNNER_TOOLING_PATHS) {
    const destination = path.join(repositoryRoot, ...relativePath.split("/"));
    await mkdir(path.dirname(destination), { recursive: true });
    await copyFile(
      path.join(SOURCE_REPOSITORY_ROOT, ...relativePath.split("/")),
      destination,
    );
  }
  const runGit = (...args) =>
    execFileAsync("git", args, { cwd: repositoryRoot, encoding: "utf8" });
  await runGit("init", "-q");
  await runGit("config", "user.email", "web06-fixture@example.invalid");
  await runGit("config", "user.name", "WEB06 fixture");
  await runGit("add", "--", ".");
  await runGit("commit", "-q", "--no-gpg-sign", "-m", "WEB06 fixture source");
  const sourceCommit = (await runGit("rev-parse", "HEAD")).stdout.trim();
  const sourceTree = (await runGit("rev-parse", "HEAD^{tree}")).stdout.trim();
  const runnerSource = await createRunnerSourceManifest(
    repositoryRoot,
    sourceCommit,
    sourceTree,
  );
  return { repositoryRoot, sourceCommit, sourceTree, runnerSource };
}

function identityRole({
  sourceCommit,
  sourceTree,
  selectedBranch,
  disposition,
}) {
  return {
    sourceCommit,
    sourceTree,
    sourceTreeState: "clean",
    archiveSha256: HASH_B,
    artifactManifestSha256: HASH_A,
    buildInfoSha256: HASH_A,
    selectedBranch,
    disposition,
  };
}

function identityManifest(roles) {
  return {
    version: "web06-target-identities-v1",
    metricContractVersion: WEB06_METRIC_CONTRACT_VERSION,
    scenarioRegistryVersion: WEB06_SCENARIO_REGISTRY_VERSION,
    behaviorPredicateVersion: WEB06_BEHAVIOR_PREDICATE_VERSION,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    roles,
  };
}

function runEnvironment() {
  const manifest = {
    version: "web06-run-environment-v1",
    toolchain: {
      rust: "rustc fixture",
      emscripten: "emcc fixture",
      node: "node fixture",
      npm: "npm fixture",
      playwright: "playwright fixture",
      chromium: "chromium fixture",
      chromiumExecutableSha256: HASH_C,
    },
    host: {
      os: "macOS",
      osVersion: "fixture",
      osBuildVersion: "fixture-build",
      arch: "arm64",
      cpuModel: "Apple fixture",
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
    ui: {
      pageSize: 6,
      aiEnabled: false,
      debugUi: "production-default",
      inspectorUi: "production-default",
    },
  };
  return {
    ...manifest,
    environmentId: digestJson(manifest),
  };
}

function observedEnvironment(environment) {
  const observation = {
    version: "web06-observed-environment-v1",
    toolchain: environment.toolchain,
    host: environment.host,
  };
  return {
    ...observation,
    observationSha256: digestJson(observation),
  };
}

function target({
  id,
  protocolMode,
  selectorPolicy,
  sourceCommit,
  sourceTree,
  origin,
}) {
  return {
    origin,
    sourceCommit,
    sourceTree,
    treeState: "clean",
    artifactSha256: HASH_A,
    archiveSha256: HASH_B,
    buildInfoSha256: HASH_A,
    protocolMode,
    selectorPolicy,
    artifactResponseGuard: {
      version: "web06-artifact-response-guard-v1",
      rootDocumentPath: "index.html",
      entries: [
        { path: "build-info.json", bytes: 10, sha256: HASH_A },
        {
          path: "public-artifact-manifest.json",
          bytes: 20,
          sha256: HASH_A,
        },
        { path: "index.html", bytes: 30, sha256: HASH_C },
      ],
    },
  };
}

function runnerSourceObservation(runnerSource) {
  const snapshot = {
    version: "web06-runner-source-observation-v1",
    sourceCommit: runnerSource.sourceCommit,
    sourceTree: runnerSource.sourceTree,
    sourceTreeState: runnerSource.sourceTreeState,
    toolingManifestSha256: runnerSource.toolingManifestSha256,
    files: runnerSource.tooling.files,
  };
  return {
    ...snapshot,
    observationSha256: digestJson(snapshot),
  };
}

function baseEnvironment({
  evidenceRoot,
  repositoryRoot,
  runnerSource,
  identity,
  environment,
  expectation,
  selectedBranch,
  disposition,
  runKind,
  runId,
  targets,
  targetOrder,
  scenarioRuns,
}) {
  return parseWeb06CollectorEnvironment(
    {
      YUNE_WEB06_EVIDENCE_ROOT: evidenceRoot,
      YUNE_WEB06_COLLECTOR_OUTPUT_PATH: path.join(
        evidenceRoot,
        "collector-output.json",
      ),
      YUNE_WEB06_INDEPENDENT_RECOMPUTE_PATH: path.join(
        evidenceRoot,
        "independent-recompute.json",
      ),
      YUNE_WEB06_SUITE_ATTESTATION_PATH: path.join(
        evidenceRoot,
        "suite-attestation.json",
      ),
      YUNE_WEB06_EXPECTATION: expectation,
      YUNE_WEB06_RUN_KIND: runKind,
      YUNE_WEB06_RUN_ID: runId,
      YUNE_WEB06_IDENTITY_MANIFEST_JSON: JSON.stringify(identity),
      YUNE_WEB06_RUNNER_SOURCE_JSON: JSON.stringify(runnerSource),
      YUNE_WEB06_RUN_ENVIRONMENT_JSON: JSON.stringify(environment),
      YUNE_WEB06_TARGETS_JSON: JSON.stringify(targets),
      YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(targetOrder),
      YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(scenarioRuns),
      YUNE_WEB06_BLOCKED_SCENARIOS_JSON: "[]",
      YUNE_WEB06_SELECTED_BRANCH: selectedBranch,
      YUNE_WEB06_DISPOSITION: disposition,
      YUNE_WEB06_PLAYWRIGHT_RETRIES: "0",
      YUNE_WEB06_PLAYWRIGHT_WORKERS: "1",
    },
    { repoRoot: repositoryRoot },
  );
}

function partialEnvelope({
  config,
  targetId,
  scenarioRunId,
  attemptId,
  attemptNumber,
  measurementStarted,
  failureCode,
  failureDimension,
  sourceObservation,
  environmentObservation,
}) {
  const run = resolveScenarioRun(scenarioRunId);
  return {
    version: "web06-raw-attempt-v1",
    target: structuredClone(config.targets[targetId]),
    expectation: config.expectation,
    scenarioRunId,
    scenarioId: run.scenarioId,
    schemaId: run.schema,
    attemptId,
    attemptNumber,
    identityManifestSha256: config.identityManifestSha256,
    runnerSourceManifestSha256: config.runnerSourceManifestSha256,
    scenarioIdsSha256: config.scenarioIdsSha256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    measurementStarted,
    measurementCompleted: false,
    runnerSourceBefore: structuredClone(sourceObservation),
    attemptSourceBefore: structuredClone(sourceObservation),
    observedEnvironment: structuredClone(environmentObservation),
    setupFailure: {
      name: "Error",
      message: `${failureCode}:fixture`,
    },
    browserFailure: {
      pageClosed: false,
      messageCode: failureCode,
    },
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
      failure: {
        code: failureCode,
        dimension: failureDimension,
      },
    },
  };
}

async function writeRaw(evidenceRoot, relativePath, envelope) {
  const destination = path.join(evidenceRoot, ...relativePath.split("/"));
  await mkdir(path.dirname(destination), { recursive: true });
  const bytes = Buffer.from(`${JSON.stringify(envelope)}\n`, "utf8");
  await writeFile(destination, bytes);
  return {
    relativePath,
    bytes: bytes.length,
    sha256: sha256(bytes),
  };
}

async function finalizeBundle({
  outerRoot,
  repositoryRoot,
  evidenceRoot,
  config,
  sourceObservation,
  scenarioResults,
  observerTriplets,
  observerEvaluation,
  verdict,
}) {
  const collectorPayload = buildCollectorOutput({
    config,
    scenarioResults,
    observerTriplets,
    observerEvaluation,
    runnerSourceBefore: sourceObservation,
    runnerSourceAfter: sourceObservation,
  });
  const collectorArtifact = await writeCanonicalJson(
    config.outputPaths.collector,
    collectorPayload,
  );
  const independentPayload = await verifyCollectorOutput({
    evidenceRoot,
    collectorOutputPath: collectorArtifact.path,
    repoRoot: repositoryRoot,
    verifyCurrentSource: false,
  });
  const independentArtifact = await writeCanonicalJson(
    config.outputPaths.independent,
    independentPayload,
  );
  const attestationArtifact = await writeSuiteAttestation({
    config,
    collectorOutputArtifact: collectorArtifact,
    independentRecomputeArtifact: independentArtifact,
    scenarioResults,
    observerTriplets,
    verdict,
    runnerSourceBefore: sourceObservation,
    runnerSourceAfter: sourceObservation,
  });
  const primaryRole =
    config.identityManifest.roles[
      ["FINAL", "PREVIEW"].includes(config.expectation) ? "FINAL" : "BASE"
    ];
  const expected = {
    expectation: config.expectation,
    disposition: config.disposition,
    selectedBranch: config.branch,
    sourceCommit: primaryRole.sourceCommit,
    sourceTree: primaryRole.sourceTree,
    sourceTreeState: primaryRole.sourceTreeState,
    archiveSha256: primaryRole.archiveSha256,
    artifactManifestSha256: primaryRole.artifactManifestSha256,
    buildInfoSha256: primaryRole.buildInfoSha256,
    identityManifest: config.identityManifest,
    identityManifestSha256: config.identityManifestSha256,
    collectorContractSha256: WEB06_COLLECTOR_CONTRACT_SHA256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
  };
  const bundle = {
    outerRoot,
    repositoryRoot,
    evidenceRoot,
    config,
    expected,
    sourceObservation,
    paths: {
      attestation: attestationArtifact.path,
      collector: collectorArtifact.path,
      independent: independentArtifact.path,
    },
    async verify(overrides = {}) {
      return verifyWeb06SuiteArtifactSet({
        attestationPath: attestationArtifact.path,
        evidenceRoot,
        expected,
        repoRoot: repositoryRoot,
        verifyCurrentSource: true,
        ...overrides,
      });
    },
    async cleanup() {
      await rm(outerRoot, { recursive: true, force: true });
    },
  };
  await bundle.verify();
  return bundle;
}

async function fixtureRoots(prefix) {
  const canonicalTemporaryRoot = await realpath(os.tmpdir());
  const outerRoot = await mkdtemp(
    path.join(canonicalTemporaryRoot, `${prefix}-`),
  );
  const source = await createCleanRunnerRepository(outerRoot);
  const evidenceRoot = path.join(outerRoot, "evidence");
  await mkdir(evidenceRoot);
  return { outerRoot, evidenceRoot, ...source };
}

export async function createBaselineSetupNoGoBundle() {
  const roots = await fixtureRoots("web06-artifact-set-baseline");
  try {
    const environment = runEnvironment();
    const identity = identityManifest({
      BASE: identityRole({
        sourceCommit: roots.sourceCommit,
        sourceTree: roots.sourceTree,
        selectedBranch: "NONE",
        disposition: "SOURCE_CURRENT_BASELINE",
      }),
    });
    const targets = {
      BASE_FULL: target({
        id: "BASE_FULL",
        protocolMode: "full",
        selectorPolicy: "explicit",
        sourceCommit: roots.sourceCommit,
        sourceTree: roots.sourceTree,
        origin: "http://127.0.0.1:4173/",
      }),
    };
    const config = baseEnvironment({
      ...roots,
      runnerSource: roots.runnerSource,
      identity,
      environment,
      expectation: "BASELINE",
      selectedBranch: "NONE",
      disposition: "SOURCE_CURRENT_BASELINE",
      runKind: "full",
      runId: "baseline-fixture",
      targets,
      targetOrder: ["BASE_FULL"],
      scenarioRuns: WEB06_BINDING_SCENARIO_ORDER,
    });
    const sourceObservation = runnerSourceObservation(roots.runnerSource);
    const environmentObservation = observedEnvironment(environment);
    const scenarioResults = [];
    for (const scenarioRunId of WEB06_BINDING_SCENARIO_ORDER) {
      const run = resolveScenarioRun(scenarioRunId);
      const attempts = [];
      for (let index = 0; index < 7; index += 1) {
        const attemptId = `attempt-${index + 1}`;
        const relativePath =
          `raw/${scenarioRunId}/${attemptId}/BASE_FULL.raw.json`;
        const envelope = partialEnvelope({
          config,
          targetId: "BASE_FULL",
          scenarioRunId,
          attemptId,
          attemptNumber: index + 1,
          measurementStarted: false,
          failureCode: "WEB06_SETUP_FAILURE",
          failureDimension: "setup",
          sourceObservation,
          environmentObservation,
        });
        const rawPacket = await writeRaw(
          roots.evidenceRoot,
          relativePath,
          envelope,
        );
        attempts.push({
          attemptId,
          measurementStarted: false,
          measurementCompleted: false,
          classification: "SETUP_INVALID",
          retainedMeasured: false,
          retainedLogicalRound: false,
          validForLatencyFrame: false,
          retainedHardRed: false,
          retryEligible: true,
          validRedObserved: false,
          rawPacket,
          runnerSummaries: {},
        });
      }
      scenarioResults.push({
        targetId: "BASE_FULL",
        scenarioRunId,
        scenarioId: run.scenarioId,
        schemaId: run.schema,
        measuredRoundCount: 0,
        validLatencyFrameRoundCount: 0,
        verdict: "SETUP_NO_GO",
        preservedHardRedAttemptIds: [],
        preservedHardRedObserved: false,
        attempts,
        runnerFiveRoundSummaries: {},
      });
    }
    return await finalizeBundle({
      ...roots,
      config,
      sourceObservation,
      scenarioResults,
      observerTriplets: [],
      verdict: "SETUP_NO_GO",
    });
  } catch (error) {
    await rm(roots.outerRoot, { recursive: true, force: true });
    throw error;
  }
}

export async function createObserverPartialBundle() {
  const roots = await fixtureRoots("web06-artifact-set-observer");
  try {
    const environment = runEnvironment();
    const roleFields = {
      sourceCommit: roots.sourceCommit,
      sourceTree: roots.sourceTree,
      selectedBranch: "NONE",
      disposition: "DIAGNOSTIC",
    };
    const identity = identityManifest({
      PRODUCT: identityRole(roleFields),
      BASE: identityRole(roleFields),
    });
    const targets = {
      PRODUCT: target({
        id: "PRODUCT",
        protocolMode: "off",
        selectorPolicy: "omitted",
        sourceCommit: roots.sourceCommit,
        sourceTree: roots.sourceTree,
        origin: "http://127.0.0.1:4173/",
      }),
      BASE_MINIMAL: target({
        id: "BASE_MINIMAL",
        protocolMode: "minimal",
        selectorPolicy: "explicit",
        sourceCommit: roots.sourceCommit,
        sourceTree: roots.sourceTree,
        origin: "http://127.0.0.1:4174/",
      }),
      BASE_FULL: target({
        id: "BASE_FULL",
        protocolMode: "full",
        selectorPolicy: "explicit",
        sourceCommit: roots.sourceCommit,
        sourceTree: roots.sourceTree,
        origin: "http://127.0.0.1:4175/",
      }),
    };
    const config = baseEnvironment({
      ...roots,
      runnerSource: roots.runnerSource,
      identity,
      environment,
      expectation: "OBSERVER",
      selectedBranch: "NONE",
      disposition: "DIAGNOSTIC",
      runKind: "observer-overhead",
      runId: "observer-fixture",
      targets,
      targetOrder: ["PRODUCT", "BASE_MINIMAL", "BASE_FULL"],
      scenarioRuns: ["rapid-long-jyutping"],
    });
    const sourceObservation = runnerSourceObservation(roots.runnerSource);
    const environmentObservation = observedEnvironment(environment);
    const observerTriplets = [];
    const modes = [
      ["product", "PRODUCT"],
      ["minimal", "BASE_MINIMAL"],
      ["full", "BASE_FULL"],
    ];
    for (let index = 0; index < 7; index += 1) {
      const attemptId = `triplet-attempt-${index + 1}`;
      const modeContextIds = modes.map(
        ([modeName]) => `${attemptId}-${modeName}-context`,
      );
      const triplet = {
        attemptId,
        valid: false,
        counterbalanceSlot: 1,
        freshContextId: modeContextIds.join("+"),
        modeContextIds,
        modeOrder: [...WEB06_OBSERVER_COUNTERBALANCE[1]],
        modeFixedBeforePageLoad: true,
      };
      for (const [modeName, targetId] of modes) {
        const behaviorRed = index === 0 && modeName === "minimal";
        const failureCode = behaviorRed
          ? "WEB06_ACTION_COMPLETION_COUNT"
          : "WEB06_SETUP_FAILURE";
        const relativePath =
          `raw/${attemptId}/${targetId}.raw.json`;
        const envelope = partialEnvelope({
          config,
          targetId,
          scenarioRunId: "rapid-long-jyutping",
          attemptId,
          attemptNumber: index + 1,
          measurementStarted: behaviorRed,
          failureCode,
          failureDimension: behaviorRed ? "behavior" : "setup",
          sourceObservation,
          environmentObservation,
        });
        const rawPacket = await writeRaw(
          roots.evidenceRoot,
          relativePath,
          envelope,
        );
        triplet[modeName] = buildIncompleteObserverModeProjection({
          rawPacket,
          measurementStarted: behaviorRed,
          behaviorRedObserved: behaviorRed,
        });
      }
      observerTriplets.push(triplet);
    }
    const observerEvaluation = evaluateObserverOverhead(observerTriplets);
    return await finalizeBundle({
      ...roots,
      config,
      sourceObservation,
      scenarioResults: [],
      observerTriplets,
      observerEvaluation,
      verdict: observerEvaluation.status,
    });
  } catch (error) {
    await rm(roots.outerRoot, { recursive: true, force: true });
    throw error;
  }
}

export function refreshSourceRoleBindings(attestation) {
  const {
    bindingsSha256: _bindingsSha256,
    ...bindings
  } = attestation.sourceArtifactRoles;
  attestation.sourceArtifactRoles.bindingsSha256 =
    sha256StableJson(bindings);
}
