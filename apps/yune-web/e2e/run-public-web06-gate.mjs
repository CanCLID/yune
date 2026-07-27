import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createReadStream } from "node:fs";
import {
  lstat,
  mkdir,
  readFile,
  realpath,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  sha256,
  validateLocalBundle,
} from "./public-artifact-verifier.mjs";
import {
  WEB06_PREVIEW_SCENARIOS,
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
const playwrightCli = path.join(
  e2eRoot,
  "node_modules",
  "@playwright",
  "test",
  "cli.js",
);
const web06Config = path.join(e2eRoot, "playwright.web06.config.ts");
const web06Spec = path.join(e2eRoot, "yune-web06-smoothness.spec.ts");
const statusName = "web06-public-gate-status.json";

export const gateScopes = Object.freeze({
  "release-certification": Object.freeze({
    grep: "@web06-preview-canary",
    servesLocalArtifact: true,
  }),
  "preview-canary": Object.freeze({
    grep: "@web06-preview-canary",
    servesLocalArtifact: false,
  }),
});

function requiredString(environment, name) {
  const value = environment[name]?.trim();
  if (!value) throw new Error(`${name} is required`);
  if (value.includes("\0") || value.includes("\r") || value.includes("\n")) {
    throw new Error(`${name} must not contain control characters`);
  }
  return value;
}

export function withoutCloudflareCredentials(environment) {
  return Object.fromEntries(
    Object.entries(environment).filter(
      ([name]) =>
        !name.startsWith("CLOUDFLARE_") &&
        !name.startsWith("CF_") &&
        !name.startsWith("WRANGLER_"),
    ),
  );
}

function fullSha(value, label, length) {
  const pattern = new RegExp(`^[0-9a-f]{${length}}$`);
  if (!pattern.test(value)) {
    throw new Error(`${label} must be a full lowercase ${length}-character SHA`);
  }
  return value;
}

function selectedOptions(arguments_, environment) {
  let commandLineScope = null;
  let verifyOnly = false;
  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    if (argument === "--scope") {
      if (commandLineScope !== null || index + 1 >= arguments_.length) {
        throw new Error("--scope must be supplied exactly once with a value");
      }
      commandLineScope = arguments_[index + 1];
      index += 1;
    } else if (argument.startsWith("--scope=")) {
      if (commandLineScope !== null) {
        throw new Error("--scope must be supplied exactly once");
      }
      commandLineScope = argument.slice("--scope=".length);
    } else if (argument === "--verify-only") {
      if (verifyOnly) throw new Error("--verify-only must be supplied at most once");
      verifyOnly = true;
    } else {
      throw new Error(`Unknown WEB06 public gate argument: ${argument}`);
    }
  }
  const environmentScope = environment.YUNE_WEB06_GATE_SCOPE?.trim() || null;
  if (
    commandLineScope !== null &&
    environmentScope !== null &&
    commandLineScope !== environmentScope
  ) {
    throw new Error("Command-line and environment WEB06 gate scopes disagree");
  }
  const scope = commandLineScope ?? environmentScope ?? "release-certification";
  if (!Object.hasOwn(gateScopes, scope)) {
    throw new Error(`Unsupported YUNE_WEB06_GATE_SCOPE: ${scope}`);
  }
  return { scope, verifyOnly };
}

function checkedRemoteUrl(value) {
  let parsed;
  try {
    parsed = new URL(value);
  } catch {
    throw new Error(`YUNE_WEB_APP_URL is not a valid URL: ${value}`);
  }
  if (
    parsed.protocol !== "https:" ||
    parsed.username !== "" ||
    parsed.password !== "" ||
    parsed.hash !== "" ||
    parsed.search !== ""
  ) {
    throw new Error(
      "YUNE_WEB_APP_URL must be an HTTPS URL without credentials, query, or fragment",
    );
  }
  if (!parsed.pathname.endsWith("/")) parsed.pathname += "/";
  return parsed.href;
}

