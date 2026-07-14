import {
  expect,
  test,
  type CDPSession,
  type Locator,
  type Page,
  type TestInfo,
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
const EXPECTED_RUSTC_VERSION =
  "rustc 1.96.1 (31fca3adb 2026-06-26)";
const EXPECTED_NODE_VERSION = "v22.16.0";
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

const RELEASE_P95_CEILING_MS = 750;
const RELEASE_MAX_CEILING_MS = 1000;
const RELEASE_CPU_THROTTLE_RATE = 4;
const RELEASE_WORKER_ACTION_MULTIPLIER = 4;
const RELEASE_KEY_INTERVAL_MS = 250;
const KEY_INTERVAL_MIN_MS = KEY_INTERVAL_MS * 0.8;
const KEY_INTERVAL_MAX_MS = KEY_INTERVAL_MS * 1.25;
const EXPECTED_VISIBLE_CANDIDATE_COUNT = 6;
const NORMAL_TYPING_INPUT =
  "ngodeigungsijigaahaidoumaaigangeihaaijansougeoi";
const NORMAL_TYPING_KEY_INTERVAL_MS = 100;
const NORMAL_TYPING_KEY_INTERVAL_MIN_MS =
  NORMAL_TYPING_KEY_INTERVAL_MS * 0.8;
const NORMAL_TYPING_KEY_INTERVAL_MAX_MS =
  NORMAL_TYPING_KEY_INTERVAL_MS * 1.25;
const NORMAL_TYPING_P95_CEILING_MS = 150;
const NORMAL_TYPING_MAX_CEILING_MS = 250;
const NORMAL_TYPING_QUEUE_WAIT_MAX_CEILING_MS = 100;

const JYUTPING_LONG_INPUTS = [
  "sihaacoenggeoisyujapgecukdou",
  "taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng",
] as const;
const LUNA_37_INPUT = "ceshiyixiachangjushuruxingnengzenyang";
const LUNA_59_INPUT =
  "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong";
const LEARNED_INPUT = "ngohaigo";

const SCENARIO_NAMES = [
  "jyutping-short",
  "jyutping-historical-long-1",
  "jyutping-historical-long-2",
  "typeduck-learned-userdb-prefix",
  "luna-short",
  "luna-37",
  "luna-59",
  "cangjie-short",
] as const;
type ScenarioName = (typeof SCENARIO_NAMES)[number];

const EXPECTED_RELEASE_DIAGNOSTIC_COUNT =
  3 +
  JYUTPING_LONG_INPUTS[0].length +
  JYUTPING_LONG_INPUTS[1].length +
  3 +
  3 +
  LUNA_37_INPUT.length +
  LUNA_59_INPUT.length +
  1;
const EXPECTED_RELEASE_CADENCE_GAP_COUNT =
  EXPECTED_RELEASE_DIAGNOSTIC_COUNT - SCENARIO_NAMES.length;

interface FinalCandidateGuard {
  mode: "first-candidate-only";
  expectedFirstCandidateText: string;
  provenance: string;
  residual: string;
}

const PUBLIC_PAGE_SOURCE_RESIDUAL =
  "No pinned page-size-6 browser fixture records the complete visible text+data-source order. The binding production timing deliberately leaves yune_inspector disabled, so ordinary source labels are null; the run records all six text rows and nullable production source fields, while only the existing authoritative first candidate is a binding order guard.";

const FINAL_CANDIDATE_GUARDS: Record<ScenarioName, FinalCandidateGuard> = {
  "jyutping-short": {
    mode: "first-candidate-only",
    expectedFirstCandidateText: "\u4fc2",
    provenance:
      "accepted public-product hai snapshot in apps/yune-web/e2e/results/web03-latency-regression-fix/local-browser-latency/samples.json",
    residual: PUBLIC_PAGE_SOURCE_RESIDUAL,
  },
  "jyutping-historical-long-1": {
    mode: "first-candidate-only",
    expectedFirstCandidateText:
      "\u6642\u4e0b\u5834\u64da\u8f38\u5165\u5605\u901f\u5ea6",
    provenance:
      "accepted byte-backed product guard in crates/yune-rime-api/tests/yune_web.rs::web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion",
    residual: PUBLIC_PAGE_SOURCE_RESIDUAL,
  },
  "jyutping-historical-long-2": {
    mode: "first-candidate-only",
    expectedFirstCandidateText:
      "\u7747\u4e0b\u5982\u679c\u6253\u597d\u5834\u5605\u53e5\u5b50\u500b\u6027\u80fd\u6703\u9ede\u6a23",
    provenance:
      "accepted byte-backed product guard in crates/yune-rime-api/tests/yune_web.rs::web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion",
    residual: PUBLIC_PAGE_SOURCE_RESIDUAL,
  },
  "typeduck-learned-userdb-prefix": {
    mode: "first-candidate-only",
    expectedFirstCandidateText: "\u6211",
    provenance:
      "accepted public-product ngo snapshot in apps/yune-web/e2e/results/web03-latency-regression-fix/local-browser-latency/samples.json; the separately asserted learned row remains visible after reload",
    residual: PUBLIC_PAGE_SOURCE_RESIDUAL,
  },
  "luna-short": {
    mode: "first-candidate-only",
    expectedFirstCandidateText: "\u597d",
    provenance:
      "pinned librime 1.17.0 hao row in crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json",
    residual: PUBLIC_PAGE_SOURCE_RESIDUAL,
  },
  "luna-37": {
    mode: "first-candidate-only",
    expectedFirstCandidateText:
      "\u6e2c\u8a66\u4e00\u4e0b\u9577\u53e5\u8f38\u5165\u6027\u80fd\u600e\u6a23",
    provenance:
      "pinned librime 1.17.0 sentence_benchmark_37 in crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json",
    residual:
      "The pinned librime fixture certifies five text rows at page size 5 but has no source labels. The production timing leaves yune_inspector disabled, so the public page-size-6 text tail and nullable source fields remain recorded, not oracle-certified.",
  },
  "luna-59": {
    mode: "first-candidate-only",
    expectedFirstCandidateText:
      "\u9019\u500b\u5f15\u64ce\u5176\u5be6\u61c9\u8a72\u652f\u6301\u8d85\u9577\u53e5\u5b50\u8f38\u5165\u624d\u80fd\u7528",
    provenance:
      "pinned librime 1.17.0 sentence_benchmark_59 in crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json",
    residual:
      "The pinned librime fixture certifies five text rows at page size 5 but has no source labels. The production timing leaves yune_inspector disabled, so the public page-size-6 text tail and nullable source fields remain recorded, not oracle-certified.",
  },
  "cangjie-short": {
    mode: "first-candidate-only",
    expectedFirstCandidateText: "\u65e5",
    provenance:
      "pinned librime 1.17.0 paging_first_input page_1 in crates/yune-core/tests/fixtures/upstream-1.17.0/cangjie5-basic.json",
    residual: PUBLIC_PAGE_SOURCE_RESIDUAL,
  },
};
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

interface CadenceEvidence {
  expectedIntervalMs: number;
  acceptedRangeMs: {
    min: number;
    max: number;
  };
  count: number;
  minMs: number | null;
  medianMs: number | null;
  p95Ms: number | null;
  maxMs: number | null;
  invalidGaps: Array<{
    afterKeyIndex: number;
    gapMs: number | null;
  }>;
  valid: boolean;
}

interface NormalTypingEvidence {
  generatedAt: string;
  url: string;
  measurementCompleted: boolean;
  passed: boolean;
  typingMode: "normal-interactive-exact-input";
  schemaId: "jyut6ping3";
  input: string;
  inputLength: number;
  diagnosticCount: number;
  keyIntervalMs: number;
  ceilingsMs: {
    p95: number;
    max: number;
    workerQueueWaitMax: number;
  };
  summary: LatencySummary;
  workerQueueWaitMaxMs: number | null;
  cadence: CadenceEvidence;
  diagnostics: PerfDiagnostic[];
  finalVisibleCandidateOrder: CandidateRowSnapshot[];
  candidateValidation: {
    mode: "page-shape-only-no-oracle-claim";
    rationale: string;
  };
  assetRequestsDuringMeasurement: AssetRequest[];
  buildInfo: PublicBuildInfo;
}

interface CandidateOrderVerification extends FinalCandidateGuard {
  matched: boolean;
}

interface ScenarioEvidence {
  name: ScenarioName;
  schemaId: string;
  input: string;
  inputLength: number;
  diagnosticCount: number;
  summary: LatencySummary;
  slowestKey: PerfDiagnostic | null;
  diagnostics: PerfDiagnostic[];
  cadence: CadenceEvidence;
  finalVisibleCandidateOrder: CandidateRowSnapshot[];
  candidateOrderVerification: CandidateOrderVerification;
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

function summarizeCadence(
  diagnostics: PerfDiagnostic[],
  expectedIntervalMs = KEY_INTERVAL_MS,
): CadenceEvidence {
  const acceptedMinMs = expectedIntervalMs * 0.8;
  const acceptedMaxMs = expectedIntervalMs * 1.25;
  const gaps = diagnostics.slice(1).map((diagnostic, index) => {
    const previous = diagnostics[index];
    const gapMs = diagnostic.keydownAt - previous.keydownAt;
    return {
      afterKeyIndex: index + 1,
      gapMs: Number.isFinite(gapMs) ? gapMs : null,
    };
  });
  const finiteGaps = gaps
    .map((gap) => gap.gapMs)
    .filter((gap): gap is number => gap !== null);
  const invalidGaps = gaps.filter(
    (gap) =>
      gap.gapMs === null ||
      gap.gapMs < acceptedMinMs ||
      gap.gapMs > acceptedMaxMs,
  );
  return {
    expectedIntervalMs,
    acceptedRangeMs: {
      min: acceptedMinMs,
      max: acceptedMaxMs,
    },
    count: gaps.length,
    minMs: percentile(finiteGaps, 0),
    medianMs: percentile(finiteGaps, 0.5),
    p95Ms: percentile(finiteGaps, 0.95),
    maxMs: percentile(finiteGaps, 1),
    invalidGaps,
    valid:
      gaps.length === Math.max(0, diagnostics.length - 1) &&
      finiteGaps.length === gaps.length &&
      invalidGaps.length === 0,
  };
}

function nextCadenceDeadline(
  previousDeadlineMs: number,
  nowMs: number,
  intervalMs: number,
  minimumGapMs = intervalMs * 0.8,
): number {
  const phasePreservingDeadlineMs = previousDeadlineMs + intervalMs;
  return phasePreservingDeadlineMs - nowMs < minimumGapMs
    ? nowMs + intervalMs
    : phasePreservingDeadlineMs;
}

async function typeAtCadence(
  page: Page,
  inputText: string,
  intervalMs: number,
): Promise<void> {
  const keys = [...inputText];
  let nextKeydownDeadlineMs = performance.now();
  for (const [index, key] of keys.entries()) {
    await page.keyboard.type(key);
    if (index + 1 >= keys.length) {
      break;
    }

    const nowMs = performance.now();
    nextKeydownDeadlineMs = nextCadenceDeadline(
      nextKeydownDeadlineMs,
      nowMs,
      intervalMs,
    );
    await page.waitForTimeout(
      Math.max(0, nextKeydownDeadlineMs - nowMs),
    );
  }
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
    expect(value.toolchain.rustcVersion).toBe(EXPECTED_RUSTC_VERSION);
    expect(value.toolchain.nodeVersion).toBe(EXPECTED_NODE_VERSION);
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
    const candidateLabel = `${scenario.name}: key ${index + 1} candidate page`;
    expect.soft(
      diagnostic.candidateCount,
      `${candidateLabel} must expose the complete configured visible page`,
    ).toBe(EXPECTED_VISIBLE_CANDIDATE_COUNT);
    expect.soft(
      diagnostic.totalCandidateCount,
      `${candidateLabel} total count must cover every visible row`,
    ).toBeGreaterThanOrEqual(EXPECTED_VISIBLE_CANDIDATE_COUNT);
    expect.soft(
      typeof diagnostic.firstCandidateText === "string" &&
        diagnostic.firstCandidateText.trim().length > 0,
      `${candidateLabel} must expose a nonempty first candidate`,
    ).toBe(true);

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
    scenario.cadence.count,
    `${scenario.name}: cadence must contain one gap per consecutive key pair`,
  ).toBe(Math.max(0, scenario.inputLength - 1));
  expect.soft(
    scenario.cadence.valid,
    `${scenario.name}: actual keydown cadence must remain within ${KEY_INTERVAL_MIN_MS}..${KEY_INTERVAL_MAX_MS} ms`,
  ).toBe(true);
  expect.soft(
    scenario.finalVisibleCandidateOrder,
    `${scenario.name}: final visible page must contain the accepted page size`,
  ).toHaveLength(EXPECTED_VISIBLE_CANDIDATE_COUNT);
  for (const [index, candidate] of scenario.finalVisibleCandidateOrder.entries()) {
    expect.soft(
      typeof candidate.text === "string" && candidate.text.trim().length > 0,
      `${scenario.name}: final candidate ${index + 1} must record nonempty text`,
    ).toBe(true);
    expect.soft(
      candidate.source === null || typeof candidate.source === "string",
      `${scenario.name}: final candidate ${index + 1} must record its nullable production source field`,
    ).toBe(true);
  }
  expect.soft(
    scenario.candidateOrderVerification.matched,
    `${scenario.name}: final first candidate must match ${scenario.candidateOrderVerification.provenance}`,
  ).toBe(true);
  expect.soft(
    scenario.diagnostics.at(-1)?.firstCandidateText,
    `${scenario.name}: final result-specific diagnostic must match the rendered first candidate`,
  ).toBe(scenario.finalVisibleCandidateOrder[0]?.text);
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
  name: ScenarioName,
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
  // Keep the absolute phase during ordinary operation, but rebase after a
  // delayed host timer rather than injecting a short catch-up gap. The receipt
  // still records and rejects the original long gap against the unchanged
  // cadence bounds.
  await typeAtCadence(page, inputText, KEY_INTERVAL_MS);
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
  const cadence = summarizeCadence(diagnostics);
  const finalVisibleCandidateOrder = await readVisibleCandidateRows(page);
  const candidateGuard = FINAL_CANDIDATE_GUARDS[name];
  const candidateOrderVerification: CandidateOrderVerification = {
    ...candidateGuard,
    matched:
      finalVisibleCandidateOrder[0]?.text ===
      candidateGuard.expectedFirstCandidateText,
  };
  const scenario: ScenarioEvidence = {
    name,
    schemaId,
    input: inputText,
    inputLength: inputText.length,
    diagnosticCount: diagnostics.length,
    summary: summarize(diagnostics),
    slowestKey,
    diagnostics,
    cadence,
    finalVisibleCandidateOrder,
    candidateOrderVerification,
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

async function writeOptionalEvidence(
  fileName: string,
  value: unknown,
): Promise<void> {
  if (!EVIDENCE_DIR) {
    return;
  }
  const fs = await import("fs/promises");
  const path = await import("path");
  await fs.mkdir(EVIDENCE_DIR, { recursive: true });
  await fs.writeFile(
    path.join(EVIDENCE_DIR, fileName),
    `${JSON.stringify(value, null, 2)}\n`,
    "utf8",
  );
}

// Keep one worker and declaration order, but do not use Playwright's serial
// failure semantics: a measured release red must not suppress the independent
// exact normal-typing receipt that runs before it.
test.describe.configure({ mode: "default" });

test("cadence scheduler rebases delayed host timers without a catch-up burst", () => {
  expect(nextCadenceDeadline(1_000, 1_040, 250)).toBe(1_250);
  expect(nextCadenceDeadline(1_000, 1_050, 250)).toBe(1_250);
  expect(nextCadenceDeadline(1_000, 1_100, 250)).toBe(1_350);
  expect(nextCadenceDeadline(1_000, 1_300, 250)).toBe(1_550);
});

test(
  "normal Jyutping typing stays responsive for the reported exact input",
  runNormalTypingCanary,
);

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
          index === 0
            ? "jyutping-historical-long-1"
            : "jyutping-historical-long-2",
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
    const scenarioNames = scenarios.map((scenario) => scenario.name);
    const verifiedKeyCount = scenarios.reduce(
      (total, scenario) => total + scenario.diagnosticCount,
      0,
    );
    const cadenceGapCount = scenarios.reduce(
      (total, scenario) => total + scenario.cadence.count,
      0,
    );
    const candidatePageProfileValid =
      scenarioNames.length === SCENARIO_NAMES.length &&
      scenarios.every(
        (scenario) =>
          scenario.diagnostics.length === scenario.inputLength &&
          scenario.diagnostics.every(
            (diagnostic) =>
              diagnostic.candidateCount ===
                EXPECTED_VISIBLE_CANDIDATE_COUNT &&
              diagnostic.totalCandidateCount >=
                EXPECTED_VISIBLE_CANDIDATE_COUNT &&
              typeof diagnostic.firstCandidateText === "string" &&
              diagnostic.firstCandidateText.trim().length > 0,
          ),
      );
    const candidateOrderProfileValid =
      scenarioNames.length === SCENARIO_NAMES.length &&
      scenarios.every(
        (scenario) =>
          scenario.finalVisibleCandidateOrder.length ===
            EXPECTED_VISIBLE_CANDIDATE_COUNT &&
          scenario.finalVisibleCandidateOrder.every(
            (candidate) =>
              typeof candidate.text === "string" &&
              candidate.text.trim().length > 0 &&
              (candidate.source === null ||
                typeof candidate.source === "string"),
          ) &&
          scenario.candidateOrderVerification.matched,
      );
    const cadenceProfileValid =
      scenarioNames.length === SCENARIO_NAMES.length &&
      cadenceGapCount === EXPECTED_RELEASE_CADENCE_GAP_COUNT &&
      scenarios.every((scenario) => scenario.cadence.valid);
    const profileValidity = {
      valid:
        candidatePageProfileValid &&
        candidateOrderProfileValid &&
        cadenceProfileValid,
      candidatePages: {
        expectedVisibleCandidateCount: EXPECTED_VISIBLE_CANDIDATE_COUNT,
        valid: candidatePageProfileValid,
      },
      candidateOrder: {
        scenarioCount: scenarios.length,
        expectedScenarioCount: SCENARIO_NAMES.length,
        valid: candidateOrderProfileValid,
      },
      cadence: {
        expectedIntervalMs: KEY_INTERVAL_MS,
        acceptedRangeMs: {
          min: KEY_INTERVAL_MIN_MS,
          max: KEY_INTERVAL_MAX_MS,
        },
        gapCount: cadenceGapCount,
        expectedGapCount: EXPECTED_RELEASE_CADENCE_GAP_COUNT,
        invalidGapCount: scenarios.reduce(
          (total, scenario) =>
            total + scenario.cadence.invalidGaps.length,
          0,
        ),
        valid: cadenceProfileValid,
      },
    };
    const diagnosticOverridesActive =
      P95_CEILING_MS !== RELEASE_P95_CEILING_MS ||
      MAX_CEILING_MS !== RELEASE_MAX_CEILING_MS ||
      CPU_THROTTLE_RATE !== RELEASE_CPU_THROTTLE_RATE ||
      KEY_INTERVAL_MS !== RELEASE_KEY_INTERVAL_MS;
    const releaseProfileDeviations: string[] = [];
    if (!IS_LOOPBACK_PREVIEW) {
      releaseProfileDeviations.push("deployed-origin-canary");
    }
    if (P95_CEILING_MS !== RELEASE_P95_CEILING_MS) {
      releaseProfileDeviations.push("p95-ceiling");
    }
    if (MAX_CEILING_MS !== RELEASE_MAX_CEILING_MS) {
      releaseProfileDeviations.push("max-ceiling");
    }
    if (CPU_THROTTLE_RATE !== RELEASE_CPU_THROTTLE_RATE) {
      releaseProfileDeviations.push("main-thread-throttle");
    }
    if (WORKER_ACTION_MULTIPLIER !== RELEASE_WORKER_ACTION_MULTIPLIER) {
      releaseProfileDeviations.push("worker-action-multiplier");
    }
    if (KEY_INTERVAL_MS !== RELEASE_KEY_INTERVAL_MS) {
      releaseProfileDeviations.push("key-interval");
    }
    if (JSON.stringify(scenarioNames) !== JSON.stringify(SCENARIO_NAMES)) {
      releaseProfileDeviations.push("scenario-order");
    }
    if (verifiedKeyCount !== EXPECTED_RELEASE_DIAGNOSTIC_COUNT) {
      releaseProfileDeviations.push("verified-key-count");
    }
    if (!profileValidity.valid) {
      releaseProfileDeviations.push("measured-profile-invalid");
    }
    const releaseProfileValid =
      IS_LOOPBACK_PREVIEW && releaseProfileDeviations.length === 0;
    const releaseGradeVerdict = !passed
      ? "fail"
      : !IS_LOOPBACK_PREVIEW
        ? "not-applicable"
        : releaseProfileValid
          ? "pass"
          : "invalid-profile";
    const evidence = {
      generatedAt: new Date().toISOString(),
      url: page.url(),
      readyAt,
      measurementCompleted,
      passed,
      thresholdVerdict: passed ? "pass" : "fail",
      releaseGradeVerdict,
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
              verifiedKeyCount,
            } satisfies CpuProfileEvidence,
      profileValidity,
      releaseProfile: {
        applicable: IS_LOOPBACK_PREVIEW,
        valid: releaseProfileValid,
        diagnosticOverridesActive,
        deviations: releaseProfileDeviations,
        expected: {
          ceilingsMs: {
            p95: RELEASE_P95_CEILING_MS,
            max: RELEASE_MAX_CEILING_MS,
          },
          keyIntervalMs: RELEASE_KEY_INTERVAL_MS,
          mainThreadThrottle: RELEASE_CPU_THROTTLE_RATE,
          workerActionMultiplier: RELEASE_WORKER_ACTION_MULTIPLIER,
          mechanism:
            "main-cdp-plus-synthetic-keydown-worker-service-amplification",
          workerStressScope: "loopback-preview",
          scenarioNames: SCENARIO_NAMES,
          verifiedKeyCount: EXPECTED_RELEASE_DIAGNOSTIC_COUNT,
        },
      },
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
    await writeOptionalEvidence("input-latency-hard-stop.json", evidence);
    await pageSession.detach().catch(() => undefined);
  }
});

