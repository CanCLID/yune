import { createHash } from "node:crypto";
import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

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

function repoRelative(file) {
  return path.relative(repoRoot, file).replaceAll(path.sep, "/");
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

async function sha256(file) {
  const data = await readFile(file);
  return createHash("sha256").update(data).digest("hex");
}

async function readJson(file) {
  return JSON.parse(await readFile(file, "utf8"));
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

const [publicAssets, publicDemoAssets] = await Promise.all(manifestPaths.map(validateManifest));
assert(
  JSON.stringify(publicAssets) === JSON.stringify(publicDemoAssets),
  "public and public-demo schema asset manifests must be identical",
);

const manifestAssetPaths = new Set(publicAssets.map(asset => asset.path));
const workerSource = await readFile(workerPath, "utf8");
for (const assetPath of workerLiteralSchemaAssets(workerSource)) {
  assert(
    manifestAssetPaths.has(assetPath),
    `worker references ${assetPath}, but it is missing from schema-asset-manifest.json`,
  );
}
await assertNoPoetPayloads();

console.log(`Schema asset manifests verified: ${publicAssets.length} assets, ${manifestPaths.map(repoRelative).join(", ")}`);