export function resolveGateContract(environment, arguments_ = []) {
  const { scope, verifyOnly } = selectedOptions(arguments_, environment);
  const scopeContract = gateScopes[scope];
  const expectedSourceCommit = fullSha(
    requiredString(environment, "YUNE_WEB06_EXPECTED_SOURCE_COMMIT"),
    "YUNE_WEB06_EXPECTED_SOURCE_COMMIT",
    40,
  );
  const expectedSourceTree = fullSha(
    requiredString(environment, "YUNE_WEB06_EXPECTED_SOURCE_TREE"),
    "YUNE_WEB06_EXPECTED_SOURCE_TREE",
    40,
  );
  const archiveSha256 = fullSha(
    requiredString(environment, "YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256"),
    "YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256",
    64,
  );
  const archivePath = path.resolve(
    requiredString(environment, "YUNE_WEB06_CERTIFIED_ARCHIVE"),
  );
  const evidenceDir = path.resolve(
    requiredString(environment, "YUNE_WEB06_EVIDENCE_ROOT"),
  );
  const outputDir = path.resolve(
    environment.YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR?.trim() ||
      path.join(evidenceDir, "playwright"),
  );
  const outputRelative = path.relative(evidenceDir, outputDir);
  if (
    outputRelative === "" ||
    outputRelative === ".." ||
    outputRelative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(outputRelative)
  ) {
    throw new Error(
      "YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR must be a strict descendant of the external evidence root",
    );
  }
  if (environment.YUNE_WEB06_DIST_ROOT?.trim()) {
    throw new Error(
      "YUNE_WEB06_DIST_ROOT is runner-owned; supplied artifact roots are forbidden",
    );
  }
  const selectedBranch = requiredString(environment, "YUNE_WEB06_SELECTED_BRANCH");
  if (!["A", "B", "C"].includes(selectedBranch)) {
    throw new Error("A preview-eligible YUNE_WEB06_SELECTED_BRANCH must be A, B, or C");
  }
  const disposition = requiredString(environment, "YUNE_WEB06_DISPOSITION");
  if (disposition !== "PRODUCTION_REDUCTION") {
    throw new Error("WEB06 preview certification requires PRODUCTION_REDUCTION");
  }
  const identityManifestJson = requiredString(
    environment,
    "YUNE_WEB06_IDENTITY_MANIFEST_JSON",
  );
  let identityManifest;
  try {
    identityManifest = JSON.parse(identityManifestJson);
  } catch {
    throw new Error("YUNE_WEB06_IDENTITY_MANIFEST_JSON must be valid JSON");
  }
  if (
    identityManifest?.version !== "web06-target-identities-v1" ||
    identityManifest.metricContractVersion !== "web06-metric-v1" ||
    identityManifest.scenarioRegistryVersion !== "web06-scenarios-v1" ||
    identityManifest.behaviorPredicateVersion !== "web06-behavior-predicates-v1" ||
    !/^[0-9a-f]{64}$/.test(identityManifest.collectorContractSha256 ?? "")
  ) {
    throw new Error("YUNE_WEB06_IDENTITY_MANIFEST_JSON does not match the frozen contract");
  }
  const identityManifestSha256 = sha256StableJson(identityManifest);
  const finalSuiteAttestationPath = path.resolve(
    requiredString(environment, "YUNE_WEB06_FINAL_SUITE_ATTESTATION"),
  );
  const finalSuiteEvidenceRoot = path.resolve(
    requiredString(environment, "YUNE_WEB06_FINAL_SUITE_EVIDENCE_ROOT"),
  );
  let runEnvironmentJson = null;
  let runEnvironmentManifest = null;
  if (!verifyOnly) {
    runEnvironmentJson = requiredString(
      environment,
      "YUNE_WEB06_RUN_ENVIRONMENT_JSON",
    );
    try {
      runEnvironmentManifest = JSON.parse(runEnvironmentJson);
    } catch {
      throw new Error("YUNE_WEB06_RUN_ENVIRONMENT_JSON must be valid JSON");
    }
    if (
      !runEnvironmentManifest ||
      typeof runEnvironmentManifest !== "object" ||
      Array.isArray(runEnvironmentManifest)
    ) {
      throw new Error("YUNE_WEB06_RUN_ENVIRONMENT_JSON must be an object");
    }
  }
  if (environment.YUNE_WEB06_BLOCKED_SCENARIOS_JSON !== undefined) {
    let blocked;
    try {
      blocked = JSON.parse(environment.YUNE_WEB06_BLOCKED_SCENARIOS_JSON);
    } catch {
      throw new Error("YUNE_WEB06_BLOCKED_SCENARIOS_JSON must be valid JSON");
    }
    if (!Array.isArray(blocked) || blocked.length !== 0) {
      throw new Error("WEB06 release and preview execution forbids blocked scenarios");
    }
  }

  if (scopeContract.servesLocalArtifact) {
    const portValue = environment.YUNE_WEB06_PREVIEW_PORT?.trim() || "4174";
    const port = Number(portValue);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      throw new Error(`Invalid YUNE_WEB06_PREVIEW_PORT: ${portValue}`);
    }
    return {
      scope,
      grep: scopeContract.grep,
      expectedSourceCommit,
      expectedSourceTree,
      archiveSha256,
      archivePath,
      evidenceDir,
      outputDir,
      distRoot: null,
      appUrl: `http://127.0.0.1:${port}/`,
      port,
      expectedPreviewScenarios: null,
      selectedBranch,
      disposition,
      identityManifest,
      identityManifestJson,
      identityManifestSha256,
      finalSuiteAttestationPath,
      finalSuiteEvidenceRoot,
      runEnvironmentJson,
      runEnvironmentManifest,
      verifyOnly,
      statusName: verifyOnly ? "web06-local-reconciliation-status.json" : statusName,
    };
  }

  return {
    scope,
    grep: scopeContract.grep,
    expectedSourceCommit,
    expectedSourceTree,
    archiveSha256,
    archivePath,
    evidenceDir,
    outputDir,
    distRoot: null,
    appUrl: checkedRemoteUrl(requiredString(environment, "YUNE_WEB_APP_URL")),
    port: null,
    expectedPreviewScenarios: [...WEB06_PREVIEW_SCENARIOS],
    selectedBranch,
    disposition,
    identityManifest,
    identityManifestJson,
    identityManifestSha256,
    finalSuiteAttestationPath,
    finalSuiteEvidenceRoot,
    runEnvironmentJson,
    runEnvironmentManifest,
    verifyOnly,
    statusName: verifyOnly ? "web06-preview-reconciliation-status.json" : statusName,
  };
}