async function runNormalTypingCanary(
  { page }: { page: Page },
  testInfo: TestInfo,
): Promise<void> {
  const postReadyAssetRequests: AssetRequest[] = [];
  await writeOptionalEvidence("normal-typing-exact-input.json", {
    generatedAt: new Date().toISOString(),
    url: null,
    measurementCompleted: false,
    passed: false,
    typingMode: "normal-interactive-exact-input",
    schemaId: "jyut6ping3",
    input: NORMAL_TYPING_INPUT,
    inputLength: NORMAL_TYPING_INPUT.length,
    failurePhase: "setup-or-measurement-incomplete",
  });
  page.on("request", (request) => {
    if (isPostReadySchemaRequest(request.url())) {
      postReadyAssetRequests.push({
        method: request.method(),
        url: request.url(),
      });
    }
  });

  // The public worker's diagnostic hook accepts only the binding 4x stress
  // profile. Omitting it is the product's ordinary 1x path.
  await page.goto("/", {
    waitUntil: "domcontentloaded",
    timeout: APP_READY_TIMEOUT_MS,
  });
  await waitForAppReady(page);
  const buildInfo = await readPublicBuildInfo(page);
  await selectSchema(page, "jyut6ping3");
  await clearComposition(page);
  await resetPerfDiagnostics(page);
  const input = composeInput(page);
  await input.focus();
  const assetRequestStart = postReadyAssetRequests.length;

  await typeAtCadence(
    page,
    NORMAL_TYPING_INPUT,
    NORMAL_TYPING_KEY_INTERVAL_MS,
  );
  await expect
    .poll(async () => (await readPerfDiagnostics(page)).length, {
      timeout: DIAGNOSTIC_TIMEOUT_MS,
    })
    .toBe(NORMAL_TYPING_INPUT.length);

  const diagnostics = await readPerfDiagnostics(page);
  const summary = summarize(diagnostics);
  const cadence = summarizeCadence(
    diagnostics,
    NORMAL_TYPING_KEY_INTERVAL_MS,
  );
  const workerQueueWaitMaxMs = percentile(
    numericValues(diagnostics, (diagnostic) => diagnostic.workerQueueWaitMs),
    1,
  );
  const finalVisibleCandidateOrder = await readVisibleCandidateRows(page);
  const assetRequestsDuringMeasurement = postReadyAssetRequests.slice(
    assetRequestStart,
  );
  const expectedPrefixes = Array.from(
    { length: NORMAL_TYPING_INPUT.length },
    (_, index) => NORMAL_TYPING_INPUT.slice(0, index + 1),
  );
  const checks = {
    diagnosticCount:
      diagnostics.length === NORMAL_TYPING_INPUT.length,
    exactPrefixes:
      JSON.stringify(
        diagnostics.map((diagnostic) =>
          diagnostic.input.replace(/\s+/g, ""),
        ),
      ) === JSON.stringify(expectedPrefixes),
    candidatePages: diagnostics.every(
      (diagnostic) =>
        diagnostic.candidateCount === EXPECTED_VISIBLE_CANDIDATE_COUNT &&
        diagnostic.totalCandidateCount >= EXPECTED_VISIBLE_CANDIDATE_COUNT &&
        typeof diagnostic.firstCandidateText === "string" &&
        diagnostic.firstCandidateText.trim().length > 0,
    ),
    completeTimings: diagnostics.every(
      (diagnostic) =>
        Number.isFinite(diagnostic.keydownAt) &&
        Number.isFinite(diagnostic.workerQueueWaitMs) &&
        Number.isFinite(diagnostic.workerProcessMs) &&
        Number.isFinite(diagnostic.workerRoundtripMs) &&
        Number.isFinite(diagnostic.responseMappingMs) &&
        Number.isFinite(diagnostic.reactUpdateMs) &&
        Number.isFinite(diagnostic.paintProxyMs) &&
        Number.isFinite(diagnostic.totalKeydownToPaintMs),
    ),
    unamplifiedWorker: diagnostics.every(
      (diagnostic) => diagnostic.workerActionMultiplier === 1,
    ),
    cadence: cadence.valid,
    p95:
      summary.p95Ms !== null &&
      summary.p95Ms <= NORMAL_TYPING_P95_CEILING_MS,
    max:
      summary.maxMs !== null &&
      summary.maxMs <= NORMAL_TYPING_MAX_CEILING_MS,
    queueWait:
      workerQueueWaitMaxMs !== null &&
      workerQueueWaitMaxMs <= NORMAL_TYPING_QUEUE_WAIT_MAX_CEILING_MS,
    finalPage:
      finalVisibleCandidateOrder.length ===
        EXPECTED_VISIBLE_CANDIDATE_COUNT &&
      finalVisibleCandidateOrder.every(
        (candidate) =>
          typeof candidate.text === "string" &&
          candidate.text.trim().length > 0 &&
          (candidate.source === null ||
            typeof candidate.source === "string"),
      ),
    finalDiagnostic:
      diagnostics.at(-1)?.firstCandidateText ===
      finalVisibleCandidateOrder[0]?.text,
    noTimedAssetRequests: assetRequestsDuringMeasurement.length === 0,
  };
  const passed = Object.values(checks).every(Boolean);
  const evidence: NormalTypingEvidence = {
    generatedAt: new Date().toISOString(),
    url: page.url(),
    measurementCompleted: true,
    passed,
    typingMode: "normal-interactive-exact-input",
    schemaId: "jyut6ping3",
    input: NORMAL_TYPING_INPUT,
    inputLength: NORMAL_TYPING_INPUT.length,
    diagnosticCount: diagnostics.length,
    keyIntervalMs: NORMAL_TYPING_KEY_INTERVAL_MS,
    ceilingsMs: {
      p95: NORMAL_TYPING_P95_CEILING_MS,
      max: NORMAL_TYPING_MAX_CEILING_MS,
      workerQueueWaitMax: NORMAL_TYPING_QUEUE_WAIT_MAX_CEILING_MS,
    },
    summary,
    workerQueueWaitMaxMs,
    cadence,
    diagnostics,
    finalVisibleCandidateOrder,
    candidateValidation: {
      mode: "page-shape-only-no-oracle-claim",
      rationale:
        "This exact product input has no pinned external candidate-order fixture. The canary binds responsiveness, every input prefix, and complete visible-page shape without treating Yune output as its own behavior oracle.",
    },
    assetRequestsDuringMeasurement,
    buildInfo,
  };
  const serialized = `${JSON.stringify(evidence, null, 2)}\n`;
  await testInfo.attach("normal-typing-exact-input", {
    body: serialized,
    contentType: "application/json",
  });
  await writeOptionalEvidence("normal-typing-exact-input.json", evidence);

  expect.soft(checks.diagnosticCount, "every typed key must be measured").toBe(
    true,
  );
  expect.soft(checks.exactPrefixes, "every exact input prefix must be observed").toBe(
    true,
  );
  expect.soft(checks.candidatePages, "every key must render a complete candidate page").toBe(
    true,
  );
  expect.soft(
    checks.completeTimings,
    "every typed key must retain a complete finite timing sample",
  ).toBe(true);
  expect.soft(checks.unamplifiedWorker, "normal typing must use the unamplified worker path").toBe(
    true,
  );
  expect.soft(
    cadence.valid,
    `normal typing cadence must remain within ${NORMAL_TYPING_KEY_INTERVAL_MIN_MS}..${NORMAL_TYPING_KEY_INTERVAL_MAX_MS} ms`,
  ).toBe(true);
  expect.soft(summary.p95Ms ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(
    NORMAL_TYPING_P95_CEILING_MS,
  );
  expect.soft(summary.maxMs ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(
    NORMAL_TYPING_MAX_CEILING_MS,
  );
  expect.soft(
    workerQueueWaitMaxMs ?? Number.POSITIVE_INFINITY,
  ).toBeLessThanOrEqual(NORMAL_TYPING_QUEUE_WAIT_MAX_CEILING_MS);
  expect.soft(checks.finalPage, "the final visible page must contain six nonempty rows").toBe(
    true,
  );
  expect.soft(checks.finalDiagnostic, "the final diagnostic and rendered first candidate must agree").toBe(
    true,
  );
  expect.soft(checks.noTimedAssetRequests, "timed typing must not fetch schema assets").toBe(
    true,
  );
}
