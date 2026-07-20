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

const e2eRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(e2eRoot, "..");
const repoRoot = path.resolve(e2eRoot, "../../..");
const playwrightCli = path.join(
  e2eRoot,
  "node_modules",
  "@playwright",
  "test",
  "cli.js",
);
const viteCli = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
const web06Config = path.join(e2eRoot, "playwright.web06.config.ts");
const web06Spec = path.join(e2eRoot, "yune-web06-smoothness.spec.ts");
const statusName = "web06-public-gate-status.json";

export const gateScopes = Object.freeze({
  full: Object.freeze({
    grep: "@web06-full",
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
  const environmentScope = environment.YUNE_WEB_WEB06_GATE_SCOPE?.trim() || null;
  if (
    commandLineScope !== null &&
    environmentScope !== null &&
    commandLineScope !== environmentScope
  ) {
    throw new Error("Command-line and environment WEB06 gate scopes disagree");
  }
  const scope = commandLineScope ?? environmentScope ?? "full";
  if (!Object.hasOwn(gateScopes, scope)) {
    throw new Error(`Unsupported YUNE_WEB_WEB06_GATE_SCOPE: ${scope}`);
  }
  if (verifyOnly && scope !== "preview-canary") {
    throw new Error("--verify-only is supported only for preview-canary reconciliation");
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
    requiredString(environment, "YUNE_WEB_EXPECTED_SOURCE_COMMIT"),
    "YUNE_WEB_EXPECTED_SOURCE_COMMIT",
    40,
  );
  const archiveSha256 = fullSha(
    requiredString(environment, "YUNE_WEB_CERTIFIED_ARCHIVE_SHA256"),
    "YUNE_WEB_CERTIFIED_ARCHIVE_SHA256",
    64,
  );
  const archivePath = path.resolve(
    requiredString(environment, "YUNE_WEB_CERTIFIED_ARCHIVE"),
  );
  const evidenceDir = path.resolve(
    requiredString(environment, "YUNE_WEB_WEB06_EVIDENCE_DIR"),
  );
  const outputDir = path.resolve(
    environment.YUNE_WEB_WEB06_OUTPUT_DIR?.trim() ||
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
      "YUNE_WEB_WEB06_OUTPUT_DIR must be a strict descendant of the external evidence root",
    );
  }
  if (environment.YUNE_WEB_WEB06_DIST_ROOT?.trim()) {
    throw new Error(
      "YUNE_WEB_WEB06_DIST_ROOT is runner-owned; supplied artifact roots are forbidden",
    );
  }

  if (scopeContract.servesLocalArtifact) {
    const portValue = environment.YUNE_WEB_WEB06_PREVIEW_PORT?.trim() || "4174";
    const port = Number(portValue);
    if (!Number.isInteger(port) || port < 1 || port > 65535) {
      throw new Error(`Invalid YUNE_WEB_WEB06_PREVIEW_PORT: ${portValue}`);
    }
    return {
      scope,
      grep: scopeContract.grep,
      expectedSourceCommit,
      archiveSha256,
      archivePath,
      evidenceDir,
      outputDir,
      distRoot: null,
      appUrl: `http://127.0.0.1:${port}/`,
      port,
      expectedPreviewScenarios: null,
      verifyOnly,
      statusName,
    };
  }

  return {
    scope,
    grep: scopeContract.grep,
    expectedSourceCommit,
    archiveSha256,
    archivePath,
    evidenceDir,
    outputDir,
    distRoot: null,
    appUrl: checkedRemoteUrl(requiredString(environment, "YUNE_WEB_APP_URL")),
    port: null,
    expectedPreviewScenarios: "existing-normal-guard,rapid-jyutping",
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

async function validateExternalEvidenceDir(evidenceDir) {
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
      env: { ...process.env, PYTHONDONTWRITEBYTECODE: "1" },
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

export async function prepareOutputPaths(contract) {
  const outputRelative = path.relative(contract.evidenceDir, contract.outputDir);
  await assertNoSymlinkComponents(contract.evidenceDir, "WEB06 evidence root");
  await assertNoSymlinkComponents(contract.outputDir, "WEB06 Playwright output");
  contract.evidenceDir = await validateExternalEvidenceDir(contract.evidenceDir);
  contract.outputDir = path.join(contract.evidenceDir, outputRelative);
  assertStrictDescendant(
    contract.evidenceDir,
    contract.outputDir,
    "YUNE_WEB_WEB06_OUTPUT_DIR",
  );

  await assertNoSymlinkComponents(contract.evidenceDir, "WEB06 evidence root");
  await mkdir(contract.evidenceDir, { recursive: true });
  await assertNoSymlinkComponents(contract.evidenceDir, "WEB06 evidence root");
  await assertNoSymlinkComponents(contract.outputDir, "WEB06 Playwright output");
  await mkdir(contract.outputDir, { recursive: true });
  await assertNoSymlinkComponents(contract.outputDir, "WEB06 Playwright output");

  const [realEvidenceDir, realOutputDir] = await Promise.all([
    realpath(contract.evidenceDir),
    realpath(contract.outputDir),
  ]);
  if (
    path.normalize(realEvidenceDir) !== path.normalize(contract.evidenceDir) ||
    path.normalize(realOutputDir) !== path.normalize(contract.outputDir)
  ) {
    throw new Error("WEB06 output canonicalization changed after directory creation");
  }
  assertStrictDescendant(realEvidenceDir, realOutputDir, "WEB06 Playwright output");
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
    throw new Error("YUNE_WEB_CERTIFIED_ARCHIVE must be a plain archive file");
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

async function extractCertifiedArchive(contract, destination) {
  await assertNoSymlinkComponents(destination, "WEB06 archive extraction root");
  await runCommand("python3", ["-c", safeExtractionProgram], {
    cwd: repoRoot,
    env: {
      ...process.env,
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

async function fetchBytes(url, label) {
  const response = await fetch(url, {
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

async function reconcileRemoteBundle(appUrl, local, expectedSourceCommit) {
  const [remoteBuildInfoBytes, remoteManifestBytes] = await Promise.all([
    fetchBytes(new URL("build-info.json", appUrl), "build-info.json"),
    fetchBytes(
      new URL("public-artifact-manifest.json", appUrl),
      "public-artifact-manifest.json",
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
        );
        validateRemoteFile(file, bytes);
      }
    },
  );
  await Promise.all(workers);
  return remoteBuildInfo;
}

async function writeStatus(contract, values, { createNew = false } = {}) {
  const statusPath = path.join(contract.evidenceDir, contract.statusName);
  if (!createNew) {
    const metadata = await lstat(statusPath);
    if (!metadata.isFile() || metadata.isSymbolicLink()) {
      throw new Error("WEB06 status receipt must remain a plain file");
    }
  }
  await writeFile(
    statusPath,
    `${JSON.stringify(
      {
        sourceCommit: contract.expectedSourceCommit,
        archiveSha256: contract.archiveSha256,
        scope: contract.scope,
        appUrl: contract.appUrl,
        ...values,
      },
      null,
      2,
    )}\n`,
    { flag: createNew ? "wx" : "w" },
  );
}

function startPreview(contract) {
  const preview = spawn(
    process.execPath,
    [
      viteCli,
      "preview",
      "--host",
      "127.0.0.1",
      "--port",
      String(contract.port),
      "--strictPort",
      "--outDir",
      contract.distRoot,
    ],
    { cwd: appRoot, env: process.env, stdio: "inherit" },
  );
  let exit = null;
  let error = null;
  let resolveStopped;
  const stopped = new Promise((resolve) => {
    resolveStopped = resolve;
  });
  preview.once("exit", (code, signal) => {
    exit = { code, signal };
    resolveStopped();
  });
  preview.once("error", (value) => {
    error = value;
    resolveStopped();
  });
  return {
    async ready() {
      const deadline = Date.now() + 30_000;
      while (Date.now() < deadline) {
        if (exit !== null) {
          throw new Error(`WEB06 public preview exited before readiness: ${JSON.stringify(exit)}`);
        }
        if (error !== null) throw error;
        try {
          await fetchBuildInfo(contract.appUrl, contract.expectedSourceCommit);
          return;
        } catch {
          await new Promise((resolve) => setTimeout(resolve, 250));
        }
      }
      throw new Error(`WEB06 public preview did not become ready at ${contract.appUrl}`);
    },
    async stop() {
      if (exit !== null || error !== null) return;
      preview.kill();
      const stoppedNormally = await Promise.race([
        stopped.then(() => true),
        new Promise((resolve) => setTimeout(() => resolve(false), 5_000)),
      ]);
      if (stoppedNormally) return;
      preview.kill("SIGKILL");
      const killed = await Promise.race([
        stopped.then(() => true),
        new Promise((resolve) => setTimeout(() => resolve(false), 2_000)),
      ]);
      if (!killed) throw new Error("WEB06 public preview did not terminate");
    },
  };
}

export async function main(environment = process.env, arguments_ = process.argv.slice(2)) {
  const contract = resolveGateContract(environment, arguments_);
  await prepareOutputPaths(contract);
  await writeStatus(
    contract,
    {
      generatedAt: new Date().toISOString(),
      measurementStarted: false,
      status: "setup-pending",
    },
    { createNew: true },
  );

  let preview = null;
  let artifactManifestSha256 = null;
  let artifactFileCount = null;
  let measurementStarted = false;
  try {
    await validateArchive(contract);
    contract.distRoot = extractionDestination(contract);
    contract.distRoot = await extractCertifiedArchive(contract, contract.distRoot);
    // Catch any mutation between the pre-extraction digest and the bytes that
    // Python opened. The runner never measures a root unless both checks agree.
    await validateArchive(contract);
    if (!contract.verifyOnly) {
      await Promise.all([stat(playwrightCli), stat(web06Config), stat(web06Spec)]);
    }
    const metadata = await lstat(contract.distRoot);
    if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
      throw new Error("YUNE_WEB_WEB06_DIST_ROOT must be a plain extracted directory");
    }
    const local = await validateLocalBundle(
      contract.distRoot,
      contract.expectedSourceCommit,
    );
    artifactManifestSha256 = sha256(local.manifestBytes);
    artifactFileCount = local.manifest.files.length;
    if (contract.scope === "full") {
      await stat(viteCli);
      preview = startPreview(contract);
      await preview.ready();
    } else {
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
        extractedArtifactRetained: false,
        measurementStarted: false,
        status: "reconciled",
      });
      return;
    }

    await writeStatus(contract, {
      generatedAt: new Date().toISOString(),
      artifactManifestSha256,
      artifactFileCount,
      measurementStarted: true,
      status: "measurement-running",
    });
    measurementStarted = true;
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
          ...environment,
          YUNE_WEB_APP_URL: contract.appUrl,
          YUNE_WEB_EXPECTED_SOURCE_COMMIT: contract.expectedSourceCommit,
          YUNE_WEB_CERTIFIED_ARCHIVE: contract.archivePath,
          YUNE_WEB_CERTIFIED_ARCHIVE_SHA256: contract.archiveSha256,
          YUNE_WEB_WEB06_GATE_SCOPE: contract.scope,
          YUNE_WEB_WEB06_EVIDENCE_DIR: contract.evidenceDir,
          YUNE_WEB_WEB06_OUTPUT_DIR: contract.outputDir,
          ...(contract.expectedPreviewScenarios === null
            ? {}
            : {
                YUNE_WEB_WEB06_EXPECTED_PREVIEW_SCENARIOS:
                  contract.expectedPreviewScenarios,
              }),
          YUNE_WEB_WEB06_DIST_ROOT: contract.distRoot,
        },
      },
    );
    if (preview !== null) {
      await preview.stop();
      preview = null;
    }
    await rm(contract.distRoot, { recursive: true });
    contract.distRoot = null;
    await writeStatus(contract, {
      generatedAt: new Date().toISOString(),
      artifactManifestSha256,
      artifactFileCount,
      extractedArtifactRetained: false,
      measurementStarted: true,
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
      extractedArtifactRetained,
      retainedExtractedArtifactRoot: extractedArtifactRetained
        ? contract.distRoot
        : null,
      measurementStarted,
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