export function validateRemoteBuildInfo(buildInfo, expectedSourceCommit) {
  if (
    buildInfo?.sourceCommit !== expectedSourceCommit ||
    buildInfo?.sourceTreeState !== "clean"
  ) {
    throw new Error("Preview build-info does not identify the expected clean source commit");
  }
  if (!/^[0-9a-f]{64}$/.test(buildInfo.publicArtifactManifestSha256 ?? "")) {
    throw new Error("Preview build-info is missing its public artifact manifest hash");
  }
  return buildInfo;
}

function runCommand(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      env: options.env,
      stdio: options.stdio ?? "inherit",
    });
    let stdout = "";
    let stderr = "";
    if (options.stdio === "pipe") {
      child.stdout.setEncoding("utf8");
      child.stderr.setEncoding("utf8");
      child.stdout.on("data", (chunk) => {
        stdout += chunk;
      });
      child.stderr.on("data", (chunk) => {
        stderr += chunk;
      });
    }
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolve({ stdout, stderr });
      } else {
        reject(
          new Error(
            `${command} ${args.join(" ")} exited with code ${code}, signal ${signal}: ${stderr.trim()}`,
          ),
        );
      }
    });
  });
}

export async function proveExactPublicGateRunnerSource(
  contract,
  environment,
  execute = runCommand,
) {
  const command = async (args) =>
    (await execute("git", args, {
      cwd: repoRoot,
      env: withoutCloudflareCredentials(environment),
      stdio: "pipe",
    })).stdout.trim();
  const [head, tree, status] = await Promise.all([
    command(["rev-parse", "HEAD"]),
    command(["rev-parse", "HEAD^{tree}"]),
    command(["status", "--porcelain", "--untracked-files=all"]),
  ]);
  if (
    head !== contract.expectedSourceCommit ||
    tree !== contract.expectedSourceTree ||
    status !== ""
  ) {
    throw new Error("WEB06 public gate requires clean exact source HEAD/tree");
  }
  return Object.freeze({ sourceCommit: head, sourceTree: tree });
}

async function validateExternalEvidenceDir(evidenceDir, environment) {
  const result = await runCommand(
    "python3",
    [
      path.join(repoRoot, "scripts", "evidence-output-path.py"),
      "validate",
      "--repo-root",
      repoRoot,
      "--path",
      evidenceDir,
    ],
    {
      cwd: repoRoot,
      env: { ...environment, PYTHONDONTWRITEBYTECODE: "1" },
      stdio: "pipe",
    },
  );
  const validated = result.stdout.trim();
  if (!path.isAbsolute(validated)) {
    throw new Error("Evidence path validator did not return an absolute destination");
  }
  return path.normalize(validated);
}

async function assertNoSymlinkComponents(absolutePath, label) {
  const resolved = path.resolve(absolutePath);
  const parsed = path.parse(resolved);
  let probe = parsed.root;
  const darwinRootAliases =
    process.platform === "darwin" ? new Set(["/etc", "/tmp", "/var"]) : new Set();
  for (const component of resolved.slice(parsed.root.length).split(path.sep)) {
    if (!component) continue;
    probe = path.join(probe, component);
    let metadata;
    try {
      metadata = await lstat(probe);
    } catch (error) {
      if (error?.code === "ENOENT") return;
      throw error;
    }
    if (metadata.isSymbolicLink()) {
      if (darwinRootAliases.has(probe)) continue;
      throw new Error(`${label} traverses a symbolic link: ${probe}`);
    }
  }
}

