import { spawn } from "node:child_process";
import { lstat, mkdir, open, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { sha256, validateLocalBundle } from "./public-artifact-verifier.mjs";
import {
  extractCertifiedArchive,
  main as runPublicGate,
  prepareOutputPaths,
  reconcileRemoteBundle,
  validateArchive,
} from "./run-public-web06-gate.mjs";
import {
  readAndValidateSuiteAttestation,
  sha256Bytes,
  sha256StableJson,
} from "./web06-suite-attestation.mjs";
import {
  installPinnedWranglerRuntime,
  preparePinnedWranglerRuntime,
  WEB06_WRANGLER_VERSION,
  wranglerDeploymentEnvironment,
} from "./web06-wrangler-runtime.mjs";

const e2eRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(e2eRoot, "../../..");
const GIT_EXECUTABLE = "/usr/bin/git";
const certifyScript = path.join(
  repoRoot,
  "apps",
  "yune-web",
  "public-demo",
  "certify-public-release.sh",
);
const WRANGLER_VERSION = WEB06_WRANGLER_VERSION;
const SHA40 = /^[0-9a-f]{40}$/;
const SHA64 = /^[0-9a-f]{64}$/;

function required(environment, name) {
  const value = environment[name]?.trim();
  if (!value || /[\0\r\n]/.test(value)) {
    throw new Error(`${name} is required and must be single-line`);
  }
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

function cloudflareCredentialValues(environment) {
  return [...new Set(
    Object.entries(environment)
      .filter(
        ([name]) =>
          name.startsWith("CLOUDFLARE_") ||
          name.startsWith("CF_") ||
          name.startsWith("WRANGLER_"),
      )
      .map(([, value]) => (typeof value === "string" ? value : ""))
      .filter((value) => value.length >= 4),
  )].sort((left, right) => right.length - left.length);
}

function redactCredentialValues(message, credentialValues) {
  let sanitized = String(message);
  for (const value of credentialValues) {
    sanitized = sanitized.split(value).join("[REDACTED]");
  }
  return sanitized;
}

function exactSha(environment, name, pattern) {
  const value = required(environment, name);
  if (!pattern.test(value)) throw new Error(`${name} is not a full lowercase hash`);
  return value;
}

function identityManifest(environment) {
  const raw = required(environment, "YUNE_WEB06_IDENTITY_MANIFEST_JSON");
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("YUNE_WEB06_IDENTITY_MANIFEST_JSON must be valid JSON");
  }
  if (
    parsed?.version !== "web06-target-identities-v1" ||
    parsed.metricContractVersion !== "web06-metric-v1" ||
    parsed.scenarioRegistryVersion !== "web06-scenarios-v1" ||
    parsed.behaviorPredicateVersion !== "web06-behavior-predicates-v1" ||
    !SHA64.test(parsed.collectorContractSha256 ?? "")
  ) {
    throw new Error("WEB06 identity manifest does not match the frozen contract");
  }
  return { raw, parsed, sha256: sha256StableJson(parsed) };
}

export function resolveCertifiedPreviewContract(environment) {
  const sourceCommit = exactSha(environment, "YUNE_WEB06_EXPECTED_SOURCE_COMMIT", SHA40);
  const sourceTree = exactSha(environment, "YUNE_WEB06_EXPECTED_SOURCE_TREE", SHA40);
  const archiveSha256 = exactSha(
    environment,
    "YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256",
    SHA64,
  );
  const selectedBranch = required(environment, "YUNE_WEB06_SELECTED_BRANCH");
  if (!["A", "B", "C"].includes(selectedBranch)) {
    throw new Error("Certified preview requires production branch A, B, or C");
  }
  if (required(environment, "YUNE_WEB06_DISPOSITION") !== "PRODUCTION_REDUCTION") {
    throw new Error("A measured no-go is not preview eligible");
  }
  const identity = identityManifest(environment);
  const finalRole = identity.parsed.roles?.FINAL;
  if (
    finalRole?.sourceCommit !== sourceCommit ||
    finalRole?.sourceTree !== sourceTree ||
    finalRole?.sourceTreeState !== "clean" ||
    finalRole?.archiveSha256 !== archiveSha256 ||
    !SHA64.test(finalRole?.buildInfoSha256 ?? "") ||
    finalRole?.selectedBranch !== selectedBranch ||
    finalRole?.disposition !== "PRODUCTION_REDUCTION"
  ) {
    throw new Error("WEB06 FINAL role does not identify the preview candidate");
  }
  return {
    sourceCommit,
    sourceTree,
    archivePath: path.resolve(required(environment, "YUNE_WEB06_CERTIFIED_ARCHIVE")),
    archiveSha256,
    selectedBranch,
    disposition: "PRODUCTION_REDUCTION",
    identity,
    finalSuiteAttestationPath: path.resolve(
      required(environment, "YUNE_WEB06_FINAL_SUITE_ATTESTATION"),
    ),
    finalSuiteEvidenceRoot: path.resolve(
      required(environment, "YUNE_WEB06_FINAL_SUITE_EVIDENCE_ROOT"),
    ),
    evidenceDir: path.resolve(required(environment, "YUNE_WEB06_EVIDENCE_ROOT")),
    outputDir: path.resolve(
      required(environment, "YUNE_WEB06_EVIDENCE_ROOT"),
      "path-preflight",
    ),
    accountId: required(environment, "CLOUDFLARE_ACCOUNT_ID"),
    apiToken: required(environment, "CLOUDFLARE_API_TOKEN"),
    projectName: "yune-web",
  };
}

function runCommand(command, arguments_, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, arguments_, {
      cwd: options.cwd,
      env: options.env,
      stdio: options.capture ? ["ignore", "pipe", "pipe"] : "inherit",
    });
    let stdout = "";
    let stderr = "";
    if (options.capture) {
      child.stdout.setEncoding("utf8");
      child.stderr.setEncoding("utf8");
      child.stdout.on("data", (chunk) => { stdout += chunk; });
      child.stderr.on("data", (chunk) => { stderr += chunk; });
    }
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolve({ stdout, stderr });
      else reject(new Error(`${command} exited with code ${code}, signal ${signal}: ${stderr.trim()}`));
    });
  });
}

