import { expect, test, chromium, type Page } from "@playwright/test";
import http from "node:http";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(__dirname, "..");
const trackedDist = path.join(appRoot, "dist");
const resultRoot = path.join(
  __dirname,
  "results",
  process.env.YUNE_WEB_JYUTPING_MEMORY_RESULT_ROOT ??
    "yune-web-jyutping-memory-attribution",
);
const phaseName =
  process.env.YUNE_WEB_JYUTPING_MEMORY_PHASE ?? "phase-0-current-runtime";
const phaseDir = path.join(resultRoot, phaseName);
const readyTimeoutMs = 120_000;
const expectSchemaSwitchPass =
  process.env.YUNE_WEB_JYUTPING_MEMORY_EXPECT_SCHEMA_SWITCH_PASS === "1";

type ScenarioKind = "clean-jyutping" | "schema-switch" | "jyutping-luna-jyutping";

interface WasmMemorySnapshot {
  currentBytes: number;
  peakBytes: number;
}

interface DebugSnapshot {
  activeSchema: string | null;
  loading: string | null;
  initialized: string | null;
  activeSchemaStorage: string | null;
  actionDiagnostics: unknown[];
  actionErrors: unknown[];
  persistenceDiagnostics: unknown[];
  lastActionError: unknown | null;
}

interface StepEvidence {
  name: string;
  expectedSchema: string;
  activeSchema: string | null;
  input: string;
  expectedTop: string;
  topCandidate: string | null;
  candidateCount: number;
  candidates: Array<{
    index: number;
    text: string | null;
    note: string | null;
    source: string | null;
    rowText: string;
  }>;
  inputValue: string;
  wasmMemory?: WasmMemorySnapshot;
  startupWasmMemory?: WasmMemorySnapshot;
  startupMarkers: Array<{ phase: string; ms: number; wasmMemory?: WasmMemorySnapshot }>;
  debug: DebugSnapshot;
  passed: boolean;
  consoleErrors: string[];
}

interface ScenarioEvidence {
  kind: ScenarioKind;
  build: "tracked-dist";
  url: string;
  initialized: boolean;
  startedAt: string;
  finishedAt: string;
  steps: StepEvidence[];
  verdict: "pass" | "candidate-missing" | "schema-mismatch" | "init-failed";
  maxObservedWasmBytes: number;
}

test.describe("M46 JYUTPING MEMORY attribution", () => {
  test.skip(
    process.env.YUNE_WEB_JYUTPING_MEMORY_ATTRIBUTION !== "1",
    "Set YUNE_WEB_JYUTPING_MEMORY_ATTRIBUTION=1 to run this opt-in evidence capture.",
  );
  test.setTimeout(30 * 60 * 1000);

  test("captures clean Jyutping and schema-switch correctness evidence", async () => {
    await assertDistExists(trackedDist, "tracked apps/yune-web dist");
    const server = await startStaticServer(trackedDist);
    const samples: ScenarioEvidence[] = [];
    try {
      samples.push(await runCleanJyutpingScenario(server.url));
      samples.push(await runSchemaSwitchScenario(server.url));
      samples.push(await runJyutpingLunaJyutpingScenario(server.url));
    } finally {
      await server.close();
    }
    await writeEvidence(samples);
    expect(samples.some((sample) => sample.kind === "clean-jyutping" && sample.initialized)).toBe(true);
    expect(samples.some((sample) => sample.kind === "schema-switch" && sample.initialized)).toBe(true);
    if (expectSchemaSwitchPass) {
      const failures = samples.flatMap((sample) =>
        sample.steps
          .filter((step) => !step.passed || step.debug.actionErrors.length > 0)
          .map((step) => ({
            scenario: sample.kind,
            step: step.name,
            activeSchema: step.activeSchema,
            expectedSchema: step.expectedSchema,
            topCandidate: step.topCandidate,
            expectedTop: step.expectedTop,
            actionErrors: step.debug.actionErrors,
            consoleErrors: step.consoleErrors,
          })),
      );
      expect(failures, JSON.stringify(failures, null, 2)).toEqual([]);
    }
  });
});