function assertStrictDescendant(root, candidate, label) {
  const relative = path.relative(root, candidate);
  if (
    relative === "" ||
    relative === ".." ||
    relative.startsWith(`..${path.sep}`) ||
    path.isAbsolute(relative)
  ) {
    throw new Error(`${label} must be a strict descendant of ${root}`);
  }
}

function directoryIdentity(metadata) {
  return Object.freeze({
    dev: metadata.dev,
    ino: metadata.ino,
    uid: metadata.uid,
    mode: metadata.mode & 0o7777,
  });
}

function sameDirectoryIdentity(left, right) {
  return (
    left.dev === right.dev &&
    left.ino === right.ino &&
    left.uid === right.uid &&
    left.mode === right.mode
  );
}

async function inspectPrivateDirectory(directory, label) {
  const requested = path.resolve(directory);
  const metadata = await lstat(requested);
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    (await realpath(requested)) !== requested
  ) {
    throw new Error(`${label} must be a canonical plain directory`);
  }
  const uid = process.getuid?.();
  if (!Number.isSafeInteger(uid) || metadata.uid !== uid) {
    throw new Error(`${label} must be owned by the current uid`);
  }
  if ((metadata.mode & 0o777) !== 0o700) {
    throw new Error(`${label} must have mode 0700`);
  }
  return directoryIdentity(metadata);
}

async function assertDirectoryIdentity(directory, expected, label) {
  const current = await inspectPrivateDirectory(directory, label);
  if (!sameDirectoryIdentity(current, expected)) {
    throw new Error(`${label} identity changed after reservation`);
  }
}

async function createPrivateDirectoryExclusive(directory, label) {
  try {
    await mkdir(directory, { mode: 0o700 });
  } catch (error) {
    if (error?.code === "EEXIST") {
      const existing = new Error(`${label} must be create-new`);
      existing.code = "EEXIST";
      throw existing;
    }
    throw error;
  }
  return inspectPrivateDirectory(directory, label);
}

async function reserveEvidenceDirectories(
  evidenceDir,
  outputDir,
  { beforeEvidenceMkdir = async () => {} } = {},
) {
  const parent = path.dirname(evidenceDir);
  const parentIdentity = await inspectPrivateDirectory(
    parent,
    "WEB06 trusted evidence parent",
  );
  await assertDirectoryIdentity(
    parent,
    parentIdentity,
    "WEB06 trusted evidence parent",
  );
  await beforeEvidenceMkdir();
  const evidenceIdentity = await createPrivateDirectoryExclusive(
    evidenceDir,
    "WEB06 evidence root",
  );
  await assertDirectoryIdentity(
    parent,
    parentIdentity,
    "WEB06 trusted evidence parent",
  );

  const relative = path.relative(evidenceDir, outputDir);
  let current = evidenceDir;
  for (const component of relative.split(path.sep)) {
    current = path.join(current, component);
    await createPrivateDirectoryExclusive(
      current,
      "WEB06 Playwright output component",
    );
    await assertDirectoryIdentity(
      evidenceDir,
      evidenceIdentity,
      "WEB06 evidence root",
    );
  }
  const outputIdentity = await inspectPrivateDirectory(
    outputDir,
    "WEB06 Playwright output",
  );
  await assertDirectoryIdentity(
    parent,
    parentIdentity,
    "WEB06 trusted evidence parent",
  );
  return Object.freeze({
    evidenceDir,
    evidenceIdentity,
    outputDir,
    outputIdentity,
    parent,
    parentIdentity,
  });
}

export async function assertPreparedOutputPaths(contract) {
  const reservation = contract.outputReservation;
  if (!reservation) {
    throw new Error("WEB06 output paths were not reserved");
  }
  await assertDirectoryIdentity(
    reservation.parent,
    reservation.parentIdentity,
    "WEB06 trusted evidence parent",
  );
  await assertDirectoryIdentity(
    reservation.evidenceDir,
    reservation.evidenceIdentity,
    "WEB06 evidence root",
  );
  await assertDirectoryIdentity(
    reservation.outputDir,
    reservation.outputIdentity,
    "WEB06 Playwright output",
  );
}

export async function prepareOutputPaths(
  contract,
  environment = process.env,
  dependencies = {},
) {
  const outputRelative = path.relative(contract.evidenceDir, contract.outputDir);
  await assertNoSymlinkComponents(contract.evidenceDir, "WEB06 evidence root");
  await assertNoSymlinkComponents(contract.outputDir, "WEB06 Playwright output");
  contract.evidenceDir = await validateExternalEvidenceDir(
    contract.evidenceDir,
    environment,
  );
  contract.outputDir = path.join(contract.evidenceDir, outputRelative);
  assertStrictDescendant(
    contract.evidenceDir,
    contract.outputDir,
    "YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR",
  );

  contract.outputReservation = await reserveEvidenceDirectories(
    contract.evidenceDir,
    contract.outputDir,
    dependencies,
  );
  await assertPreparedOutputPaths(contract);
  return contract.outputReservation;
}

