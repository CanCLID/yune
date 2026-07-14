import {
  expect,
  test,
  type CDPSession,
  type Locator,
  type Page,
} from "@playwright/test";

const APP_READY_TIMEOUT_MS = 300_000;
const DIAGNOSTIC_TIMEOUT_MS = 120_000;
const CPU_THROTTLE_RATE = positiveNumberFromEnvironment(
  "YUNE_WEB_LATENCY_CPU_THROTTLE",
  4,
);
const P95_CEILING_MS = positiveNumberFromEnvironment(
  "YUNE_WEB_LATENCY_P95_MS",
  750,
);
const MAX_CEILING_MS = positiveNumberFromEnvironment(
  "YUNE_WEB_LATENCY_MAX_MS",
  1000,
);
const KEY_INTERVAL_MS = positiveNumberFromEnvironment(
  "YUNE_WEB_LATENCY_KEY_INTERVAL_MS",
  250,
);
const APP_URL_VALUE = process.env.YUNE_WEB_APP_URL;
if (!APP_URL_VALUE) {
  throw new Error("YUNE_WEB_APP_URL must identify the artifact under test");
}
const APP_URL = new URL(APP_URL_VALUE);
const EXPECTED_SOURCE_COMMIT = process.env.YUNE_WEB_EXPECTED_SOURCE_COMMIT
  ?.trim()
  .toLowerCase();
if (
  EXPECTED_SOURCE_COMMIT !== undefined &&
  !/^[0-9a-f]{40}$/.test(EXPECTED_SOURCE_COMMIT)
) {
  throw new Error(
    `YUNE_WEB_EXPECTED_SOURCE_COMMIT must be a full lowercase SHA; received ${EXPECTED_SOURCE_COMMIT}`,
  );
}
const EXPECTED_EMSDK_VERSION = "4.0.23";
const EXPECTED_EMSCRIPTEN_RELEASE_COMMIT =
  "aaa43392544d695232b70eda706d751f18980c2a";
const EXPECTED_EMSDK_REPOSITORY_COMMIT =
  "db04e88298d9916fc51fcd3743045ca3eb695127";
const REQUIRE_PINNED_TOOLCHAIN_RECEIPT =
  process.env.YUNE_WEB_REQUIRE_TOOLCHAIN_RECEIPT === "1";
const IS_LOOPBACK_PREVIEW = [
  "127.0.0.1",
  "localhost",
  "::1",
  "[::1]",
].includes(APP_URL.hostname);
if (!IS_LOOPBACK_PREVIEW && EXPECTED_SOURCE_COMMIT === undefined) {
  throw new Error(
    "YUNE_WEB_EXPECTED_SOURCE_COMMIT is required for a deployed-origin canary",
  );
}
const WORKER_ACTION_MULTIPLIER = IS_LOOPBACK_PREVIEW ? 4 : 1;
const EVIDENCE_DIR = process.env.YUNE_WEB_LATENCY_EVIDENCE_DIR;

const JYUTPING_LONG_INPUTS = [
  "sihaacoenggeoisyujapgecukdou",
  "taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng",
] as const;
const LUNA_37_INPUT = "ceshiyixiachangjushuruxingnengzenyang";
const LUNA_59_INPUT =
  "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong";
const LEARNED_INPUT = "ngohaigo";
const LEARNED_TEXT = "我係個";

interface PerfDiagnostic {
  input: string;
  key?: string;
  keydownAt: number;
  workerQueuedAt: number;
  workerStartedAt: number;
  workerFinishedAt: number;
  responseReceivedAt: number;
  responseMappingFinishedAt: number;
  stateAppliedAt: number;
  paintObservedAt: number;
  candidateCount: number;
  totalCandidateCount: number;
  firstCandidateText?: string;
  workerQueueWaitMs?: number;
  workerProcessMs?: number;
  workerRoundtripMs?: number;
  workerBaseElapsedMs?: number;
  workerAmplificationMs?: number;
  workerActionMultiplier?: number;
  workerEffectiveMultiplier?: number;
  responseMappingMs: number;
  reactUpdateMs: number;
  paintProxyMs: number;
  totalWorkerActionMs?: number;
  totalKeydownToPaintMs: number;
}