export async function proveExactPreviewSource(contract, execute = runCommand) {
  const cleanEnvironment = {
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_TERMINAL_PROMPT: "0",
    HOME: path.parse(repoRoot).root,
    LANG: "C",
    LC_ALL: "C",
    PATH: "/usr/bin:/bin",
  };
  const command = async (arguments_) =>
    (await execute(GIT_EXECUTABLE, arguments_, {
      cwd: repoRoot,
      env: cleanEnvironment,
      capture: true,
    })).stdout.trim();
  const [head, tree, status] = await Promise.all([
    command(["rev-parse", "HEAD"]),
    command(["rev-parse", "HEAD^{tree}"]),
    command(["status", "--porcelain", "--untracked-files=all"]),
  ]);
  if (
    head !== contract.sourceCommit ||
    tree !== contract.sourceTree ||
    status !== ""
  ) {
    throw new Error("Certified preview requires clean exact candidate HEAD/tree");
  }
  return Object.freeze({ sourceCommit: head, sourceTree: tree });
}

function parsePreviewUrl(output) {
  const matches = output.match(/https:\/\/[A-Za-z0-9.-]+\.pages\.dev\/?/g) ?? [];
  const selected = matches.at(-1);
  if (!selected) throw new Error("Pinned Wrangler did not report a pages.dev preview URL");
  const parsed = new URL(selected);
  if (parsed.protocol !== "https:" || !parsed.hostname.endsWith(".pages.dev")) {
    throw new Error("Wrangler reported an invalid preview URL");
  }
  if (!parsed.pathname.endsWith("/")) parsed.pathname += "/";
  return parsed.href;
}

export function validateLocalCertificationReceipt(receipt, expected) {
  if (
    receipt?.version !== "web06-local-release-certification-v1" ||
    receipt.operation !== "local-no-build-certification" ||
    receipt.sourceCommit !== expected.sourceCommit ||
    receipt.sourceTree !== expected.sourceTree ||
    receipt.archiveSha256 !== expected.archiveSha256 ||
    receipt.artifactManifestSha256 !== expected.artifactManifestSha256 ||
    receipt.finalSuiteAttestationSha256 !== expected.finalSuiteAttestationSha256 ||
    receipt.web03UnchangedStatus !== "passed" ||
    receipt.defaultMinimalCompatibilityStatus !== "passed" ||
    receipt.selectorPolicy !== "omitted" ||
    receipt.buildInvoked !== false ||
    receipt.status !== "passed"
  ) {
    throw new Error("Local certification receipt is not preview eligible");
  }
  return receipt;
}

