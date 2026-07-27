import { spawn } from "node:child_process";
import { constants as fsConstants } from "node:fs";
import {
  open,
  lstat,
  mkdir,
  readFile,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import { sha256, validateLocalBundle } from "./public-artifact-verifier.mjs";
import {
  extractCertifiedArchive,
  prepareOutputPaths,
  reconcileRemoteBundle,
  validateArchive,
} from "./run-public-web06-gate.mjs";
import {
  assertCloudflareInterlock,
  validatePreviewCanaryStatus,
} from "./run-web06-certified-preview.mjs";
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
const CANONICAL_REPOSITORY_URL = "https://github.com/CanCLID/yune.git";
const CANONICAL_MAIN_REF = "refs/heads/main";
const WRANGLER_VERSION = WEB06_WRANGLER_VERSION;
const PRODUCTION_URL = "https://yune-web.pages.dev/";
const SHA40 = /^[0-9a-f]{40}$/;
const SHA64 = /^[0-9a-f]{64}$/;
const APPROVAL_CONFIRMATION = "PROMOTE_EXACT_PREVIEW_BYTES";
const PARTIAL_LIMITATIONS = Object.freeze([
  "PEER_DATA_CONFOUNDED",
  "PEER_RATIO_OMITTED",
]);

function required(environment, name) {
  const value = environment[name]?.trim();
  if (!value || /[\0\r\n]/.test(value)) {
    throw new Error(`${name} is required and must be single-line`);
  }
  return value;
}

function exactHash(environment, name, pattern) {
  const value = required(environment, name);
  if (!pattern.test(value)) throw new Error(`${name} is not a full lowercase hash`);
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

function gitProofEnvironment() {
  return Object.freeze({
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_TERMINAL_PROMPT: "0",
    HOME: path.parse(repoRoot).root,
    LANG: "C",
    LC_ALL: "C",
    PATH: "/usr/bin:/bin",
  });
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

function parseIdentity(environment) {
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
  return { parsed, sha256: sha256StableJson(parsed) };
}

export function resolveProductionContract(environment) {
  const sourceCommit = exactHash(
    environment,
    "YUNE_WEB06_EXPECTED_SOURCE_COMMIT",
    SHA40,
  );
  const sourceTree = exactHash(
    environment,
    "YUNE_WEB06_EXPECTED_SOURCE_TREE",
    SHA40,
  );
  const archiveSha256 = exactHash(
    environment,
    "YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256",
    SHA64,
  );
  const previewReceiptSha256 = exactHash(
    environment,
    "YUNE_WEB06_GREEN_PREVIEW_RECEIPT_SHA256",
    SHA64,
  );
  const approvalSha256 = exactHash(
    environment,
    "YUNE_WEB06_PRODUCTION_APPROVAL_SHA256",
    SHA64,
  );
  const selectedBranch = required(environment, "YUNE_WEB06_SELECTED_BRANCH");
  if (!["A", "B", "C"].includes(selectedBranch)) {
    throw new Error("Production promotion requires selected branch A, B, or C");
  }
  if (required(environment, "YUNE_WEB06_DISPOSITION") !== "PRODUCTION_REDUCTION") {
    throw new Error("Measured no-go evidence is not production eligible");
  }
  if (
    required(environment, "YUNE_WEB06_PRODUCTION_CONFIRMATION") !==
    APPROVAL_CONFIRMATION
  ) {
    throw new Error(`Production confirmation must be ${APPROVAL_CONFIRMATION}`);
  }
  const approvalRecord = required(
    environment,
    "YUNE_WEB06_PRODUCTION_APPROVAL_RECORD",
  );
  const identity = parseIdentity(environment);
  const finalRole = identity.parsed.roles?.FINAL;
  if (
    finalRole?.sourceCommit !== sourceCommit ||
    finalRole?.sourceTree !== sourceTree ||
    finalRole?.sourceTreeState !== "clean" ||
    finalRole?.archiveSha256 !== archiveSha256 ||
    !SHA64.test(finalRole?.artifactManifestSha256 ?? "") ||
    !SHA64.test(finalRole?.buildInfoSha256 ?? "") ||
    finalRole?.selectedBranch !== selectedBranch ||
    finalRole?.disposition !== "PRODUCTION_REDUCTION"
  ) {
    throw new Error("WEB06 FINAL identity role does not identify the production candidate");
  }
  const evidenceDir = path.resolve(required(environment, "YUNE_WEB06_EVIDENCE_ROOT"));
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
    previewReceiptPath: path.resolve(
      required(environment, "YUNE_WEB06_GREEN_PREVIEW_RECEIPT"),
    ),
    previewReceiptSha256,
    previewCanaryStatusPath: path.resolve(
      required(environment, "YUNE_WEB06_GREEN_PREVIEW_CANARY_STATUS"),
    ),
    approvalPath: path.resolve(
      required(environment, "YUNE_WEB06_PRODUCTION_APPROVAL_FILE"),
    ),
    approvalLedgerRoot: path.resolve(
      required(environment, "YUNE_WEB06_PRODUCTION_APPROVAL_LEDGER_ROOT"),
    ),
    approvalSha256,
    approvalRecord,
    evidenceDir,
    outputDir: path.join(evidenceDir, "path-preflight"),
    accountId: required(environment, "CLOUDFLARE_PRODUCTION_ACCOUNT_ID"),
    apiToken: required(environment, "CLOUDFLARE_PRODUCTION_API_TOKEN"),
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

async function readJsonPlain(file, label) {
  const metadata = await lstat(file);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error(`${label} must be a plain file`);
  }
  const bytes = await readFile(file);
  let payload;
  try {
    payload = JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new Error(`${label} is not valid JSON`);
  }
  return { payload, bytes, sha256: sha256Bytes(bytes) };
}

function checkedPreviewUrl(value) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error("Green preview receipt contains an invalid URL");
  }
  if (
    url.protocol !== "https:" ||
    !url.hostname.endsWith(".pages.dev") ||
    url.hostname === "yune-web.pages.dev" ||
    url.username !== "" ||
    url.password !== "" ||
    url.search !== "" ||
    url.hash !== ""
  ) {
    throw new Error("Green preview receipt does not identify a preview-only pages.dev URL");
  }
  if (!url.pathname.endsWith("/")) url.pathname += "/";
  return url.href;
}

function validWranglerInstallIdentity(identity) {
  return (
    SHA64.test(identity?.nodeExecutableSha256 ?? "") &&
    SHA64.test(identity?.npmCliSha256 ?? "") &&
    SHA64.test(identity?.npmPackageManifestSha256 ?? "") &&
    typeof identity?.npmVersion === "string" &&
    /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(identity.npmVersion)
  );
}

function validWranglerRuntimeIdentity(identity) {
  return (
    identity?.version === WRANGLER_VERSION &&
    identity.resolved ===
      "https://registry.npmjs.org/wrangler/-/wrangler-4.111.0.tgz" &&
    identity.integrity ===
      "sha512-bffpI9EyrnpKkF/1S+RaIv8oRD93GtbsA7TlfWwOsGJGB7VO3jVbdGzpC9TU7Bqom3z7jUxcte4Z9MPhaQ4HoQ==" &&
    SHA64.test(identity.rootPackageSha256 ?? "") &&
    SHA64.test(identity.lockfileSha256 ?? "") &&
    SHA64.test(identity.packageManifestSha256 ?? "") &&
    SHA64.test(identity.entrypointSha256 ?? "") &&
    SHA64.test(identity.nodeExecutableSha256 ?? "") &&
    SHA64.test(identity.installedClosureSha256 ?? "") &&
    Number.isSafeInteger(identity.installedClosureFileCount) &&
    identity.installedClosureFileCount > 0 &&
    Number.isSafeInteger(identity.installedClosureBytes) &&
    identity.installedClosureBytes > 0 &&
    typeof identity.nodeVersion === "string" &&
    /^v\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(identity.nodeVersion)
  );
}

export function validateGreenPreviewReceipt(receipt, expected) {
  const previewUrl = checkedPreviewUrl(receipt?.previewUrl);
  if (
    receipt?.version !== "web06-certified-preview-v1" ||
    receipt.operation !== "preview-only-no-build" ||
    receipt.sourceCommit !== expected.sourceCommit ||
    receipt.sourceTree !== expected.sourceTree ||
    receipt.archiveSha256 !== expected.archiveSha256 ||
    receipt.artifactManifestSha256 !== expected.artifactManifestSha256 ||
    receipt.finalSuiteAttestationSha256 !== expected.finalSuiteAttestationSha256 ||
    !SHA64.test(receipt.localCertificationReceiptSha256 ?? "") ||
    !SHA64.test(receipt.previewCanaryStatusSha256 ?? "") ||
    !SHA64.test(receipt.previewSuiteAttestationSha256 ?? "") ||
    !SHA64.test(receipt.previewAuthorizationConsumptionSha256 ?? "") ||
    receipt.wranglerVersion !== WRANGLER_VERSION ||
    !validWranglerInstallIdentity(receipt.wranglerInstallIdentity) ||
    !validWranglerRuntimeIdentity(receipt.wranglerRuntimeIdentity) ||
    receipt.previewBranch !== `web06-preview-${expected.sourceCommit.slice(0, 12)}` ||
    typeof receipt.previewDeployment?.id !== "string" ||
    receipt.previewDeployment.id === "" ||
    receipt.previewDeployment.url !== previewUrl ||
    receipt.previewDeployment.environment !== "preview" ||
    receipt.previewDeployment.latestStageStatus !== "success" ||
    receipt.previewDeployment.triggerType !== "ad_hoc" ||
    receipt.previewDeployment.branch !== receipt.previewBranch ||
    receipt.previewDeployment.commitHash !== expected.sourceCommit ||
    receipt.previewDeployment.commitDirty !== false ||
    receipt.projectInterlock?.productionBranch !== "main" ||
    receipt.projectInterlock?.productionDeploymentsEnabled !== false ||
    receipt.projectInterlock?.previewDeploymentSetting !== "none" ||
    !Number.isSafeInteger(receipt.remoteManifestFileCount) ||
    receipt.remoteManifestFileCount <= 0 ||
    receipt.previewMutationStarted !== true ||
    receipt.productionPromotionAttempted !== false ||
    receipt.peerPackageAlignment !== "DATA_CONFOUNDED" ||
    receipt.peerRatioStatus !== "OMITTED" ||
    receipt.fullSuccessEligible !== false ||
    receipt.milestoneDisposition !== "PENDING_EXPLICIT_PARTIAL_APPROVAL" ||
    receipt.closeoutAuthorized !== false ||
    receipt.productionPromotionAuthorized !== false ||
    receipt.status !== "passed"
  ) {
    throw new Error("Green preview receipt is not production eligible");
  }
  return { receipt, previewUrl };
}

async function revalidateProductionBundle(
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
    throw new Error("Certified production bundle identity changed after preflight");
  }
  return local;
}

