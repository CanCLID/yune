import { createHash } from "node:crypto";
import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const defaultScriptRoot = path.dirname(scriptPath);
const defaultAppRoot = path.resolve(defaultScriptRoot, "..");
const manifestVersion = "web03-three-schema-launch-v1";
const coverageVersion = "m59-reach03-v1";
const formalismVersion = "m60-reachability-v1";

export const requiredExtraAssets = [
  { after: "luna_pinyin_yune_reverse.dict.yaml", path: "luna_pinyin_yune_reverse.table.bin" },
  { after: "luna_pinyin_yune_reverse.table.bin", path: "luna_pinyin_yune_reverse.reverse.bin" },
  { after: "luna_pinyin_yune_reverse.reverse.bin", path: "luna_pinyin_yune_reverse.prism.bin" },
  { after: "cangjie5.dict.yaml", path: "cangjie5.table.bin" },
  { after: "cangjie5.table.bin", path: "cangjie5.reverse.bin" },
  { after: "cangjie5.reverse.bin", path: "cangjie5.prism.bin" },
];

async function sha256(file) {
  const data = await readFile(file);
  return createHash("sha256").update(data).digest("hex");
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function insertAfter(assets, afterPath, entry) {
  const existingIndex = assets.findIndex(asset => asset.path === entry.path);
  if (existingIndex !== -1) {
    assets[existingIndex] = { ...assets[existingIndex], ...entry };
    return;
  }
  const afterIndex = assets.findIndex(asset => asset.path === afterPath);
  if (afterIndex === -1) {
    throw new Error(`Cannot insert ${entry.path}; ${afterPath} is missing from the manifest template`);
  }
  assets.splice(afterIndex + 1, 0, entry);
}

async function discoverSchemaAssets(publicSchemaRoot) {
  const pending = [publicSchemaRoot];
  const schemaAssets = [];
  while (pending.length > 0) {
    const directory = pending.pop();
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const fullPath = path.join(directory, entry.name);
      if (entry.isDirectory()) {
        pending.push(fullPath);
      } else if (entry.isFile() && entry.name.endsWith(".schema.yaml")) {
        schemaAssets.push(path.relative(publicSchemaRoot, fullPath).replaceAll(path.sep, "/"));
      }
    }
  }
  return schemaAssets.sort((left, right) => left.localeCompare(right));
}

async function schemaIdForAsset(publicSchemaRoot, assetPath) {
  const source = await readFile(path.join(publicSchemaRoot, ...assetPath.split("/")), "utf8");
  const match = /^\s*schema_id:\s*([A-Za-z0-9_]+)\s*$/m.exec(source);
  if (match === null) {
    throw new Error(`Manifest schema asset does not declare schema_id: ${assetPath}`);
  }
  return match[1];
}

/**
 * Reconcile manifest schema assets with the live acceptance registry.
 *
 * This helper deliberately changes only schemaAssets. In particular, the M59
 * coverage version, M60 formalism version, root inventory, and opt-out rows are
 * carried through unchanged. A new asset is blocking-open and never receives
 * or suggests an opt-out.
 */
export function reconcileSchemaCoverage({ coverage, manifestedSchemaAssets, schemaIdsByAsset }) {
  assert(coverage !== null && typeof coverage === "object", "Coverage registry must be an object");
  assert(coverage.version === coverageVersion, `Coverage registry must retain ${coverageVersion}`);
  assert(
    coverage.reachabilityFormalismVersion === formalismVersion,
    `Coverage registry must retain ${formalismVersion}`,
  );
  assert(Array.isArray(coverage.schemaRoots), "Coverage registry must retain schemaRoots");
  assert(Array.isArray(coverage.reachabilityOptOuts), "Coverage registry must retain reachabilityOptOuts");
  assert(Array.isArray(coverage.schemaAssets), "Coverage registry must contain schemaAssets");
  assert(Array.isArray(manifestedSchemaAssets), "Manifest schema asset list must be an array");
  assert(schemaIdsByAsset instanceof Map, "schemaIdsByAsset must be a Map");

  const existingCoverage = new Map(coverage.schemaAssets.map(row => [row.asset, row]));
  const next = structuredClone(coverage);
  next.schemaAssets = manifestedSchemaAssets.map(assetPath => {
    const existing = existingCoverage.get(assetPath);
    if (existing !== undefined) return structuredClone(existing);
    const schemaId = schemaIdsByAsset.get(assetPath);
    assert(typeof schemaId === "string" && schemaId.length > 0, `Missing schema id for ${assetPath}`);
    return {
      asset: assetPath,
      schemaId,
      status: "open",
      disposition: "unclassified",
      acceptanceId: `open-${schemaId}`,
      reason: "Automatically opened by update-schema-asset-manifest.mjs; classify ownership and add real-path acceptance before the manifest gate can pass.",
    };
  });
  return next;
}