async function runCleanJyutpingScenario(baseUrl: string): Promise<ScenarioEvidence> {
  return runScenario("clean-jyutping", baseUrl, async (page) => [
    await captureTypingStep(page, "jyutping-only-nei", "jyut6ping3", "nei", "\u4f60"),
  ]);
}

async function runSchemaSwitchScenario(baseUrl: string): Promise<ScenarioEvidence> {
  return runScenario("schema-switch", baseUrl, async (page) => {
    const steps: StepEvidence[] = [];
    await selectSchema(page, "cangjie5");
    steps.push(await captureTypingStep(page, "cangjie-a", "cangjie5", "a", "\u65e5"));
    await selectSchema(page, "luna_pinyin");
    steps.push(await captureTypingStep(page, "luna-hao", "luna_pinyin", "hao", "\u597d"));
    await selectSchema(page, "jyut6ping3");
    steps.push(await captureTypingStep(page, "jyutping-nei-after-switch", "jyut6ping3", "nei", "\u4f60"));
    return steps;
  });
}

async function runJyutpingLunaJyutpingScenario(baseUrl: string): Promise<ScenarioEvidence> {
  return runScenario("jyutping-luna-jyutping", baseUrl, async (page) => {
    const steps: StepEvidence[] = [];
    steps.push(await captureTypingStep(page, "jyutping-nei-before-luna", "jyut6ping3", "nei", "\u4f60"));
    await selectSchema(page, "luna_pinyin");
    steps.push(await captureTypingStep(page, "luna-hao-after-jyutping", "luna_pinyin", "hao", "\u597d"));
    await selectSchema(page, "jyut6ping3");
    steps.push(await captureTypingStep(page, "jyutping-nei-after-luna", "jyut6ping3", "nei", "\u4f60"));
    return steps;
  });
}

async function runScenario(
  kind: ScenarioKind,
  baseUrl: string,
  collect: (page: Page) => Promise<StepEvidence[]>,
): Promise<ScenarioEvidence> {
  const startedAt = new Date().toISOString();
  const userDataDir = path.join(os.tmpdir(), `yune-web-m46-${process.pid}-${kind}-${Date.now()}`);
  await fs.rm(userDataDir, { recursive: true, force: true });
  await fs.mkdir(userDataDir, { recursive: true });
  const context = await chromium.launchPersistentContext(userDataDir, {
    headless: true,
    locale: "zh-HK",
    viewport: { width: 1365, height: 900 },
  });
  await context.addInitScript(() => {
    localStorage.setItem("activeSchema", "jyut6ping3");
    localStorage.setItem("uiLanguage", "en");
    localStorage.setItem("enableAI", "false");
  });
  const page = await context.newPage();
  const consoleErrors = captureConsoleErrors(page);
  const url = `${baseUrl}/?benchmark=m46-jyutping-memory&scenario=${encodeURIComponent(kind)}`;
  let initialized = false;
  let steps: StepEvidence[] = [];
  try {
    await page.goto(url, { waitUntil: "domcontentloaded" });
    initialized = await waitForAppReady(page).then(() => true, () => false);
    if (initialized) {
      steps = await collect(page);
    }
  } finally {
    await context.close();
    await fs.rm(userDataDir, { recursive: true, force: true });
  }
  for (const step of steps) {
    step.consoleErrors.push(...consoleErrors);
  }
  const verdict = scenarioVerdict(initialized, steps);
  return {
    kind,
    build: "tracked-dist",
    url,
    initialized,
    startedAt,
    finishedAt: new Date().toISOString(),
    steps,
    verdict,
    maxObservedWasmBytes: Math.max(
      0,
      ...steps.flatMap((step) => [
        step.wasmMemory?.currentBytes ?? 0,
        step.wasmMemory?.peakBytes ?? 0,
        step.startupWasmMemory?.currentBytes ?? 0,
        step.startupWasmMemory?.peakBytes ?? 0,
        ...step.startupMarkers.flatMap((marker) => [
          marker.wasmMemory?.currentBytes ?? 0,
          marker.wasmMemory?.peakBytes ?? 0,
        ]),
      ]),
    ),
  };
}