interface LatencySummary {
  medianMs: number | null;
  p95Ms: number | null;
  maxMs: number | null;
  attributionP95Ms: {
    workerQueueWait: number | null;
    workerProcess: number | null;
    workerRoundtrip: number | null;
    responseMapping: number | null;
    reactUpdate: number | null;
    paintProxy: number | null;
  };
}

interface ScenarioEvidence {
  name: string;
  schemaId: string;
  input: string;
  inputLength: number;
  diagnosticCount: number;
  summary: LatencySummary;
  slowestKey: PerfDiagnostic | null;
  diagnostics: PerfDiagnostic[];
  assetRequestsDuringMeasurement: AssetRequest[];
  learnedCandidateVisible?: boolean;
  persistedAfterReload?: boolean;
  forcedCompleteFirstPageStable?: boolean;
}

interface AssetRequest {
  method: string;
  url: string;
}

interface PublicBuildInfo {
  generatedFor: string;
  schemaBytes: number;
  builtAt: string;
  sourceCommit: string;
  sourceTreeState: string;
  schemaManifestSha256: string;
  wasmSha256: string;
  publicArtifactManifestSha256: string;
  toolchain: {
    emsdkVersion: string;
    emscriptenReleaseCommit: string;
    emsdkRepositoryCommit: string;
    emccVersion: string;
    rustcVersion: string;
    nodeVersion: string;
  } | null;
}

interface PublicArtifactManifest {
  generatedFor: string;
  version: string;
  files: Array<{
    path: string;
    bytes: number;
    sha256: string;
  }>;
}

interface CandidateRowSnapshot {
  text: string | null;
  source: string | null;
  rowText: string;
}

interface CpuProfileEvidence {
  mainThread: {
    requestedRate: number;
    iterations: number;
    baselineMs: number;
    throttledMs: number;
    ratio: number;
  };
  workerActionMultiplier: number;
  mechanism:
    | "main-cdp-plus-synthetic-keydown-worker-service-amplification"
    | "main-cdp-only-postdeploy-canary";
  workerStressScope: "loopback-preview" | "disabled-on-deployed-origin";
  verifiedKeyCount: number;
}

function positiveNumberFromEnvironment(name: string, fallback: number): number {
  const raw = process.env[name];
  if (raw === undefined || raw.trim() === "") {
    return fallback;
  }
  const value = Number(raw);
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`${name} must be a positive number; received ${raw}`);
  }
  return value;
}

async function runMainThreadCpuProbe(
  page: Page,
  iterations: number,
): Promise<number> {
  return page.evaluate((count) => {
    const started = performance.now();
    let value = 0x811c9dc5;
    for (let index = 0; index < count; index += 1) {
      value = (Math.imul(value ^ index, 16777619) + 1013904223) | 0;
    }
    if (!Number.isFinite(value)) {
      throw new Error("Main-thread CPU probe returned invalid data");
    }
    return performance.now() - started;
  }, iterations);
}

async function applyAndVerifyMainThreadThrottle(
  page: Page,
  pageSession: CDPSession,
  rate: number,
): Promise<{
  requestedRate: number;
  iterations: number;
  baselineMs: number;
  throttledMs: number;
  ratio: number;
}> {
  if (rate < 1 || rate > 16) {
    throw new Error(
      `YUNE_WEB_LATENCY_CPU_THROTTLE must be between 1 and 16; received ${rate}`,
    );
  }
  await runMainThreadCpuProbe(page, 100_000);
  let iterations = 5_000_000;
  let baselineMs = await runMainThreadCpuProbe(page, iterations);
  for (let attempt = 0; attempt < 2 && baselineMs < 20; attempt += 1) {
    iterations = Math.min(
      100_000_000,
      Math.ceil((iterations * 30) / Math.max(baselineMs, 1)),
    );
    baselineMs = await runMainThreadCpuProbe(page, iterations);
  }
  await pageSession.send("Emulation.setCPUThrottlingRate", { rate });
  const throttledMs = await runMainThreadCpuProbe(page, iterations);
  const ratio = throttledMs / baselineMs;
  const minimumRatio = Math.max(1, rate * 0.6);
  const maximumRatio = Math.max(2, rate * 1.8);
  expect(
    ratio,
    `main-thread CPU slowdown must approximate ${rate}x`,
  ).toBeGreaterThanOrEqual(minimumRatio);
  expect(ratio).toBeLessThanOrEqual(maximumRatio);
  return { requestedRate: rate, iterations, baselineMs, throttledMs, ratio };
}

