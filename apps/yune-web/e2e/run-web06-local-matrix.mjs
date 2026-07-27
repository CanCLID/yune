import { spawn } from "node:child_process";
import { lstat, mkdir, rm, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { sha256, validateLocalBundle } from "./public-artifact-verifier.mjs";
import {
  extractCertifiedArchive,
  prepareOutputPaths,
  validateArchive,
} from "./run-public-web06-gate.mjs";
import {
  WEB06_BRANCH_B_SCENARIOS,
  WEB06_BINDING_SCENARIOS,
  createRunnerSourceManifest,
  readAndValidateSuiteAttestation,
  sha256Bytes,
  sha256StableJson,
} from "./web06-suite-attestation.mjs";
import {
  loadSealedArtifactSnapshot,
  sealedArtifactResponseGuard,
  startSealedArtifactServer,
} from "./web06-sealed-artifact-server.mjs";

const e2eRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(e2eRoot, "../../..");
const playwrightCli = path.join(e2eRoot, "node_modules", "@playwright", "test", "cli.js");
const configPath = path.join(e2eRoot, "playwright.web06.config.ts");
const specPath = path.join(e2eRoot, "yune-web06-smoothness.spec.ts");
const SAFE_RUN_ID = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;

const MATRIX = Object.freeze({
  OBSERVER: Object.freeze({
    roles: Object.freeze(["PRODUCT", "BASE"]),
    targetOrder: Object.freeze(["PRODUCT", "BASE_MINIMAL", "BASE_FULL"]),
    scenarios: Object.freeze(["rapid-long-jyutping"]),
    runKind: "observer-overhead",
  }),
  BASELINE: Object.freeze({
    roles: Object.freeze(["BASE"]),
    targetOrder: Object.freeze(["BASE_FULL"]),
    scenarios: WEB06_BINDING_SCENARIOS,
    runKind: "full",
  }),
  FINAL: Object.freeze({
    roles: Object.freeze(["FINAL"]),
    targetOrder: Object.freeze(["FINAL_FULL"]),
    scenarios: WEB06_BINDING_SCENARIOS,
    runKind: "full",
  }),
});

function required(env, name) {
  const value = env[name]?.trim();
  if (!value || /[\0\r\n]/.test(value)) throw new Error(`${name} is required and must be single-line`);
  return value;
}

function withoutCloudflareCredentials(environment) {
  return Object.fromEntries(
    Object.entries(environment).filter(
      ([name]) =>
        !name.startsWith("CLOUDFLARE_") &&
        !name.startsWith("CF_") &&
        !name.startsWith("WRANGLER_"),
    ),
  );
}

function parseIdentity(raw) {
  let identity;
  try {
    identity = JSON.parse(raw);
  } catch {
    throw new Error("YUNE_WEB06_IDENTITY_MANIFEST_JSON must be valid JSON");
  }
  if (identity?.version !== "web06-target-identities-v1") {
    throw new Error("WEB06 identity manifest version mismatch");
  }
  if (
    identity.metricContractVersion !== "web06-metric-v1" ||
    identity.scenarioRegistryVersion !== "web06-scenarios-v1" ||
    identity.behaviorPredicateVersion !== "web06-behavior-predicates-v1" ||
    !/^[0-9a-f]{64}$/.test(identity.collectorContractSha256 ?? "")
  ) {
    throw new Error("WEB06 identity manifest contract fields are invalid");
  }
  return identity;
}

function runEnvironmentJson(env) {
  const raw = required(env, "YUNE_WEB06_RUN_ENVIRONMENT_JSON");
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("YUNE_WEB06_RUN_ENVIRONMENT_JSON must be valid JSON");
  }
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("YUNE_WEB06_RUN_ENVIRONMENT_JSON must be an object");
  }
  if (env.YUNE_WEB06_BLOCKED_SCENARIOS_JSON !== undefined) {
    let blocked;
    try {
      blocked = JSON.parse(env.YUNE_WEB06_BLOCKED_SCENARIOS_JSON);
    } catch {
      throw new Error("YUNE_WEB06_BLOCKED_SCENARIOS_JSON must be valid JSON");
    }
    if (!Array.isArray(blocked) || blocked.length !== 0) {
      throw new Error("Binding WEB06 execution requires blocked scenarios to be []");
    }
  }
  return raw;
}