async function captureTypingStep(
  page: Page,
  name: string,
  expectedSchema: string,
  input: string,
  expectedTop: string,
): Promise<StepEvidence> {
  await clearComposition(page);
  const inputField = page.locator("input[type='text'], textarea").first();
  await inputField.focus();
  await inputField.type(input, { delay: 120 });
  await page.waitForTimeout(1500);
  const candidates = await readCandidates(page);
  const activeSchema = await activeSchemaId(page);
  const diagnostics = await startupMarker(page);
  const wasmMemory = await activeWasmMemory(page);
  const debug = await debugSnapshot(page);
  const topCandidate = candidates[0]?.text ?? null;
  return {
    name,
    expectedSchema,
    activeSchema,
    input,
    expectedTop,
    topCandidate,
    candidateCount: candidates.length,
    candidates,
    inputValue: await inputField.inputValue(),
    wasmMemory,
    startupWasmMemory: diagnostics?.wasmMemory,
    startupMarkers: diagnostics?.markers ?? [],
    debug,
    passed: activeSchema === expectedSchema && topCandidate === expectedTop,
    consoleErrors: [],
  };
}

async function waitForAppReady(page: Page): Promise<void> {
  await page.waitForFunction(
    () =>
      document.documentElement.dataset.yuneInitialized === "true" &&
      document.documentElement.dataset.yuneLoading === "false" &&
      document.querySelector("[data-yune-loading-indicator]") === null,
    undefined,
    { timeout: readyTimeoutMs },
  );
}

async function selectSchema(page: Page, schema: string): Promise<void> {
  await clearComposition(page);
  const select = page.locator("[data-yune-schema-switcher] select");
  await expect(select).toBeVisible({ timeout: 10_000 });
  await select.selectOption(schema);
  await expect
    .poll(() => activeSchemaId(page), { timeout: readyTimeoutMs })
    .toBe(schema);
  await waitForAppReady(page);
  await page.waitForTimeout(500);
}

async function clearComposition(page: Page): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await inputField.focus();
  for (
    let attempts = 0;
    attempts < 4 && (await page.locator(".candidate-panel").count()) > 0;
    attempts += 1
  ) {
    await page.keyboard.press("Escape").catch(() => undefined);
    await page.waitForTimeout(150);
  }
  await inputField.fill("");
}

async function readCandidates(page: Page): Promise<StepEvidence["candidates"]> {
  return await page.locator(".candidate-panel .candidates tbody").evaluateAll((elements) =>
    elements.map((element, index) => {
      const firstRow = element.querySelector("tr");
      const cells = Array.from(firstRow?.querySelectorAll("td") ?? []);
      return {
        index,
        text: element.getAttribute("data-candidate-text"),
        note: cells[2]?.textContent?.trim() || null,
        source: element.getAttribute("data-source"),
        rowText: element.textContent?.replace(/\s+/g, " ").trim() ?? "",
      };
    }),
  );
}

async function activeSchemaId(page: Page): Promise<string | null> {
  return await page.evaluate(() => document.documentElement.dataset.yuneActiveSchema ?? null);
}