export async function proveExactCurrentSource(contract, execute = runCommand) {
  const cleanEnvironment = gitProofEnvironment();
  const localCommand = async (arguments_) =>
    (await execute(GIT_EXECUTABLE, arguments_, {
      cwd: repoRoot,
      env: cleanEnvironment,
      capture: true,
    })).stdout.trim();
  const remoteCommand = async (arguments_) =>
    (await execute(GIT_EXECUTABLE, arguments_, {
      cwd: path.parse(repoRoot).root,
      env: cleanEnvironment,
      capture: true,
    })).stdout.trim();
  const [head, tree, status, remote] = await Promise.all([
    localCommand(["rev-parse", "HEAD"]),
    localCommand(["rev-parse", "HEAD^{tree}"]),
    localCommand(["status", "--porcelain", "--untracked-files=all"]),
    remoteCommand([
      "ls-remote",
      "--exit-code",
      CANONICAL_REPOSITORY_URL,
      CANONICAL_MAIN_REF,
    ]),
  ]);
  const remoteFields = remote.split(/\s+/u);
  const currentMain =
    remoteFields.length === 2 && remoteFields[1] === CANONICAL_MAIN_REF
      ? remoteFields[0]
      : "";
  if (
    head !== contract.sourceCommit ||
    tree !== contract.sourceTree ||
    status !== "" ||
    currentMain !== contract.sourceCommit
  ) {
    throw new Error(
      "Production promotion requires clean exact HEAD/tree/canonical main equality",
    );
  }
}

