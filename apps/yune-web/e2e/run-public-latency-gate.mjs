import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, readdir, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const e2eRoot = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(e2eRoot, "..");
const publicDist = path.join(appRoot, "public-demo", "dist");
const viteCli = path.join(appRoot, "node_modules", "vite", "bin", "vite.js");
const playwrightCli = path.join(
  e2eRoot,
  "node_modules",
  "@playwright",
  "test",
  "cli.js",
);
const host = "127.0.0.1";
const port = Number(process.env.YUNE_WEB_LATENCY_PREVIEW_PORT ?? "4173");
if (!Number.isInteger(port) || port < 1 || port > 65535) {
  throw new Error(`Invalid YUNE_WEB_LATENCY_PREVIEW_PORT: ${port}`);
}
const appUrl = `http://${host}:${port}/`;
const artifactManifestName = "public-artifact-manifest.json";

async function commandOutput(command, args, cwd) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
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
    child.once("exit", (code) => {
      if (code === 0) {
        resolve(stdout);
      } else {
        reject(
          new Error(
            `${command} ${args.join(" ")} exited with ${code}: ${stderr.trim()}`,
          ),
        );
      }
    });
  });
}

await Promise.all([
  stat(path.join(publicDist, "build-info.json")),
  stat(path.join(publicDist, artifactManifestName)),
  stat(
    path.join(
      publicDist,
      "schema",
      "jyut6ping3_mobile.prism.bin.part0",
    ),
  ),
  stat(
    path.join(
      publicDist,
      "schema",
      "jyut6ping3_mobile.prism.bin.part1",
    ),
  ),
  stat(viteCli),
  stat(playwrightCli),
]);
const buildInfo = JSON.parse(
  await readFile(path.join(publicDist, "build-info.json"), "utf8"),
);
if (!/^[0-9a-f]{40}$/.test(buildInfo.sourceCommit ?? "")) {
  throw new Error("Public build-info.json is missing its full source commit");
}
const currentHead = (
  await commandOutput("git", ["rev-parse", "HEAD"], e2eRoot)
)
  .trim()
  .toLowerCase();
if (currentHead !== buildInfo.sourceCommit) {
  throw new Error(
    `Public artifact commit ${buildInfo.sourceCommit} does not match current HEAD ${currentHead}`,
  );
}
const currentStatus = await commandOutput(
  "git",
  ["status", "--porcelain", "--untracked-files=all"],
  e2eRoot,
);
if (currentStatus.trim() !== "") {
  throw new Error("Release latency gate requires the current Git tree to be clean");
}
if (buildInfo.sourceTreeState !== "clean") {
  throw new Error(
    `Release latency gate requires a clean source tree; build recorded ${buildInfo.sourceTreeState}`,
  );
}

async function sha256(file) {
  return createHash("sha256").update(await readFile(file)).digest("hex");
}

function checkedArtifactPath(relative) {
  if (
    typeof relative !== "string" ||
    relative === "" ||
    relative.includes("\\") ||
    relative.startsWith("/") ||
    relative.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new Error(`Invalid public artifact path: ${JSON.stringify(relative)}`);
  }
  return path.join(publicDist, ...relative.split("/"));
}

async function listedArtifactPaths(root, relativeRoot = "") {
  const entries = await readdir(path.join(root, relativeRoot), {
    withFileTypes: true,
  });
  const files = [];
  entries.sort((left, right) =>
    left.name < right.name ? -1 : left.name > right.name ? 1 : 0,
  );
  for (const entry of entries) {
    const relative = path.posix.join(
      relativeRoot.replaceAll("\\", "/"),
      entry.name,
    );
    if (relative === "build-info.json" || relative === artifactManifestName) {
      continue;
    }
    if (entry.isDirectory()) {
      files.push(...(await listedArtifactPaths(root, relative)));
    } else if (entry.isFile()) {
      files.push(relative);
    } else {
      throw new Error(`Unsupported public artifact entry: ${relative}`);
    }
  }
  return files;
}

const artifactManifestPath = path.join(publicDist, artifactManifestName);
const artifactManifestBytes = await readFile(artifactManifestPath);
const artifactManifestSha256 = createHash("sha256")
  .update(artifactManifestBytes)
  .digest("hex");