async function debugSnapshot(page: Page): Promise<DebugSnapshot> {
  return await page.evaluate(() => {
    const parseDatasetJson = <T,>(key: string, fallback: T): T => {
      const raw = document.documentElement.dataset[key];
      if (!raw) {
        return fallback;
      }
      try {
        return JSON.parse(raw) as T;
      } catch {
        return fallback;
      }
    };
    const debug = (
      window as typeof window & {
        __YUNE_WEB_DEBUG__?: {
          actionDiagnostics?: () => unknown[];
          actionErrors?: () => unknown[];
          persistenceDiagnostics?: () => unknown[];
        };
      }
    ).__YUNE_WEB_DEBUG__;
    return {
      activeSchema: document.documentElement.dataset.yuneActiveSchema ?? null,
      loading: document.documentElement.dataset.yuneLoading ?? null,
      initialized: document.documentElement.dataset.yuneInitialized ?? null,
      activeSchemaStorage: localStorage.getItem("activeSchema"),
      actionDiagnostics:
        debug?.actionDiagnostics?.() ??
        parseDatasetJson<unknown[]>("yuneActionDiagnostics", []),
      actionErrors:
        debug?.actionErrors?.() ??
        parseDatasetJson<unknown[]>("yuneActionErrors", []),
      persistenceDiagnostics:
        debug?.persistenceDiagnostics?.() ??
        parseDatasetJson<unknown[]>("yunePersistenceDiagnostics", []),
      lastActionError: parseDatasetJson<unknown | null>("yuneLastActionError", null),
    };
  });
}

async function startupMarker(page: Page): Promise<{
  wasmMemory?: WasmMemorySnapshot;
  markers?: Array<{ phase: string; ms: number; wasmMemory?: WasmMemorySnapshot }>;
} | undefined> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]") as Array<{
      source?: string;
      marker?: {
        phase?: string;
        wasmMemory?: WasmMemorySnapshot;
        markers?: Array<{ phase: string; ms: number; wasmMemory?: WasmMemorySnapshot }>;
      };
    }>;
    return diagnostics.findLast((entry) => entry.source === "yune-startup" && entry.marker?.phase === "startup:complete")?.marker;
  });
}

async function activeWasmMemory(page: Page): Promise<WasmMemorySnapshot | undefined> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yuneActionDiagnostics ?? "[]") as Array<{
      result?: { memory?: { wasmHeapBytes?: number; peakWasmHeapBytes?: number } };
    }>;
    for (let index = diagnostics.length - 1; index >= 0; index -= 1) {
      const memory = diagnostics[index]?.result?.memory;
      if (typeof memory?.wasmHeapBytes === "number" || typeof memory?.peakWasmHeapBytes === "number") {
        return {
          currentBytes: memory.wasmHeapBytes ?? 0,
          peakBytes: memory.peakWasmHeapBytes ?? memory.wasmHeapBytes ?? 0,
        };
      }
    }
    return undefined;
  });
}

function scenarioVerdict(
  initialized: boolean,
  steps: StepEvidence[],
): ScenarioEvidence["verdict"] {
  if (!initialized) {
    return "init-failed";
  }
  if (steps.some((step) => step.activeSchema !== step.expectedSchema)) {
    return "schema-mismatch";
  }
  if (steps.some((step) => step.topCandidate !== step.expectedTop)) {
    return "candidate-missing";
  }
  return "pass";
}

function captureConsoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") {
      errors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    errors.push(error.message);
  });
  return errors;
}

async function writeEvidence(samples: ScenarioEvidence[]): Promise<void> {
  await fs.mkdir(phaseDir, { recursive: true });
  await fs.writeFile(path.join(phaseDir, "samples.json"), `${JSON.stringify(samples, null, 2)}\n`);
  await fs.writeFile(path.join(phaseDir, "summary.csv"), summaryCsv(samples));
  await fs.writeFile(path.join(phaseDir, "report.md"), reportMarkdown(samples));
}