function composeInput(page: Page): Locator {
  return page.locator(".yd-input-area").first();
}

async function waitForAppReady(page: Page): Promise<void> {
  await page.waitForFunction(
    () =>
      document.documentElement.dataset.yuneInitialized === "true" &&
      document.documentElement.dataset.yuneLoading === "false",
    undefined,
    { timeout: APP_READY_TIMEOUT_MS },
  );
  await expect(page.locator("[data-yune-loading-indicator]")).toHaveCount(0, {
    timeout: APP_READY_TIMEOUT_MS,
  });
}

async function clearComposition(page: Page): Promise<void> {
  const input = composeInput(page);
  await input.focus();
  for (
    let attempts = 0;
    attempts < 4 && (await page.locator(".candidate-panel").count()) > 0;
    attempts += 1
  ) {
    await page.keyboard.press("Escape").catch(() => undefined);
    await page.waitForTimeout(150);
  }
  await input.fill("");
  await expect(page.locator(".candidate-panel")).toHaveCount(0, {
    timeout: 10_000,
  });
}

async function resetPerfDiagnostics(page: Page): Promise<void> {
  await page.evaluate(() => {
    document.documentElement.dataset.yuneActionDiagnostics = "[]";
    document.documentElement.dataset.yuneTypingDiagnostics = "[]";
    document.documentElement.dataset.yunePerfDiagnostics = "[]";
  });
}

async function readPerfDiagnostics(page: Page): Promise<PerfDiagnostic[]> {
  return page.evaluate(() =>
    JSON.parse(
      document.documentElement.dataset.yunePerfDiagnostics ?? "[]",
    ),
  );
}

async function selectSchema(page: Page, schemaId: string): Promise<void> {
  await clearComposition(page);
  const select = page.locator("[data-yune-schema-switcher] select");
  await expect(select).toBeVisible({ timeout: 10_000 });
  if ((await select.inputValue()) !== schemaId) {
    await select.selectOption(schemaId);
  }
  await expect
    .poll(
      () =>
        page.evaluate(
          () => document.documentElement.dataset.yuneActiveSchema ?? null,
        ),
      { timeout: APP_READY_TIMEOUT_MS },
    )
    .toBe(schemaId);
  await waitForAppReady(page);
  await page.waitForTimeout(250);
}

function percentile(values: number[], proportion: number): number | null {
  if (values.length === 0) {
    return null;
  }
  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.ceil((sorted.length - 1) * proportion);
  return sorted[Math.min(index, sorted.length - 1)];
}

function numericValues(
  diagnostics: PerfDiagnostic[],
  select: (diagnostic: PerfDiagnostic) => number | undefined,
): number[] {
  return diagnostics
    .map(select)
    .filter((value): value is number =>
      typeof value === "number" && Number.isFinite(value),
    );
}

function summarize(diagnostics: PerfDiagnostic[]): LatencySummary {
  const totals = numericValues(
    diagnostics,
    (diagnostic) => diagnostic.totalKeydownToPaintMs,
  );
  return {
    medianMs: percentile(totals, 0.5),
    p95Ms: percentile(totals, 0.95),
    maxMs: percentile(totals, 1),
    attributionP95Ms: {
      workerQueueWait: percentile(
        numericValues(
          diagnostics,
          (diagnostic) => diagnostic.workerQueueWaitMs,
        ),
        0.95,
      ),
      workerProcess: percentile(
        numericValues(
          diagnostics,
          (diagnostic) => diagnostic.workerProcessMs,
        ),
        0.95,
      ),
      workerRoundtrip: percentile(
        numericValues(
          diagnostics,
          (diagnostic) => diagnostic.workerRoundtripMs,
        ),
        0.95,
      ),
      responseMapping: percentile(
        numericValues(
          diagnostics,
          (diagnostic) => diagnostic.responseMappingMs,
        ),
        0.95,
      ),
      reactUpdate: percentile(
        numericValues(diagnostics, (diagnostic) => diagnostic.reactUpdateMs),
        0.95,
      ),
      paintProxy: percentile(
        numericValues(diagnostics, (diagnostic) => diagnostic.paintProxyMs),
        0.95,
      ),
    },
  };
}