async function sha256File(file) {
  return new Promise((resolve, reject) => {
    const hash = createHash("sha256");
    const stream = createReadStream(file);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.once("error", reject);
    stream.once("end", () => resolve(hash.digest("hex")));
  });
}

export async function validateArchive(contract) {
  const [metadata, siblingMetadata] = await Promise.all([
    lstat(contract.archivePath),
    lstat(`${contract.archivePath}.sha256`),
  ]);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error("YUNE_WEB06_CERTIFIED_ARCHIVE must be a plain archive file");
  }
  if (!siblingMetadata.isFile() || siblingMetadata.isSymbolicLink()) {
    throw new Error("Certified archive sibling digest must be a plain file");
  }
  const [actual, siblingBytes] = await Promise.all([
    sha256File(contract.archivePath),
    readFile(`${contract.archivePath}.sha256`, "utf8"),
  ]);
  const sibling = siblingBytes.trim();
  if (!/^[0-9a-f]{64}$/.test(sibling)) {
    throw new Error("Certified archive sibling digest is invalid");
  }
  if (actual !== contract.archiveSha256 || actual !== sibling) {
    throw new Error("Certified archive bytes do not match their frozen SHA-256 identity");
  }
}

const safeExtractionProgram = String.raw`
import os
import pathlib
import tarfile

archive = pathlib.Path(os.environ["YUNE_WEB_ARCHIVE_TO_EXTRACT"])
destination = pathlib.Path(os.environ["YUNE_WEB_ARCHIVE_DESTINATION"])
destination.mkdir(parents=True, exist_ok=False)
seen = set()
with tarfile.open(archive, "r:gz") as bundle:
    members = bundle.getmembers()
    for member in members:
        relative = pathlib.PurePosixPath(member.name)
        normalized = relative.as_posix()
        if normalized == "." and member.isdir():
            continue
        if (
            relative.is_absolute()
            or ".." in relative.parts
            or "\\" in member.name
            or not (member.isfile() or member.isdir())
            or normalized == "."
            or normalized in seen
        ):
            raise SystemExit(f"unsafe archive member: {member.name!r}")
        seen.add(normalized)
    bundle.extractall(destination, members=members, filter="data")
`;

function extractionDestination(contract) {
  const suffix = contract.verifyOnly ? "preview-reconciliation" : contract.scope;
  return path.join(contract.evidenceDir, `.web06-sealed-artifact-${suffix}`);
}

export async function extractCertifiedArchive(
  contract,
  destination,
  environment = process.env,
) {
  await assertNoSymlinkComponents(destination, "WEB06 archive extraction root");
  await runCommand("python3", ["-c", safeExtractionProgram], {
    cwd: repoRoot,
    env: {
      ...environment,
      PYTHONDONTWRITEBYTECODE: "1",
      YUNE_WEB_ARCHIVE_TO_EXTRACT: contract.archivePath,
      YUNE_WEB_ARCHIVE_DESTINATION: destination,
    },
    stdio: "pipe",
  });
  await assertNoSymlinkComponents(destination, "WEB06 archive extraction root");
  const canonical = await realpath(destination);
  if (path.normalize(canonical) !== path.normalize(destination)) {
    throw new Error("WEB06 archive extraction root escaped its canonical path");
  }
  assertStrictDescendant(
    contract.evidenceDir,
    canonical,
    "WEB06 archive extraction root",
  );
  return canonical;
}

async function fetchBuildInfo(appUrl, expectedSourceCommit) {
  const response = await fetch(new URL("build-info.json", appUrl), {
    cache: "no-store",
    signal: AbortSignal.timeout(20_000),
  });
  if (!response.ok) {
    throw new Error(`Could not read preview build-info.json: HTTP ${response.status}`);
  }
  return validateRemoteBuildInfo(await response.json(), expectedSourceCommit);
}