async function inspectApprovalLedgerRoot(ledgerRoot) {
  const requested = path.resolve(ledgerRoot);
  const metadata = await lstat(requested);
  const uid = process.getuid?.();
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    (await realpath(requested)) !== requested ||
    !Number.isSafeInteger(uid) ||
    metadata.uid !== uid ||
    (metadata.mode & 0o777) !== 0o700
  ) {
    throw new Error(
      "WEB06 production approval ledger must be canonical, current-uid owned, and mode 0700",
    );
  }
  return Object.freeze({
    dev: metadata.dev,
    ino: metadata.ino,
    uid: metadata.uid,
    mode: metadata.mode & 0o7777,
  });
}

async function assertApprovalLedgerRoot(ledgerRoot, expected) {
  const current = await inspectApprovalLedgerRoot(ledgerRoot);
  if (
    current.dev !== expected.dev ||
    current.ino !== expected.ino ||
    current.uid !== expected.uid ||
    current.mode !== expected.mode
  ) {
    throw new Error("WEB06 production approval ledger identity changed");
  }
}

async function consumeApproval(contract) {
  const approval = await readJsonPlain(
    contract.approvalPath,
    "WEB06 production approval",
  );
  if (approval.sha256 !== contract.approvalSha256) {
    throw new Error("WEB06 production approval bytes changed");
  }
  const declaredNonce =
    typeof approval.payload?.nonce === "string"
      ? approval.payload.nonce
      : "invalid-nonce";
  const consumptionKey = sha256StableJson({
    version: "web06-production-approval-ledger-key-v1",
    approvalSha256: approval.sha256,
    approvalNonce: declaredNonce,
    sourceCommit: contract.sourceCommit,
    sourceTree: contract.sourceTree,
    archiveSha256: contract.archiveSha256,
    artifactManifestSha256:
      contract.identity.parsed.roles.FINAL.artifactManifestSha256,
    previewReceiptSha256: contract.previewReceiptSha256,
  });
  const ledgerIdentity = await inspectApprovalLedgerRoot(
    contract.approvalLedgerRoot,
  );
  const consumptionPath = path.join(
    contract.approvalLedgerRoot,
    `${consumptionKey}.json`,
  );
  let handle;
  try {
    handle = await open(
      consumptionPath,
      fsConstants.O_WRONLY |
        fsConstants.O_CREAT |
        fsConstants.O_EXCL |
        (fsConstants.O_NOFOLLOW ?? 0),
      0o600,
    );
  } catch (error) {
    if (error?.code === "EEXIST") {
      const consumed = new Error(
        "The exact WEB06 production approval has already been consumed",
      );
      consumed.code = "EEXIST";
      throw consumed;
    }
    throw error;
  }
  const receipt = Buffer.from(`${JSON.stringify({
    version: "web06-production-approval-consumption-v1",
    consumptionKey,
    approvalSha256: approval.sha256,
    approvalNonceSha256: sha256Bytes(Buffer.from(declaredNonce)),
    sourceCommit: contract.sourceCommit,
    sourceTree: contract.sourceTree,
    archiveSha256: contract.archiveSha256,
    artifactManifestSha256:
      contract.identity.parsed.roles.FINAL.artifactManifestSha256,
    previewReceiptSha256: contract.previewReceiptSha256,
    consumedAt: new Date().toISOString(),
    status: "consumed",
  }, null, 2)}\n`);
  try {
    await handle.writeFile(receipt);
    await handle.sync();
  } finally {
    await handle.close();
  }
  await assertApprovalLedgerRoot(
    contract.approvalLedgerRoot,
    ledgerIdentity,
  );
  const payload = validateProductionApprovalPayload(approval.payload, {
    sourceCommit: contract.sourceCommit,
    sourceTree: contract.sourceTree,
    archiveSha256: contract.archiveSha256,
    previewReceiptSha256: contract.previewReceiptSha256,
    approvalRecord: contract.approvalRecord,
  });
  return {
    approvalSha256: approval.sha256,
    approvalRecordSha256: sha256Bytes(Buffer.from(contract.approvalRecord)),
    approvalNonceSha256: sha256Bytes(Buffer.from(payload.nonce)),
    consumptionKey,
    consumptionSha256: sha256Bytes(receipt),
  };
}