function isPostReadySchemaRequest(url: string): boolean {
  const pathname = new URL(url).pathname;
  return (
    /(?:^|\/)schema(?:\/|$)/i.test(pathname) ||
    /\.part\d+(?:\.|$)/i.test(pathname) ||
    /(?:^|[-_.])manifest(?:[-_.]|$)/i.test(pathname)
  );
}

async function readPublicBuildInfo(page: Page): Promise<PublicBuildInfo> {
  const response = await page.request.get(
    new URL("/build-info.json", page.url()).toString(),
    { failOnStatusCode: false },
  );
  expect(
    response.ok(),
    "latency gate requires an already-built public artifact with build-info.json",
  ).toBe(true);
  expect(response.headers()["content-type"] ?? "").toContain(
    "application/json",
  );
  const value = (await response.json()) as PublicBuildInfo;
  expect(value.generatedFor).toBe("yune-web");
  expect(value.schemaBytes).toBeGreaterThan(0);
  expect(Number.isNaN(Date.parse(value.builtAt))).toBe(false);
  expect(value.sourceCommit).toMatch(/^[0-9a-f]{40}$/);
  if (EXPECTED_SOURCE_COMMIT !== undefined) {
    expect(value.sourceCommit).toBe(EXPECTED_SOURCE_COMMIT);
  }
  expect(value.sourceTreeState).toBe("clean");
  expect(value.schemaManifestSha256).toMatch(/^[0-9a-f]{64}$/);
  expect(value.wasmSha256).toMatch(/^[0-9a-f]{64}$/);
  expect(value.publicArtifactManifestSha256).toMatch(/^[0-9a-f]{64}$/);
  if (REQUIRE_PINNED_TOOLCHAIN_RECEIPT) {
    expect(
      value.toolchain,
      "release artifact must record its pinned toolchain",
    ).not.toBeNull();
    if (value.toolchain === null) {
      throw new Error("Release artifact is missing its pinned toolchain receipt");
    }
    expect(value.toolchain.emsdkVersion).toBe(EXPECTED_EMSDK_VERSION);
    expect(value.toolchain.emscriptenReleaseCommit).toBe(
      EXPECTED_EMSCRIPTEN_RELEASE_COMMIT,
    );
    expect(value.toolchain.emsdkRepositoryCommit).toBe(
      EXPECTED_EMSDK_REPOSITORY_COMMIT,
    );
    expect(value.toolchain.emccVersion).toContain(EXPECTED_EMSDK_VERSION);
    expect(value.toolchain.rustcVersion).toMatch(/^rustc \d+\.\d+\.\d+/);
    expect(value.toolchain.nodeVersion).toMatch(/^v\d+\.\d+\.\d+/);
  }

  const { createHash } = await import("node:crypto");
  const artifactManifestResponse = await page.request.get(
    new URL("/public-artifact-manifest.json", page.url()).toString(),
    { failOnStatusCode: false },
  );
  expect(artifactManifestResponse.ok(), "public artifact manifest must be fetchable").toBe(true);
  const artifactManifestBytes = await artifactManifestResponse.body();
  expect(
    createHash("sha256").update(artifactManifestBytes).digest("hex"),
    "served artifact manifest must match build-info",
  ).toBe(value.publicArtifactManifestSha256);
  const artifactManifest = JSON.parse(
    artifactManifestBytes.toString("utf8"),
  ) as PublicArtifactManifest;
  expect(artifactManifest.generatedFor).toBe("yune-web");
  expect(artifactManifest.version).toBe("web03-public-artifact-v1");
  expect(Array.isArray(artifactManifest.files)).toBe(true);
  const files = new Map(
    artifactManifest.files.map((file) => [file.path, file] as const),
  );
  expect(files.size).toBe(artifactManifest.files.length);
  for (const file of artifactManifest.files) {
    expect(file.path).toMatch(/^(?!\/)(?!.*(?:^|\/)\.\.?(?:\/|$))(?!.*\\).+$/);
    expect(Number.isSafeInteger(file.bytes) && file.bytes >= 0).toBe(true);
    expect(file.sha256).toMatch(/^[0-9a-f]{64}$/);
  }
  expect(files.get("schema-asset-manifest.json")?.sha256).toBe(
    value.schemaManifestSha256,
  );
  expect(files.get("yune-web.wasm")?.sha256).toBe(value.wasmSha256);
  for (const required of [
    "index.html",
    "worker.js",
    "yune-web.js",
    "yune-web.wasm",
    "schema-asset-manifest.json",
  ]) {
    expect(files.has(required), `artifact inventory must include ${required}`).toBe(true);
  }
  expect(
    [...files.keys()].some((pathname) => /^assets\/.*\.js$/.test(pathname)),
    "artifact inventory must include the rendered app JavaScript bundle",
  ).toBe(true);
  const servedRuntimeFiles = artifactManifest.files.filter(
    (file) =>
      [
        "index.html",
        "worker.js",
        "yune-web.js",
        "yune-web.wasm",
        "schema-asset-manifest.json",
      ].includes(file.path) ||
      /^assets\/.*\.(?:js|css)$/.test(file.path),
  );
  for (const file of servedRuntimeFiles) {
    expect(file.bytes).toBeGreaterThan(0);
    expect(file.sha256).toMatch(/^[0-9a-f]{64}$/);
    const asset = await page.request.get(new URL(`/${file.path}`, page.url()).toString(), {
      failOnStatusCode: false,
    });
    expect(asset.ok(), `public runtime artifact ${file.path} must be fetchable`).toBe(true);
    const body = await asset.body();
    expect(body.byteLength, `${file.path} byte length must match inventory`).toBe(file.bytes);
    const actual = createHash("sha256").update(body).digest("hex");
    expect(actual, `${file.path} must match the artifact inventory`).toBe(file.sha256);
  }
  return value;
}

