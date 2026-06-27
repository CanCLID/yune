import { cp, mkdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(appRoot, "../..");
const publicRoot = path.join(appRoot, "public");
const wasmRoot = path.join(repoRoot, "target", "wasm32-unknown-emscripten", "release");

async function fileExists(file) {
	try {
		await stat(file);
		return true;
	} catch {
		return false;
	}
}

async function firstExisting(paths) {
	for (const candidate of paths) {
		if (await fileExists(candidate)) {
			return candidate;
		}
	}
	return null;
}

async function copyRuntime() {
	const jsSource = await firstExisting([
		path.join(wasmRoot, "yune-web.js"),
		path.join(wasmRoot, "typeduck_web_module.js"),
		path.join(wasmRoot, "deps", "yune_web_module.js"),
	]);
	const wasmSource = await firstExisting([
		path.join(wasmRoot, "yune-web.wasm"),
		path.join(wasmRoot, "typeduck_web_module.wasm"),
		path.join(wasmRoot, "deps", "yune_web_module.wasm"),
	]);

	if (jsSource === null || wasmSource === null) {
		throw new Error(
			[
				"Missing yune-web WASM runtime artifacts.",
				"Build them first with scripts/yune-web-wasm-build.sh, then rerun npm --prefix apps/yune-web run build.",
			].join(" "),
		);
	}

	const jsRuntime = await readFile(jsSource, "utf8");
	if (!jsRuntime.includes("createYuneWebModule") || !jsRuntime.includes('Module["HEAPU8"]')) {
		throw new Error(
			[
				`Stale yune-web WASM runtime artifact at ${path.relative(repoRoot, jsSource)}.`,
				"Build a fresh runtime with scripts/yune-web-wasm-build.sh before running the app so WASM heap metrics are available.",
			].join(" "),
		);
	}

	await mkdir(publicRoot, { recursive: true });
	await cp(jsSource, path.join(publicRoot, "yune-web.js"), { force: true });
	await cp(wasmSource, path.join(publicRoot, "yune-web.wasm"), { force: true });
	console.log(`Prepared yune-web runtime from ${path.relative(repoRoot, jsSource)} and ${path.relative(repoRoot, wasmSource)}`);
}

await copyRuntime();
