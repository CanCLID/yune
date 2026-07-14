import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { cp, mkdir, mkdtemp, readFile, readdir, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const publicRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(publicRoot, "..");
const repoRoot = path.resolve(appRoot, "../..");
const runtimeRoot = path.join(repoRoot, "packages/yune-web-runtime");
const manifestPath = path.join(publicRoot, "schema-asset-manifest.json");
const outputDir = path.resolve(process.argv[2] ?? path.join(publicRoot, "dist"));
const viteOutputDir = await mkdtemp(path.join(tmpdir(), "yune-web-public-build-"));

// Cloudflare Pages rejects any single deployed asset larger than 25 MiB. Schema
// payloads that exceed the cap (e.g. the jyut6ping3_mobile prism) are emitted as
// ordered `<name>.partN` chunks that the browser worker fetches and reassembles
// before writing them into the WASM filesystem. Keep these bounds in sync with
// PAGES_MAX_ASSET_BYTES / SPLIT_CHUNK_BYTES in apps/yune-web/src/worker.ts.
const PAGES_MAX_ASSET_BYTES = 25 * 1024 * 1024;
const SPLIT_CHUNK_BYTES = 20 * 1024 * 1024;
const PUBLIC_ARTIFACT_MANIFEST = "public-artifact-manifest.json";

function publicBuildToolchain() {
	const fields = {
		emsdkVersion: process.env.YUNE_WEB_EMSDK_VERSION,
		emscriptenReleaseCommit: process.env.YUNE_WEB_EMSCRIPTEN_RELEASE_COMMIT,
		emsdkRepositoryCommit: process.env.YUNE_WEB_EMSDK_REPOSITORY_COMMIT,
		emccVersion: process.env.YUNE_WEB_EMCC_VERSION,
		rustcVersion: process.env.YUNE_WEB_RUSTC_VERSION,
		nodeVersion: process.env.YUNE_WEB_NODE_VERSION,
	};
	const present = Object.values(fields).filter(value => value !== undefined && value.trim() !== "");
	if (present.length === 0) {
		if (process.env.YUNE_WEB_REQUIRE_TOOLCHAIN_RECEIPT === "1") {
			throw new Error("Public release build is missing its pinned toolchain receipt");
		}
		return null;
	}
	if (present.length !== Object.keys(fields).length) {
		throw new Error("Public build toolchain receipt is incomplete");
	}
	return fields;
}

function commandPath(name) {
	return path.join(appRoot, "node_modules", ".bin", `${name}${process.platform === "win32" ? ".cmd" : ""}`);
}

async function ensurePathInside(child, parent) {
	const resolvedParent = path.resolve(parent);
	const resolvedChild = path.resolve(child);
	const relative = path.relative(resolvedParent, resolvedChild);
	if (relative.startsWith("..") || path.isAbsolute(relative)) {
		throw new Error(`OutputDir must stay under ${resolvedParent}`);
	}
}

async function run(command, args, options = {}) {
	await new Promise((resolve, reject) => {
		const child = spawn(command, args, {
			cwd: options.cwd,
			env: { ...process.env, ...options.env },
			stdio: "inherit",
			shell: process.platform === "win32",
		});
		child.on("error", reject);
		child.on("exit", code => {
			if (code === 0) resolve();
			else reject(new Error(`${command} ${args.join(" ")} exited with ${code}`));
		});
	});
}

async function commandOutput(command, args, options = {}) {
	return new Promise((resolve, reject) => {
		const child = spawn(command, args, {
			cwd: options.cwd,
			env: { ...process.env, ...options.env },
			stdio: ["ignore", "pipe", "pipe"],
			shell: process.platform === "win32",
		});
		let stdout = "";
		let stderr = "";
		child.stdout.setEncoding("utf8");
		child.stderr.setEncoding("utf8");
		child.stdout.on("data", chunk => { stdout += chunk; });
		child.stderr.on("data", chunk => { stderr += chunk; });
		child.on("error", reject);
		child.on("exit", code => {
			if (code === 0) resolve(stdout);
			else reject(new Error(`${command} ${args.join(" ")} exited with ${code}: ${stderr.trim()}`));
		});
	});
}

async function sourceIdentity() {
	const gitCommit = (await commandOutput("git", ["rev-parse", "HEAD"], { cwd: repoRoot }))
		.trim()
		.toLowerCase();
	if (!/^[0-9a-f]{40}$/.test(gitCommit)) {
		throw new Error(`Public build Git HEAD is not a full SHA: ${gitCommit}`);
	}
	const environmentCommit = (process.env.CF_PAGES_COMMIT_SHA ?? process.env.GITHUB_SHA)
		?.trim()
		.toLowerCase();
	if (environmentCommit !== undefined && environmentCommit !== gitCommit) {
		throw new Error(`Public build environment commit ${environmentCommit} does not match Git HEAD ${gitCommit}`);
	}
	const status = await commandOutput(
		"git",
		["status", "--porcelain", "--untracked-files=all"],
		{ cwd: repoRoot },
	);
	return {
		sourceCommit: gitCommit,
		sourceTreeState: status.trim() === "" ? "clean" : "dirty",
	};
}

async function fileExists(file) {
	try {
		await stat(file);
		return true;
	} catch {
		return false;
	}
}

async function copyFileWithParents(source, target) {
	await mkdir(path.dirname(target), { recursive: true });
	await cp(source, target, { force: true });
}

async function splitOversizedAsset(target, relative, expectedBytes) {
	const data = await readFile(target);
	if (data.byteLength !== expectedBytes) {
		throw new Error(`Split source ${relative} is ${data.byteLength} bytes, expected ${expectedBytes}`);
	}
	const partCount = Math.ceil(data.byteLength / SPLIT_CHUNK_BYTES);
	for (let index = 0; index < partCount; index += 1) {
		const chunk = data.subarray(index * SPLIT_CHUNK_BYTES, (index + 1) * SPLIT_CHUNK_BYTES);
		if (chunk.byteLength > PAGES_MAX_ASSET_BYTES) {
			throw new Error(`Split part ${relative}.part${index} is ${chunk.byteLength} bytes, over the Cloudflare Pages 25 MiB limit`);
		}
		await writeFile(`${target}.part${index}`, chunk);
	}
	await rm(target, { force: true });
	console.log(`Split ${relative} into ${partCount} parts (${data.byteLength} bytes) to satisfy the Cloudflare Pages 25 MiB asset limit`);
}

async function sha256(file) {
	const data = await readFile(file);
	return createHash("sha256").update(data).digest("hex");
}

async function publicArtifactFiles(root, relativeRoot = "") {
	const entries = await readdir(path.join(root, relativeRoot), { withFileTypes: true });
	const files = [];
	for (const entry of entries.sort((left, right) => left.name < right.name ? -1 : left.name > right.name ? 1 : 0)) {
		const relative = path.posix.join(relativeRoot.replaceAll("\\", "/"), entry.name);
		if (relative === "build-info.json" || relative === PUBLIC_ARTIFACT_MANIFEST) {
			continue;
		}
		if (entry.isDirectory()) {
			files.push(...await publicArtifactFiles(root, relative));
		} else if (entry.isFile()) {
			const absolute = path.join(root, ...relative.split("/"));
			files.push({
				path: relative,
				bytes: (await stat(absolute)).size,
				sha256: await sha256(absolute),
			});
		} else {
			throw new Error(`Unsupported public artifact entry: ${relative}`);
		}
	}
	return files;
}

await ensurePathInside(outputDir, publicRoot);

const esbuild = commandPath("esbuild");
const vite = commandPath("vite");
if (!await fileExists(esbuild)) throw new Error(`Missing esbuild at ${esbuild}. Run npm --prefix apps/yune-web install first.`);
if (!await fileExists(vite)) throw new Error(`Missing Vite at ${vite}. Run npm --prefix apps/yune-web install first.`);

console.log("Building @yune-ime/yune-web-runtime");
await run("npm", ["--prefix", runtimeRoot, "run", "build"], { cwd: repoRoot });

console.log("Preparing yune-web WASM runtime");
await run("node", [path.join(appRoot, "scripts", "prepare-runtime.mjs")], { cwd: appRoot });

console.log("Bundling yune-web worker");
await run(esbuild, [
	"src/worker.ts",
	"--bundle",
	"--format=iife",
	"--outdir=public",
	"--define:YUNE_PUBLIC_DEMO_BUILD=true",
	"--minify",
], { cwd: appRoot });

console.log("Building yune-web app");
await run(vite, ["build", "--mode", "public", "--outDir", viteOutputDir, "--emptyOutDir"], {
	cwd: appRoot,
	env: { VITE_YUNE_PUBLIC_DEMO: "1" },
});

await rm(outputDir, { recursive: true, force: true });
await mkdir(outputDir, { recursive: true });
await cp(viteOutputDir, outputDir, { recursive: true, force: true });
await rm(viteOutputDir, { recursive: true, force: true });

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
if (manifest.generatedFor !== "yune-web" || manifest.version !== "web03-three-schema-launch-v1") {
	throw new Error("Unexpected schema asset manifest metadata");
}

const outputSchema = path.join(outputDir, "schema");
await rm(outputSchema, { recursive: true, force: true });
await mkdir(outputSchema, { recursive: true });

const sourceSchema = path.join(appRoot, "public", "schema");
for (const asset of manifest.assets) {
	const relative = asset.path;
	const source = path.join(sourceSchema, ...relative.split("/"));
	const target = path.join(outputSchema, ...relative.split("/"));
	if (!await fileExists(source)) throw new Error(`Missing public schema source asset: ${relative}`);
	await copyFileWithParents(source, target);
	const actualHash = await sha256(target);
	if (actualHash !== asset.sha256) {
		throw new Error(`SHA-256 mismatch for ${relative}. Expected ${asset.sha256}, got ${actualHash}`);
	}
	if (asset.bytes > PAGES_MAX_ASSET_BYTES) {
		await splitOversizedAsset(target, relative, asset.bytes);
	}
}

for (const file of ["README.md", "PROVENANCE.md", "asset-manifest.md", "cache-policy.md", "schema-asset-manifest.json", "_headers"]) {
	await copyFileWithParents(path.join(publicRoot, file), path.join(outputDir, file));
}

const totalSchemaBytes = manifest.assets.reduce((sum, asset) => sum + asset.bytes, 0);
const wasmOutput = path.join(outputDir, "yune-web.wasm");
if (!await fileExists(wasmOutput)) {
	throw new Error(`Public build is missing ${wasmOutput}`);
}
const identity = await sourceIdentity();
const toolchain = publicBuildToolchain();
const artifactManifestPath = path.join(outputDir, PUBLIC_ARTIFACT_MANIFEST);
await writeFile(artifactManifestPath, JSON.stringify({
	generatedFor: "yune-web",
	version: "web03-public-artifact-v1",
	files: await publicArtifactFiles(outputDir),
}, null, 2) + "\n");
await writeFile(path.join(outputDir, "build-info.json"), JSON.stringify({
	generatedFor: "yune-web",
	schemaBytes: totalSchemaBytes,
	builtAt: new Date().toISOString(),
	...identity,
	toolchain,
	schemaManifestSha256: await sha256(manifestPath),
	wasmSha256: await sha256(wasmOutput),
	publicArtifactManifestSha256: await sha256(artifactManifestPath),
}, null, 2) + "\n");
console.log(`Built yune-web public demo at ${outputDir}`);
console.log(`Pinned schema payload bytes: ${totalSchemaBytes}`);
