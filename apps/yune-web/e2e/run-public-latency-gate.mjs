import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import {
  mkdtemp,
  mkdir,
  readFile,
  readdir,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

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
const latencyEvidenceName = "input-latency-hard-stop.json";
const normalTypingEvidenceName = "normal-typing-exact-input.json";
const normalTypingInput =
  "ngodeigungsijigaahaidoumaaigangeihaaijansougeoi";
const normalTypingKeyIntervalMs = 100;
const normalTypingP95CeilingMs = 150;
const normalTypingMaxCeilingMs = 250;
const normalTypingQueueWaitMaxCeilingMs = 100;
const releaseScenarioNames = [
  "jyutping-short",
  "jyutping-historical-long-1",
  "jyutping-historical-long-2",
  "typeduck-learned-userdb-prefix",
  "luna-short",
  "luna-37",
  "luna-59",
  "cangjie-short",
];
const releaseScenarioInputLengths = [3, 28, 52, 3, 3, 37, 59, 1];
const releaseScenarioInputs = [
  "hai",
  "sihaacoenggeoisyujapgecukdou",
  "taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng",
  "ngo",
  "hao",
  "ceshiyixiachangjushuruxingnengzenyang",
  "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
  "a",
];
const releaseExpectedFirstCandidateTexts = [
  "\u4fc2",
  "\u6642\u4e0b\u5834\u64da\u8f38\u5165\u5605\u901f\u5ea6",
  "\u7747\u4e0b\u5982\u679c\u6253\u597d\u5834\u5605\u53e5\u5b50\u500b\u6027\u80fd\u6703\u9ede\u6a23",
  "\u6211",
  "\u597d",
  "\u6e2c\u8a66\u4e00\u4e0b\u9577\u53e5\u8f38\u5165\u6027\u80fd\u600e\u6a23",
  "\u9019\u500b\u5f15\u64ce\u5176\u5be6\u61c9\u8a72\u652f\u6301\u8d85\u9577\u53e5\u5b50\u8f38\u5165\u624d\u80fd\u7528",
  "\u65e5",
];
const releaseP95CeilingMs = 750;
const releaseMaxCeilingMs = 1000;
const releaseKeyIntervalMs = 250;
const releaseCpuThrottleRate = 4;
const releaseWorkerActionMultiplier = 4;
const releaseVisibleCandidateCount = 6;
const releaseVerifiedKeyCount = 186;
const releaseCadenceGapCount =
  releaseVerifiedKeyCount - releaseScenarioNames.length;
const releaseMechanism =
  "main-cdp-plus-synthetic-keydown-worker-service-amplification";
const releaseWorkerStressScope = "loopback-preview";
const releaseRustcVersion = "rustc 1.96.1 (31fca3adb 2026-06-26)";
const releaseNodeVersion = "v22.16.0";

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
await mkdir(outputDir, { recursive: true });
const gateStartedAt = new Date().toISOString();
await Promise.all([
  writeFile(
    path.join(outputDir, latencyEvidenceName),
    `${JSON.stringify(
      {
        generatedAt: gateStartedAt,
        url: appUrl,
        measurementCompleted: false,
        passed: false,
        thresholdVerdict: "fail",
        releaseGradeVerdict: "fail",
        failurePhase: "preview-or-browser-setup-incomplete",
        buildInfo,
      },
      null,
      2,
    )}\n`,
    "utf8",
  ),
  writeFile(
    path.join(outputDir, normalTypingEvidenceName),
    `${JSON.stringify(
      {
        generatedAt: gateStartedAt,
        url: appUrl,
        measurementCompleted: false,
        passed: false,
        typingMode: "normal-interactive-exact-input",
        schemaId: "jyut6ping3",
        input: normalTypingInput,
        inputLength: normalTypingInput.length,
        failurePhase: "preview-or-browser-setup-incomplete",
        buildInfo,
      },
      null,
      2,
    )}\n`,
    "utf8",
  ),
]);
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
          YUNE_WEB_EXPECTED_SOURCE_COMMIT: currentHead,
          YUNE_WEB_LATENCY_OUTPUT_DIR: outputDir,
          YUNE_WEB_LATENCY_EVIDENCE_DIR: outputDir,
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

function sameJson(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function isNonemptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function percentile(values, proportion) {
  if (values.length === 0) {
    return null;
  }
  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.ceil((sorted.length - 1) * proportion);
  return sorted[Math.min(index, sorted.length - 1)];
}

function emitFullReceipt(prefix, rawReceipt) {
  const rawBytes = Buffer.from(rawReceipt, "utf8");
  const encoded = gzipSync(rawBytes, { level: 9 }).toString("base64");
  const chunkSize = 12 * 1024;
  const chunks = Array.from(
    { length: Math.ceil(encoded.length / chunkSize) },
    (_, index) => encoded.slice(index * chunkSize, (index + 1) * chunkSize),
  );
  console.error(
    `${prefix}_FULL_MANIFEST=${JSON.stringify({
      encoding: "gzip+base64",
      rawBytes: rawBytes.byteLength,
      encodedBytes: encoded.length,
      sha256: createHash("sha256").update(rawBytes).digest("hex"),
      chunkCount: chunks.length,
    })}`,
  );
  for (const [index, chunk] of chunks.entries()) {
    console.error(
      `${prefix}_FULL_CHUNK_${String(index + 1).padStart(4, "0")}_OF_${String(chunks.length).padStart(4, "0")}=${chunk}`,
    );
  }
}

function assertReleaseLatencyReceipt(evidence) {
  const violations = [];
  const requireValue = (condition, message) => {
    if (!condition) {
      violations.push(message);
    }
  };

  requireValue(
    evidence.measurementCompleted === true &&
      evidence.passed === true &&
      evidence.thresholdVerdict === "pass" &&
      evidence.releaseGradeVerdict === "pass" &&
      evidence.recordedFailureCount === 0 &&
      evidence.buildInfo?.sourceCommit === currentHead,
    "receipt is not a passing release-grade measurement for current HEAD",
  );
  requireValue(
    sameJson(evidence.ceilingsMs, {
      p95: releaseP95CeilingMs,
      max: releaseMaxCeilingMs,
    }),
    "release ceilings must be exactly p95=750 ms and max=1000 ms",
  );
  requireValue(
    evidence.keyIntervalMs === releaseKeyIntervalMs,
    "release key interval must be exactly 250 ms",
  );
  requireValue(
    evidence.cpuProfile?.mainThread?.requestedRate ===
      releaseCpuThrottleRate,
    "release main-thread throttle must be exactly 4x",
  );
  requireValue(
    evidence.cpuProfile?.workerActionMultiplier ===
      releaseWorkerActionMultiplier,
    "release worker action multiplier must be exactly 4x",
  );
  requireValue(
    evidence.cpuProfile?.mechanism === releaseMechanism,
    `release mechanism must be ${releaseMechanism}`,
  );
  requireValue(
    evidence.cpuProfile?.workerStressScope === releaseWorkerStressScope,
    "release worker stress scope must be loopback-preview",
  );
  requireValue(
    evidence.buildInfo?.toolchain?.rustcVersion === releaseRustcVersion &&
      evidence.buildInfo?.toolchain?.nodeVersion === releaseNodeVersion,
    `release toolchain must use ${releaseRustcVersion} and Node ${releaseNodeVersion}`,
  );

  const scenarios = Array.isArray(evidence.scenarios)
    ? evidence.scenarios
    : [];
  requireValue(
    sameJson(
      scenarios.map((scenario) => scenario?.name),
      releaseScenarioNames,
    ),
    "release scenarios must use the exact 8-name order",
  );

  let diagnosticCount = 0;
  let cadenceGapCount = 0;
  for (const [index, scenario] of scenarios.entries()) {
    const diagnostics = Array.isArray(scenario?.diagnostics)
      ? scenario.diagnostics
      : [];
    diagnosticCount += diagnostics.length;
    requireValue(
      Number.isSafeInteger(scenario?.inputLength) &&
        scenario.input === releaseScenarioInputs[index] &&
        scenario.inputLength === releaseScenarioInputLengths[index] &&
        scenario.diagnosticCount === scenario.inputLength &&
        diagnostics.length === scenario.inputLength,
      `${scenario?.name ?? `scenario-${index + 1}`} must contain one diagnostic per input key`,
    );
    const expectedPrefixes = Array.from(
      { length: releaseScenarioInputs[index]?.length ?? 0 },
      (_, prefixIndex) =>
        releaseScenarioInputs[index].slice(0, prefixIndex + 1),
    );
    requireValue(
      sameJson(
        diagnostics.map((diagnostic) =>
          typeof diagnostic?.input === "string"
            ? diagnostic.input.replace(/\s+/g, "")
            : null,
        ),
        expectedPrefixes,
      ),
      `${scenario?.name ?? `scenario-${index + 1}`} diagnostics must cover every exact input prefix in order`,
    );
    requireValue(
      Number.isFinite(scenario?.summary?.p95Ms) &&
        scenario.summary.p95Ms <= releaseP95CeilingMs &&
        Number.isFinite(scenario?.summary?.maxMs) &&
        scenario.summary.maxMs <= releaseMaxCeilingMs,
      `${scenario?.name ?? `scenario-${index + 1}`} summary must satisfy the binding 750/1000 ms ceilings`,
    );
    for (const [diagnosticIndex, diagnostic] of diagnostics.entries()) {
      requireValue(
        diagnostic?.candidateCount === releaseVisibleCandidateCount &&
          Number.isFinite(diagnostic?.totalCandidateCount) &&
          diagnostic.totalCandidateCount >= releaseVisibleCandidateCount &&
          isNonemptyString(diagnostic?.firstCandidateText),
        `${scenario?.name ?? `scenario-${index + 1}`} diagnostic ${diagnosticIndex + 1} must prove a full six-row visible page`,
      );
    }

    const finalVisibleOrder = Array.isArray(
      scenario?.finalVisibleCandidateOrder,
    )
      ? scenario.finalVisibleCandidateOrder
      : [];
    requireValue(
      finalVisibleOrder.length === releaseVisibleCandidateCount &&
        finalVisibleOrder.every(
          (candidate) =>
            isNonemptyString(candidate?.text) &&
            (candidate?.source === null ||
              typeof candidate?.source === "string"),
        ),
      `${scenario?.name ?? `scenario-${index + 1}`} must record six final text rows and nullable production source fields`,
    );
    requireValue(
      scenario?.candidateOrderVerification?.matched === true &&
        scenario.candidateOrderVerification.mode ===
          "first-candidate-only" &&
        scenario.candidateOrderVerification.expectedFirstCandidateText ===
          releaseExpectedFirstCandidateTexts[index] &&
        finalVisibleOrder[0]?.text ===
          releaseExpectedFirstCandidateTexts[index] &&
        diagnostics.at(-1)?.firstCandidateText ===
          finalVisibleOrder[0]?.text &&
        isNonemptyString(scenario.candidateOrderVerification.provenance) &&
        isNonemptyString(scenario.candidateOrderVerification.residual),
      `${scenario?.name ?? `scenario-${index + 1}`} must carry the declared provenance and residual for its first-candidate guard`,
    );

    const expectedGapCount = Math.max(0, diagnostics.length - 1);
    const cadence = scenario?.cadence;
    const rawCadenceGaps = Array.isArray(scenario?.diagnostics)
      ? scenario.diagnostics.slice(1).map((diagnostic, gapIndex) => ({
          afterKeyIndex: gapIndex + 1,
          gapMs:
            diagnostic?.keydownAt -
            scenario.diagnostics[gapIndex]?.keydownAt,
        }))
      : [];
    const rawFiniteCadenceGaps = rawCadenceGaps.map((gap) => gap.gapMs);
    const rawInvalidCadenceGaps = rawCadenceGaps.filter(
      (gap) =>
        !Number.isFinite(gap.gapMs) ||
        gap.gapMs < 200 ||
        gap.gapMs > 312.5,
    );
    cadenceGapCount += Number.isSafeInteger(cadence?.count)
      ? cadence.count
      : 0;
    requireValue(
      rawCadenceGaps.length === expectedGapCount &&
        rawFiniteCadenceGaps.every(Number.isFinite) &&
        rawInvalidCadenceGaps.length === 0,
      `${scenario?.name ?? `scenario-${index + 1}`} raw keydown timestamps must prove every cadence gap within 200..312.5 ms`,
    );
    requireValue(
      cadence?.expectedIntervalMs === releaseKeyIntervalMs &&
        sameJson(cadence?.acceptedRangeMs, { min: 200, max: 312.5 }) &&
        cadence?.count === expectedGapCount &&
        Array.isArray(cadence?.invalidGaps) &&
        cadence.invalidGaps.length === 0 &&
        cadence?.valid === true,
      `${scenario?.name ?? `scenario-${index + 1}`} must have a valid 200..312.5 ms cadence profile`,
    );
    const cadenceStats = [
      cadence?.minMs,
      cadence?.medianMs,
      cadence?.p95Ms,
      cadence?.maxMs,
    ];
    requireValue(
      expectedGapCount === 0
        ? cadenceStats.every((value) => value === null)
        : cadenceStats.every(
            (value) =>
              Number.isFinite(value) && value >= 200 && value <= 312.5,
          ),
      `${scenario?.name ?? `scenario-${index + 1}`} cadence summary must be complete and within range`,
    );
    requireValue(
      expectedGapCount === 0
        ? cadenceStats.every((value) => value === null)
        : cadence?.minMs === percentile(rawFiniteCadenceGaps, 0) &&
          cadence?.medianMs === percentile(rawFiniteCadenceGaps, 0.5) &&
          cadence?.p95Ms === percentile(rawFiniteCadenceGaps, 0.95) &&
          cadence?.maxMs === percentile(rawFiniteCadenceGaps, 1) &&
          sameJson(cadence?.invalidGaps, rawInvalidCadenceGaps),
      `${scenario?.name ?? `scenario-${index + 1}`} cadence summary must be recomputed from raw keydown timestamps`,
    );
  }

  requireValue(
    diagnosticCount === releaseVerifiedKeyCount &&
      evidence.cpuProfile?.verifiedKeyCount === releaseVerifiedKeyCount,
    "release receipt must contain exactly 186 diagnostics and verified keys",
  );
  requireValue(
    cadenceGapCount === releaseCadenceGapCount,
    `release receipt must contain exactly ${releaseCadenceGapCount} consecutive-key cadence gaps`,
  );
  requireValue(
    evidence.profileValidity?.valid === true &&
      evidence.profileValidity?.candidatePages?.valid === true &&
      evidence.profileValidity?.candidatePages
        ?.expectedVisibleCandidateCount === releaseVisibleCandidateCount &&
      evidence.profileValidity?.candidateOrder?.valid === true &&
      evidence.profileValidity?.cadence?.valid === true &&
      evidence.profileValidity?.cadence?.gapCount ===
        releaseCadenceGapCount &&
      evidence.profileValidity?.cadence?.invalidGapCount === 0,
    "release receipt must declare the measured candidate and cadence profile valid",
  );
  requireValue(
    evidence.releaseProfile?.applicable === true &&
      evidence.releaseProfile?.valid === true &&
      evidence.releaseProfile?.diagnosticOverridesActive === false &&
      Array.isArray(evidence.releaseProfile?.deviations) &&
      evidence.releaseProfile.deviations.length === 0,
    "diagnostic overrides or non-loopback profiles cannot produce a release-grade pass",
  );

  if (violations.length > 0) {
    throw new Error(
      `Latency evidence violates the binding release profile:\n- ${violations.join("\n- ")}`,
    );
  }
}

function assertNormalTypingReceipt(evidence) {
  const violations = [];
  const requireValue = (condition, message) => {
    if (!condition) {
      violations.push(message);
    }
  };
  const diagnostics = Array.isArray(evidence?.diagnostics)
    ? evidence.diagnostics
    : [];
  const expectedPrefixes = Array.from(
    { length: normalTypingInput.length },
    (_, index) => normalTypingInput.slice(0, index + 1),
  );
  const totalPaintSamples = diagnostics
    .map((diagnostic) => diagnostic?.totalKeydownToPaintMs)
    .filter(Number.isFinite);
  const queueWaitSamples = diagnostics
    .map((diagnostic) => diagnostic?.workerQueueWaitMs)
    .filter(Number.isFinite);
  const rawCadenceGaps = diagnostics.slice(1).map(
    (diagnostic, index) =>
      diagnostic?.keydownAt - diagnostics[index]?.keydownAt,
  );

  requireValue(
    evidence?.measurementCompleted === true &&
      evidence.passed === true &&
      evidence.buildInfo?.sourceCommit === currentHead,
    "normal-typing receipt must be a passing measurement for current HEAD",
  );
  requireValue(
    evidence?.typingMode === "normal-interactive-exact-input" &&
      evidence.schemaId === "jyut6ping3" &&
      evidence.input === normalTypingInput &&
      evidence.inputLength === normalTypingInput.length &&
      evidence.diagnosticCount === normalTypingInput.length &&
      diagnostics.length === normalTypingInput.length,
    "normal-typing receipt must cover the exact reported Jyutping input",
  );
  requireValue(
    evidence?.keyIntervalMs === normalTypingKeyIntervalMs &&
      sameJson(evidence.ceilingsMs, {
        p95: normalTypingP95CeilingMs,
        max: normalTypingMaxCeilingMs,
        workerQueueWaitMax: normalTypingQueueWaitMaxCeilingMs,
      }),
    "normal-typing cadence and ceilings must remain binding",
  );
  requireValue(
    sameJson(
      diagnostics.map((diagnostic) =>
        typeof diagnostic?.input === "string"
          ? diagnostic.input.replace(/\s+/g, "")
          : null,
      ),
      expectedPrefixes,
    ),
    "normal-typing diagnostics must cover every exact input prefix in order",
  );
  requireValue(
    diagnostics.every(
      (diagnostic) =>
        diagnostic?.candidateCount === releaseVisibleCandidateCount &&
        Number.isFinite(diagnostic?.totalCandidateCount) &&
        diagnostic.totalCandidateCount >= releaseVisibleCandidateCount &&
        isNonemptyString(diagnostic?.firstCandidateText) &&
        diagnostic?.workerActionMultiplier === 1,
    ),
    "normal-typing diagnostics must use the unamplified worker and render complete pages",
  );
  requireValue(
    totalPaintSamples.length === normalTypingInput.length &&
      queueWaitSamples.length === normalTypingInput.length &&
      diagnostics.every(
        (diagnostic) =>
          Number.isFinite(diagnostic?.keydownAt) &&
          Number.isFinite(diagnostic?.workerProcessMs) &&
          Number.isFinite(diagnostic?.workerRoundtripMs) &&
          Number.isFinite(diagnostic?.responseMappingMs) &&
          Number.isFinite(diagnostic?.reactUpdateMs) &&
          Number.isFinite(diagnostic?.paintProxyMs),
      ),
    "normal-typing receipt must retain one complete finite timing sample per key",
  );
  requireValue(
    Number.isFinite(evidence?.summary?.p95Ms) &&
      evidence.summary.p95Ms <= normalTypingP95CeilingMs &&
      evidence.summary.p95Ms === percentile(totalPaintSamples, 0.95) &&
      Number.isFinite(evidence?.summary?.maxMs) &&
      evidence.summary.maxMs <= normalTypingMaxCeilingMs &&
      evidence.summary.maxMs === percentile(totalPaintSamples, 1) &&
      Number.isFinite(evidence?.workerQueueWaitMaxMs) &&
      evidence.workerQueueWaitMaxMs <= normalTypingQueueWaitMaxCeilingMs &&
      evidence.workerQueueWaitMaxMs === percentile(queueWaitSamples, 1),
    "normal-typing latency or queue wait exceeds its binding ceiling",
  );
  requireValue(
    evidence?.cadence?.expectedIntervalMs === normalTypingKeyIntervalMs &&
      sameJson(evidence.cadence.acceptedRangeMs, { min: 80, max: 125 }) &&
      evidence.cadence.count === normalTypingInput.length - 1 &&
      Array.isArray(evidence.cadence.invalidGaps) &&
      evidence.cadence.invalidGaps.length === 0 &&
      evidence.cadence.valid === true &&
      rawCadenceGaps.length === normalTypingInput.length - 1 &&
      rawCadenceGaps.every(
        (value) => Number.isFinite(value) && value >= 80 && value <= 125,
      ) &&
      evidence.cadence.minMs === percentile(rawCadenceGaps, 0) &&
      evidence.cadence.medianMs === percentile(rawCadenceGaps, 0.5) &&
      evidence.cadence.p95Ms === percentile(rawCadenceGaps, 0.95) &&
      evidence.cadence.maxMs === percentile(rawCadenceGaps, 1) &&
      [
        evidence.cadence.minMs,
        evidence.cadence.medianMs,
        evidence.cadence.p95Ms,
        evidence.cadence.maxMs,
      ].every(
        (value) => Number.isFinite(value) && value >= 80 && value <= 125,
      ),
    "normal-typing receipt must prove all 46 cadence gaps within 80..125 ms",
  );
  const finalRows = Array.isArray(evidence?.finalVisibleCandidateOrder)
    ? evidence.finalVisibleCandidateOrder
    : [];
  requireValue(
    finalRows.length === releaseVisibleCandidateCount &&
      finalRows.every(
        (candidate) =>
          isNonemptyString(candidate?.text) &&
          (candidate?.source === null ||
            typeof candidate?.source === "string"),
      ) &&
      diagnostics.at(-1)?.firstCandidateText === finalRows[0]?.text,
    "normal-typing final page must contain six nonempty rows and match the final diagnostic",
  );
  requireValue(
    evidence?.candidateValidation?.mode ===
      "page-shape-only-no-oracle-claim" &&
      isNonemptyString(evidence.candidateValidation.rationale),
    "normal-typing receipt must preserve the explicit no-oracle candidate boundary",
  );
  requireValue(
    Array.isArray(evidence?.assetRequestsDuringMeasurement) &&
      evidence.assetRequestsDuringMeasurement.length === 0,
    "normal-typing timed window must not fetch schema assets",
  );

  if (violations.length > 0) {
    throw new Error(
      `Normal-typing evidence violates its binding profile:\n- ${violations.join("\n- ")}`,
    );
  }
}

async function reportLatencyEvidence(
  prefix,
  requirePassingReceipt = false,
  includeFullReceipt = false,
) {
  const evidencePath = path.join(outputDir, latencyEvidenceName);
  try {
    const rawReceipt = await readFile(evidencePath, "utf8");
    const evidence = JSON.parse(rawReceipt);
    if (requirePassingReceipt) {
      assertReleaseLatencyReceipt(evidence);
    }
    const summary = {
      generatedAt: evidence.generatedAt,
      measurementCompleted: evidence.measurementCompleted,
      passed: evidence.passed,
      thresholdVerdict: evidence.thresholdVerdict,
      releaseGradeVerdict: evidence.releaseGradeVerdict,
      recordedFailureCount: evidence.recordedFailureCount,
      cpuProfile: evidence.cpuProfile,
      profileValidity: evidence.profileValidity,
      releaseProfile: evidence.releaseProfile,
      ceilingsMs: evidence.ceilingsMs,
      keyIntervalMs: evidence.keyIntervalMs,
      buildInfo: evidence.buildInfo,
      scenarios: Array.isArray(evidence.scenarios)
        ? evidence.scenarios.map((scenario) => ({
            name: scenario.name,
            inputLength: scenario.inputLength,
            summary: scenario.summary,
            cadence: scenario.cadence,
            finalVisibleCandidateOrder:
              scenario.finalVisibleCandidateOrder,
            candidateOrderVerification:
              scenario.candidateOrderVerification,
            slowestKey: scenario.slowestKey,
          }))
        : [],
    };
    console.error(`${prefix}=${JSON.stringify(summary)}`);
    if (includeFullReceipt) {
      emitFullReceipt(prefix, rawReceipt);
    }
  } catch (error) {
    if (requirePassingReceipt) {
      throw new Error(
        `WEB03 latency gate did not produce a valid passing receipt at ${evidencePath}`,
        { cause: error },
      );
    }
    console.error(
      `WEB03 latency evidence unavailable at ${evidencePath}: ${error}`,
    );
  }
}

async function reportNormalTypingEvidence(
  prefix,
  attemptOutputDir,
  requirePassingReceipt = false,
  includeFullReceipt = false,
) {
  const evidencePath = path.join(attemptOutputDir, normalTypingEvidenceName);
  try {
    const rawReceipt = await readFile(evidencePath, "utf8");
    const evidence = JSON.parse(rawReceipt);
    if (requirePassingReceipt) {
      assertNormalTypingReceipt(evidence);
    }
    const summary = {
      generatedAt: evidence.generatedAt,
      measurementCompleted: evidence.measurementCompleted,
      passed: evidence.passed,
      schemaId: evidence.schemaId,
      input: evidence.input,
      inputLength: evidence.inputLength,
      keyIntervalMs: evidence.keyIntervalMs,
      ceilingsMs: evidence.ceilingsMs,
      summary: evidence.summary,
      workerQueueWaitMaxMs: evidence.workerQueueWaitMaxMs,
      cadence: evidence.cadence,
      finalVisibleCandidateOrder: evidence.finalVisibleCandidateOrder,
      candidateValidation: evidence.candidateValidation,
      buildInfo: evidence.buildInfo,
    };
    console.error(`${prefix}=${JSON.stringify(summary)}`);
    if (includeFullReceipt) {
      emitFullReceipt(prefix, rawReceipt);
    }
  } catch (error) {
    if (requirePassingReceipt) {
      throw new Error(
        `Normal-typing gate did not produce a valid passing receipt at ${evidencePath}`,
        { cause: error },
      );
    }
    console.error(
      `Normal-typing evidence unavailable at ${evidencePath}: ${error}`,
    );
  }
}

let gatePassed = false;
try {
  await runLatencyGate();
  await reportLatencyEvidence("WEB03_LATENCY_PASS_EVIDENCE", true);
  await reportNormalTypingEvidence(
    "WEB03_NORMAL_TYPING_PASS_EVIDENCE",
    outputDir,
    true,
  );
  gatePassed = true;
} catch (error) {
  await reportLatencyEvidence(
    "WEB03_LATENCY_FAILURE_EVIDENCE",
    false,
    true,
  );
  await reportNormalTypingEvidence(
    "WEB03_NORMAL_TYPING_FAILURE_EVIDENCE",
    outputDir,
    false,
    true,
  );
  console.error(`WEB03 latency failure artifacts preserved at ${outputDir}`);
  throw error;
} finally {
  try {
    await stopPreview();
  } finally {
    if (temporaryOutput !== null && gatePassed) {
      await rm(temporaryOutput, { recursive: true, force: true });
    }
  }
}
