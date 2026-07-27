import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { constants as fsConstants } from "node:fs";
import {
  lstat,
  mkdir,
  open,
  readdir,
  realpath,
} from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";

export const WEB06_WRANGLER_VERSION = "4.111.0";
const WRANGLER_RESOLVED =
  "https://registry.npmjs.org/wrangler/-/wrangler-4.111.0.tgz";
const WRANGLER_INTEGRITY =
  "sha512-bffpI9EyrnpKkF/1S+RaIv8oRD93GtbsA7TlfWwOsGJGB7VO3jVbdGzpC9TU7Bqom3z7jUxcte4Z9MPhaQ4HoQ==";
const e2eRoot = path.dirname(fileURLToPath(import.meta.url));

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

const sha256 = (bytes) =>
  createHash("sha256").update(bytes).digest("hex");

async function readCanonicalPlainFile(file, label) {
  const requested = path.resolve(file);
  const canonical = await realpath(requested);
  assert(canonical === requested, `${label} path is not canonical`);
  const before = await lstat(requested);
  assert(
    before.isFile() && !before.isSymbolicLink(),
    `${label} must be a plain file`,
  );
  assert(
    (before.mode & 0o022) === 0,
    `${label} must not be group/world writable`,
  );
  const handle = await open(
    requested,
    fsConstants.O_RDONLY | (fsConstants.O_NOFOLLOW ?? 0),
  );
  let bytes;
  try {
    const opened = await handle.stat();
    assert(opened.isFile(), `${label} must be a plain file`);
    assert(
      opened.dev === before.dev &&
        opened.ino === before.ino &&
        opened.size === before.size,
      `${label} changed before it was opened`,
    );
    bytes = await handle.readFile();
    const after = await handle.stat();
    assert(
      after.dev === opened.dev &&
        after.ino === opened.ino &&
        after.size === bytes.byteLength &&
        after.mtimeMs === opened.mtimeMs &&
        after.ctimeMs === opened.ctimeMs,
      `${label} changed while it was read`,
    );
  } finally {
    await handle.close();
  }
  return Object.freeze({
    path: requested,
    bytes,
    sha256: sha256(bytes),
  });
}

async function inspectPrivateDirectory(directory, label) {
  const requested = path.resolve(directory);
  const canonical = await realpath(requested);
  const metadata = await lstat(requested);
  const uid = process.getuid?.();
  assert(canonical === requested, `${label} path is not canonical`);
  assert(
    metadata.isDirectory() && !metadata.isSymbolicLink(),
    `${label} must be a plain directory`,
  );
  assert(
    Number.isSafeInteger(uid) && metadata.uid === uid,
    `${label} must be owned by the current uid`,
  );
  assert(
    (metadata.mode & 0o777) === 0o700,
    `${label} must have mode 0700`,
  );
  return Object.freeze({
    dev: metadata.dev,
    ino: metadata.ino,
    uid: metadata.uid,
    mode: metadata.mode & 0o7777,
  });
}

async function writeExclusivePlainFile(file, bytes, label) {
  const handle = await open(
    file,
    fsConstants.O_WRONLY |
      fsConstants.O_CREAT |
      fsConstants.O_EXCL |
      (fsConstants.O_NOFOLLOW ?? 0),
    0o600,
  );
  try {
    await handle.writeFile(bytes);
    await handle.sync();
  } finally {
    await handle.close();
  }
  const written = await readCanonicalPlainFile(file, label);
  assert(written.sha256 === sha256(bytes), `${label} bytes changed`);
}

async function fingerprintInstalledClosure(nodeModulesRoot) {
  const rows = [];
  let totalBytes = 0;
  async function visit(directory, relativeDirectory = "") {
    const entries = await readdir(directory, { withFileTypes: true });
    entries.sort((left, right) => left.name.localeCompare(right.name, "en"));
    for (const entry of entries) {
      const relative = relativeDirectory
        ? `${relativeDirectory}/${entry.name}`
        : entry.name;
      if (relative === ".bin") {
        rows.push(Object.freeze({ path: ".bin", kind: "excluded-bin-links" }));
        continue;
      }
      const absolute = path.join(directory, entry.name);
      const metadata = await lstat(absolute);
      const uid = process.getuid?.();
      assert(
        Number.isSafeInteger(uid) && metadata.uid === uid,
        `Installed Wrangler closure member has foreign ownership: ${relative}`,
      );
      assert(
        (metadata.mode & 0o022) === 0,
        `Installed Wrangler closure member is group/world writable: ${relative}`,
      );
      assert(
        !metadata.isSymbolicLink(),
        `Installed Wrangler closure contains a symbolic link: ${relative}`,
      );
      if (metadata.isDirectory()) {
        assert(
          (await realpath(absolute)) === absolute,
          `Installed Wrangler closure directory is not canonical: ${relative}`,
        );
        rows.push(Object.freeze({ path: relative, kind: "directory" }));
        await visit(absolute, relative);
      } else if (metadata.isFile()) {
        const file = await readCanonicalPlainFile(
          absolute,
          `Installed Wrangler closure member ${relative}`,
        );
        totalBytes += file.bytes.byteLength;
        rows.push(Object.freeze({
          path: relative,
          kind: "file",
          bytes: file.bytes.byteLength,
          sha256: file.sha256,
        }));
      } else {
        throw new Error(
          `Installed Wrangler closure member is not a plain file or directory: ${relative}`,
        );
      }
    }
  }
  await visit(nodeModulesRoot);
  const manifestBytes = Buffer.from(JSON.stringify(rows));
  return Object.freeze({
    fileCount: rows.filter((row) => row.kind === "file").length,
    totalBytes,
    sha256: sha256(manifestBytes),
  });
}