async function revalidatePreviewBundle(
  contract,
  distRoot,
  expectedArtifactManifestSha256,
) {
  await validateArchive(contract);
  const local = await validateLocalBundle(distRoot, contract.sourceCommit);
  if (
    sha256(local.manifestBytes) !== expectedArtifactManifestSha256 ||
    sha256(local.buildInfoBytes) !==
      contract.identity.parsed.roles.FINAL.buildInfoSha256
  ) {
    throw new Error("Certified preview bundle identity changed after preflight");
  }
  return local;
}

export async function assertCloudflareInterlock(contract, fetchImplementation) {
  const response = await fetchImplementation(
    `https://api.cloudflare.com/client/v4/accounts/${encodeURIComponent(contract.accountId)}/pages/projects/${contract.projectName}`,
    {
      headers: { Authorization: `Bearer ${contract.apiToken}` },
      signal: AbortSignal.timeout(20_000),
    },
  );
  if (!response.ok) throw new Error(`Cloudflare project interlock returned HTTP ${response.status}`);
  const payload = await response.json();
  const config = payload?.result?.source?.config;
  if (
    payload?.success !== true ||
    payload?.result?.production_branch !== "main" ||
    config?.production_deployments_enabled !== false ||
    config?.preview_deployment_setting !== "none"
  ) {
    throw new Error("Cloudflare Git auto-deploy interlock is not closed");
  }
  return {
    productionBranch: "main",
    productionDeploymentsEnabled: false,
    previewDeploymentSetting: "none",
  };
}

async function listPreviewDeployments(contract, fetchImplementation) {
  const rows = [];
  for (let page = 1; page <= 100; page += 1) {
    const url = new URL(
      `https://api.cloudflare.com/client/v4/accounts/${encodeURIComponent(contract.accountId)}/pages/projects/${contract.projectName}/deployments`,
    );
    url.searchParams.set("page", String(page));
    url.searchParams.set("per_page", "100");
    const response = await fetchImplementation(url.href, {
      headers: { Authorization: `Bearer ${contract.apiToken}` },
      signal: AbortSignal.timeout(20_000),
    });
    if (!response.ok) {
      throw new Error(`Cloudflare preview inventory returned HTTP ${response.status}`);
    }
    const payload = await response.json();
    if (payload?.success !== true || !Array.isArray(payload.result)) {
      throw new Error("Cloudflare preview inventory is malformed");
    }
    rows.push(...payload.result);
    const currentPage = payload.result_info?.page;
    const totalPages = payload.result_info?.total_pages;
    if (
      !Number.isSafeInteger(currentPage) ||
      currentPage !== page ||
      !Number.isSafeInteger(totalPages) ||
      totalPages < page ||
      totalPages > 100
    ) {
      throw new Error("Cloudflare preview inventory pagination is malformed");
    }
    if (page === totalPages) return rows;
  }
  throw new Error("Cloudflare preview inventory exceeded its fail-closed page limit");
}

export async function assertNoPriorPreviewDeployment(
  contract,
  previewBranch,
  fetchImplementation,
) {
  const rows = await listPreviewDeployments(contract, fetchImplementation);
  const prior = rows.find(
    (row) =>
      row?.environment === "preview" &&
      row?.deployment_trigger?.metadata?.branch === previewBranch &&
      row?.deployment_trigger?.metadata?.commit_hash === contract.sourceCommit,
  );
  if (prior !== undefined) {
    const consumed = new Error(
      "The one authorized WEB06 preview was already attempted for this source and branch",
    );
    consumed.code = "EEXIST";
    throw consumed;
  }
  return rows;
}

function normalizedPagesUrl(value) {
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    return null;
  }
  if (
    parsed.protocol !== "https:" ||
    !parsed.hostname.endsWith(".pages.dev") ||
    parsed.username !== "" ||
    parsed.password !== "" ||
    parsed.search !== "" ||
    parsed.hash !== ""
  ) {
    return null;
  }
  if (!parsed.pathname.endsWith("/")) parsed.pathname += "/";
  return parsed.href;
}