function checkedPort(env, name, fallback) {
  const value = Number(env[name] ?? fallback);
  if (!Number.isInteger(value) || value < 1024 || value > 65535) {
    throw new Error(`${name} must be an unprivileged TCP port`);
  }
  return value;
}

export function resolveMatrixContract(env) {
  const expectation = required(env, "YUNE_WEB06_EXPECTATION");
  const matrix = MATRIX[expectation];
  if (!matrix) throw new Error("YUNE_WEB06_EXPECTATION must be OBSERVER, BASELINE, or FINAL");
  const selectedBranch = required(env, "YUNE_WEB06_SELECTED_BRANCH");
  const disposition = required(env, "YUNE_WEB06_DISPOSITION");
  if (expectation === "FINAL") {
    if (!["A", "B", "C", "NONE"].includes(selectedBranch)) {
      throw new Error("FINAL requires selected branch A, B, C, or NONE");
    }
    if (
      (selectedBranch === "NONE" && disposition !== "MEASURED_NO_GO") ||
      (selectedBranch !== "NONE" && disposition !== "PRODUCTION_REDUCTION")
    ) {
      throw new Error("FINAL disposition does not match its selected branch");
    }
  } else if (selectedBranch !== "NONE") {
    throw new Error(`${expectation} requires selected branch NONE`);
  } else if (
    (expectation === "OBSERVER" && disposition !== "DIAGNOSTIC") ||
    (expectation === "BASELINE" && disposition !== "SOURCE_CURRENT_BASELINE")
  ) {
    throw new Error(`${expectation} disposition is invalid`);
  }
  const evidenceRoot = path.resolve(required(env, "YUNE_WEB06_EVIDENCE_ROOT"));
  const runId = required(env, "YUNE_WEB06_RUN_ID");
  if (!SAFE_RUN_ID.test(runId)) throw new Error("YUNE_WEB06_RUN_ID is not a safe evidence segment");
  const identityManifestJson = required(env, "YUNE_WEB06_IDENTITY_MANIFEST_JSON");
  const identity = parseIdentity(identityManifestJson);
  const environmentJson = runEnvironmentJson(env);
  const environmentManifest = JSON.parse(environmentJson);
  const roles = {};
  for (const roleName of matrix.roles) {
    const pinned = identity.roles?.[roleName];
    const expectedRoleDisposition =
      roleName === "FINAL"
        ? disposition
        : expectation === "OBSERVER"
          ? "DIAGNOSTIC"
          : "SOURCE_CURRENT_BASELINE";
    if (
      !/^[0-9a-f]{40}$/.test(pinned?.sourceCommit ?? "") ||
      !/^[0-9a-f]{40}$/.test(pinned?.sourceTree ?? "") ||
      pinned?.sourceTreeState !== "clean" ||
      !/^[0-9a-f]{64}$/.test(pinned?.archiveSha256 ?? "") ||
      !/^[0-9a-f]{64}$/.test(pinned?.artifactManifestSha256 ?? "") ||
      !/^[0-9a-f]{64}$/.test(pinned?.buildInfoSha256 ?? "") ||
      pinned?.selectedBranch !==
        (roleName === "FINAL" ? selectedBranch : "NONE") ||
      pinned?.disposition !== expectedRoleDisposition
    ) {
      throw new Error(`WEB06 identity role ${roleName} is invalid`);
    }
    roles[roleName] = {
      ...pinned,
      archivePath: path.resolve(required(env, `YUNE_WEB06_${roleName}_ARCHIVE`)),
      port: checkedPort(env, `YUNE_WEB06_${roleName}_PORT`, roleName === "PRODUCT" ? 4181 : roleName === "BASE" ? 4182 : 4183),
    };
  }
  const ports = Object.values(roles).map((role) => role.port);
  if (new Set(ports).size !== ports.length) throw new Error("WEB06 artifact ports must be distinct");
  const scenarios = [...matrix.scenarios];
  if (expectation === "FINAL" && selectedBranch === "B") {
    scenarios.push(...WEB06_BRANCH_B_SCENARIOS);
  }
  return {
    expectation,
    selectedBranch,
    disposition,
    evidenceRoot,
    runId,
    identity,
    identityManifestJson,
    environmentJson,
    environmentManifestSha256: sha256Bytes(
      Buffer.from(JSON.stringify(environmentManifest)),
    ),
    environmentId: environmentManifest.environmentId,
    roles,
    targetOrder: [...matrix.targetOrder],
    scenarios,
    runKind: matrix.runKind,
    suiteAttestationPath: path.join(evidenceRoot, "raw", "suite-attestation.json"),
    playwrightOutputDir: path.join(evidenceRoot, "playwright"),
    rawEvidenceRoot: path.join(evidenceRoot, "raw"),
    collectorOutputPath: path.join(evidenceRoot, "raw", "collector-output.json"),
    independentRecomputePath: path.join(
      evidenceRoot,
      "raw",
      "independent-recompute.json",
    ),
  };
}