function summaryCsv(samples: ScenarioEvidence[]): string {
  const header = [
    "kind",
    "initialized",
    "verdict",
    "steps",
    "failedSteps",
    "maxObservedWasmBytes",
    "workerActionErrors",
    "url",
  ];
  const rows = samples.map((sample) => [
    sample.kind,
    String(sample.initialized),
    sample.verdict,
    String(sample.steps.length),
    sample.steps.filter((step) => !step.passed).map((step) => step.name).join(" "),
    String(sample.maxObservedWasmBytes),
    String(sampleActionErrorCount(sample)),
    sample.url,
  ]);
  return [header, ...rows].map((row) => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function reportMarkdown(samples: ScenarioEvidence[]): string {
  const rows = samples
    .map(
      (sample) =>
        `| ${sample.kind} | ${sample.initialized ? "yes" : "no"} | ${sample.verdict} | ${sample.steps.length} | ${sample.steps.filter((step) => !step.passed).map((step) => step.name).join(", ") || "-"} | ${bytes(sample.maxObservedWasmBytes)} | ${sampleActionErrorCount(sample)} |`,
    )
    .join("\n");
  const stepRows = samples
    .flatMap((sample) =>
      sample.steps.map(
        (step) =>
          `| ${sample.kind} | ${step.name} | ${step.activeSchema ?? "-"} | ${step.input} | ${step.topCandidate ?? "-"} | ${step.candidateCount} | ${step.passed ? "pass" : "fail"} | ${bytes(step.wasmMemory?.currentBytes ?? step.startupWasmMemory?.currentBytes ?? 0)} | ${bytes(step.wasmMemory?.peakBytes ?? step.startupWasmMemory?.peakBytes ?? 0)} | ${step.debug.actionErrors.length} |`,
      ),
    )
    .join("\n");
  return `# M46 Jyutping Memory Attribution Browser Evidence

| Scenario | Initialized | Verdict | Steps | Failed steps | Max observed WASM | Worker action errors |
| --- | --- | --- | ---: | --- | ---: | ---: |
${rows}

## Steps

| Scenario | Step | Active schema | Input | Top candidate | Candidate count | Result | WASM current | WASM peak | Worker action errors |
| --- | --- | --- | --- | --- | ---: | --- | ---: | ---: | ---: |
${stepRows}
`;
}

function sampleActionErrorCount(sample: ScenarioEvidence): number {
  return sample.steps.reduce((total, step) => total + step.debug.actionErrors.length, 0);
}

function bytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0 B";
  }
  const mib = value / 1024 / 1024;
  return `${mib.toFixed(1)} MiB`;
}

function csvEscape(value: string): string {
  return /[",\n]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value;
}

async function assertDistExists(dir: string, label: string): Promise<void> {
  await fs.access(path.join(dir, "index.html")).catch((error) => {
    throw new Error(`${label} is missing; run npm.cmd --prefix apps/yune-web run build first (${error})`);
  });
}

async function startStaticServer(root: string): Promise<{ url: string; close: () => Promise<void> }> {
  const server = http.createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
      const pathname = decodeURIComponent(requestUrl.pathname);
      const normalized = pathname === "/" ? "/index.html" : pathname;
      const filePath = path.resolve(root, `.${normalized}`);
      if (!filePath.startsWith(path.resolve(root))) {
        response.writeHead(403);
        response.end("forbidden");
        return;
      }
      const data = await fs.readFile(filePath);
      response.writeHead(200, {
        "content-type": contentType(filePath),
        "cache-control": "no-cache",
      });
      response.end(data);
    } catch {
      response.writeHead(404);
      response.end("not found");
    }
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  if (address === null || typeof address === "string") {
    throw new Error("static server did not expose a TCP address");
  }
  return {
    url: `http://127.0.0.1:${address.port}`,
    close: () => new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve()))),
  };
}

function contentType(file: string): string {
  switch (path.extname(file).toLowerCase()) {
    case ".html":
      return "text/html; charset=utf-8";
    case ".js":
      return "application/javascript; charset=utf-8";
    case ".css":
      return "text/css; charset=utf-8";
    case ".wasm":
      return "application/wasm";
    case ".json":
      return "application/json; charset=utf-8";
    case ".yaml":
    case ".yml":
    case ".txt":
      return "text/plain; charset=utf-8";
    default:
      return "application/octet-stream";
  }
}
