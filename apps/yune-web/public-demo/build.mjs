import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { cp, mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const publicRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(publicRoot, "..");
const repoRoot = path.resolve(appRoot, "../..");
const runtimeRoot = path.join(repoRoot, "packages/yune-web-runtime");
const manifestPath = path.join(publicRoot, "schema-asset-manifest.json");
const outputDir = path.resolve(process.argv[2] ?? path.join(publicRoot, "dist"));

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

async function sha256(file) {
	const data = await readFile(file);
	return createHash("sha256").update(data).digest("hex");
}

await ensurePathInside(outputDir, publicRoot);

const esbuild = commandPath("esbuild");
const vite = commandPath("vite");
if (!await fileExists(esbuild)) throw new Error(`Missing esbuild at ${esbuild}. Run npm --prefix apps/yune-web install first.`);
if (!await fileExists(vite)) throw new Error(`Missing Vite at ${vite}. Run npm --prefix apps/yune-web install first.`);

console.log("Building @yune-ime/yune-web-runtime");
await run("npm", ["--prefix", runtimeRoot, "run", "build"], { cwd: repoRoot });

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
await run(vite, ["build", "--mode", "public"], {
	cwd: appRoot,
	env: { VITE_YUNE_PUBLIC_DEMO: "1" },
});

await rm(outputDir, { recursive: true, force: true });
await mkdir(outputDir, { recursive: true });
await cp(path.join(appRoot, "dist"), outputDir, { recursive: true, force: true });

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
if (manifest.generatedFor !== "yune-web" || manifest.version !== "m31-yune-web-public-demo-v1") {
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
}

for (const file of ["README.md", "PROVENANCE.md", "asset-manifest.md", "cache-policy.md", "schema-asset-manifest.json", "_headers"]) {
	await copyFileWithParents(path.join(publicRoot, file), path.join(outputDir, file));
}

const totalSchemaBytes = manifest.assets.reduce((sum, asset) => sum + asset.bytes, 0);
await writeFile(path.join(outputDir, "build-info.json"), JSON.stringify({
	generatedFor: "yune-web",
	schemaBytes: totalSchemaBytes,
	builtAt: new Date().toISOString(),
}, null, 2) + "\n");
console.log(`Built yune-web public demo at ${outputDir}`);
console.log(`Pinned schema payload bytes: ${totalSchemaBytes}`);