export function validateProductionApprovalPayload(payload, expected) {
  if (
      payload?.version !== "web06-production-approval-v1" ||
      payload.sourceCommit !== expected.sourceCommit ||
      payload.sourceTree !== expected.sourceTree ||
      payload.archiveSha256 !== expected.archiveSha256 ||
      payload.previewReceiptSha256 !== expected.previewReceiptSha256 ||
      payload.approvalRecord !== expected.approvalRecord ||
      payload.confirmation !== APPROVAL_CONFIRMATION ||
      payload.acceptedMilestoneDisposition !== "PARTIAL" ||
      JSON.stringify(payload.acceptedLimitations) !==
        JSON.stringify(PARTIAL_LIMITATIONS) ||
      payload.previewFullSuccessEligible !== false ||
      !SHA64.test(payload.nonce ?? "") ||
      typeof payload.approvedAt !== "string" ||
      !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(payload.approvedAt)
    ) {
      throw new Error("WEB06 production approval is not bound to this exact promotion");
    }
  return payload;
}

async function listProductionDeployments(contract, fetchImplementation) {
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
    if (!response.ok) throw new Error(`Cloudflare deployment inventory returned HTTP ${response.status}`);
    const payload = await response.json();
    if (payload?.success !== true || !Array.isArray(payload.result)) {
      throw new Error("Cloudflare deployment inventory is malformed");
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
      throw new Error("Cloudflare deployment inventory pagination is malformed");
    }
    if (page === totalPages) return rows;
  }
  throw new Error("Cloudflare deployment inventory exceeded its fail-closed page limit");
}