function assertPublicSplitStartup(requests: AssetRequest[]): void {
  const paths = requests.map((request) => new URL(request.url).pathname);
  expect(
    paths.some((path) => path === "/schema-asset-manifest.json"),
    "public startup must fetch the pinned schema asset manifest",
  ).toBe(true);
  for (const index of [0, 1]) {
    expect(
      paths.filter(
        (path) =>
          path === `/schema/jyut6ping3_mobile.prism.bin.part${index}`,
      ),
      `public startup must fetch split Jyutping prism part ${index}`,
    ).toHaveLength(1);
  }
  expect(
    paths.filter(
      (path) => path === "/schema/jyut6ping3_mobile.prism.bin",
    ),
    "public startup must not request the unsplit over-limit Jyutping prism",
  ).toHaveLength(0);
}

async function readVisibleCandidateRows(
  page: Page,
): Promise<CandidateRowSnapshot[]> {
  return page
    .locator(".candidate-panel .candidates tbody")
    .evaluateAll((rows) =>
      rows.map((row) => ({
        text: row.getAttribute("data-candidate-text"),
        source: row.getAttribute("data-source"),
        rowText: row.textContent?.replace(/\s+/g, " ").trim() ?? "",
      })),
    );
}

function assertScenario(scenario: ScenarioEvidence): void {
  const prefixInputs = Array.from(
    { length: scenario.input.length },
    (_, index) => scenario.input.slice(0, index + 1),
  );
  expect.soft(
    scenario.diagnosticCount,
    `${scenario.name}: every burst key must produce a paint diagnostic`,
  ).toBe(scenario.inputLength);
  expect.soft(
    scenario.diagnostics.map((diagnostic) =>
      diagnostic.input.replace(/\s+/g, ""),
    ),
    `${scenario.name}: diagnostics must cover each normalized input prefix in order`,
  ).toEqual(prefixInputs);
  expect.soft(
    scenario.assetRequestsDuringMeasurement,
    `${scenario.name}: schema, split-part, and manifest assets must not be fetched during timed typing`,
  ).toEqual([]);
  for (const [index, diagnostic] of scenario.diagnostics.entries()) {
    const label = `${scenario.name}: key ${index + 1} worker slowdown`;
    expect.soft(
      diagnostic.workerActionMultiplier,
      `${label} must use the declared worker action multiplier`,
    ).toBe(WORKER_ACTION_MULTIPLIER);
    expect.soft(
      Number.isFinite(diagnostic.workerBaseElapsedMs),
      `${label} must report its unamplified action time`,
    ).toBe(true);
    expect.soft(
      Number.isFinite(diagnostic.workerAmplificationMs),
      `${label} must report its measured amplification time`,
    ).toBe(true);
    const baseMs = diagnostic.workerBaseElapsedMs ?? Number.NaN;
    const amplificationMs =
      diagnostic.workerAmplificationMs ?? Number.NEGATIVE_INFINITY;
    const amplificationFloorMs = Math.max(
      0,
      baseMs * (WORKER_ACTION_MULTIPLIER - 1) * 0.8,
    );
    const amplificationCeilingMs = Math.max(
      10,
      baseMs * (WORKER_ACTION_MULTIPLIER - 1) * 1.25 + 10,
    );
    expect.soft(
      amplificationMs,
      `${label} must materially realize the declared multiplier`,
    ).toBeGreaterThanOrEqual(amplificationFloorMs);
    expect.soft(
      amplificationMs,
      `${label} must not overshoot the declared synthetic profile`,
    ).toBeLessThanOrEqual(amplificationCeilingMs);
    expect.soft(
      diagnostic.workerEffectiveMultiplier,
      `${label} must record its effective worker service-time multiplier`,
    ).toBeGreaterThanOrEqual(
      1 + (WORKER_ACTION_MULTIPLIER - 1) * 0.8,
    );
    expect.soft(
      diagnostic.workerProcessMs ?? Number.NEGATIVE_INFINITY,
      `${label} total worker time must include base plus amplification`,
    ).toBeGreaterThanOrEqual(baseMs + amplificationMs - 2);
  }
  expect.soft(
    scenario.summary.p95Ms,
    `${scenario.name}: p95 keydown-to-paint exceeds ${P95_CEILING_MS} ms`,
  ).not.toBeNull();
  expect.soft(
    scenario.summary.p95Ms ?? Number.POSITIVE_INFINITY,
    `${scenario.name}: p95 keydown-to-paint exceeds ${P95_CEILING_MS} ms`,
  ).toBeLessThanOrEqual(P95_CEILING_MS);
  expect.soft(
    scenario.summary.maxMs ?? Number.POSITIVE_INFINITY,
    `${scenario.name}: max keydown-to-paint exceeds ${MAX_CEILING_MS} ms`,
  ).toBeLessThanOrEqual(MAX_CEILING_MS);
}