/** Production updater, injectable only so its exact path can be exercised safely in tests. */
export async function runSchemaAssetManifestUpdater({
  appRoot = defaultAppRoot,
  repoRoot = path.resolve(appRoot, "../.."),
  extras = requiredExtraAssets,
  log = message => console.log(message),
} = {}) {
  const publicSchemaRoot = path.join(appRoot, "public", "schema");
  const outputManifestPaths = [
    path.join(appRoot, "public", "schema-asset-manifest.json"),
    path.join(appRoot, "public-demo", "schema-asset-manifest.json"),
  ];
  const coveragePath = path.join(appRoot, "schema-acceptance-coverage.json");
  const schemaPath = relativePath => path.join(publicSchemaRoot, ...relativePath.split("/"));

  const template = JSON.parse(await readFile(outputManifestPaths[1], "utf8"));
  if (template.generatedFor !== "yune-web") {
    throw new Error(`Unexpected manifest generatedFor: ${template.generatedFor}`);
  }

  const assets = template.assets.map(asset => ({
    path: asset.path,
    tier: asset.tier,
    required: asset.required,
  }));
  for (const extra of extras) {
    insertAfter(assets, extra.after, {
      path: extra.path,
      tier: "shared",
      required: true,
    });
  }

  const manifestedPaths = new Set(assets.map(asset => asset.path));
  for (const assetPath of await discoverSchemaAssets(publicSchemaRoot)) {
    if (manifestedPaths.has(assetPath)) continue;
    let insertionIndex = -1;
    for (let index = 0; index < assets.length; index += 1) {
      if (assets[index].path.endsWith(".schema.yaml")) insertionIndex = index;
    }
    assets.splice(insertionIndex + 1, 0, {
      path: assetPath,
      tier: "shared",
      required: true,
    });
    manifestedPaths.add(assetPath);
  }

  const resolvedAssets = [];
  for (const asset of assets) {
    const source = schemaPath(asset.path);
    const fileStat = await stat(source);
    if (!fileStat.isFile()) throw new Error(`Manifest asset is not a file: ${asset.path}`);
    resolvedAssets.push({
      path: asset.path,
      sha256: await sha256(source),
      bytes: fileStat.size,
      tier: asset.tier,
      required: asset.required,
    });
  }

  const coverage = JSON.parse(await readFile(coveragePath, "utf8"));
  const manifestedSchemaAssets = resolvedAssets
    .map(asset => asset.path)
    .filter(assetPath => assetPath.endsWith(".schema.yaml"));
  const schemaIdsByAsset = new Map();
  for (const assetPath of manifestedSchemaAssets) {
    schemaIdsByAsset.set(assetPath, await schemaIdForAsset(publicSchemaRoot, assetPath));
  }
  const reconciledCoverage = reconcileSchemaCoverage({
    coverage,
    manifestedSchemaAssets,
    schemaIdsByAsset,
  });

  const manifest = {
    version: manifestVersion,
    generatedFor: "yune-web",
    sourceRoot: path.relative(repoRoot, publicSchemaRoot).replaceAll(path.sep, "/"),
    assets: resolvedAssets,
  };
  const serialized = `${JSON.stringify(manifest, null, 2)}\n`;
  for (const outputPath of outputManifestPaths) {
    await writeFile(outputPath, serialized);
    log(`Updated ${path.relative(repoRoot, outputPath).replaceAll(path.sep, "/")}`);
  }
  await writeFile(coveragePath, `${JSON.stringify(reconciledCoverage, null, 2)}\n`);
  log(`Updated ${path.relative(repoRoot, coveragePath).replaceAll(path.sep, "/")}`);
  return { manifest, coverage: reconciledCoverage };
}

export async function main(options) {
  return await runSchemaAssetManifestUpdater(options);
}

if (process.argv[1] && path.resolve(process.argv[1]) === scriptPath) {
  await main();
}