async function fetchBytes(url, label, fetchImplementation = globalThis.fetch) {
  const response = await fetchImplementation(url, {
    cache: "no-store",
    signal: AbortSignal.timeout(30_000),
  });
  if (!response.ok) {
    throw new Error(`Could not read deployed ${label}: HTTP ${response.status}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

function deployedArtifactUrl(appUrl, relative) {
  const encoded = relative.split("/").map(encodeURIComponent).join("/");
  return new URL(encoded, appUrl);
}

export function validateRemoteMetadata(
  remoteBuildInfoBytes,
  remoteManifestBytes,
  local,
  expectedSourceCommit,
) {
  let remoteBuildInfo;
  try {
    remoteBuildInfo = JSON.parse(remoteBuildInfoBytes.toString("utf8"));
  } catch {
    throw new Error("Deployed build-info.json is not valid JSON");
  }
  validateRemoteBuildInfo(remoteBuildInfo, expectedSourceCommit);
  if (!remoteBuildInfoBytes.equals(local.buildInfoBytes)) {
    throw new Error("Deployed build-info.json differs from the sealed artifact");
  }
  if (!remoteManifestBytes.equals(local.manifestBytes)) {
    throw new Error("Deployed public artifact manifest differs from the sealed artifact");
  }
  if (sha256(remoteManifestBytes) !== remoteBuildInfo.publicArtifactManifestSha256) {
    throw new Error("Deployed public artifact manifest hash disagrees with build-info.json");
  }
  return remoteBuildInfo;
}

export function validateRemoteFile(file, bytes) {
  if (bytes.byteLength !== file.bytes || sha256(bytes) !== file.sha256) {
    throw new Error(`Deployed public artifact ${file.path} differs from the sealed inventory`);
  }
}

export async function reconcileRemoteBundle(
  appUrl,
  local,
  expectedSourceCommit,
  fetchImplementation = globalThis.fetch,
) {
  const [remoteBuildInfoBytes, remoteManifestBytes] = await Promise.all([
    fetchBytes(
      new URL("build-info.json", appUrl),
      "build-info.json",
      fetchImplementation,
    ),
    fetchBytes(
      new URL("public-artifact-manifest.json", appUrl),
      "public-artifact-manifest.json",
      fetchImplementation,
    ),
  ]);
  const remoteBuildInfo = validateRemoteMetadata(
    remoteBuildInfoBytes,
    remoteManifestBytes,
    local,
    expectedSourceCommit,
  );

  let nextIndex = 0;
  const workers = Array.from(
    { length: Math.min(4, local.manifest.files.length) },
    async () => {
      while (nextIndex < local.manifest.files.length) {
        const index = nextIndex;
        nextIndex += 1;
        const file = local.manifest.files[index];
        const bytes = await fetchBytes(
          deployedArtifactUrl(appUrl, file.path),
          `artifact file ${file.path}`,
          fetchImplementation,
        );
        validateRemoteFile(file, bytes);
      }
    },
  );
  await Promise.all(workers);
  return remoteBuildInfo;
}

async function writeStatus(contract, values) {
  await assertPreparedOutputPaths(contract);
  const statusPath = path.join(contract.evidenceDir, contract.statusName);
  await writeFile(
    statusPath,
    `${JSON.stringify(
      {
        version: "web06-public-gate-status-v1",
        sourceCommit: contract.expectedSourceCommit,
        sourceTree: contract.expectedSourceTree,
        archiveSha256: contract.archiveSha256,
        disposition: contract.disposition,
        scope: contract.scope,
        appUrl: contract.appUrl,
        ...values,
      },
      null,
      2,
    )}\n`,
    { flag: "wx", mode: 0o600 },
  );
  await assertPreparedOutputPaths(contract);
}

export async function main(
  environment = process.env,
  arguments_ = process.argv.slice(2),
  dependencies = {},
) {
  const contract = resolveGateContract(environment, arguments_);
  const readSuiteAttestation =
    dependencies.readAndValidateSuiteAttestation ??
    readAndValidateSuiteAttestation;
  const childEnvironment = withoutCloudflareCredentials({
    ...process.env,
    ...environment,
  });
  if (!childEnvironment.PATH && process.env.PATH) {
    childEnvironment.PATH = process.env.PATH;
  }
  let runnerSourceManifest = null;
  if (!contract.verifyOnly) {
    const runnerSource = await proveExactPublicGateRunnerSource(
      contract,
      childEnvironment,
    );
    runnerSourceManifest = await createRunnerSourceManifest(
      repoRoot,
      runnerSource.sourceCommit,
      runnerSource.sourceTree,
    );
  }
  await prepareOutputPaths(contract, childEnvironment);

  let preview = null;
  let artifactManifestSha256 = null;
  let artifactFileCount = null;
  let canaryStarted = false;
  let finalSuiteAttestationSha256 = null;
  let previewSuiteAttestationSha256 = null;
  try {
    await validateArchive(contract);
    contract.distRoot = extractionDestination(contract);
    contract.distRoot = await extractCertifiedArchive(
      contract,
      contract.distRoot,
      childEnvironment,
    );
    // Catch any mutation between the pre-extraction digest and the bytes that
    // Python opened. The runner never measures a root unless both checks agree.
    await validateArchive(contract);
    if (!contract.verifyOnly) {
      await Promise.all([stat(playwrightCli), stat(web06Config), stat(web06Spec)]);
    }
    const metadata = await lstat(contract.distRoot);
    if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
      throw new Error("YUNE_WEB06_DIST_ROOT must be a plain extracted directory");
    }
    const local = await validateLocalBundle(
      contract.distRoot,
      contract.expectedSourceCommit,
    );
    artifactManifestSha256 = sha256(local.manifestBytes);
    artifactFileCount = local.manifest.files.length;
    const finalAttestation = await readSuiteAttestation(
      contract.finalSuiteAttestationPath,
      {
        expectation: "FINAL",
        sourceCommit: contract.expectedSourceCommit,
        sourceTree: contract.expectedSourceTree,
        archiveSha256: contract.archiveSha256,
        artifactManifestSha256,
        selectedBranch: contract.selectedBranch,
        disposition: "PRODUCTION_REDUCTION",
        identityManifestSha256: contract.identityManifestSha256,
        collectorContractSha256:
          contract.identityManifest.collectorContractSha256,
        identityManifest: contract.identityManifest,
        evidenceRoot: contract.finalSuiteEvidenceRoot,
      },
    );
    finalSuiteAttestationSha256 = finalAttestation.sha256;
    const finalRole = contract.identityManifest.roles?.FINAL;
    if (
      finalRole?.sourceCommit !== contract.expectedSourceCommit ||
      finalRole?.sourceTree !== contract.expectedSourceTree ||
      finalRole?.sourceTreeState !== "clean" ||
      finalRole?.archiveSha256 !== contract.archiveSha256 ||
      finalRole?.artifactManifestSha256 !== artifactManifestSha256 ||
      finalRole?.buildInfoSha256 !== sha256(local.buildInfoBytes) ||
      finalRole?.selectedBranch !== contract.selectedBranch ||
      finalRole?.disposition !== "PRODUCTION_REDUCTION"
    ) {
      throw new Error("WEB06 FINAL identity role does not match the sealed release candidate");
    }
    if (contract.scope === "release-certification" && !contract.verifyOnly) {
      const snapshot = await loadSealedArtifactSnapshot(contract.distRoot, local);
      preview = await startSealedArtifactServer(snapshot, {
        host: "127.0.0.1",
        port: contract.port,
      });
      if (preview.appUrl !== contract.appUrl) {
        throw new Error("Sealed artifact server bound an unexpected origin");
      }
      await fetchBuildInfo(contract.appUrl, contract.expectedSourceCommit);
      preview.assertHealthy();
    } else if (contract.scope === "preview-canary") {
      const buildInfo = await reconcileRemoteBundle(
        contract.appUrl,
        local,
        contract.expectedSourceCommit,
      );
      if (buildInfo.publicArtifactManifestSha256 !== artifactManifestSha256) {
        throw new Error("Deployed and sealed artifact manifest identities disagree");
      }
    }

    if (contract.verifyOnly) {
      await rm(contract.distRoot, { recursive: true });
      contract.distRoot = null;
      await writeStatus(contract, {
        generatedAt: new Date().toISOString(),
        artifactManifestSha256,
        artifactFileCount,
        finalSuiteAttestationSha256,
        extractedArtifactRetained: false,
        canaryStarted: false,
        status: "reconciled",
      });
      return;
    }

    const collectorEvidenceRoot = path.join(contract.evidenceDir, "collector");
    await createPrivateDirectoryExclusive(
      collectorEvidenceRoot,
      "WEB06 collector evidence root",
    );
    const previewAttestationPath = path.join(
      collectorEvidenceRoot,
      "suite-attestation.json",
    );
    const targets = {
      FINAL_MINIMAL: {
        origin: contract.appUrl,
        sourceCommit: contract.expectedSourceCommit,
        sourceTree: contract.expectedSourceTree,
        treeState: "clean",
        artifactSha256: artifactManifestSha256,
        archiveSha256: contract.archiveSha256,
        buildInfoSha256: sha256(local.buildInfoBytes),
        artifactResponseGuard: sealedArtifactResponseGuard(local),
        protocolMode: "minimal",
        selectorPolicy: "omitted",
      },
    };
    canaryStarted = true;
    let browserFailure = null;
    try {
      await runCommand(
        process.execPath,
        [
          playwrightCli,
          "test",
          "--config",
          path.basename(web06Config),
          "--grep",
          contract.grep,
          "--workers=1",
          "--retries=0",
        ],
        {
          cwd: e2eRoot,
          env: {
            ...childEnvironment,
            YUNE_WEB_APP_URL: contract.appUrl,
            YUNE_WEB06_EXPECTED_SOURCE_COMMIT: contract.expectedSourceCommit,
            YUNE_WEB06_EXPECTED_SOURCE_TREE: contract.expectedSourceTree,
            YUNE_WEB06_CERTIFIED_ARCHIVE: contract.archivePath,
            YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256: contract.archiveSha256,
            YUNE_WEB06_GATE_SCOPE: contract.scope,
            YUNE_WEB06_EVIDENCE_ROOT: collectorEvidenceRoot,
            YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR: contract.outputDir,
            YUNE_WEB06_EXPECTATION: "PREVIEW",
            YUNE_WEB06_RUN_KIND: "preview-canary",
            YUNE_WEB06_SELECTED_BRANCH: contract.selectedBranch,
            YUNE_WEB06_DISPOSITION: contract.disposition,
            YUNE_WEB06_IDENTITY_MANIFEST_JSON: contract.identityManifestJson,
            YUNE_WEB06_RUN_ENVIRONMENT_JSON: contract.runEnvironmentJson,
            YUNE_WEB06_RUNNER_SOURCE_JSON: JSON.stringify(runnerSourceManifest),
            YUNE_WEB06_BLOCKED_SCENARIOS_JSON: "[]",
            YUNE_WEB06_TARGETS_JSON: JSON.stringify(targets),
            YUNE_WEB06_TARGET_ORDER_JSON: JSON.stringify(["FINAL_MINIMAL"]),
            YUNE_WEB06_SCENARIOS_JSON: JSON.stringify(WEB06_PREVIEW_SCENARIOS),
            YUNE_WEB06_PLAYWRIGHT_RETRIES: "0",
            YUNE_WEB06_PLAYWRIGHT_WORKERS: "1",
            YUNE_WEB06_RUN_ID: `${contract.scope}-canary`,
            YUNE_WEB06_SUITE_ATTESTATION_PATH: previewAttestationPath,
            YUNE_WEB06_COLLECTOR_OUTPUT_PATH: path.join(
              collectorEvidenceRoot,
              "collector-output.json",
            ),
            YUNE_WEB06_INDEPENDENT_RECOMPUTE_PATH: path.join(
              collectorEvidenceRoot,
              "independent-recompute.json",
            ),
            YUNE_WEB06_DIST_ROOT: contract.distRoot,
          },
        },
      );
    } catch (error) {
      browserFailure = error;
    }
    if (preview !== null) preview.assertHealthy();
    if (browserFailure !== null) throw browserFailure;
    await proveExactPublicGateRunnerSource(contract, childEnvironment);
    const previewAttestation = await readSuiteAttestation(
      previewAttestationPath,
      {
        expectation: "PREVIEW",
        sourceCommit: contract.expectedSourceCommit,
        sourceTree: contract.expectedSourceTree,
        archiveSha256: contract.archiveSha256,
        artifactManifestSha256,
        selectedBranch: contract.selectedBranch,
        disposition: contract.disposition,
        identityManifestSha256: contract.identityManifestSha256,
        collectorContractSha256:
          contract.identityManifest.collectorContractSha256,
        environmentManifestSha256: sha256Bytes(
          Buffer.from(JSON.stringify(contract.runEnvironmentManifest)),
        ),
        environmentId: contract.runEnvironmentManifest.environmentId,
        identityManifest: contract.identityManifest,
        evidenceRoot: collectorEvidenceRoot,
      },
    );
    previewSuiteAttestationSha256 = previewAttestation.sha256;
    if (preview !== null) {
      preview.assertHealthy();
      await preview.stop();
      preview = null;
    }
    await rm(contract.distRoot, { recursive: true });
    contract.distRoot = null;
    await writeStatus(contract, {
      generatedAt: new Date().toISOString(),
      artifactManifestSha256,
      artifactFileCount,
      finalSuiteAttestationSha256,
      previewSuiteAttestationSha256,
      extractedArtifactRetained: false,
      canaryStarted: true,
      status: "passed",
    });
  } catch (error) {
    let extractedArtifactRetained = false;
    if (contract.distRoot !== null) {
      try {
        await lstat(contract.distRoot);
        extractedArtifactRetained = true;
      } catch (metadataError) {
        if (metadataError?.code !== "ENOENT") throw metadataError;
      }
    }
    await writeStatus(contract, {
      generatedAt: new Date().toISOString(),
      artifactManifestSha256,
      artifactFileCount,
      finalSuiteAttestationSha256,
      previewSuiteAttestationSha256,
      extractedArtifactRetained,
      retainedExtractedArtifactRoot: extractedArtifactRetained
        ? contract.distRoot
        : null,
      canaryStarted,
      status: "failed",
      failure: error instanceof Error ? error.message : String(error),
    });
    throw error;
  } finally {
    if (preview !== null) await preview.stop();
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
