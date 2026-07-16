import { createHash } from "node:crypto";
import { spawn } from "node:child_process";
import { lstat, readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptRoot, "..");
const repoRoot = path.resolve(appRoot, "../..");
const publicSchemaRoot = path.join(appRoot, "public", "schema");
const expectedSourceRoot = "apps/yune-web/public/schema";
const expectedVersion = "web03-three-schema-launch-v1";
const manifestPaths = [
  path.join(appRoot, "public", "schema-asset-manifest.json"),
  path.join(appRoot, "public-demo", "schema-asset-manifest.json"),
];
const workerPath = path.join(appRoot, "src", "worker.ts");
const schemaOptionsPath = path.join(appRoot, "src", "consts.ts");
const coveragePath = path.join(appRoot, "schema-acceptance-coverage.json");
const expectedCoverageVersion = "m59-reach03-v1";
const expectedFormalismVersion = "m60-reachability-v1";

function repoRelative(file) {
  return path.relative(repoRoot, file).replaceAll(path.sep, "/");
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertObject(value, label) {
  assert(value !== null && typeof value === "object" && !Array.isArray(value), `${label} must be an object`);
}

function assertNonEmptyString(value, label) {
  assert(typeof value === "string" && value.trim().length > 0, `${label} must be a non-empty string`);
}

function assertRepoRelativePath(relativePath, label, { directory = false } = {}) {
  assertNonEmptyString(relativePath, `${label} path`);
  assert(!relativePath.includes("\\"), `${label} path must use forward slashes`);
  assert(!path.posix.isAbsolute(relativePath), `${label} path must be repository-relative`);
  assert(!/^[A-Za-z]:/.test(relativePath), `${label} path must not be drive-absolute`);
  const parts = relativePath.split("/");
  if (directory) {
    assert(relativePath.endsWith("/"), `${label} root path must end with /`);
    parts.pop();
  } else {
    assert(!relativePath.endsWith("/"), `${label} file path must not end with /`);
  }
  assert(!parts.includes(""), `${label} path must be normalized`);
  assert(!parts.includes(".") && !parts.includes(".."), `${label} path must not escape the repository`);
}

function resolveUnder(root, relativePath, label) {
  assertRepoRelativePath(relativePath, label);
  const resolved = path.resolve(root, ...relativePath.split("/"));
  assert(resolved.startsWith(`${path.resolve(root)}${path.sep}`), `${label} path escapes the repository`);
  return resolved;
}

function assertStringArray(value, label) {
  assert(Array.isArray(value) && value.length > 0, `${label} must be a non-empty array`);
  const seen = new Set();
  for (const entry of value) {
    assertNonEmptyString(entry, `${label} entry`);
    assert(!seen.has(entry), `${label} contains duplicate ${entry}`);
    seen.add(entry);
  }
}

function parseIsoDate(value, label) {
  assert(typeof value === "string" && /^\d{4}-\d{2}-\d{2}$/.test(value), `${label} must use YYYY-MM-DD`);
  const parsed = new Date(`${value}T00:00:00.000Z`);
  assert(!Number.isNaN(parsed.valueOf()) && parsed.toISOString().slice(0, 10) === value, `${label} is invalid`);
  return parsed;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

async function runCommand(command, args, { cwd, label }) {
  return await new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
    const stdout = [];
    const stderr = [];
    child.stdout.on("data", chunk => stdout.push(chunk));
    child.stderr.on("data", chunk => stderr.push(chunk));
    child.on("error", error => reject(new Error(`${label} could not start: ${error.message}`)));
    child.on("close", code => {
      const stdoutText = Buffer.concat(stdout).toString("utf8");
      const stderrText = Buffer.concat(stderr).toString("utf8");
      if (code !== 0) {
        reject(new Error(`${label} failed with exit ${code}: ${stderrText.trim() || stdoutText.trim()}`));
      } else {
        resolve({ stdout: stdoutText, stderr: stderrText });
      }
    });
  });
}

export async function discoverGitTrackedPaths(root, pathspec) {
  const args = ["-C", root, "ls-files", "-z", "--"];
  if (pathspec !== undefined) args.push(pathspec);
  const { stdout } = await runCommand("git", args, {
    cwd: root,
    label: pathspec === undefined ? "git tracked-path discovery" : "git tracked-schema discovery",
  });
  return stdout.split("\0").filter(Boolean);
}

export async function runReachabilityAudit({
  root = repoRoot,
  cargoRoot = repoRoot,
  productRoot = expectedSourceRoot,
  sharedDataRoot = productRoot,
  userDataRoot = productRoot,
  schemaAssets,
} = {}) {
  assert(Array.isArray(schemaAssets) && schemaAssets.length > 0, "Rust audit requires governed schema assets");
  const sharedDataDirectory = path.resolve(root, ...sharedDataRoot.split("/"));
  const userDataDirectory = path.resolve(root, ...userDataRoot.split("/"));
  const args = [
    "run",
    "--locked",
    "-q",
    "-p",
    "yune-rime-api",
    "--release",
    "--bin",
    "yune-schema-reachability-audit",
    "--",
    "--repo-root",
    path.resolve(root),
    "--shared-data-dir",
    sharedDataDirectory,
    "--user-data-dir",
    userDataDirectory,
  ];
  for (const schemaAsset of schemaAssets) args.push("--schema-asset", schemaAsset);
  const { stdout } = await runCommand("cargo", args, { cwd: cargoRoot, label: "Rust reachability audit" });
  let parsed;
  try {
    parsed = JSON.parse(stdout);
  } catch (error) {
    throw new Error(`Rust reachability audit returned malformed JSON: ${error.message}`);
  }
  return parsed;
}

async function sha256(file) {
  const data = await readFile(file);
  return createHash("sha256").update(data).digest("hex");
}

async function readJson(file) {
  return JSON.parse(await readFile(file, "utf8"));
}

function sortedStrings(values) {
  return [...values].sort((left, right) => left.localeCompare(right));
}