function matchingNewPreviewDeployments(
  rows,
  previousIds,
  contract,
  previewBranch,
  previewUrl,
) {
  return rows.filter(
    (row) =>
      typeof row?.id === "string" &&
      row.id !== "" &&
      !previousIds.has(row.id) &&
      normalizedPagesUrl(row?.url) === previewUrl &&
      row?.environment === "preview" &&
      row?.latest_stage?.status === "success" &&
      row?.deployment_trigger?.type === "ad_hoc" &&
      row?.deployment_trigger?.metadata?.branch === previewBranch &&
      row?.deployment_trigger?.metadata?.commit_hash === contract.sourceCommit &&
      row?.deployment_trigger?.metadata?.commit_dirty === false,
  );
}

export async function resolveNewPreviewDeployment(
  contract,
  previewBranch,
  previewUrl,
  previousIds,
  fetchImplementation,
  delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)),
) {
  for (let attempt = 1; attempt <= 6; attempt += 1) {
    const rows = await listPreviewDeployments(contract, fetchImplementation);
    const matches = matchingNewPreviewDeployments(
      rows,
      previousIds,
      contract,
      previewBranch,
      previewUrl,
    );
    if (matches.length > 1) {
      throw new Error("Cloudflare exposed multiple matching WEB06 preview deployments");
    }
    if (matches.length === 1) return matches[0];
    if (attempt < 6) await delay(2_000);
  }
  throw new Error("Cloudflare did not expose the exact new WEB06 preview deployment");
}

export async function consumePreviewAuthorization(contract, previewBranch) {
  const consumptionPath = `${contract.archivePath}.web06-preview-consumed`;
  let handle;
  try {
    handle = await open(consumptionPath, "wx", 0o600);
  } catch (error) {
    if (error?.code === "EEXIST") {
      const consumed = new Error(
        "The one authorized WEB06 preview has already been consumed",
      );
      consumed.code = "EEXIST";
      throw consumed;
    }
    throw error;
  }
  const bytes = Buffer.from(`${JSON.stringify({
    version: "web06-preview-authorization-consumption-v1",
    sourceCommit: contract.sourceCommit,
    sourceTree: contract.sourceTree,
    archiveSha256: contract.archiveSha256,
    previewBranch,
    consumedAt: new Date().toISOString(),
    status: "consumed",
  }, null, 2)}\n`);
  try {
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
  return {
    path: consumptionPath,
    sha256: sha256Bytes(bytes),
  };
}

async function readJsonPlain(file, label) {
  const metadata = await lstat(file);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error(`${label} must be a plain file`);
  }
  try {
    const bytes = await readFile(file);
    return { payload: JSON.parse(bytes.toString("utf8")), bytes };
  } catch {
    throw new Error(`${label} is not valid JSON`);
  }
}

export function validatePreviewCanaryStatus(receipt, expected) {
  if (
    receipt?.version !== "web06-public-gate-status-v1" ||
    receipt.sourceCommit !== expected.sourceCommit ||
    receipt.sourceTree !== expected.sourceTree ||
    receipt.archiveSha256 !== expected.archiveSha256 ||
    receipt.artifactManifestSha256 !== expected.artifactManifestSha256 ||
    receipt.finalSuiteAttestationSha256 !==
      expected.finalSuiteAttestationSha256 ||
    receipt.disposition !== "PRODUCTION_REDUCTION" ||
    receipt.scope !== "preview-canary" ||
    receipt.appUrl !== expected.previewUrl ||
    receipt.canaryStarted !== true ||
    !SHA64.test(receipt.previewSuiteAttestationSha256 ?? "") ||
    receipt.status !== "passed"
  ) {
    throw new Error("WEB06 preview canary status is not production eligible");
  }
  return receipt;
}