function parseJson(bytes, label) {
  try {
    return JSON.parse(bytes.toString("utf8"));
  } catch {
    throw new Error(`${label} is not valid JSON`);
  }
}

function assertPinnedLockMetadata(rootPayload, lockPayload) {
  const locked = lockPayload?.packages?.["node_modules/wrangler"];
  assert(
    rootPayload?.name === "@yune-ime/yune-web-e2e" &&
      lockPayload?.name === rootPayload.name &&
      lockPayload?.lockfileVersion === 3 &&
      rootPayload?.devDependencies?.wrangler === WEB06_WRANGLER_VERSION &&
      lockPayload?.packages?.[""]?.devDependencies?.wrangler ===
        WEB06_WRANGLER_VERSION,
    "WEB06 Wrangler dependency is not exact in package metadata",
  );
  assert(
    locked?.version === WEB06_WRANGLER_VERSION &&
      locked.resolved === WRANGLER_RESOLVED &&
      locked.integrity === WRANGLER_INTEGRITY &&
      locked.bin?.wrangler === "bin/wrangler.js",
    "WEB06 Wrangler lock identity is invalid",
  );
  return locked;
}

function runVersion(command, arguments_, environment) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, arguments_, {
      cwd: path.parse(command).root,
      env: environment,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolve({ stdout, stderr });
      else {
        reject(
          new Error(
            `Pinned Wrangler version check exited with code ${code}, signal ${signal}: ${stderr.trim()}`,
          ),
        );
      }
    });
  });
}

function versionEnvironment(baseEnvironment) {
  const environment = {
    LANG: baseEnvironment.LANG || "C",
    LC_ALL: baseEnvironment.LC_ALL || "C",
    NO_COLOR: "1",
    WRANGLER_SEND_METRICS: "false",
  };
  if (baseEnvironment.TMPDIR) environment.TMPDIR = baseEnvironment.TMPDIR;
  return environment;
}

function installationEnvironment(root, baseEnvironment) {
  const nodeDirectory = path.dirname(
    baseEnvironment.WEB06_PINNED_NODE_EXECUTABLE,
  );
  return Object.freeze({
    HOME: path.join(root, ".home"),
    LANG: baseEnvironment.LANG || "C",
    LC_ALL: baseEnvironment.LC_ALL || "C",
    NO_COLOR: "1",
    NO_UPDATE_NOTIFIER: "1",
    PATH: `${nodeDirectory}:/usr/bin:/bin`,
    TMPDIR: path.join(root, ".tmp"),
    npm_config_audit: "false",
    npm_config_cache: path.join(root, ".npm-cache"),
    npm_config_fund: "false",
    npm_config_ignore_scripts: "true",
    npm_config_update_notifier: "false",
  });
}

function runInstall(command, arguments_, options) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, arguments_, {
      cwd: options.cwd,
      env: options.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolve({ stdout, stderr });
      else {
        reject(
          new Error(
            `Credential-free npm ci exited with code ${code}, signal ${signal}: ${stderr.trim()}`,
          ),
        );
      }
    });
  });
}