function assertSameStrings(actual, expected, message) {
  assert(
    JSON.stringify(sortedStrings(actual)) === JSON.stringify(sortedStrings(expected)),
    `${message}: actual=${JSON.stringify(sortedStrings(actual))}, expected=${JSON.stringify(sortedStrings(expected))}`,
  );
}

function declaredSchemaIds(source) {
  const uncommented = source
    .split(/\r?\n/)
    .filter(line => !line.trimStart().startsWith("#"))
    .join("\n");
  const pattern = /(?:^|[\s{,\[])(?:-\s*)?schema\s*:\s*(?:"([^"]+)"|'([^']+)'|([A-Za-z0-9_]+))(?=\s|[,}\]]|$)/gm;
  return [...uncommented.matchAll(pattern)].map(match => match[1] ?? match[2] ?? match[3]);
}

function sourceSchemaId(source) {
  return /^\s*schema_id:\s*([A-Za-z0-9_]+)\s*$/m.exec(source)?.[1];
}

assertSameStrings(
  declaredSchemaIds(`
schema_list:
  - schema: alpha
  - schema: "beta"
flow: [{ schema: 'gamma' }, {schema: delta}]
# - schema: ignored
`),
  ["alpha", "beta", "gamma", "delta"],
  "schema carrier parser must cover block, quoted, and flow forms",
);

function resolveRepoFile(relativePath, label) {
  assert(typeof relativePath === "string" && relativePath.length > 0, `${label} path must be non-empty`);
  assert(!relativePath.includes("\\"), `${label} path must use forward slashes`);
  assert(!path.isAbsolute(relativePath), `${label} path must be repository-relative`);
  assert(!relativePath.split("/").includes(".."), `${label} path must not escape the repository`);
  const resolved = path.resolve(repoRoot, ...relativePath.split("/"));
  assert(
    resolved === repoRoot || resolved.startsWith(`${repoRoot}${path.sep}`),
    `${label} path escapes the repository: ${relativePath}`,
  );
  return resolved;
}

export async function treeAssetPaths(root = publicSchemaRoot) {
  const pending = [root];
  const assets = [];
  while (pending.length > 0) {
    const directory = pending.pop();
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const fullPath = path.join(directory, entry.name);
      const relativePath = path.relative(root, fullPath).replaceAll(path.sep, "/");
      const entryKind = schemaTreeEntryKind(entry, relativePath);
      if (entryKind === "directory") {
        pending.push(fullPath);
      } else {
        assets.push(relativePath);
      }
    }
  }
  return sortedStrings(assets);
}

export function schemaTreeEntryKind(entry, relativePath) {
  if (entry.isDirectory()) return "directory";
  if (entry.isFile()) return "file";
  const entryType = entry.isSymbolicLink() ? "symbolic link" : "unsupported entry";
  throw new Error(`${entryType} is not allowed in the public schema tree: ${relativePath}`);
}

export async function assertManifestMatchesTree(assetPaths, root = publicSchemaRoot) {
  assertSameStrings(
    assetPaths,
    await treeAssetPaths(root),
    "manifest assets must match every public schema tree asset",
  );
}

function configuredSchemaOptionIds(source) {
  const block = /SCHEMA_OPTIONS[^=]*=\s*\[([\s\S]*?)\n\];/m.exec(source)?.[1];
  assert(typeof block === "string", `${repoRelative(schemaOptionsPath)} must declare SCHEMA_OPTIONS`);
  return [...block.matchAll(/\bid\s*:\s*"([A-Za-z0-9_]+)"/g)].map(match => match[1]);
}

function playgroundSchemaEntries(source) {
  const block = /const PLAYGROUND_SCHEMAS[^=]*=\s*{([\s\S]*?)\n};/m.exec(source)?.[1];
  assert(typeof block === "string", `${repoRelative(workerPath)} must declare PLAYGROUND_SCHEMAS`);
  const entries = [];
  for (const match of block.matchAll(/^\s*([A-Za-z0-9_]+):\s*{([\s\S]*?)^\s*},?\s*$/gm)) {
    const body = match[2];
    const runtimeId = /\bruntimeId\s*:\s*"([A-Za-z0-9_]+)"/.exec(body)?.[1];
    assert(typeof runtimeId === "string", `PLAYGROUND_SCHEMAS.${match[1]} must declare runtimeId`);
    entries.push({
      optionId: match[1],
      runtimeId,
      deployedSchemaPath: /\bdeployedSchemaPath\s*:\s*"([^"]+\.schema\.yaml)"/.exec(body)?.[1],
    });
  }
  assert(entries.length > 0, `${repoRelative(workerPath)} PLAYGROUND_SCHEMAS must not be empty`);
  return entries;
}

function canonicalAsset(asset) {
  return {
    path: asset.path,
    sha256: asset.sha256,
    bytes: asset.bytes,
    tier: asset.tier,
    required: asset.required === true,
  };
}

async function validateManifest(file) {
  const manifest = await readJson(file);
  assert(manifest.version === expectedVersion, `${repoRelative(file)} has unexpected version`);
  assert(manifest.generatedFor === "yune-web", `${repoRelative(file)} has unexpected generatedFor`);
  assert(manifest.sourceRoot === expectedSourceRoot, `${repoRelative(file)} has unexpected sourceRoot`);
  assert(Array.isArray(manifest.assets), `${repoRelative(file)} must contain an assets array`);

  const seen = new Set();
  for (const asset of manifest.assets) {
    assert(typeof asset.path === "string" && asset.path.length > 0, "manifest asset path must be non-empty");
    assert(!asset.path.includes("\\"), `${asset.path} must use forward slashes`);
    assert(!asset.path.startsWith("/") && !asset.path.split("/").includes(".."), `${asset.path} must be relative`);
    assert(!asset.path.endsWith(".poet.bin"), `${asset.path} is optional poet storage and must not be public payload`);
    assert(!seen.has(asset.path), `duplicate manifest asset ${asset.path}`);
    seen.add(asset.path);

    const source = path.join(publicSchemaRoot, ...asset.path.split("/"));
    const fileStat = await stat(source);
    assert(fileStat.isFile(), `${asset.path} is not a file`);
    assert(fileStat.size === asset.bytes, `${asset.path} bytes mismatch: manifest ${asset.bytes}, tree ${fileStat.size}`);
    const actualSha = await sha256(source);
    assert(actualSha === asset.sha256, `${asset.path} sha256 mismatch: manifest ${asset.sha256}, tree ${actualSha}`);
  }
  return manifest.assets.map(canonicalAsset);
}