export async function main(environment = process.env, dependencies = {}) {
  const contract = resolveCertifiedPreviewContract(environment);
  const credentialValues = cloudflareCredentialValues({
    ...process.env,
    ...environment,
  });
  try {
    await lstat(contract.evidenceDir);
    throw new Error("YUNE_WEB06_EVIDENCE_ROOT must be create-new for preview");
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  const execute = dependencies.runCommand ?? runCommand;
  const fetchImplementation = dependencies.fetch ?? globalThis.fetch;
  const runCanary = dependencies.runPublicGate ?? runPublicGate;
  const readSuiteAttestation =
    dependencies.readAndValidateSuiteAttestation ??
    readAndValidateSuiteAttestation;
  const delay = dependencies.delay ?? ((milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)));
  const installWranglerRuntime =
    dependencies.installPinnedWranglerRuntime ??
    installPinnedWranglerRuntime;
  const prepareWranglerRuntime =
    dependencies.preparePinnedWranglerRuntime ??
    preparePinnedWranglerRuntime;
  const credentialFreeEnvironment = withoutCloudflareCredentials({
    ...process.env,
    ...environment,
  });
  await prepareOutputPaths(contract, credentialFreeEnvironment);
  const distRoot = path.join(contract.evidenceDir, "certified-dist");
  let status = "failed";
  let previewUrl = null;
  let artifactManifestSha256 = null;
  let finalSuiteAttestationSha256 = null;
  let localCertificationReceiptSha256 = null;
  let previewCanaryStatusSha256 = null;
  let previewSuiteAttestationSha256 = null;
  let previewAuthorizationConsumptionSha256 = null;
  let previewMutationStarted = false;
  let previewDeployment = null;
  let wranglerInstallIdentity = null;
  let wranglerRuntime = null;
  try {
    await proveExactPreviewSource(contract, execute);
    await validateArchive(contract);
    await extractCertifiedArchive(contract, distRoot, credentialFreeEnvironment);
    await validateArchive(contract);
    const local = await validateLocalBundle(distRoot, contract.sourceCommit);
    artifactManifestSha256 = sha256(local.manifestBytes);
    if (contract.identity.parsed.roles.FINAL.artifactManifestSha256 !== artifactManifestSha256) {
      throw new Error("Certified archive manifest differs from the FINAL identity role");
    }
    if (
      contract.identity.parsed.roles.FINAL.buildInfoSha256 !==
      sha256(local.buildInfoBytes)
    ) {
      throw new Error("Certified archive build-info differs from the FINAL identity role");
    }
    const finalSuite = await readSuiteAttestation(
      contract.finalSuiteAttestationPath,
      {
        expectation: "FINAL",
        sourceCommit: contract.sourceCommit,
        sourceTree: contract.sourceTree,
        archiveSha256: contract.archiveSha256,
        artifactManifestSha256,
        selectedBranch: contract.selectedBranch,
        disposition: contract.disposition,
        identityManifestSha256: contract.identity.sha256,
        collectorContractSha256: contract.identity.parsed.collectorContractSha256,
        identityManifest: contract.identity.parsed,
        evidenceRoot: contract.finalSuiteEvidenceRoot,
      },
    );
    finalSuiteAttestationSha256 = finalSuite.sha256;

    const localCertificationRoot = path.join(contract.evidenceDir, "local-certification");
    await mkdir(localCertificationRoot, { mode: 0o700 });
    const localCertificationReceipt = path.join(
      localCertificationRoot,
      "web06-local-certification.json",
    );
    const localArtifactReceipt = path.join(
      localCertificationRoot,
      "local-artifact-verification.json",
    );
    const latencyRoot = path.join(localCertificationRoot, "web03-latency");
    await execute("bash", [
      certifyScript,
      "--receipt",
      localCertificationReceipt,
    ], {
      cwd: repoRoot,
      env: {
        ...credentialFreeEnvironment,
        YUNE_WEB_RELEASE_CERTIFICATION_MODE: "measured-web06",
        YUNE_WEB06_CERTIFIED_ARCHIVE: contract.archivePath,
        YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256: contract.archiveSha256,
        YUNE_WEB06_CERTIFIED_DIST_ROOT: distRoot,
        YUNE_WEB06_EXPECTED_SOURCE_COMMIT: contract.sourceCommit,
        YUNE_WEB06_EXPECTED_SOURCE_TREE: contract.sourceTree,
        YUNE_WEB06_EVIDENCE_ROOT: localCertificationRoot,
        YUNE_WEB_LOCAL_ARTIFACT_RECEIPT: localArtifactReceipt,
        YUNE_WEB_LATENCY_OUTPUT_DIR: latencyRoot,
        YUNE_WEB_LATENCY_EVIDENCE_DIR: latencyRoot,
      },
    });
    const localCertification = await readJsonPlain(
      localCertificationReceipt,
      "WEB06 local certification receipt",
    );
    validateLocalCertificationReceipt(
      localCertification.payload,
      {
        sourceCommit: contract.sourceCommit,
        sourceTree: contract.sourceTree,
        archiveSha256: contract.archiveSha256,
        artifactManifestSha256,
        finalSuiteAttestationSha256,
      },
    );
    localCertificationReceiptSha256 = sha256Bytes(localCertification.bytes);

    await revalidatePreviewBundle(
      contract,
      distRoot,
      artifactManifestSha256,
    );

    const wranglerRuntimeRoot = path.join(
      contract.evidenceDir,
      "wrangler-runtime",
    );
    const installedWrangler = await installWranglerRuntime({
      installationRoot: wranglerRuntimeRoot,
      sourceRoot: e2eRoot,
      baseEnvironment: credentialFreeEnvironment,
    });
    wranglerInstallIdentity = installedWrangler.identity;
    wranglerRuntime = await prepareWranglerRuntime({
      root: installedWrangler.root,
      baseEnvironment: credentialFreeEnvironment,
    });
    const wranglerHome = path.join(contract.evidenceDir, "wrangler-home");
    const wranglerTemporary = path.join(contract.evidenceDir, "wrangler-tmp");
    const wranglerCwd = path.join(contract.evidenceDir, "wrangler-cwd");
    for (const directory of [wranglerHome, wranglerTemporary, wranglerCwd]) {
      await mkdir(directory, { mode: 0o700 });
    }
    const projectInterlock = await assertCloudflareInterlock(
      contract,
      fetchImplementation,
    );
    const previewBranch = `web06-preview-${contract.sourceCommit.slice(0, 12)}`;
    if (previewBranch === "main" || previewBranch === "production") {
      throw new Error("Preview command selected a forbidden production branch");
    }
    await proveExactPreviewSource(contract, execute);
    await revalidatePreviewBundle(
      contract,
      distRoot,
      artifactManifestSha256,
    );
    const priorDeployments = await assertNoPriorPreviewDeployment(
      contract,
      previewBranch,
      fetchImplementation,
    );
    const priorDeploymentIds = new Set(
      priorDeployments.map((row) => row?.id).filter(
        (id) => typeof id === "string" && id !== "",
      ),
    );
    const previewAuthorization = await consumePreviewAuthorization(
      contract,
      previewBranch,
    );
    previewAuthorizationConsumptionSha256 = previewAuthorization.sha256;
    await wranglerRuntime.assertCurrent();
    previewMutationStarted = true;
    const deploy = await execute(
      wranglerRuntime.command,
      [
        ...wranglerRuntime.argumentsPrefix,
        "pages",
        "deploy",
        distRoot,
        "--project-name",
        contract.projectName,
        "--branch",
        previewBranch,
        "--commit-hash",
        contract.sourceCommit,
        "--commit-dirty=false",
      ],
      {
        cwd: wranglerCwd,
        env: wranglerDeploymentEnvironment({
          accountId: contract.accountId,
          apiToken: contract.apiToken,
          home: wranglerHome,
          temporaryDirectory: wranglerTemporary,
          baseEnvironment: credentialFreeEnvironment,
        }),
        capture: true,
      },
    );
    await wranglerRuntime.assertCurrent();
    previewUrl = parsePreviewUrl(`${deploy.stdout}\n${deploy.stderr}`);
    previewDeployment = await resolveNewPreviewDeployment(
      contract,
      previewBranch,
      previewUrl,
      priorDeploymentIds,
      fetchImplementation,
      delay,
    );
    await reconcileRemoteBundle(
      previewUrl,
      local,
      contract.sourceCommit,
      fetchImplementation,
    );

    const canaryRoot = path.join(contract.evidenceDir, "preview-canary");
    await runCanary(
      {
        ...credentialFreeEnvironment,
        YUNE_WEB_APP_URL: previewUrl,
        YUNE_WEB06_EVIDENCE_ROOT: canaryRoot,
        YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR: path.join(canaryRoot, "playwright"),
        YUNE_WEB06_GATE_SCOPE: "preview-canary",
      },
      ["--scope=preview-canary"],
    );
    const previewCanaryStatusPath = path.join(
      canaryRoot,
      "web06-public-gate-status.json",
    );
    const previewCanary = await readJsonPlain(
      previewCanaryStatusPath,
      "WEB06 preview canary status",
    );
    validatePreviewCanaryStatus(previewCanary.payload, {
      sourceCommit: contract.sourceCommit,
      sourceTree: contract.sourceTree,
      archiveSha256: contract.archiveSha256,
      artifactManifestSha256,
      finalSuiteAttestationSha256,
      previewUrl,
    });
    previewCanaryStatusSha256 = sha256Bytes(previewCanary.bytes);
    previewSuiteAttestationSha256 =
      previewCanary.payload.previewSuiteAttestationSha256;
    await revalidatePreviewBundle(
      contract,
      distRoot,
      artifactManifestSha256,
    );
    await proveExactPreviewSource(contract, execute);
    status = "passed";
    await writeFile(
      path.join(contract.evidenceDir, "web06-certified-preview-status.json"),
      `${JSON.stringify(
        {
          version: "web06-certified-preview-v1",
          operation: "preview-only-no-build",
          sourceCommit: contract.sourceCommit,
          sourceTree: contract.sourceTree,
          archiveSha256: contract.archiveSha256,
          artifactManifestSha256,
          finalSuiteAttestationSha256,
          localCertificationReceiptSha256,
          previewCanaryStatusSha256,
          previewSuiteAttestationSha256,
          previewAuthorizationConsumptionSha256,
          wranglerVersion: WRANGLER_VERSION,
          wranglerInstallIdentity,
          wranglerRuntimeIdentity: wranglerRuntime.identity,
          previewBranch,
          previewUrl,
          previewDeployment: {
            id: previewDeployment.id,
            url: previewUrl,
            environment: previewDeployment.environment,
            latestStageStatus: previewDeployment.latest_stage.status,
            triggerType: previewDeployment.deployment_trigger.type,
            branch: previewDeployment.deployment_trigger.metadata.branch,
            commitHash: previewDeployment.deployment_trigger.metadata.commit_hash,
            commitDirty: previewDeployment.deployment_trigger.metadata.commit_dirty,
          },
          projectInterlock,
          remoteManifestFileCount: local.manifest.files.length,
          productionPromotionAttempted: false,
          previewMutationStarted,
          peerPackageAlignment: "DATA_CONFOUNDED",
          peerRatioStatus: "OMITTED",
          fullSuccessEligible: false,
          milestoneDisposition: "PENDING_EXPLICIT_PARTIAL_APPROVAL",
          closeoutAuthorized: false,
          productionPromotionAuthorized: false,
          status,
        },
        null,
        2,
      )}\n`,
      { flag: "wx", mode: 0o600 },
    );
    return { previewUrl, status };
  } catch (error) {
    const failure = redactCredentialValues(
      error instanceof Error ? error.message : String(error),
      credentialValues,
    );
    await writeFile(
      path.join(contract.evidenceDir, "web06-certified-preview-failure.json"),
      `${JSON.stringify(
        {
          version: "web06-certified-preview-v1",
          operation: "preview-only-no-build",
          sourceCommit: contract.sourceCommit,
          sourceTree: contract.sourceTree,
          archiveSha256: contract.archiveSha256,
          artifactManifestSha256,
          finalSuiteAttestationSha256,
          localCertificationReceiptSha256,
          previewCanaryStatusSha256,
          previewSuiteAttestationSha256,
          previewAuthorizationConsumptionSha256,
          wranglerInstallIdentity,
          wranglerRuntimeIdentity: wranglerRuntime?.identity ?? null,
          previewUrl,
          previewDeploymentId: previewDeployment?.id ?? null,
          productionPromotionAttempted: false,
          previewMutationStarted,
          status,
          failure,
        },
        null,
        2,
      )}\n`,
      { flag: "wx", mode: 0o600 },
    );
    const sanitizedError = new Error(failure);
    if (error && typeof error === "object" && "code" in error) {
      sanitizedError.code = error.code;
    }
    throw sanitizedError;
  }
}

const invoked = process.argv[1] ? pathToFileURL(process.argv[1]).href : null;
if (invoked === import.meta.url) {
  try {
    await main();
  } catch (error) {
    console.error(error);
    process.exitCode = 1;
  }
}