async function measureBurst(
  page: Page,
  name: string,
  schemaId: string,
  inputText: string,
  postReadyAssetRequests: AssetRequest[],
): Promise<ScenarioEvidence> {
  await clearComposition(page);
  await resetPerfDiagnostics(page);
  const input = composeInput(page);
  await input.focus();
  const assetRequestStart = postReadyAssetRequests.length;

  // Four keys per second sustains ordinary interactive typing while still
  // exposing queue growth. An 80 ms or zero-delay 59-key injection instead
  // turns a 4x worker profile into synthetic event flooding.
  await page.keyboard.type(inputText, { delay: KEY_INTERVAL_MS });
  await expect
    .poll(async () => (await readPerfDiagnostics(page)).length, {
      timeout: DIAGNOSTIC_TIMEOUT_MS,
    })
    .toBe(inputText.length);

  const diagnostics = (await readPerfDiagnostics(page)).map((diagnostic) => {
    const baseMs = diagnostic.workerBaseElapsedMs;
    const amplificationMs = diagnostic.workerAmplificationMs;
    return {
      ...diagnostic,
      workerEffectiveMultiplier:
        baseMs !== undefined && baseMs > 0 && amplificationMs !== undefined
          ? (baseMs + amplificationMs) / baseMs
          : undefined,
    };
  });
  const slowestKey = diagnostics.reduce<PerfDiagnostic | null>(
    (slowest, diagnostic) =>
      slowest === null ||
      diagnostic.totalKeydownToPaintMs > slowest.totalKeydownToPaintMs
        ? diagnostic
        : slowest,
    null,
  );
  const scenario: ScenarioEvidence = {
    name,
    schemaId,
    input: inputText,
    inputLength: inputText.length,
    diagnosticCount: diagnostics.length,
    summary: summarize(diagnostics),
    slowestKey,
    diagnostics,
    assetRequestsDuringMeasurement: postReadyAssetRequests.slice(
      assetRequestStart,
    ),
  };
  assertScenario(scenario);
  return scenario;
}