export async function installPinnedWranglerRuntime({
  installationRoot,
  sourceRoot = e2eRoot,
  nodeExecutable = process.execPath,
  npmLauncher = path.join(path.dirname(process.execPath), "npm"),
  baseEnvironment = process.env,
  executeInstall = runInstall,
} = {}) {
  assert(
    typeof installationRoot === "string" && path.isAbsolute(installationRoot),
    "Pinned Wrangler installation root must be absolute",
  );
  const requestedRoot = path.resolve(installationRoot);
  const parent = path.dirname(requestedRoot);
  const parentIdentity = await inspectPrivateDirectory(
    parent,
    "Pinned Wrangler installation parent",
  );
  try {
    await mkdir(requestedRoot, { mode: 0o700 });
  } catch (error) {
    if (error?.code === "EEXIST") {
      const existing = new Error(
        "Pinned Wrangler installation root must be create-new",
      );
      existing.code = "EEXIST";
      throw existing;
    }
    throw error;
  }
  await inspectPrivateDirectory(
    requestedRoot,
    "Pinned Wrangler installation root",
  );
  const currentParent = await inspectPrivateDirectory(
    parent,
    "Pinned Wrangler installation parent",
  );
  assert(
    isDeepStrictEqual(currentParent, parentIdentity),
    "Pinned Wrangler installation parent changed during reservation",
  );

  const sourcePackage = await readCanonicalPlainFile(
    path.join(path.resolve(sourceRoot), "package.json"),
    "WEB06 source package manifest",
  );
  const sourceLock = await readCanonicalPlainFile(
    path.join(path.resolve(sourceRoot), "package-lock.json"),
    "WEB06 source package lock",
  );
  assertPinnedLockMetadata(
    parseJson(sourcePackage.bytes, sourcePackage.path),
    parseJson(sourceLock.bytes, sourceLock.path),
  );
  await writeExclusivePlainFile(
    path.join(requestedRoot, "package.json"),
    sourcePackage.bytes,
    "Installed WEB06 package manifest",
  );
  await writeExclusivePlainFile(
    path.join(requestedRoot, "package-lock.json"),
    sourceLock.bytes,
    "Installed WEB06 package lock",
  );
  for (const name of [".home", ".tmp", ".npm-cache"]) {
    await mkdir(path.join(requestedRoot, name), { mode: 0o700 });
    await inspectPrivateDirectory(
      path.join(requestedRoot, name),
      `Pinned Wrangler ${name} directory`,
    );
  }
  const canonicalNode = await realpath(path.resolve(nodeExecutable));
  const canonicalNpmCli = await realpath(path.resolve(npmLauncher));
  const node = await readCanonicalPlainFile(
    canonicalNode,
    "Pinned npm installer Node executable",
  );
  const npmCli = await readCanonicalPlainFile(
    canonicalNpmCli,
    "Pinned npm CLI",
  );
  const npmRoot = path.dirname(path.dirname(canonicalNpmCli));
  const npmPackage = await readCanonicalPlainFile(
    path.join(npmRoot, "package.json"),
    "Pinned npm package manifest",
  );
  const npmPayload = parseJson(npmPackage.bytes, npmPackage.path);
  assert(
    npmPayload?.name === "npm" &&
      typeof npmPayload.version === "string" &&
      /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(npmPayload.version) &&
      path.relative(npmRoot, canonicalNpmCli) ===
        path.join("bin", "npm-cli.js"),
    "Pinned npm CLI identity is invalid",
  );
  const installEnvironment = installationEnvironment(requestedRoot, {
    ...baseEnvironment,
    WEB06_PINNED_NODE_EXECUTABLE: canonicalNode,
  });
  await executeInstall(
    canonicalNode,
    [
      canonicalNpmCli,
      "ci",
      "--ignore-scripts",
      "--no-audit",
      "--no-fund",
    ],
    {
      cwd: requestedRoot,
      env: installEnvironment,
    },
  );
  await inspectPrivateDirectory(
    requestedRoot,
    "Pinned Wrangler installation root",
  );
  return Object.freeze({
    root: requestedRoot,
    identity: Object.freeze({
      nodeExecutableSha256: node.sha256,
      npmCliSha256: npmCli.sha256,
      npmPackageManifestSha256: npmPackage.sha256,
      npmVersion: npmPayload.version,
    }),
  });
}

export function wranglerDeploymentEnvironment({
  accountId,
  apiToken,
  home,
  temporaryDirectory,
  baseEnvironment = process.env,
}) {
  assert(
    typeof accountId === "string" && accountId !== "",
    "Scoped Cloudflare account ID is missing",
  );
  assert(
    typeof apiToken === "string" && apiToken !== "",
    "Scoped Cloudflare API token is missing",
  );
  const environment = {
    CI: "1",
    CLOUDFLARE_ACCOUNT_ID: accountId,
    CLOUDFLARE_API_TOKEN: apiToken,
    HOME: path.resolve(home),
    LANG: baseEnvironment.LANG || "C",
    LC_ALL: baseEnvironment.LC_ALL || "C",
    NO_COLOR: "1",
    TMPDIR: path.resolve(temporaryDirectory),
    WRANGLER_SEND_METRICS: "false",
  };
  return Object.freeze(environment);
}

