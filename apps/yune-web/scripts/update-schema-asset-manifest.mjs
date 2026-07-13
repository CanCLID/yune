import { createHash } from "node:crypto";
import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptRoot, "..");
const repoRoot = path.resolve(appRoot, "../..");
const publicSchemaRoot = path.join(appRoot, "public", "schema");
const manifestVersion = "web03-three-schema-launch-v1";
const outputManifestPaths = [
	path.join(appRoot, "public", "schema-asset-manifest.json"),
	path.join(appRoot, "public-demo", "schema-asset-manifest.json"),
];
const coveragePath = path.join(appRoot, "schema-acceptance-coverage.json");

const requiredExtraAssets = [
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

function schemaPath(relativePath) {
	return path.join(publicSchemaRoot, ...relativePath.split("/"));
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

async function schemaIdForAsset(assetPath) {
	const source = await readFile(schemaPath(assetPath), "utf8");
	const match = /^\s*schema_id:\s*([A-Za-z0-9_]+)\s*$/m.exec(source);
	if (match === null) {
		throw new Error(`Manifest schema asset does not declare schema_id: ${assetPath}`);
	}
	return match[1];
}

async function trackedSchemaAssets() {
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

const template = JSON.parse(await readFile(outputManifestPaths[1], "utf8"));
if (template.generatedFor !== "yune-web") {
	throw new Error(`Unexpected manifest generatedFor: ${template.generatedFor}`);
}

const assets = template.assets.map(asset => ({
	path: asset.path,
	tier: asset.tier,
	required: asset.required,
}));

for (const extra of requiredExtraAssets) {
	insertAfter(assets, extra.after, {
		path: extra.path,
		tier: "shared",
		required: true,
	});
}

const manifestedPaths = new Set(assets.map(asset => asset.path));
for (const assetPath of await trackedSchemaAssets()) {
	if (manifestedPaths.has(assetPath)) {
		continue;
	}
	let insertionIndex = -1;
	for (let index = 0; index < assets.length; index += 1) {
		if (assets[index].path.endsWith(".schema.yaml")) {
			insertionIndex = index;
		}
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
	if (!fileStat.isFile()) {
		throw new Error(`Manifest asset is not a file: ${asset.path}`);
	}
	resolvedAssets.push({
		path: asset.path,
		sha256: await sha256(source),
		bytes: fileStat.size,
		tier: asset.tier,
		required: asset.required,
	});
}

const coverage = JSON.parse(await readFile(coveragePath, "utf8"));
if (coverage.version !== "m59-reach03-v1" || !Array.isArray(coverage.schemaAssets)) {
	throw new Error("Unexpected or malformed schema acceptance coverage registry");
}
const existingCoverage = new Map(coverage.schemaAssets.map(row => [row.asset, row]));
const manifestedSchemaAssets = resolvedAssets
	.map(asset => asset.path)
	.filter(assetPath => assetPath.endsWith(".schema.yaml"));
coverage.schemaAssets = [];
for (const assetPath of manifestedSchemaAssets) {
	const existing = existingCoverage.get(assetPath);
	if (existing !== undefined) {
		coverage.schemaAssets.push(existing);
		continue;
	}
	const schemaId = await schemaIdForAsset(assetPath);
	coverage.schemaAssets.push({
		asset: assetPath,
		schemaId,
		status: "open",
		disposition: "unclassified",
		acceptanceId: `open-${schemaId}`,
		reason: "Automatically opened by update-schema-asset-manifest.mjs; classify and add real-path acceptance before the manifest gate can pass.",
	});
}

const manifest = {
	version: manifestVersion,
	generatedFor: "yune-web",
	sourceRoot: path.relative(repoRoot, publicSchemaRoot).replaceAll(path.sep, "/"),
	assets: resolvedAssets,
};

const serialized = `${JSON.stringify(manifest, null, 2)}\n`;
for (const outputPath of outputManifestPaths) {
	await writeFile(outputPath, serialized);
	console.log(`Updated ${path.relative(repoRoot, outputPath).replaceAll(path.sep, "/")}`);
}
await writeFile(coveragePath, `${JSON.stringify(coverage, null, 2)}\n`);
console.log(`Updated ${path.relative(repoRoot, coveragePath).replaceAll(path.sep, "/")}`);