function matchingNewProductionDeployments(rows, previousIds, sourceCommit) {
  return rows.filter(
    (row) =>
      typeof row?.id === "string" &&
      row.id !== "" &&
      typeof row?.url === "string" &&
      /^https:\/\/[A-Za-z0-9.-]+\.pages\.dev\/?$/.test(row.url) &&
      !previousIds.has(row?.id) &&
      row?.environment === "production" &&
      row?.latest_stage?.status === "success" &&
      row?.deployment_trigger?.type === "ad_hoc" &&
      row?.deployment_trigger?.metadata?.branch === "main" &&
      row?.deployment_trigger?.metadata?.commit_hash === sourceCommit &&
      row?.deployment_trigger?.metadata?.commit_dirty === false,
  );
}

export function assertNoPriorProductionSourceDeployment(rows, sourceCommit) {
  const prior = rows.find(
    (row) =>
      typeof row?.id === "string" &&
      row.id !== "" &&
      row?.environment === "production" &&
      row?.latest_stage?.status === "success" &&
      row?.deployment_trigger?.type === "ad_hoc" &&
      row?.deployment_trigger?.metadata?.branch === "main" &&
      row?.deployment_trigger?.metadata?.commit_hash === sourceCommit &&
      row?.deployment_trigger?.metadata?.commit_dirty === false,
  );
  if (prior !== undefined) {
    const duplicate = new Error(
      "Cloudflare already carries a successful production deployment for this exact source identity",
    );
    duplicate.code = "EEXIST";
    throw duplicate;
  }
}

export async function resolveNewProductionDeployment(
  contract,
  previousIds,
  fetchImplementation,
  delay,
) {
  for (let attempt = 1; attempt <= 6; attempt += 1) {
    const rows = await listProductionDeployments(contract, fetchImplementation);
    const matches = matchingNewProductionDeployments(
      rows,
      previousIds,
      contract.sourceCommit,
    );
    if (matches.length > 1) {
      throw new Error("Cloudflare exposed multiple matching WEB06 production deployments");
    }
    if (matches.length === 1) return matches[0];
    if (attempt < 6) await delay(2_000);
  }
  throw new Error("Cloudflare did not expose a new source-bound production deployment");
}