function workerLiteralSchemaAssets(source) {
  const assets = new Set();
  const literalPattern = /["']([^"']+\.(?:yaml|txt|bin|ocd2|json))["']/g;
  let match;
  while ((match = literalPattern.exec(source)) !== null) {
    const candidate = match[1];
    if (
      candidate === "schema-asset-manifest.json" ||
      candidate.includes("${") ||
      candidate.startsWith("http:")
    ) {
      continue;
    }
    if (candidate.startsWith("schema/")) {
      assets.add(candidate.slice("schema/".length));
    } else {
      assets.add(candidate);
    }
  }
  return [...assets].sort();
}

async function assertNoPoetPayloads() {
  const pending = [publicSchemaRoot];
  while (pending.length > 0) {
    const dir = pending.pop();
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        pending.push(fullPath);
      } else if (entry.name.endsWith(".poet.bin")) {
        throw new Error(`${repoRelative(fullPath)} is optional poet storage and must not be committed as public payload`);
      }
    }
  }
}

function rootForPath(schemaRoots, candidate) {
  const matches = schemaRoots.filter(root => candidate.startsWith(root.path));
  assert(matches.length === 1, `${candidate} must match exactly one classified schema root (matched ${matches.length})`);
  return matches[0];
}

async function assertTrackedRegularFile(relativePath, label, { root, trackedSet }) {
  assertRepoRelativePath(relativePath, label);
  assert(trackedSet.has(relativePath), `${label} is not tracked: ${relativePath}`);
  return await assertPathHasNoSymlinkComponents(relativePath, label, { root, directory: false });
}

async function assertPathHasNoSymlinkComponents(relativePath, label, { root, directory }) {
  assertRepoRelativePath(relativePath, label, { directory });
  const parts = relativePath.split("/").filter(Boolean);
  let current = path.resolve(root);
  for (const [index, part] of parts.entries()) {
    current = path.join(current, part);
    const componentStat = await lstat(current);
    const componentPath = parts.slice(0, index + 1).join("/");
    assert(!componentStat.isSymbolicLink(), `${label} traverses symbolic link component ${componentPath}`);
    if (index < parts.length - 1) {
      assert(componentStat.isDirectory(), `${label} traverses non-directory component ${componentPath}`);
    } else if (directory) {
      assert(componentStat.isDirectory(), `${label} is not a directory: ${relativePath}`);
    } else {
      assert(componentStat.isFile(), `${label} is not a regular file: ${relativePath}`);
    }
  }
  return resolveUnder(root, relativePath.replace(/\/$/, ""), label);
}

function reconciliationKey({ settingAsset, configPath, schemaAsset }) {
  return `${settingAsset}\0${configPath}\0${schemaAsset}`;
}

function acceptedProductRow(coverageByFullAsset, schemaAsset, label) {
  const row = coverageByFullAsset.get(schemaAsset);
  assert(row !== undefined, `${label} names unknown or unmanifested schema asset ${schemaAsset}`);
  assert(row.status === "accepted", `${label} names unresolved schema asset ${schemaAsset}`);
  assert(row.disposition !== "dependency_only", `${label} cannot use dependency-only schema ${schemaAsset}`);
  assertNonEmptyString(row.acceptanceId, `${label}.acceptanceId`);
  assertObject(row.acceptance, `${label} accepted real-path row`);
  return row;
}

/**
 * Validate M60 root classification, Rust audit output, and the exact false /
 * opt-out bijection. This is the same validator called by the production entry
 * point; tests inject only roots, tracked paths, audit output, and a fixed date.
 */