async function readRuntimeIdentity({
  root,
  nodeExecutable,
}) {
  const canonicalRoot = await realpath(path.resolve(root));
  assert(
    canonicalRoot === path.resolve(root),
    "WEB06 e2e package root must be canonical",
  );
  const nodeModulesRoot = path.join(canonicalRoot, "node_modules");
  const wranglerRoot = path.join(nodeModulesRoot, "wrangler");
  for (const [directory, label] of [
    [nodeModulesRoot, "WEB06 node_modules"],
    [wranglerRoot, "Pinned Wrangler package"],
  ]) {
    const metadata = await lstat(directory);
    assert(
      metadata.isDirectory() &&
        !metadata.isSymbolicLink() &&
        (await realpath(directory)) === directory,
      `${label} must be a canonical plain directory`,
    );
  }

  const rootPackage = await readCanonicalPlainFile(
    path.join(canonicalRoot, "package.json"),
    "WEB06 e2e package manifest",
  );
  const lockfile = await readCanonicalPlainFile(
    path.join(canonicalRoot, "package-lock.json"),
    "WEB06 e2e package lock",
  );
  const wranglerPackage = await readCanonicalPlainFile(
    path.join(wranglerRoot, "package.json"),
    "Pinned Wrangler package manifest",
  );
  const rootPayload = parseJson(rootPackage.bytes, rootPackage.path);
  const lockPayload = parseJson(lockfile.bytes, lockfile.path);
  const packagePayload = parseJson(
    wranglerPackage.bytes,
    wranglerPackage.path,
  );
  const locked = assertPinnedLockMetadata(rootPayload, lockPayload);
  assert(
    packagePayload?.name === "wrangler" &&
      packagePayload.version === WEB06_WRANGLER_VERSION &&
      packagePayload.bin?.wrangler === "bin/wrangler.js",
    "Installed Wrangler package identity is invalid",
  );
  const entrypoint = await readCanonicalPlainFile(
    path.join(wranglerRoot, "bin", "wrangler.js"),
    "Pinned Wrangler entrypoint",
  );
  assert(
    path.relative(wranglerRoot, entrypoint.path) ===
      path.join("bin", "wrangler.js"),
    "Pinned Wrangler entrypoint escaped its package",
  );

  const canonicalNode = await realpath(path.resolve(nodeExecutable));
  const node = await readCanonicalPlainFile(
    canonicalNode,
    "Pinned Node executable",
  );
  const installedClosure = await fingerprintInstalledClosure(nodeModulesRoot);
  return Object.freeze({
    command: node.path,
    argumentsPrefix: Object.freeze([entrypoint.path]),
    identity: Object.freeze({
      version: WEB06_WRANGLER_VERSION,
      resolved: locked.resolved,
      integrity: locked.integrity,
      rootPackageSha256: rootPackage.sha256,
      lockfileSha256: lockfile.sha256,
      packageManifestSha256: wranglerPackage.sha256,
      entrypointSha256: entrypoint.sha256,
      nodeExecutableSha256: node.sha256,
      installedClosureFileCount: installedClosure.fileCount,
      installedClosureBytes: installedClosure.totalBytes,
      installedClosureSha256: installedClosure.sha256,
    }),
    comparison: Object.freeze({
      root: canonicalRoot,
      command: node.path,
      entrypoint: entrypoint.path,
      rootPackageSha256: rootPackage.sha256,
      lockfileSha256: lockfile.sha256,
      packageManifestSha256: wranglerPackage.sha256,
      entrypointSha256: entrypoint.sha256,
      nodeExecutableSha256: node.sha256,
      installedClosureFileCount: installedClosure.fileCount,
      installedClosureBytes: installedClosure.totalBytes,
      installedClosureSha256: installedClosure.sha256,
      integrity: locked.integrity,
      version: packagePayload.version,
    }),
  });
}

export async function preparePinnedWranglerRuntime({
  root = e2eRoot,
  nodeExecutable = process.execPath,
  baseEnvironment = process.env,
  executeVersion = runVersion,
} = {}) {
  const initial = await readRuntimeIdentity({ root, nodeExecutable });
  const nodeVersion = await executeVersion(
    initial.command,
    ["--version"],
    versionEnvironment(baseEnvironment),
  );
  assert(
    /^v\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/u.test(
      nodeVersion.stdout.trim(),
    ),
    `Pinned Node version check is malformed: ${nodeVersion.stdout.trim()}`,
  );
  const version = await executeVersion(
    initial.command,
    [...initial.argumentsPrefix, "--version"],
    versionEnvironment(baseEnvironment),
  );
  assert(
    version.stdout.trim() === WEB06_WRANGLER_VERSION,
    `Pinned Wrangler version mismatch: ${version.stdout.trim()}`,
  );
  return Object.freeze({
    command: initial.command,
    argumentsPrefix: initial.argumentsPrefix,
    identity: Object.freeze({
      ...initial.identity,
      nodeVersion: nodeVersion.stdout.trim(),
    }),
    async assertCurrent() {
      const current = await readRuntimeIdentity({ root, nodeExecutable });
      assert(
        isDeepStrictEqual(current.comparison, initial.comparison),
        "Pinned Wrangler runtime bytes changed after admission",
      );
    },
  });
}