async function reconcileProductionWithRetry(
  contract,
  local,
  fetchImplementation,
  delay,
) {
  let lastError;
  for (let attempt = 1; attempt <= 6; attempt += 1) {
    try {
      return await reconcileRemoteBundle(
        PRODUCTION_URL,
        local,
        contract.sourceCommit,
        fetchImplementation,
      );
    } catch (error) {
      lastError = error;
      if (attempt < 6) await delay(2_000);
    }
  }
  throw lastError;
}

export async function main(environment = process.env, dependencies = {}) {
  const contract = resolveProductionContract(environment);
  const credentialValues = cloudflareCredentialValues({
    ...process.env,
    ...environment,
  });
  try {
    await lstat(contract.evidenceDir);
    throw new Error("YUNE_WEB06_EVIDENCE_ROOT must be create-new for production");
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
  const execute = dependencies.runCommand ?? runCommand;
  const fetchImplementation = dependencies.fetch ?? globalThis.fetch;
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
  let artifactManifestSha256 = null;
  let finalSuiteAttestationSha256 = null;
  let previewReceipt = null;
  let previewReceiptSha256 = null;
  let previewCanaryStatusSha256 = null;
  let approval = null;
  let productionMutationStarted = false;
  let deployment = null;
  let wranglerInstallIdentity = null;
  let wranglerRuntime = null;
  try {
    await proveExactCurrentSource(contract, execute);
    await validateArchive(contract);
    await extractCertifiedArchive(contract, distRoot, credentialFreeEnvironment);
    await validateArchive(contract);
    const local = await validateLocalBundle(distRoot, contract.sourceCommit);
    artifactManifestSha256 = sha256(local.manifestBytes);
    if (
      contract.identity.parsed.roles.FINAL.artifactManifestSha256 !==
      artifactManifestSha256
    ) {
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
        collectorContractSha256:
          contract.identity.parsed.collectorContractSha256,
        identityManifest: contract.identity.parsed,
        evidenceRoot: contract.finalSuiteEvidenceRoot,
      },
    );
    finalSuiteAttestationSha256 = finalSuite.sha256;

    const preview = await readJsonPlain(
      contract.previewReceiptPath,
      "WEB06 green preview receipt",
    );
    if (preview.sha256 !== contract.previewReceiptSha256) {
      throw new Error("WEB06 green preview receipt bytes changed");
    }
    previewReceiptSha256 = preview.sha256;
    previewReceipt = validateGreenPreviewReceipt(preview.payload, {
      sourceCommit: contract.sourceCommit,
      sourceTree: contract.sourceTree,
      archiveSha256: contract.archiveSha256,
      artifactManifestSha256,
      finalSuiteAttestationSha256,
    });
    const previewCanary = await readJsonPlain(
      contract.previewCanaryStatusPath,
      "WEB06 green preview canary status",
    );
    if (
      previewCanary.sha256 !==
      previewReceipt.receipt.previewCanaryStatusSha256
    ) {
      throw new Error("WEB06 preview receipt does not bind the supplied canary status");
    }
    validatePreviewCanaryStatus(previewCanary.payload, {
      sourceCommit: contract.sourceCommit,
      sourceTree: contract.sourceTree,
      archiveSha256: contract.archiveSha256,
      artifactManifestSha256,
      finalSuiteAttestationSha256,
      previewUrl: previewReceipt.previewUrl,
    });
    if (
      previewCanary.payload.previewSuiteAttestationSha256 !==
      previewReceipt.receipt.previewSuiteAttestationSha256
    ) {
      throw new Error("WEB06 preview suite identity changed after the green canary");
    }
    previewCanaryStatusSha256 = previewCanary.sha256;

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
    approval = await consumeApproval(contract);
    const projectInterlock = await assertCloudflareInterlock(
      contract,
      fetchImplementation,
    );
    const priorDeployments = await listProductionDeployments(
      contract,
      fetchImplementation,
    );
    assertNoPriorProductionSourceDeployment(
      priorDeployments,
      contract.sourceCommit,
    );
    const priorIds = new Set(priorDeployments.map((row) => row?.id).filter(Boolean));

    // Recheck after credential-bearing environment admission and immediately
    // before the single allowed mutation. A superseding main consumes the
    // one-shot approval but cannot deploy stale bytes.
    await proveExactCurrentSource(contract, execute);
    await revalidateProductionBundle(
      contract,
      distRoot,
      artifactManifestSha256,
    );
    await wranglerRuntime.assertCurrent();
    productionMutationStarted = true;
    await execute(
      wranglerRuntime.command,
      [
        ...wranglerRuntime.argumentsPrefix,
        "pages",
        "deploy",
        distRoot,
        "--project-name",
        contract.projectName,
        "--branch",
        "main",
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
    deployment = await resolveNewProductionDeployment(
      contract,
      priorIds,
      fetchImplementation,
      delay,
    );
    await reconcileProductionWithRetry(
      contract,
      local,
      fetchImplementation,
      delay,
    );
    await proveExactCurrentSource(contract, execute);
    await revalidateProductionBundle(
      contract,
      distRoot,
      artifactManifestSha256,
    );
    await rm(distRoot, { recursive: true });
    await writeFile(
      path.join(contract.evidenceDir, "web06-production-promotion-status.json"),
      `${JSON.stringify({
        version: "web06-production-promotion-v1",
        operation: "promote-exact-preview-bytes-no-build",
        sourceCommit: contract.sourceCommit,
        sourceTree: contract.sourceTree,
        archiveSha256: contract.archiveSha256,
        artifactManifestSha256,
        finalSuiteAttestationSha256,
        previewReceiptSha256,
        previewCanaryStatusSha256,
        approvalSha256: approval.approvalSha256,
        approvalRecordSha256: approval.approvalRecordSha256,
        approvalNonceSha256: approval.approvalNonceSha256,
        approvalConsumptionKey: approval.consumptionKey,
        approvalConsumptionSha256: approval.consumptionSha256,
        wranglerVersion: WRANGLER_VERSION,
        wranglerInstallIdentity,
        wranglerRuntimeIdentity: wranglerRuntime.identity,
        projectInterlock,
        deploymentId: deployment.id,
        deploymentUrl: deployment.url,
        productionDeployment: {
          id: deployment.id,
          url: deployment.url,
          environment: deployment.environment,
          latestStageStatus: deployment.latest_stage.status,
          triggerType: deployment.deployment_trigger.type,
          branch: deployment.deployment_trigger.metadata.branch,
          commitHash: deployment.deployment_trigger.metadata.commit_hash,
          commitDirty: deployment.deployment_trigger.metadata.commit_dirty,
        },
        productionUrl: PRODUCTION_URL,
        milestoneDisposition: "PARTIAL",
        peerPackageAlignment: "DATA_CONFOUNDED",
        peerRatioStatus: "OMITTED",
        fullSuccessEligible: false,
        remoteManifestFileCount: local.manifest.files.length,
        buildInvoked: false,
        productionMutationStarted: true,
        status: "passed",
      }, null, 2)}\n`,
      { flag: "wx", mode: 0o600 },
    );
    return { deploymentId: deployment.id, productionUrl: PRODUCTION_URL };
  } catch (error) {
    const failure = redactCredentialValues(
      error instanceof Error ? error.message : String(error),
      credentialValues,
    );
    await writeFile(
      path.join(contract.evidenceDir, "web06-production-promotion-failure.json"),
      `${JSON.stringify({
        version: "web06-production-promotion-v1",
        operation: "promote-exact-preview-bytes-no-build",
        sourceCommit: contract.sourceCommit,
        sourceTree: contract.sourceTree,
        archiveSha256: contract.archiveSha256,
        artifactManifestSha256,
        finalSuiteAttestationSha256,
        previewReceiptSha256,
        previewCanaryStatusSha256,
        approvalSha256: approval?.approvalSha256 ?? null,
        approvalConsumptionKey: approval?.consumptionKey ?? null,
        wranglerInstallIdentity,
        wranglerRuntimeIdentity: wranglerRuntime?.identity ?? null,
        productionMutationStarted,
        deploymentId: deployment?.id ?? null,
        buildInvoked: false,
        status: "failed",
        failure,
      }, null, 2)}\n`,
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