function runCommand(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd: options.cwd, env: options.env, stdio: "inherit" });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolve();
      else reject(new Error(`${command} exited with code ${code}, signal ${signal}`));
    });
  });
}

function runCapture(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => { stdout += chunk; });
    child.stderr.on("data", (chunk) => { stderr += chunk; });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolve({ stdout, stderr });
      else reject(new Error(`${command} exited with code ${code}, signal ${signal}: ${stderr.trim()}`));
    });
  });
}

export async function proveExactMatrixRunnerSource(
  contract,
  environment,
  execute = runCapture,
) {
  const roleName = contract.expectation === "FINAL" ? "FINAL" : "BASE";
  const pinned = contract.roles[roleName];
  if (!pinned) throw new Error(`WEB06 runner identity role ${roleName} is missing`);
  const command = async (args) =>
    (await execute("git", args, {
      cwd: repoRoot,
      env: withoutCloudflareCredentials(environment),
    })).stdout.trim();
  const [head, tree, status] = await Promise.all([
    command(["rev-parse", "HEAD"]),
    command(["rev-parse", "HEAD^{tree}"]),
    command(["status", "--porcelain", "--untracked-files=all"]),
  ]);
  if (
    head !== pinned.sourceCommit ||
    tree !== pinned.sourceTree ||
    status !== ""
  ) {
    throw new Error(
      `WEB06 ${contract.expectation} runner requires clean exact ${roleName} HEAD/tree`,
    );
  }
  return Object.freeze({ roleName, sourceCommit: head, sourceTree: tree });
}

export async function prepareMatrixOutputPaths(contract, environment) {
  const outputContract = {
    evidenceDir: contract.evidenceRoot,
    outputDir: contract.playwrightOutputDir,
  };
  await prepareOutputPaths(outputContract, environment);
  contract.evidenceRoot = outputContract.evidenceDir;
  contract.playwrightOutputDir = outputContract.outputDir;
  contract.rawEvidenceRoot = path.join(contract.evidenceRoot, "raw");
  await mkdir(contract.rawEvidenceRoot, { mode: 0o700 });
  contract.suiteAttestationPath = path.join(
    contract.rawEvidenceRoot,
    "suite-attestation.json",
  );
  contract.collectorOutputPath = path.join(
    contract.rawEvidenceRoot,
    "collector-output.json",
  );
  contract.independentRecomputePath = path.join(
    contract.rawEvidenceRoot,
    "independent-recompute.json",
  );
  return contract;
}

export function artifactResponseGuard(local) {
  return sealedArtifactResponseGuard(local);
}

