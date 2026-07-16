import { createHash } from "node:crypto";
import { readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";

export const manifestName = "public-artifact-manifest.json";
export const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

const expectedToolchain = {
  emsdkVersion: "4.0.23",
  emscriptenReleaseCommit: "aaa43392544d695232b70eda706d751f18980c2a",
  emsdkRepositoryCommit: "db04e88298d9916fc51fcd3743045ca3eb695127",
  emccVersion:
    "emcc (Emscripten gcc/clang-like replacement + linker emulating GNU ld) 4.0.23 (7a5d93b50f6a3a35e85a0d2fc9e667b8498e6aed)",
  rustcVersion: "rustc 1.96.1 (31fca3adb 2026-06-26)",
  nodeVersion: "v22.16.0",
};

function checkedArtifactPath(distRoot, relative) {
  if (
    typeof relative !== "string" ||
    relative === "" ||
    relative.includes("\\") ||
    relative.startsWith("/") ||
    relative.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new Error(`Invalid public artifact path: ${JSON.stringify(relative)}`);
  }
  return path.join(distRoot, ...relative.split("/"));
}

async function listedArtifactPaths(distRoot, relativeRoot = "") {
  const entries = await readdir(path.join(distRoot, relativeRoot), {
    withFileTypes: true,
  });
  const files = [];
  entries.sort((left, right) =>
    left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
  );
  for (const entry of entries) {
    const relative = path.posix.join(relativeRoot, entry.name);
    if (relative === "build-info.json" || relative === manifestName) continue;
    if (entry.isDirectory()) {
      files.push(...(await listedArtifactPaths(distRoot, relative)));
    } else if (entry.isFile()) {
      files.push(relative);
    } else {
      throw new Error(`Unsupported public artifact entry: ${relative}`);
    }
  }
  return files;
}

export async function validateLocalBundle(distRootValue, expectedCommit) {
  const distRoot = path.resolve(distRootValue);
  const buildInfoBytes = await readFile(path.join(distRoot, "build-info.json"));
  const buildInfo = JSON.parse(buildInfoBytes.toString("utf8"));
  const manifestBytes = await readFile(path.join(distRoot, manifestName));
  const manifest = JSON.parse(manifestBytes.toString("utf8"));

  if (buildInfo.sourceCommit !== expectedCommit || buildInfo.sourceTreeState !== "clean") {
    throw new Error("Local release bundle does not identify the expected clean source commit");
  }
  for (const [key, expected] of Object.entries(expectedToolchain)) {
    if (buildInfo.toolchain?.[key] !== expected) {
      throw new Error(`Local release bundle has unexpected toolchain field ${key}`);
    }
  }
  if (sha256(manifestBytes) !== buildInfo.publicArtifactManifestSha256) {
    throw new Error("Local public artifact manifest does not match build-info.json");
  }
  if (
    manifest.generatedFor !== "yune-web" ||
    manifest.version !== "web03-public-artifact-v1" ||
    !Array.isArray(manifest.files)
  ) {
    throw new Error("Local public artifact manifest metadata is invalid");
  }

  const inventory = new Map();
  const expectedPaths = [];
  for (const file of manifest.files) {
    const absolute = checkedArtifactPath(distRoot, file?.path);
    if (
      !Number.isSafeInteger(file?.bytes) ||
      file.bytes < 0 ||
      !/^[0-9a-f]{64}$/.test(file?.sha256 ?? "") ||
      inventory.has(file.path)
    ) {
      throw new Error(`Invalid public artifact row: ${JSON.stringify(file)}`);
    }
    const metadata = await stat(absolute);
    const bytes = await readFile(absolute);
    if (metadata.size !== file.bytes || sha256(bytes) !== file.sha256) {
      throw new Error(`Local public artifact ${file.path} differs from its inventory row`);
    }
    expectedPaths.push(file.path);
    inventory.set(file.path, file);
  }
  const actualPaths = await listedArtifactPaths(distRoot);
  if (JSON.stringify(actualPaths) !== JSON.stringify(expectedPaths)) {
    throw new Error("Local public artifact manifest does not exactly reconcile to dist");
  }
  for (const required of [
    "index.html",
    "worker.js",
    "yune-web.wasm",
    "schema-asset-manifest.json",
  ]) {
    if (!inventory.has(required)) {
      throw new Error(`Local public artifact manifest is missing ${required}`);
    }
  }
  if (![...inventory.keys()].some((file) => /^assets\/.*\.js$/.test(file))) {
    throw new Error("Local public artifact manifest is missing the app bundle");
  }
  if (inventory.get("yune-web.wasm").sha256 !== buildInfo.wasmSha256) {
    throw new Error("Local public artifact inventory and build-info disagree on WASM");
  }
  if (
    inventory.get("schema-asset-manifest.json").sha256 !==
    buildInfo.schemaManifestSha256
  ) {
    throw new Error("Local public artifact inventory and build-info disagree on schema manifest");
  }
  return { buildInfoBytes, manifestBytes, manifest, inventory };
}