async function learnTypeDuckRow(page: Page): Promise<void> {
  const toggle = page.getByLabel(/User Dictionary|用戶詞庫/).last();
  await expect(toggle).toBeVisible({ timeout: 10_000 });
  if (!(await toggle.isChecked())) {
    await toggle.check({ force: true });
    await waitForAppReady(page);
  }

  await clearComposition(page);
  const input = composeInput(page);
  await input.focus();
  await page.keyboard.type(LEARNED_INPUT, { delay: 0 });
  await expect
    .poll(
      async () =>
        page
          .locator(".candidate-panel .candidates tbody")
          .first()
          .getAttribute("data-candidate-text"),
      { timeout: DIAGNOSTIC_TIMEOUT_MS },
    )
    .toBe(LEARNED_TEXT);
  await page.keyboard.press("Space");
  await expect(input).toHaveValue(LEARNED_TEXT, {
    timeout: DIAGNOSTIC_TIMEOUT_MS,
  });
}

async function writeOptionalEvidence(value: unknown): Promise<void> {
  if (!EVIDENCE_DIR) {
    return;
  }
  const fs = await import("fs/promises");
  const path = await import("path");
  await fs.mkdir(EVIDENCE_DIR, { recursive: true });
  await fs.writeFile(
    path.join(EVIDENCE_DIR, "input-latency-hard-stop.json"),
    `${JSON.stringify(value, null, 2)}\n`,
    "utf8",
  );
}

test.describe.configure({ mode: "serial" });