if (artifactManifestSha256 !== buildInfo.publicArtifactManifestSha256) {
  throw new Error(
    `Public artifact manifest hash ${artifactManifestSha256} does not match build-info ${buildInfo.publicArtifactManifestSha256}`,
  );
}
const artifactManifest = JSON.parse(artifactManifestBytes.toString("utf8"));
if (
  artifactManifest.generatedFor !== "yune-web" ||
  artifactManifest.version !== "web03-public-artifact-v1" ||
  !Array.isArray(artifactManifest.files)
) {
  throw new Error("Public artifact manifest metadata is invalid");
}
const expectedPaths = [];
const inventory = new Map();
for (const file of artifactManifest.files) {
  const absolute = checkedArtifactPath(file?.path);
  if (
    !Number.isSafeInteger(file?.bytes) ||
    file.bytes < 0 ||
    !/^[0-9a-f]{64}$/.test(file?.sha256 ?? "") ||
    inventory.has(file.path)
  ) {
    throw new Error(`Invalid public artifact row: ${JSON.stringify(file)}`);
  }
  const metadata = await stat(absolute);
  const actualHash = await sha256(absolute);
  if (metadata.size !== file.bytes || actualHash !== file.sha256) {
    throw new Error(
      `Public artifact ${file.path} does not match its inventory row`,
    );
  }
  expectedPaths.push(file.path);
  inventory.set(file.path, file);
}
const actualPaths = await listedArtifactPaths(publicDist);
if (JSON.stringify(actualPaths) !== JSON.stringify(expectedPaths)) {
  throw new Error("Public artifact manifest does not exactly reconcile to dist");
}
for (const required of ["index.html", "worker.js", "yune-web.wasm", "schema-asset-manifest.json"]) {
  if (!inventory.has(required)) {
    throw new Error(`Public artifact manifest is missing ${required}`);
  }
}
if (![...inventory.keys()].some((file) => /^assets\/.*\.js$/.test(file))) {
  throw new Error("Public artifact manifest is missing the rendered app bundle");
}
if (inventory.get("yune-web.wasm").sha256 !== buildInfo.wasmSha256) {
  throw new Error("Public artifact inventory and build-info disagree on WASM");
}
if (
  inventory.get("schema-asset-manifest.json").sha256 !==
  buildInfo.schemaManifestSha256
) {
  throw new Error(
    "Public artifact inventory and build-info disagree on the schema manifest",
  );
}

const temporaryOutput = process.env.YUNE_WEB_LATENCY_OUTPUT_DIR
  ? null
  : await mkdtemp(path.join(tmpdir(), "yune-web-input-latency-"));
const outputDir =
  process.env.YUNE_WEB_LATENCY_OUTPUT_DIR ?? temporaryOutput;
const preview = spawn(
  process.execPath,
  [
    viteCli,
    "preview",
    "--host",
    host,
    "--port",
    String(port),
    "--strictPort",
    "--outDir",
    publicDist,
  ],
  {
    cwd: appRoot,
    env: process.env,
    stdio: "inherit",
  },
);
let previewExit = null;
let previewError = null;
let finishPreview;
const previewStopped = new Promise((resolve) => {
  finishPreview = resolve;
});
preview.once("exit", (code, signal) => {
  previewExit = { code, signal };
  finishPreview();
});
preview.once("error", (error) => {
  previewError = error;
  finishPreview();
});

async function waitForPreview() {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    if (previewExit !== null) {
      throw new Error(
        `Public preview exited before readiness: ${JSON.stringify(previewExit)}`,
      );
    }
    if (previewError !== null) {
      throw previewError;
    }
    try {
      const response = await fetch(new URL("build-info.json", appUrl));
      if (response.ok) {
        const value = await response.json();
        if (value.sourceCommit === buildInfo.sourceCommit) {
          return;
        }
      }
    } catch {
      // The preview may still be binding its port.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Public preview did not become ready at ${appUrl}`);
}

async function stopPreview() {
  if (previewExit !== null || previewError !== null) {
    return;
  }
  preview.kill();
  const stopped = await Promise.race([
    previewStopped.then(() => true),
    new Promise((resolve) => setTimeout(() => resolve(false), 5_000)),
  ]);
  if (stopped) {
    return;
  }
  preview.kill("SIGKILL");
  const killed = await Promise.race([
    previewStopped.then(() => true),
    new Promise((resolve) => setTimeout(() => resolve(false), 2_000)),
  ]);
  if (!killed) {
    throw new Error("Public preview did not terminate after SIGKILL");
  }
}

async function runLatencyGate() {
  await waitForPreview();
  return new Promise((resolve, reject) => {
    const child = spawn(
      process.execPath,
      [playwrightCli, "test", "--config", "playwright.latency.config.ts"],
      {
        cwd: e2eRoot,
        env: {
          ...process.env,
          YUNE_WEB_APP_URL: appUrl,
          YUNE_WEB_LATENCY_OUTPUT_DIR: outputDir,
        },
        stdio: "inherit",
      },
    );
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) {
        resolve();
      } else {
        reject(
          new Error(
            `Input-latency gate exited with code ${code}, signal ${signal}`,
          ),
        );
      }
    });
  });
}

try {
  await runLatencyGate();
} finally {
  try {
    await stopPreview();
  } finally {
    if (temporaryOutput !== null) {
      await rm(temporaryOutput, { recursive: true, force: true });
    }
  }
}