function targetsFor(contract, artifacts, servers) {
  const target = (id, roleName, protocolMode, selectorPolicy) => ({
    origin: servers[roleName].origin,
    sourceCommit: contract.roles[roleName].sourceCommit,
    sourceTree: contract.roles[roleName].sourceTree,
    treeState: "clean",
    artifactSha256: contract.roles[roleName].artifactManifestSha256,
    archiveSha256: contract.roles[roleName].archiveSha256,
    buildInfoSha256: contract.roles[roleName].buildInfoSha256,
    artifactResponseGuard: artifactResponseGuard(artifacts[roleName].local),
    protocolMode,
    selectorPolicy,
  });
  if (contract.expectation === "OBSERVER") {
    return {
      PRODUCT: target("PRODUCT", "PRODUCT", "off", "omitted"),
      BASE_MINIMAL: target("BASE_MINIMAL", "BASE", "minimal", "explicit"),
      BASE_FULL: target("BASE_FULL", "BASE", "full", "explicit"),
    };
  }
  if (contract.expectation === "BASELINE") {
    return { BASE_FULL: target("BASE_FULL", "BASE", "full", "explicit") };
  }
  return { FINAL_FULL: target("FINAL_FULL", "FINAL", "full", "explicit") };
}

export async function main(env = process.env) {
  const contract = resolveMatrixContract(env);
  const childEnvironment = withoutCloudflareCredentials({
    ...process.env,
    ...env,
  });
  try {
    await lstat(contract.evidenceRoot);
    throw new Error("YUNE_WEB06_EVIDENCE_ROOT must not already exist");
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  const runnerSource = await proveExactMatrixRunnerSource(
    contract,
    childEnvironment,
  );
  const runnerSourceManifest = await createRunnerSourceManifest(
    repoRoot,
    runnerSource.sourceCommit,
    runnerSource.sourceTree,
  );
  await prepareMatrixOutputPaths(contract, childEnvironment);
  await Promise.all([stat(playwrightCli), stat(configPath), stat(specPath)]);
  const artifacts = {};
  const servers = {};
  try {
    for (const [roleName, role] of Object.entries(contract.roles)) {
      const archiveContract = {
        archivePath: role.archivePath,
        archiveSha256: role.archiveSha256,
        evidenceDir: contract.evidenceRoot,
      };
      await validateArchive(archiveContract);
      const destination = path.join(contract.evidenceRoot, `.sealed-${roleName.toLowerCase()}`);
      const distRoot = await extractCertifiedArchive(
        archiveContract,
        destination,
        childEnvironment,
      );
      await validateArchive(archiveContract);
      const local = await validateLocalBundle(distRoot, role.sourceCommit);
      if (sha256(local.manifestBytes) !== role.artifactManifestSha256) {
        throw new Error(`${roleName} artifact manifest does not match its pinned identity`);
      }
      if (sha256(local.buildInfoBytes) !== role.buildInfoSha256) {
        throw new Error(`${roleName} build-info does not match its pinned identity`);
      }
      const snapshot = await loadSealedArtifactSnapshot(distRoot, local);
      artifacts[roleName] = { distRoot, local, snapshot };
      const sealedServer = await startSealedArtifactServer(snapshot, {
        host: "127.0.0.1",
        port: role.port,
      });
      servers[roleName] = {
        ...sealedServer,
        origin: sealedServer.appUrl,
      };
      const response = await fetch(new URL("build-info.json", sealedServer.appUrl), {
        cache: "no-store",
        signal: AbortSignal.timeout(2_000),
      });
      const servedBuildInfo = response.ok ? await response.json() : null;
      if (
        servedBuildInfo?.sourceCommit !== role.sourceCommit ||
        servedBuildInfo?.sourceTreeState !== "clean"
      ) {
        throw new Error(`${roleName} sealed artifact server failed readiness`);
      }
      sealedServer.assertHealthy();
    }
    const targets = targetsFor(contract, artifacts, servers);
    let browserFailure = null;
    try {
      await runCommand(
        process.execPath,
        [playwrightCli, "test", "--config", path.basename(configPath), "--grep", "@web06-full", "--workers=1", "--retries=0"],
        {
          cwd: e2eRoot,
          env: {
            ...childEnvironment,
            YUNE_WEB06_EXPECTATION: contract.expectation,
            YUNE_WEB06_RUN_KIND: contract.runKind,
            YUNE_WEB06_SELECTED_BRANCH: contract.selectedBranch,
            YUNE_WEB06_DISPOSITION: contract.disposition,
            YUNE_WEB06_IDENTITY_MANIFEST_JSON: contract.identityManifestJson,
            YUNE_WEB06_RUN_ENVIRONMENT_JSON: contract.environmentJson,
            YUNE_WEB06_RUNNER_SOURCE_JSON: JSON.stringify(runnerSourceManifest),
            YUNE_WEB06_BLOCKED_SCENARIOS_JSON: "[]",
            YUNE_WEB06_TARGETS_JSON: JSON.stringify(targets),
            YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(contract.targetOrder),
            YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(contract.scenarios),
            YUNE_WEB06_PLAYWRIGHT_RETRIES: "0",
            YUNE_WEB06_PLAYWRIGHT_WORKERS: "1",
            YUNE_WEB06_RUN_ID: contract.runId,
            YUNE_WEB06_EVIDENCE_ROOT: contract.rawEvidenceRoot,
            YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR: contract.playwrightOutputDir,
            YUNE_WEB06_SUITE_ATTESTATION_PATH: contract.suiteAttestationPath,
            YUNE_WEB06_COLLECTOR_OUTPUT_PATH: contract.collectorOutputPath,
            YUNE_WEB06_INDEPENDENT_RECOMPUTE_PATH:
              contract.independentRecomputePath,
          },
        },
      );
    } catch (error) {
      browserFailure = error;
    }
    for (const server of Object.values(servers)) server.assertHealthy();
    if (browserFailure !== null) throw browserFailure;
    await proveExactMatrixRunnerSource(contract, childEnvironment);
    const primaryRole = contract.expectation === "OBSERVER" ? "BASE" : contract.expectation === "BASELINE" ? "BASE" : "FINAL";
    await readAndValidateSuiteAttestation(contract.suiteAttestationPath, {
      expectation: contract.expectation,
      sourceCommit: contract.roles[primaryRole].sourceCommit,
      sourceTree: contract.roles[primaryRole].sourceTree,
      archiveSha256: contract.roles[primaryRole].archiveSha256,
      artifactManifestSha256: contract.roles[primaryRole].artifactManifestSha256,
      selectedBranch: contract.selectedBranch,
      disposition: contract.disposition,
      identityManifestSha256: sha256StableJson(contract.identity),
      collectorContractSha256: contract.identity.collectorContractSha256,
      environmentManifestSha256: contract.environmentManifestSha256,
      environmentId: contract.environmentId,
      identityManifest: contract.identity,
      evidenceRoot: contract.rawEvidenceRoot,
      ...(contract.expectation === "OBSERVER"
        ? {
            observerArtifacts: Object.fromEntries(
              ["PRODUCT", "BASE"].map((roleName) => [roleName, {
                sourceCommit: contract.roles[roleName].sourceCommit,
                sourceTree: contract.roles[roleName].sourceTree,
                sourceTreeState: "clean",
                archiveSha256: contract.roles[roleName].archiveSha256,
                artifactManifestSha256: contract.roles[roleName].artifactManifestSha256,
                buildInfoSha256: contract.roles[roleName].buildInfoSha256,
              }]),
            ),
          }
        : {}),
    });
    for (const roleName of Object.keys(contract.roles)) {
      await validateArchive({
        archivePath: contract.roles[roleName].archivePath,
        archiveSha256: contract.roles[roleName].archiveSha256,
      });
    }
  } finally {
    await Promise.all(Object.values(servers).map((server) => server.stop()));
    await Promise.all(Object.values(artifacts).map((artifact) => rm(artifact.distRoot, { recursive: true, force: true })));
  }
}

const invokedPath = process.argv[1] ? pathToFileURL(process.argv[1]).href : null;
if (invokedPath === import.meta.url) {
  try {
    await main();
  } catch (error) {
    console.error(error);
    process.exitCode = 1;
  }
}