test("WEB-03 input latency hard stop covers all public schemas and learned TypeDuck state", async ({
  page,
}, testInfo) => {
  const pageSession = await page.context().newCDPSession(page);

  const scenarios: ScenarioEvidence[] = [];
  const postReadyAssetRequests: AssetRequest[] = [];
  const schemaSelectionRequests: Record<string, AssetRequest[]> = {};
  let startupAssetRequests: AssetRequest[] = [];
  let buildInfo: PublicBuildInfo | null = null;
  let mainThreadThrottleEvidence: CpuProfileEvidence["mainThread"] | null =
    null;
  let readyAt: string | null = null;
  let measurementCompleted = false;
  let hardFailure = false;

  try {
    page.on("request", (request) => {
      if (isPostReadySchemaRequest(request.url())) {
        postReadyAssetRequests.push({
          method: request.method(),
          url: request.url(),
        });
      }
    });
    mainThreadThrottleEvidence = await applyAndVerifyMainThreadThrottle(
      page,
      pageSession,
      CPU_THROTTLE_RATE,
    );
    const initialPath = IS_LOOPBACK_PREVIEW
      ? `/?yuneLatencyWorkerActionMultiplier=${WORKER_ACTION_MULTIPLIER}`
      : "/";
    await page.goto(
      initialPath,
      {
        waitUntil: "domcontentloaded",
        timeout: APP_READY_TIMEOUT_MS,
      },
    );
    await waitForAppReady(page);
    readyAt = new Date().toISOString();
    startupAssetRequests = [...postReadyAssetRequests];
    assertPublicSplitStartup(startupAssetRequests);
    buildInfo = await readPublicBuildInfo(page);
    let selectionStart = postReadyAssetRequests.length;
    await selectSchema(page, "jyut6ping3");
    schemaSelectionRequests.jyut6ping3 = postReadyAssetRequests.slice(
      selectionStart,
    );
    scenarios.push(
      await measureBurst(
        page,
        "jyutping-short",
        "jyut6ping3",
        "hai",
        postReadyAssetRequests,
      ),
    );
    for (const [index, input] of JYUTPING_LONG_INPUTS.entries()) {
      scenarios.push(
        await measureBurst(
          page,
          `jyutping-historical-long-${index + 1}`,
          "jyut6ping3",
          input,
          postReadyAssetRequests,
        ),
      );
    }

    await learnTypeDuckRow(page);
    await page.reload({
      waitUntil: "domcontentloaded",
      timeout: APP_READY_TIMEOUT_MS,
    });
    await waitForAppReady(page);
    await expect
      .poll(
        () =>
          page.evaluate(
            () => document.documentElement.dataset.yuneActiveSchema ?? null,
          ),
        { timeout: APP_READY_TIMEOUT_MS },
      )
      .toBe("jyut6ping3");
    const learned = await measureBurst(
      page,
      "typeduck-learned-userdb-prefix",
      "jyut6ping3",
      "ngo",
      postReadyAssetRequests,
    );
    learned.learnedCandidateVisible = await page
      .locator(`.candidate-panel [data-candidate-text="${LEARNED_TEXT}"]`)
      .first()
      .isVisible();
    learned.persistedAfterReload = learned.learnedCandidateVisible;
    expect.soft(
      learned.learnedCandidateVisible,
      "TypeDuck learned lane must restore the UI-committed row after reload",
    ).toBe(true);
    const boundedFirstPage = await readVisibleCandidateRows(page);
    await page.keyboard.press("PageDown");
    await expect
      .poll(() => readVisibleCandidateRows(page), {
        timeout: DIAGNOSTIC_TIMEOUT_MS,
      })
      .not.toEqual(boundedFirstPage);
    await page.keyboard.press("PageUp");
    await expect
      .poll(() => readVisibleCandidateRows(page), {
        timeout: DIAGNOSTIC_TIMEOUT_MS,
      })
      .toEqual(boundedFirstPage);
    learned.forcedCompleteFirstPageStable = true;
    scenarios.push(learned);

    selectionStart = postReadyAssetRequests.length;
    await selectSchema(page, "luna_pinyin");
    schemaSelectionRequests.luna_pinyin = postReadyAssetRequests.slice(
      selectionStart,
    );
    scenarios.push(
      await measureBurst(
        page,
        "luna-short",
        "luna_pinyin",
        "hao",
        postReadyAssetRequests,
      ),
      await measureBurst(
        page,
        "luna-37",
        "luna_pinyin",
        LUNA_37_INPUT,
        postReadyAssetRequests,
      ),
      await measureBurst(
        page,
        "luna-59",
        "luna_pinyin",
        LUNA_59_INPUT,
        postReadyAssetRequests,
      ),
    );

    selectionStart = postReadyAssetRequests.length;
    await selectSchema(page, "cangjie5");
    schemaSelectionRequests.cangjie5 = postReadyAssetRequests.slice(
      selectionStart,
    );
    scenarios.push(
      await measureBurst(
        page,
        "cangjie-short",
        "cangjie5",
        "a",
        postReadyAssetRequests,
      ),
    );
    measurementCompleted = true;
  } catch (error) {
    hardFailure = true;
    throw error;
  } finally {
    const passed =
      measurementCompleted && !hardFailure && testInfo.errors.length === 0;
    const evidence = {
      generatedAt: new Date().toISOString(),
      url: page.url(),
      readyAt,
      measurementCompleted,
      passed,
      thresholdVerdict: passed ? "pass" : "fail",
      recordedFailureCount: testInfo.errors.length + Number(hardFailure),
      browser: {
        project: testInfo.project.name,
        version: page.context().browser()?.version() ?? null,
      },
      cpuProfile:
        mainThreadThrottleEvidence === null
          ? null
          : {
              mainThread: mainThreadThrottleEvidence,
              workerActionMultiplier: WORKER_ACTION_MULTIPLIER,
              mechanism: IS_LOOPBACK_PREVIEW
                ? "main-cdp-plus-synthetic-keydown-worker-service-amplification"
                : "main-cdp-only-postdeploy-canary",
              workerStressScope: IS_LOOPBACK_PREVIEW
                ? "loopback-preview"
                : "disabled-on-deployed-origin",
              verifiedKeyCount: scenarios.reduce(
                (total, scenario) => total + scenario.diagnosticCount,
                0,
              ),
            } satisfies CpuProfileEvidence,
      ceilingsMs: {
        p95: P95_CEILING_MS,
        max: MAX_CEILING_MS,
      },
      typingMode: "sustained-interactive",
      keyIntervalMs: KEY_INTERVAL_MS,
      buildInfo,
      scenarios,
      startupAssetRequests,
      schemaSelectionRequests,
      postReadyAssetRequests: postReadyAssetRequests.slice(
        startupAssetRequests.length,
      ),
    };
    const serialized = `${JSON.stringify(evidence, null, 2)}\n`;
    await testInfo.attach("input-latency-hard-stop", {
      body: serialized,
      contentType: "application/json",
    });
    await writeOptionalEvidence(evidence);
    await pageSession.detach().catch(() => undefined);
  }
});