export async function validateReachabilityFormalism({
  coverage,
  auditResult,
  trackedPaths,
  trackedSchemaPaths,
  root = repoRoot,
  productManifestAssets,
  coverageRegistryPath = "apps/yune-web/schema-acceptance-coverage.json",
  asOfDate = new Date(),
}) {
  assertObject(coverage, "coverage registry");
  assert(coverage.version === expectedCoverageVersion, `coverage registry must retain ${expectedCoverageVersion}`);
  assert(
    coverage.reachabilityFormalismVersion === expectedFormalismVersion,
    `coverage registry must declare ${expectedFormalismVersion}`,
  );
  assert(Array.isArray(coverage.schemaRoots) && coverage.schemaRoots.length > 0, "coverage registry must declare schemaRoots");
  assert(Array.isArray(coverage.reachabilityOptOuts), "coverage registry must declare reachabilityOptOuts");
  assert(Array.isArray(coverage.schemaAssets), "coverage registry must declare schemaAssets");
  assertObject(coverage.mechanismContract, "coverage mechanismContract");
  assert(
    coverage.mechanismContract.canonical === "docs/contracts/schema-general-reachability.md",
    "coverage mechanismContract must link the canonical reachability contract",
  );
  assert(
    coverage.mechanismContract.profilePrecedence.includes("prefix_fallback_owned"),
    "coverage mechanismContract must state per-input prefix_fallback_owned precedence",
  );

  const trackedSet = new Set(trackedPaths);
  assert(trackedSet.size === trackedPaths.length, "injected tracked path list contains duplicates");
  const trackedSchemaSet = new Set(trackedSchemaPaths);
  assert(trackedSchemaSet.size === trackedSchemaPaths.length, "tracked schema path list contains duplicates");
  assertSameStrings(
    [...trackedSet].filter(trackedPath => trackedPath.endsWith(".schema.yaml")),
    trackedSchemaPaths,
    "tracked schema discovery must equal the complete tracked schema set",
  );
  for (const schemaPath of trackedSchemaPaths) {
    assert(schemaPath.endsWith(".schema.yaml"), `tracked schema discovery returned non-schema path ${schemaPath}`);
    assert(trackedSet.has(schemaPath), `tracked schema path is absent from complete tracked set: ${schemaPath}`);
    await assertTrackedRegularFile(schemaPath, "tracked schema asset", { root, trackedSet });
  }

  const allowedClassifications = new Set(["product", "test-fixture", "historical-evidence"]);
  const seenRootPaths = new Set();
  const roots = coverage.schemaRoots;
  for (const [index, schemaRoot] of roots.entries()) {
    assertObject(schemaRoot, `schemaRoots[${index}]`);
    assertRepoRelativePath(schemaRoot.path, `schemaRoots[${index}]`, { directory: true });
    assert(!seenRootPaths.has(schemaRoot.path), `duplicate schema root ${schemaRoot.path}`);
    seenRootPaths.add(schemaRoot.path);
    assert(
      allowedClassifications.has(schemaRoot.classification),
      `${schemaRoot.path} has unknown classification ${schemaRoot.classification}`,
    );
    await assertPathHasNoSymlinkComponents(schemaRoot.path, "schema root", { root, directory: true });
    if (schemaRoot.classification === "product") {
      await assertTrackedRegularFile(schemaRoot.manifest, `${schemaRoot.path} manifest`, { root, trackedSet });
      await assertTrackedRegularFile(schemaRoot.acceptanceRegistry, `${schemaRoot.path} acceptance registry`, {
        root,
        trackedSet,
      });
    } else {
      assertNonEmptyString(schemaRoot.disposition, `${schemaRoot.path} non-product disposition`);
      assert(schemaRoot.manifest === undefined, `${schemaRoot.path} non-product root must not name a product manifest`);
      assert(
        schemaRoot.acceptanceRegistry === undefined,
        `${schemaRoot.path} non-product root must not name an acceptance registry`,
      );
    }
  }
  for (let left = 0; left < roots.length; left += 1) {
    for (let right = left + 1; right < roots.length; right += 1) {
      assert(
        !roots[left].path.startsWith(roots[right].path) && !roots[right].path.startsWith(roots[left].path),
        `schema roots overlap: ${roots[left].path} and ${roots[right].path}`,
      );
    }
  }
  const classifiedCounts = new Map(roots.map(schemaRoot => [schemaRoot.path, 0]));
  for (const schemaPath of trackedSchemaPaths) {
    const schemaRoot = rootForPath(roots, schemaPath);
    classifiedCounts.set(schemaRoot.path, classifiedCounts.get(schemaRoot.path) + 1);
  }
  for (const schemaRoot of roots) {
    assert(classifiedCounts.get(schemaRoot.path) > 0, `schema root is stale or empty: ${schemaRoot.path}`);
  }

  const productRoots = roots.filter(schemaRoot => schemaRoot.classification === "product");
  assert(productRoots.length === 1, `exactly one product schema root is supported; found ${productRoots.length}`);
  const productRoot = productRoots[0];
  assert(coverage.manifest === productRoot.manifest, "coverage manifest must match the registered product root manifest");
  assert(
    productRoot.acceptanceRegistry === coverageRegistryPath,
    "product root acceptanceRegistry must name the live coverage registry",
  );
  const manifestSchemaAssets = productManifestAssets.filter(asset => asset.endsWith(".schema.yaml"));
  const manifestSchemaSet = new Set(manifestSchemaAssets);
  assert(manifestSchemaSet.size === manifestSchemaAssets.length, "product manifest contains duplicate schema assets");
  const coverageAssetSet = new Set();
  const coverageByFullAsset = new Map();
  for (const row of coverage.schemaAssets) {
    assertObject(row, "schema acceptance row");
    assertRepoRelativePath(row.asset, "schema acceptance asset");
    assert(!coverageAssetSet.has(row.asset), `duplicate schema acceptance asset ${row.asset}`);
    coverageAssetSet.add(row.asset);
    assert(manifestSchemaSet.has(row.asset), `stale or unmanifested schema acceptance row ${row.asset}`);
    const fullAsset = `${productRoot.path}${row.asset}`;
    assert(trackedSchemaSet.has(fullAsset), `schema acceptance row is not a tracked current schema: ${row.asset}`);
    coverageByFullAsset.set(fullAsset, row);
  }
  assertSameStrings(coverage.schemaAssets.map(row => row.asset), manifestSchemaAssets, "product schema coverage must equal manifest schemas");
  const trackedProductSchemas = trackedSchemaPaths.filter(schemaPath => rootForPath(roots, schemaPath).classification === "product");
  assertSameStrings(
    trackedProductSchemas,
    manifestSchemaAssets.map(asset => `${productRoot.path}${asset}`),
    "tracked product schemas must equal manifested product schemas",
  );

  assertObject(auditResult, "Rust reachability audit output");
  assert(auditResult.version === expectedFormalismVersion, "Rust reachability audit has unexpected version");
  assert(Array.isArray(auditResult.tuples), "Rust reachability audit must contain tuples");
  assert(auditResult.unresolved === undefined, "Rust reachability audit must not contain unresolved directives");
  const tupleKeys = new Set();
  const falseByKey = new Map();
  for (const [index, tuple] of auditResult.tuples.entries()) {
    const label = `audit tuple ${index}`;
    assertObject(tuple, label);
    await assertTrackedRegularFile(tuple.schemaAsset, `${label}.schemaAsset`, { root, trackedSet });
    const schemaRow = coverageByFullAsset.get(tuple.schemaAsset);
    assert(schemaRow !== undefined, `${label} names non-product or unmanifested schema ${tuple.schemaAsset}`);
    assert(tuple.schemaId === schemaRow.schemaId, `${label}.schemaId does not match registry`);
    assert(
      typeof tuple.component === "string" && /^(?:script|table|r10n)_translator(?:@[A-Za-z0-9_.-]+)?$/.test(tuple.component),
      `${label}.component is not a deployed dictionary translator prescription`,
    );
    assertNonEmptyString(tuple.namespace, `${label}.namespace`);
    assertRepoRelativePath(tuple.namespace, `${label}.namespace`);
    assert(
      tuple.configPath === `${tuple.namespace}/leading_syllable_reachability`,
      `${label}.configPath does not match its namespace`,
    );
    assert(typeof tuple.effective === "boolean", `${label}.effective must be boolean`);
    assert(["script", "table", "r10n"].includes(tuple.translatorArm), `${label}.translatorArm is unsupported`);
    if (tuple.runtimeArm !== undefined) assertNonEmptyString(tuple.runtimeArm, `${label}.runtimeArm`);
    const tupleKey = `${tuple.schemaAsset}\0${tuple.component}\0${tuple.namespace}`;
    assert(!tupleKeys.has(tupleKey), `${label} duplicates schema/component/namespace tuple`);
    tupleKeys.add(tupleKey);

    assert(Array.isArray(tuple.sourceTrace) && tuple.sourceTrace.length > 0, `${label}.sourceTrace must be non-empty`);
    const traceAssets = new Set();
    for (const [traceIndex, trace] of tuple.sourceTrace.entries()) {
      assertObject(trace, `${label}.sourceTrace[${traceIndex}]`);
      assertNonEmptyString(trace.kind, `${label}.sourceTrace[${traceIndex}].kind`);
      assertRepoRelativePath(trace.asset, `${label}.sourceTrace[${traceIndex}].asset`);
      if (trackedSet.has(trace.asset)) {
        await assertTrackedRegularFile(trace.asset, `${label}.sourceTrace[${traceIndex}].asset`, { root, trackedSet });
        traceAssets.add(trace.asset);
      } else {
        assert(
          trace.optional === true,
          `${label}.sourceTrace[${traceIndex}].asset is unresolved and not optional: ${trace.asset}`,
        );
      }
      if (trace.reference !== undefined) assertNonEmptyString(trace.reference, `${label}.sourceTrace[${traceIndex}].reference`);
      if (trace.optional !== undefined) {
        assert(typeof trace.optional === "boolean", `${label}.sourceTrace[${traceIndex}].optional must be boolean`);
      }
    }
    assert(Array.isArray(tuple.assetHashes) && tuple.assetHashes.length > 0, `${label}.assetHashes must be non-empty`);
    const sortedHashAssets = tuple.assetHashes.map(entry => entry.asset).sort((left, right) => left.localeCompare(right));
    assert(
      JSON.stringify(tuple.assetHashes.map(entry => entry.asset)) === JSON.stringify(sortedHashAssets),
      `${label}.assetHashes must be sorted`,
    );
    const hashedAssets = new Set();
    for (const [hashIndex, hashEntry] of tuple.assetHashes.entries()) {
      assertObject(hashEntry, `${label}.assetHashes[${hashIndex}]`);
      assert(!hashedAssets.has(hashEntry.asset), `${label}.assetHashes contains duplicate ${hashEntry.asset}`);
      hashedAssets.add(hashEntry.asset);
      const source = await assertTrackedRegularFile(hashEntry.asset, `${label}.assetHashes[${hashIndex}].asset`, {
        root,
        trackedSet,
      });
      assert(/^[0-9a-f]{64}$/.test(hashEntry.sha256), `${label}.assetHashes[${hashIndex}].sha256 is malformed`);
      assert((await sha256(source)) === hashEntry.sha256, `${label} asset hash mismatch for ${hashEntry.asset}`);
    }
    for (const traceAsset of traceAssets) {
      assert(hashedAssets.has(traceAsset), `${label} source trace asset lacks a hash: ${traceAsset}`);
    }
    assertSameStrings(
      [...hashedAssets],
      [...traceAssets],
      `${label} asset hashes must equal contributing tracked source-trace assets`,
    );
    if (tuple.settingAsset === null) {
      assert(tuple.effective, `${label} inherited default cannot be explicit false without a setting asset`);
    } else {
      await assertTrackedRegularFile(tuple.settingAsset, `${label}.settingAsset`, { root, trackedSet });
      assert(traceAssets.has(tuple.settingAsset), `${label}.settingAsset is absent from its source trace`);
      assert(hashedAssets.has(tuple.settingAsset), `${label}.settingAsset is absent from its asset hashes`);
    }
    if (!tuple.effective) {
      assert(tuple.settingAsset !== null, `${label} false setting must name settingAsset`);
      const key = reconciliationKey(tuple);
      if (!falseByKey.has(key)) falseByKey.set(key, []);
      falseByKey.get(key).push(tuple);
    }
  }

  const decisionPath = "docs/decisions.md";
  const decisionFile = await assertTrackedRegularFile(decisionPath, "decision registry", { root, trackedSet });
  const decisionSource = await readFile(decisionFile, "utf8");
  const optOutIds = new Set();
  const optOutKeys = new Set();
  const asOf = asOfDate instanceof Date ? asOfDate : new Date(asOfDate);
  assert(!Number.isNaN(asOf.valueOf()), "asOfDate must be a valid date");
  const asOfDay = parseIsoDate(asOf.toISOString().slice(0, 10), "asOfDate");
  for (const [index, optOut] of coverage.reachabilityOptOuts.entries()) {
    const label = `reachabilityOptOuts[${index}]`;
    assertObject(optOut, label);
    assert(typeof optOut.optOutId === "string" && /^[a-z0-9][a-z0-9-]*$/.test(optOut.optOutId), `${label}.optOutId is malformed`);
    assert(!optOutIds.has(optOut.optOutId), `duplicate opt-out id ${optOut.optOutId}`);
    optOutIds.add(optOut.optOutId);
    await assertTrackedRegularFile(optOut.schemaAsset, `${label}.schemaAsset`, { root, trackedSet });
    await assertTrackedRegularFile(optOut.settingAsset, `${label}.settingAsset`, { root, trackedSet });
    const schemaRow = acceptedProductRow(coverageByFullAsset, optOut.schemaAsset, label);
    assert(optOut.schemaId === schemaRow.schemaId, `${label}.schemaId does not match accepted schema row`);
    assert(optOut.acceptanceId === schemaRow.acceptanceId, `${label}.acceptanceId does not match accepted real-path row`);
    assertNonEmptyString(optOut.configPath, `${label}.configPath`);
    assertRepoRelativePath(optOut.configPath, `${label}.configPath`);
    assert(optOut.configPath.endsWith("/leading_syllable_reachability"), `${label}.configPath is not reachability`);
    assertObject(optOut.source, `${label}.source`);
    assert(/^https:\/\//.test(optOut.source.repository), `${label}.source.repository must be an https URL`);
    assert(/^[0-9a-fA-F]{40}$/.test(optOut.source.commit), `${label}.source.commit must be 40 hex`);
    assertNonEmptyString(optOut.owner, `${label}.owner`);
    assertNonEmptyString(optOut.reason, `${label}.reason`);
    assertStringArray(optOut.affectedSurfaces, `${label}.affectedSurfaces`);
    assertObject(optOut.evidence, `${label}.evidence`);
    assert(["oracle", "owner-spec"].includes(optOut.evidence.kind), `${label}.evidence.kind is unsupported`);
    await assertTrackedRegularFile(optOut.evidence.path, `${label}.evidence.path`, { root, trackedSet });
    assert(/^[0-9a-fA-F]{40}$/.test(optOut.evidence.sourceCommit), `${label}.evidence.sourceCommit must be 40 hex`);
    assertObject(optOut.approval, `${label}.approval`);
    assert(/^D-\d+$/.test(optOut.approval.decisionId), `${label}.approval.decisionId is malformed`);
    const decisionHeading = new RegExp(`(?:^|\\n)\\*\\*${escapeRegExp(optOut.approval.decisionId)}(?:\\s|/)`, "m");
    assert(decisionHeading.test(decisionSource), `${label}.approval decision is missing from docs/decisions.md`);
    assertNonEmptyString(optOut.approval.approver, `${label}.approval.approver`);
    const approvedOn = parseIsoDate(optOut.approval.approvedOn, `${label}.approval.approvedOn`);
    assert(approvedOn <= asOfDay, `${label}.approval.approvedOn is in the future`);
    assertObject(optOut.review, `${label}.review`);
    assertStringArray(optOut.review.triggers, `${label}.review.triggers`);
    const reviewBy = parseIsoDate(optOut.review.reviewBy, `${label}.review.reviewBy`);
    assert(reviewBy >= asOfDay, `${label} expired on ${optOut.review.reviewBy}`);
    const key = reconciliationKey(optOut);
    assert(!optOutKeys.has(key), `${label} duplicates reconciliation key`);
    optOutKeys.add(key);
    assert(falseByKey.has(key), `${label} has no matching effective false tuple`);
  }
  for (const key of falseByKey.keys()) {
    assert(optOutKeys.has(key), `effective false tuple has no approved opt-out row: ${key.replaceAll("\0", " | ")}`);
  }
  assert(falseByKey.size === optOutKeys.size, "explicit-false and opt-out collections are not bijective");
  return {
    trackedSchemas: trackedSchemaPaths.length,
    productSchemas: trackedProductSchemas.length,
    auditTuples: auditResult.tuples.length,
    optOuts: coverage.reachabilityOptOuts.length,
  };
}

async function validateAcceptanceCoverage(publicAssets) {
  const coverage = await readJson(coveragePath);
  assert(coverage.version === expectedCoverageVersion, `${repoRelative(coveragePath)} has unexpected version`);
  assert(
    coverage.manifest === "apps/yune-web/public/schema-asset-manifest.json",
    `${repoRelative(coveragePath)} must bind the canonical public manifest`,
  );
  assert(
    typeof coverage.mechanismContract?.schemaGeneralDefault === "string" &&
      coverage.mechanismContract.schemaGeneralDefault.includes("leading_syllable_reachability"),
    "coverage registry must document the schema-general reachability owner",
  );
  assert(
    typeof coverage.mechanismContract?.profilePrecedence === "string" &&
      coverage.mechanismContract.profilePrecedence.includes("prefix_fallback"),
    "coverage registry must document TypeDuck profile precedence",
  );

  const schemaAssetPaths = publicAssets
    .map(asset => asset.path)
    .filter(assetPath => assetPath.endsWith(".schema.yaml"));
  assert(Array.isArray(coverage.schemaAssets), "coverage registry must contain schemaAssets");
  assert(
    JSON.stringify(coverage.schemaAssets.map(row => row.asset)) === JSON.stringify(schemaAssetPaths),
    `coverage registry must follow manifest schema-asset order exactly: manifest=${JSON.stringify(schemaAssetPaths)}, coverage=${JSON.stringify(coverage.schemaAssets.map(row => row.asset))}`,
  );

  const seenAssets = new Set();
  const coverageByAsset = new Map(coverage.schemaAssets.map(row => [row.asset, row]));
  const acceptanceById = new Map();
  const allowedDispositions = new Set([
    "top_level_selectable",
    "runtime_alias",
    "deployed_runtime_mirror",
    "dev_selectable",
    "dependency_only",
  ]);
  for (const row of coverage.schemaAssets) {
    assert(typeof row.asset === "string" && row.asset.length > 0, "coverage asset must be non-empty");
    assert(!seenAssets.has(row.asset), `duplicate coverage asset ${row.asset}`);
    seenAssets.add(row.asset);
    assert(row.status === "accepted", `${row.asset} coverage is unresolved (${row.status ?? "missing status"})`);
    assert(allowedDispositions.has(row.disposition), `${row.asset} has unknown disposition ${row.disposition}`);
    assert(typeof row.schemaId === "string" && row.schemaId.length > 0, `${row.asset} must name schemaId`);
    assert(typeof row.acceptanceId === "string" && row.acceptanceId.length > 0, `${row.asset} must name acceptanceId`);

    const schemaSourcePath = path.join(publicSchemaRoot, ...row.asset.split("/"));
    const schemaSource = await readFile(schemaSourcePath, "utf8");
    assert(
      sourceSchemaId(schemaSource) === row.schemaId,
      `${row.asset} schema_id does not match coverage schemaId ${row.schemaId}`,
    );

    if (row.disposition === "dependency_only") {
      assert(row.acceptance === undefined, `${row.asset} dependency-only row must not invent a top-level acceptance`);
      assert(
        Array.isArray(row.dependencyOwners) && row.dependencyOwners.length > 0,
        `${row.asset} dependency-only row must name its top-level owner`,
      );
      for (const owner of row.dependencyOwners) {
        const ownerRow = coverageByAsset.get(owner);
        assert(ownerRow !== undefined, `${row.asset} names unknown dependency owner ${owner}`);
        assert(ownerRow.disposition !== "dependency_only", `${row.asset} dependency owner ${owner} is not top-level`);
      }
      assert(typeof row.reason === "string" && row.reason.length > 0, `${row.asset} dependency disposition needs a reason`);
      continue;
    }

    const acceptance = row.acceptance;
    assert(acceptance !== null && typeof acceptance === "object", `${row.asset} must bind executable acceptance`);
    for (const field of [
      "testFile",
      "testName",
      "caseLabel",
      "oracleFixture",
      "input",
      "expectedFinal",
      "defaultOn",
      "explicitFalse",
    ]) {
      assert(typeof acceptance[field] === "string" && acceptance[field].length > 0, `${row.asset} acceptance.${field} is required`);
    }
    const testPath = resolveRepoFile(acceptance.testFile, `${row.asset} acceptance test`);
    const testSource = await readFile(testPath, "utf8");
    assert(
      testSource.includes(`fn ${acceptance.testName}(`),
      `${row.asset} acceptance test ${acceptance.testName} is missing from ${acceptance.testFile}`,
    );
    assert(
      testSource.includes(`label: "${acceptance.caseLabel}"`),
      `${row.asset} acceptance case ${acceptance.caseLabel} is missing from ${acceptance.testFile}`,
    );
    assert(testSource.includes(acceptance.input), `${row.asset} acceptance input is not bound by its test source`);
    assert(testSource.includes(acceptance.expectedFinal), `${row.asset} expected output is not bound by its test source`);
    const fixturePath = resolveRepoFile(acceptance.oracleFixture, `${row.asset} oracle fixture`);
    assert((await stat(fixturePath)).isFile(), `${row.asset} oracle fixture is not a file: ${acceptance.oracleFixture}`);

    const priorAcceptance = acceptanceById.get(row.acceptanceId);
    if (priorAcceptance === undefined) {
      acceptanceById.set(row.acceptanceId, acceptance);
    } else {
      assert(
        JSON.stringify(priorAcceptance) === JSON.stringify(acceptance),
        `${row.asset} reuses acceptanceId ${row.acceptanceId} with different evidence`,
      );
    }
  }

  const carrierAssetPaths = publicAssets
    .map(asset => asset.path)
    .filter(assetPath => /(^|\/)default(?:\.custom)?\.yaml$/.test(assetPath));
  assert(Array.isArray(coverage.configurationCarriers), "coverage registry must name configuration carriers");
  assertSameStrings(
    coverage.configurationCarriers.map(carrier => carrier.asset),
    carrierAssetPaths,
    "coverage configuration carriers must match manifested default YAML assets",
  );
  for (const carrier of coverage.configurationCarriers) {
    assert(Array.isArray(carrier.declaredSchemaIds), `${carrier.asset} must declare schema ids`);
    const carrierPath = path.join(publicSchemaRoot, ...carrier.asset.split("/"));
    const source = await readFile(carrierPath, "utf8");
    assertSameStrings(
      declaredSchemaIds(source),
      carrier.declaredSchemaIds,
      `${carrier.asset} declared schema ids changed`,
    );
    for (const schemaId of carrier.declaredSchemaIds) {
      const accepted = coverage.schemaAssets.some(
        row => row.schemaId === schemaId && row.status === "accepted" && row.disposition !== "dependency_only" && row.acceptance,
      );
      assert(accepted, `${carrier.asset} promotes ${schemaId} without accepted top-level real-path coverage`);
    }
  }

  const schemaOptionSource = await readFile(schemaOptionsPath, "utf8");
  const schemaOptionIds = configuredSchemaOptionIds(schemaOptionSource);
  for (const schemaId of schemaOptionIds) {
    const accepted = coverage.schemaAssets.some(
      row => row.schemaId === schemaId && row.status === "accepted" && row.disposition !== "dependency_only" && row.acceptance,
    );
    assert(accepted, `SCHEMA_OPTIONS exposes ${schemaId} without accepted real-path coverage`);
  }
  const workerSource = await readFile(workerPath, "utf8");
  const playgroundSchemas = playgroundSchemaEntries(workerSource);
  assertSameStrings(
    playgroundSchemas.map(entry => entry.optionId),
    schemaOptionIds,
    "SCHEMA_OPTIONS and PLAYGROUND_SCHEMAS keys must match",
  );
  for (const entry of playgroundSchemas) {
    const runtimeAccepted = coverage.schemaAssets.some(
      row => row.schemaId === entry.runtimeId && row.status === "accepted" && row.disposition !== "dependency_only" && row.acceptance,
    );
    assert(
      runtimeAccepted,
      `PLAYGROUND_SCHEMAS.${entry.optionId} runtimeId ${entry.runtimeId} lacks accepted real-path coverage`,
    );
    if (entry.deployedSchemaPath !== undefined) {
      const deployed = coverage.schemaAssets.find(row => row.asset === entry.deployedSchemaPath);
      assert(
        deployed?.status === "accepted" && deployed.disposition !== "dependency_only" && deployed.acceptance,
        `PLAYGROUND_SCHEMAS.${entry.optionId} deployed schema ${entry.deployedSchemaPath} lacks accepted coverage`,
      );
    }
  }

  const requiredValidationLabels = [
    "product-jyutping",
    "canonical-jyutping",
    "cangjie5",
    "luna-pinyin",
    "luna-pinyin-octagram",
    "double-pinyin",
    "bopomofo",
    "stroke-null-map",
    "product-jyutping-plain",
  ];
  assert(Array.isArray(coverage.validationRows), "coverage registry must bind the mandatory deployment matrix");
  assertSameStrings(
    coverage.validationRows.map(row => row.caseLabel),
    requiredValidationLabels,
    "coverage validation rows must match the mandatory deployment matrix",
  );
  const validationTestPath = resolveRepoFile(coverage.validationTestFile, "validation test");
  const validationTestSource = await readFile(validationTestPath, "utf8");
  assert(
    validationTestSource.includes("SCHEMA_ACCEPTANCE_COVERAGE"),
    "deployment matrix test must consume the checked-in coverage registry",
  );
  const seenValidationLabels = new Set();
  const validationByAcceptanceId = new Map();
  for (const row of coverage.validationRows) {
    assert(!seenValidationLabels.has(row.caseLabel), `duplicate validation row ${row.caseLabel}`);
    seenValidationLabels.add(row.caseLabel);
    for (const field of [
      "caseLabel",
      "schemaId",
      "acceptanceId",
      "scope",
      "testName",
      "evidenceFile",
      "input",
      "expectedFinal",
      "defaultOn",
      "explicitFalse",
    ]) {
      assert(typeof row[field] === "string" && row[field].length > 0, `validation ${row.caseLabel}.${field} is required`);
    }
    assert(row.status === "accepted", `validation ${row.caseLabel} is unresolved (${row.status ?? "missing status"})`);
    assert(
      !validationByAcceptanceId.has(row.acceptanceId),
      `duplicate validation acceptanceId ${row.acceptanceId}`,
    );
    validationByAcceptanceId.set(row.acceptanceId, row);
    const evidencePath = resolveRepoFile(row.evidenceFile, `validation ${row.caseLabel} evidence`);
    assert((await stat(evidencePath)).isFile(), `validation ${row.caseLabel} evidence is not a file`);
    assert(
      validationTestSource.includes(`fn ${row.testName}(`),
      `validation ${row.caseLabel} test ${row.testName} is missing`,
    );
    assert(
      validationTestSource.includes(`label: "${row.caseLabel}"`),
      `validation ${row.caseLabel} case is missing from the executable matrix`,
    );
  }
  for (const schemaRow of coverage.schemaAssets.filter(row => row.disposition !== "dependency_only")) {
    const validation = validationByAcceptanceId.get(schemaRow.acceptanceId);
    assert(
      validation !== undefined,
      `${schemaRow.asset} acceptanceId ${schemaRow.acceptanceId} does not resolve to an executable validation row`,
    );
    const acceptance = schemaRow.acceptance;
    for (const [validationField, acceptanceField] of [
      ["caseLabel", "caseLabel"],
      ["testName", "testName"],
      ["evidenceFile", "oracleFixture"],
      ["input", "input"],
      ["expectedFinal", "expectedFinal"],
      ["defaultOn", "defaultOn"],
      ["explicitFalse", "explicitFalse"],
    ]) {
      assert(
        validation[validationField] === acceptance[acceptanceField],
        `${schemaRow.asset} ${acceptanceField} does not match validation ${schemaRow.acceptanceId}`,
      );
    }
    assert(
      validation.schemaId === schemaRow.schemaId,
      `${schemaRow.asset} schemaId does not match validation ${schemaRow.acceptanceId}`,
    );
  }
}

export async function validateSchemaAssetManifest({
  trackedPaths,
  trackedSchemaPaths,
  auditResult,
  reachabilityCoverage,
  asOfDate = new Date(),
} = {}) {
  const [publicAssets, publicDemoAssets] = await Promise.all(manifestPaths.map(validateManifest));
  assert(
    JSON.stringify(publicAssets) === JSON.stringify(publicDemoAssets),
    "public and public-demo schema asset manifests must be identical",
  );

  const manifestAssetPaths = new Set(publicAssets.map(asset => asset.path));
  await assertManifestMatchesTree(publicAssets.map(asset => asset.path));
  const workerSource = await readFile(workerPath, "utf8");
  for (const assetPath of workerLiteralSchemaAssets(workerSource)) {
    assert(
      manifestAssetPaths.has(assetPath),
      `worker references ${assetPath}, but it is missing from schema-asset-manifest.json`,
    );
  }
  await assertNoPoetPayloads();
  await validateAcceptanceCoverage(publicAssets);

  const actualTrackedPaths = trackedPaths ?? await discoverGitTrackedPaths(repoRoot);
  const actualTrackedSchemaPaths = trackedSchemaPaths ?? await discoverGitTrackedPaths(repoRoot, "*.schema.yaml");
  const coverage = reachabilityCoverage ?? await readJson(coveragePath);
  const productSchemaAssets = publicAssets
    .map(asset => asset.path)
    .filter(assetPath => assetPath.endsWith(".schema.yaml"))
    .map(assetPath => `${expectedSourceRoot}/${assetPath}`);
  const actualAuditResult = auditResult ?? await runReachabilityAudit({
    root: repoRoot,
    productRoot: expectedSourceRoot,
    schemaAssets: productSchemaAssets,
  });
  const formalism = await validateReachabilityFormalism({
    coverage,
    auditResult: actualAuditResult,
    trackedPaths: actualTrackedPaths,
    trackedSchemaPaths: actualTrackedSchemaPaths,
    root: repoRoot,
    productManifestAssets: publicAssets.map(asset => asset.path),
    asOfDate,
  });
  return { publicAssets, formalism };
}

export async function main() {
  const result = await validateSchemaAssetManifest();
  console.log(
    `Schema asset manifests verified: ${result.publicAssets.length} assets, ${manifestPaths.map(repoRelative).join(", ")}; M59 coverage and M60 reachability formalism accepted (${result.formalism.auditTuples} audit tuples, ${result.formalism.optOuts} opt-outs)`,
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
  await main();
}
