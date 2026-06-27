import { test, expect, chromium, type Page } from "@playwright/test";

import { statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import { mkdir, readFile, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  writeComparatorEvidence,
  type ComparatorResource,
  type ComparatorSample,
  type ComparatorWorkerMemory,
} from "./startup-benchmark/comparator-metrics";
import { appSchemaId, type StartupSchema } from "./startup-benchmark/scenarios";
import type { WasmMemorySnapshot } from "./startup-benchmark/metrics";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(__dirname, "..");
const resultRoot = path.join(
  __dirname,
  "results",
  process.env.YUNE_WEB_COMPARATOR_RESULT_ROOT ?? "yune-web-vs-my-rime-baseline",
);
const phaseName = process.env.YUNE_WEB_COMPARATOR_PHASE ?? "baseline";
const phaseDir = path.join(resultRoot, phaseName);
const trackedDist = path.join(appRoot, "dist");
const publicDist = path.join(appRoot, "public-demo", "dist");
const includeMyRime = process.env.YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME === "1";
const myRimeUrl = process.env.YUNE_WEB_COMPARATOR_MY_RIME_URL ?? "https://my-rime.vercel.app/";
const sampleCount = Math.max(1, Math.floor(Number(process.env.YUNE_WEB_COMPARATOR_SAMPLES ?? "3")));
const readyTimeoutMs = 120_000;

interface ComparatorScenario {
  id: string;
  app: "yune-web" | "my-rime";
  build: string;
  schema: "luna_pinyin" | "jyutping";
  runtimeSchema?: StartupSchema;
  input: string;
  publicDemo?: boolean;
}

const yuneScenarios: ComparatorScenario[] = [
  {
    id: "yune-tracked",
    app: "yune-web",
    build: "tracked-dist",
    schema: "luna_pinyin",
    runtimeSchema: "luna_pinyin",
    input: "ni",
  },
  {
    id: "yune-tracked",
    app: "yune-web",
    build: "tracked-dist",
    schema: "jyutping",
    runtimeSchema: "jyut6ping3_mobile",
    input: "nei",
  },
  {
    id: "yune-public-demo",
    app: "yune-web",
    build: "public-demo-dist",
    schema: "luna_pinyin",
    runtimeSchema: "luna_pinyin",
    input: "ni",
    publicDemo: true,
  },
  {
    id: "yune-public-demo",
    app: "yune-web",
    build: "public-demo-dist",
    schema: "jyutping",
    runtimeSchema: "jyut6ping3_mobile",
    input: "nei",
    publicDemo: true,
  },
];

const myRimeScenarios: ComparatorScenario[] = [
  {
    id: "my-rime-live",
    app: "my-rime",
    build: "vercel-live-c73ea17",
    schema: "luna_pinyin",
    input: "ni",
  },
  {
    id: "my-rime-live",
    app: "my-rime",
    build: "vercel-live-c73ea17",
    schema: "jyutping",
    input: "nei",
  },
];

test.describe("YUNE WEB COMPARATOR benchmark", () => {
  test.skip(process.env.YUNE_WEB_COMPARATOR_BASELINE !== "1", "Set YUNE_WEB_COMPARATOR_BASELINE=1 to run this opt-in benchmark.");
  test.setTimeout(60 * 60 * 1000);

  test("YUNE WEB COMPARATOR browser baseline", async () => {
    await assertDistExists(trackedDist, "tracked apps/yune-web dist");
    await assertDistExists(publicDist, "public-demo dist");
    const trackedServer = await startStaticServer(trackedDist);
    const publicServer = await startStaticServer(publicDist);
    const samples: ComparatorSample[] = [];
    try {
      for (const scenario of yuneScenarios) {
        for (let index = 0; index < sampleCount; index += 1) {
          samples.push(await runYuneScenarioSample(
            scenario,
            index,
            scenario.publicDemo ? publicServer.url : trackedServer.url,
            scenario.publicDemo ? publicDist : trackedDist,
          ));
        }
      }
      if (includeMyRime) {
        for (const scenario of myRimeScenarios) {
          for (let index = 0; index < sampleCount; index += 1) {
            samples.push(await runMyRimeScenarioSample(scenario, index));
          }
        }
      }
    } finally {
      await trackedServer.close();
      await publicServer.close();
    }
    await writeComparatorEvidence(phaseDir, samples);
    expect(samples.length).toBeGreaterThan(0);
  });
});

async function runYuneScenarioSample(
  scenario: ComparatorScenario,
  sampleIndex: number,
  baseUrl: string,
  distRoot: string,
): Promise<ComparatorSample> {
  const userDataDir = await freshUserDataDir(scenario, sampleIndex);
  const context = await chromium.launchPersistentContext(userDataDir, {
    headless: true,
    viewport: { width: 1365, height: 900 },
    locale: "zh-HK",
  });
  try {
    await context.addInitScript(({ schema }) => {
      localStorage.setItem("activeSchema", schema);
      localStorage.setItem("uiLanguage", "en");
      localStorage.setItem("enableAI", "false");
    }, { schema: appSchemaId(scenario.runtimeSchema ?? "luna_pinyin") });
    const page = await context.newPage();
    const consoleErrors = captureConsoleErrors(page);
    const url = `${baseUrl}/?benchmark=yune-web-comparator&schema=${encodeURIComponent(scenario.runtimeSchema ?? "luna_pinyin")}&scenario=${encodeURIComponent(scenario.id)}&sample=${sampleIndex}`;
    const startedAt = Date.now();
    await loadAndWaitYuneReady(page, url, scenario.runtimeSchema ?? "luna_pinyin");
    const readyAt = Date.now();
    const readyStartup = await yuneStartupMarker(page);
    const input = page.locator("input[type='text'], textarea").first();
    await input.fill("");
    const beforePerfCount = await yunePerfCount(page);
    const inputStartedAt = Date.now();
    await input.click();
    await page.keyboard.type(scenario.input, { delay: 5 });
    await page.waitForFunction(
      minCount => {
        const diagnostics = JSON.parse(document.documentElement.dataset.yunePerfDiagnostics ?? "[]") as Array<{
          candidateCount?: number;
          totalCandidateCount?: number;
        }>;
        return diagnostics.slice(Number(minCount)).some(entry =>
          (entry.candidateCount ?? entry.totalCandidateCount ?? 0) > 0
        );
      },
      beforePerfCount,
      { timeout: 30_000 },
    );
    const candidateAt = Date.now();
    const candidatePerf = await latestYunePerf(page);
    const firstCandidateText = await firstYuneCandidateText(page);
    const commitStartedAt = Date.now();
    await page.keyboard.press("Space");
    await expect.poll(async () => {
      const value = await readInputValue(page);
      return value.length > 0 && value !== scenario.input ? value : "";
    }, { timeout: 30_000 }).not.toBe("");
    const commitAt = Date.now();
    const committedValue = await readInputValue(page);
    const commitPerf = await latestYunePerf(page);
    let resources = [
      ...await collectPageResources(page),
      ...await collectWorkerResources(page),
    ];
    resources = appendYuneSyntheticResources(resources, readyStartup, distRoot, url);
    return {
      scenarioId: scenario.id,
      app: scenario.app,
      build: scenario.build,
      schema: scenario.schema,
      schemaInput: scenario.input,
      sampleIndex,
      url,
      readyToInputMs: readyAt - startedAt,
      inputToCandidateMs: candidateAt - inputStartedAt,
      commitMs: commitAt - commitStartedAt,
      firstCandidateText,
      committedValue,
      wasmMemory: {
        ready: readyStartup?.wasmMemory,
        candidate: yuneWasmFromPerf(candidatePerf),
        commit: yuneWasmFromPerf(commitPerf),
      },
      yunePerf: {
        internalKeydownToPaintMs: candidatePerf?.totalKeydownToPaintMs,
        workerProcessMs: candidatePerf?.workerProcessMs,
        workerRoundtripMs: candidatePerf?.workerRoundtripMs,
        firstCandidateText: candidatePerf?.firstCandidateText,
      },
      browserMemory: await collectBrowserMemory(page),
      resources,
      storageEstimate: await storageEstimate(page),
      workerUrls: page.workers().map(worker => worker.url()),
      consoleErrors,
    };
  } finally {
    await context.close();
    await rm(userDataDir, { recursive: true, force: true });
  }
}

async function runMyRimeScenarioSample(
  scenario: ComparatorScenario,
  sampleIndex: number,
): Promise<ComparatorSample> {
  const userDataDir = await freshUserDataDir(scenario, sampleIndex);
  const context = await chromium.launchPersistentContext(userDataDir, {
    headless: true,
    viewport: { width: 1365, height: 900 },
    locale: "zh-HK",
  });
  try {
    const page = await context.newPage();
    const consoleErrors = captureConsoleErrors(page);
    const url = myRimeScenarioUrl(scenario, sampleIndex);
    const startedAt = Date.now();
    await loadAndWaitMyRimeReady(page, url);
    await expect.poll(
      async () => (await myRimeWorkerMemory(page))?.heapBytes ?? 0,
      { timeout: readyTimeoutMs },
    ).toBeGreaterThan(0);
    const readyAt = Date.now();
    const readyMemory = await myRimeWorkerMemory(page);
    const input = await editableInput(page);
    await clearEditable(input);
    const inputStartedAt = Date.now();
    await input.click();
    await page.keyboard.type(scenario.input, { delay: 5 });
    await waitForMyRimeCandidate(page, scenario.input);
    const candidateAt = Date.now();
    const candidateMemory = await myRimeWorkerMemory(page);
    const firstCandidateText = await firstMyRimeCandidateText(page);
    const commitStartedAt = Date.now();
    await page.keyboard.press("Space");
    await expect.poll(async () => readEditableValue(input), { timeout: 30_000 }).not.toBe("");
    const commitAt = Date.now();
    const committedValue = await readEditableValue(input);
    const commitMemory = await myRimeWorkerMemory(page);
    return {
      scenarioId: scenario.id,
      app: scenario.app,
      build: scenario.build,
      schema: scenario.schema,
      schemaInput: scenario.input,
      sampleIndex,
      url,
      readyToInputMs: readyAt - startedAt,
      inputToCandidateMs: candidateAt - inputStartedAt,
      commitMs: commitAt - commitStartedAt,
      firstCandidateText,
      committedValue,
      wasmMemory: {
        ready: workerMemorySnapshot(readyMemory),
        candidate: workerMemorySnapshot(candidateMemory),
        commit: workerMemorySnapshot(commitMemory),
        worker: commitMemory ?? candidateMemory ?? readyMemory,
      },
      browserMemory: await collectBrowserMemory(page),
      resources: [
        ...await collectPageResources(page),
        ...await collectWorkerResources(page),
      ],
      storageEstimate: await storageEstimate(page),
      workerUrls: page.workers().map(worker => worker.url()),
      consoleErrors,
    };
  } finally {
    await context.close();
    await rm(userDataDir, { recursive: true, force: true });
  }
}

async function freshUserDataDir(scenario: ComparatorScenario, sampleIndex: number): Promise<string> {
  const dir = path.join(
    os.tmpdir(),
    `yune-web-comparator-${process.pid}-${scenario.id}-${scenario.schema}-${sampleIndex}-${Date.now()}`,
  );
  await rm(dir, { recursive: true, force: true });
  await mkdir(dir, { recursive: true });
  return dir;
}

async function loadAndWaitYuneReady(page: Page, url: string, schema: StartupSchema): Promise<void> {
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    ({ expectedSchema, appSchema }) => {
      const root = document.documentElement;
      const textarea = document.querySelector("textarea") as HTMLTextAreaElement | null;
      const diagnostics = JSON.parse(root.dataset.yunePersistenceDiagnostics ?? "[]") as Array<{
        source?: string;
        marker?: { phase?: string };
      }>;
      const startupComplete = diagnostics.some(entry =>
        entry.source === "yune-startup" && entry.marker?.phase === "startup:complete"
      );
      const activeSchema = root.dataset.yuneActiveSchema;
      const expectedActive = expectedSchema === "jyut6ping3_mobile"
        ? activeSchema === "jyut6ping3" || activeSchema === "jyut6ping3_mobile"
        : activeSchema === appSchema;
      return root.dataset.yuneInitialized === "true"
        && root.dataset.yuneLoading !== "true"
        && startupComplete
        && expectedActive
        && textarea !== null
        && !textarea.disabled
        && document.querySelector("[data-yune-loading-indicator]") === null;
    },
    { expectedSchema: schema, appSchema: appSchemaId(schema) },
    { timeout: readyTimeoutMs },
  );
}

async function loadAndWaitMyRimeReady(page: Page, url: string): Promise<void> {
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    () => {
      const editable = document.querySelector("textarea, input[type='text'], [contenteditable='true']");
      const copyLinkButton = [...document.querySelectorAll("button")]
        .find(button => button.getAttribute("title") === "Copy link for current IME") as HTMLButtonElement | undefined;
      return editable !== null
        && !(editable as HTMLInputElement | HTMLTextAreaElement).disabled
        && copyLinkButton !== undefined
        && !copyLinkButton.disabled;
    },
    undefined,
    { timeout: readyTimeoutMs },
  );
}

async function editableInput(page: Page) {
  const input = page.locator("textarea, input[type='text'], [contenteditable='true']").first();
  await expect(input).toBeVisible({ timeout: readyTimeoutMs });
  return input;
}

async function clearEditable(locator: ReturnType<Page["locator"]>): Promise<void> {
  await locator.evaluate((element) => {
    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
      element.value = "";
      element.dispatchEvent(new Event("input", { bubbles: true }));
      return;
    }
    element.textContent = "";
    element.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function readEditableValue(locator: ReturnType<Page["locator"]>): Promise<string> {
  return await locator.evaluate((element) => {
    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
      return element.value;
    }
    return element.textContent ?? "";
  });
}

async function readInputValue(page: Page): Promise<string> {
  return await page.locator("input[type='text'], textarea").first().inputValue();
}

async function yunePerfCount(page: Page): Promise<number> {
  return await page.evaluate(() => (JSON.parse(document.documentElement.dataset.yunePerfDiagnostics ?? "[]") as unknown[]).length);
}

async function latestYunePerf(page: Page): Promise<{
  totalKeydownToPaintMs?: number;
  workerProcessMs?: number;
  workerRoundtripMs?: number;
  firstCandidateText?: string;
  wasmHeapBytes?: number;
  peakWasmHeapBytes?: number;
} | undefined> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yunePerfDiagnostics ?? "[]") as Array<{
      totalKeydownToPaintMs?: number;
      workerProcessMs?: number;
      workerRoundtripMs?: number;
      firstCandidateText?: string;
      wasmHeapBytes?: number;
      peakWasmHeapBytes?: number;
    }>;
    return diagnostics.at(-1);
  });
}

async function firstYuneCandidateText(page: Page): Promise<string | undefined> {
  return await page.locator(".candidate-panel .candidates tbody").first()
    .getAttribute("data-candidate-text")
    .then(value => value ?? undefined)
    .catch(() => undefined);
}

async function firstMyRimeCandidateText(page: Page): Promise<string | undefined> {
  return await page.evaluate(() => {
    const lines = document.body.innerText.split(/\n+/).map(line => line.trim()).filter(Boolean);
    for (const line of lines) {
      const match = line.match(/^(?:\d+\s+)(\S+)/);
      if (match?.[1]) {
        return match[1];
      }
    }
    return lines.find(line => line.includes("\u4f60"))?.trim();
  });
}

async function waitForMyRimeCandidate(page: Page, input: string): Promise<void> {
  try {
    await page.waitForFunction(
      expectedInput => {
        const body = document.body.innerText;
        return body.includes(String(expectedInput))
          && /(?:^|\n)\s*1\s+\S+/.test(body);
      },
      input,
      { timeout: 30_000 },
    );
  } catch (error) {
    const snapshot = await page.evaluate(() => ({
      activeTag: document.activeElement?.tagName,
      activeClass: document.activeElement?.getAttribute("class"),
      activeValue: document.activeElement instanceof HTMLInputElement || document.activeElement instanceof HTMLTextAreaElement
        ? document.activeElement.value
        : document.activeElement?.textContent,
      bodyTail: document.body.innerText.slice(-1200),
      editables: [...document.querySelectorAll("textarea, input[type='text'], [contenteditable='true']")].map(element => ({
        tag: element.tagName,
        className: element.getAttribute("class"),
        disabled: (element as HTMLInputElement | HTMLTextAreaElement).disabled ?? false,
        value: element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement ? element.value : element.textContent,
      })),
    }));
    throw new Error(`Timed out waiting for My RIME candidate for input ${input}: ${JSON.stringify(snapshot)}; ${String(error)}`);
  }
}

async function yuneStartupMarker(page: Page): Promise<{
  wasmMemory?: WasmMemorySnapshot;
  wasmGlue?: string;
  wasmBinary?: string;
  loadedExplicitAssets?: string[];
  loadedSharedAssets?: string[];
} | undefined> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]") as Array<{
      source?: string;
      marker?: {
        phase?: string;
        wasmMemory?: { currentBytes: number; peakBytes: number };
        wasmGlue?: string;
        wasmBinary?: string;
        loadedExplicitAssets?: string[];
        loadedSharedAssets?: string[];
      };
    }>;
    return diagnostics
      .slice()
      .reverse()
      .find(entry => entry.source === "yune-startup" && entry.marker?.phase === "startup:complete")
      ?.marker;
  });
}

function yuneWasmFromPerf(perf: { wasmHeapBytes?: number; peakWasmHeapBytes?: number } | undefined): WasmMemorySnapshot | undefined {
  if (perf?.wasmHeapBytes === undefined && perf?.peakWasmHeapBytes === undefined) {
    return undefined;
  }
  return {
    currentBytes: perf.wasmHeapBytes ?? perf.peakWasmHeapBytes ?? 0,
    peakBytes: perf.peakWasmHeapBytes ?? perf.wasmHeapBytes ?? 0,
  };
}

async function myRimeWorkerMemory(page: Page): Promise<ComparatorWorkerMemory | undefined> {
  const workers = page.workers();
  for (const worker of workers.slice().reverse()) {
    const value = await worker.evaluate(() => {
      const scope = globalThis as typeof globalThis & {
        Module?: { HEAPU8?: Uint8Array; wasmMemory?: WebAssembly.Memory };
        HEAPU8?: Uint8Array;
        wasmMemory?: WebAssembly.Memory;
      };
      const moduleHeapBytes = scope.Module?.HEAPU8?.byteLength ?? scope.Module?.HEAPU8?.buffer.byteLength;
      const globalHeapBytes = scope.HEAPU8?.byteLength ?? scope.HEAPU8?.buffer.byteLength;
      const wasmMemoryBytes = scope.Module?.wasmMemory?.buffer.byteLength ?? scope.wasmMemory?.buffer.byteLength;
      const heapBytes = moduleHeapBytes ?? globalHeapBytes ?? wasmMemoryBytes;
      return {
        heapBytes,
        moduleHeapBytes,
        globalHeapBytes,
        wasmMemoryBytes,
        exportedKeys: Object.keys(scope.Module ?? {}).slice(0, 20),
      };
    }).catch(() => undefined);
    if (value?.heapBytes) {
      return value;
    }
  }
  return undefined;
}

function workerMemorySnapshot(memory: ComparatorWorkerMemory | undefined): WasmMemorySnapshot | undefined {
  if (!memory?.heapBytes) {
    return undefined;
  }
  return {
    currentBytes: memory.heapBytes,
    peakBytes: memory.heapBytes,
  };
}

async function collectPageResources(page: Page): Promise<ComparatorResource[]> {
  return await page.evaluate(() =>
    performance.getEntriesByType("resource").map(entry => {
      const resource = entry as PerformanceResourceTiming;
      return {
        context: "page" as const,
        name: resource.name,
        initiatorType: resource.initiatorType,
        transferSize: resource.transferSize,
        encodedBodySize: resource.encodedBodySize,
        decodedBodySize: resource.decodedBodySize,
        duration: Math.round(resource.duration),
      };
    })
  );
}

async function collectWorkerResources(page: Page): Promise<ComparatorResource[]> {
  const resources: ComparatorResource[] = [];
  for (const worker of page.workers()) {
    const entries = await worker.evaluate(() =>
      performance.getEntriesByType("resource").map(entry => {
        const resource = entry as PerformanceResourceTiming;
        return {
          name: resource.name,
          initiatorType: resource.initiatorType,
          transferSize: resource.transferSize,
          encodedBodySize: resource.encodedBodySize,
          decodedBodySize: resource.decodedBodySize,
          duration: Math.round(resource.duration),
        };
      })
    ).catch(() => []);
    resources.push(...entries.map(entry => ({ ...entry, context: "worker" as const })));
  }
  return resources;
}

function appendYuneSyntheticResources(
  resources: ComparatorResource[],
  startup: {
    wasmGlue?: string;
    wasmBinary?: string;
    loadedExplicitAssets?: string[];
    loadedSharedAssets?: string[];
  } | undefined,
  distRoot: string,
  pageUrl: string,
): ComparatorResource[] {
  const existing = new Set(resources.map(resource => stripQuery(resource.name)));
  const names = new Set<string>();
  if (startup?.wasmGlue) {
    names.add(startup.wasmGlue);
  }
  if (startup?.wasmBinary) {
    names.add(startup.wasmBinary);
  }
  for (const asset of startup?.loadedExplicitAssets ?? []) {
    names.add(`schema/${asset}`);
  }
  for (const asset of startup?.loadedSharedAssets ?? []) {
    names.add(`schema/${asset}`);
  }
  return [
    ...resources,
    ...[...names].flatMap(name => {
      const url = new URL(name, pageUrl).toString();
      if (existing.has(stripQuery(url))) {
        return [];
      }
      const file = path.join(distRoot, ...name.split("/"));
      return [{
        context: "synthetic-worker" as const,
        name: url,
        initiatorType: "worker",
        transferSize: syntheticSize(file),
        encodedBodySize: syntheticSize(file),
        decodedBodySize: syntheticSize(file),
        duration: 0,
      }];
    }),
  ];
}

function syntheticSize(file: string): number {
  try {
    return Number(statSync(file).size);
  } catch {
    return 0;
  }
}

function stripQuery(name: string): string {
  try {
    const url = new URL(name);
    url.search = "";
    url.hash = "";
    return url.toString();
  } catch {
    return name.split("?")[0] ?? name;
  }
}

async function collectBrowserMemory(page: Page): Promise<Record<string, number>> {
  const values: Record<string, number> = {};
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Performance.enable");
  const metrics = await cdp.send("Performance.getMetrics");
  for (const metric of metrics.metrics) {
    if (["JSHeapUsedSize", "JSHeapTotalSize", "Nodes", "Documents", "LayoutCount", "RecalcStyleCount"].includes(metric.name)) {
      values[metric.name] = metric.value;
    }
  }
  const uaMemory = await page.evaluate(async () => {
    const performanceWithMemory = performance as Performance & {
      measureUserAgentSpecificMemory?: () => Promise<{ bytes: number }>;
      memory?: { usedJSHeapSize?: number; totalJSHeapSize?: number; jsHeapSizeLimit?: number };
    };
    try {
      if (performanceWithMemory.measureUserAgentSpecificMemory) {
        return { userAgentSpecificMemoryBytes: (await performanceWithMemory.measureUserAgentSpecificMemory()).bytes };
      }
    } catch {
      return {};
    }
    return {
      usedJSHeapSize: performanceWithMemory.memory?.usedJSHeapSize,
      totalJSHeapSize: performanceWithMemory.memory?.totalJSHeapSize,
      jsHeapSizeLimit: performanceWithMemory.memory?.jsHeapSizeLimit,
    };
  });
  Object.assign(values, Object.fromEntries(
    Object.entries(uaMemory).filter((entry): entry is [string, number] => typeof entry[1] === "number"),
  ));
  return values;
}

async function storageEstimate(page: Page): Promise<{ usage?: number; quota?: number } | undefined> {
  return await page.evaluate(async () => {
    if (!navigator.storage?.estimate) {
      return undefined;
    }
    return navigator.storage.estimate();
  });
}

function captureConsoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", msg => {
    if (msg.type() === "error" || msg.type() === "warning") {
      errors.push(`console:${msg.type()} ${msg.text()}`);
    }
  });
  page.on("pageerror", error => {
    errors.push(`pageerror: ${error.message}`);
  });
  page.on("response", response => {
    if (response.status() >= 400) {
      errors.push(`response:${response.status()} ${response.url()}`);
    }
  });
  return errors;
}

function myRimeScenarioUrl(scenario: ComparatorScenario, sampleIndex: number): string {
  const url = new URL(myRimeUrl);
  url.searchParams.set("schemaId", scenario.schema === "jyutping" ? "jyut6ping3" : "luna_pinyin");
  if (scenario.schema === "jyutping") {
    url.searchParams.set("variantName", "\u6e2f");
  }
  url.searchParams.set("codexBaseline", "1");
  url.searchParams.set("sample", String(sampleIndex));
  return url.toString();
}

async function startStaticServer(root: string): Promise<{ url: string; close: () => Promise<void> }> {
  const server = createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
      const rawPath = decodeURIComponent(requestUrl.pathname === "/" ? "/index.html" : requestUrl.pathname);
      const relative = rawPath.replace(/^\/+/, "");
      const file = path.resolve(root, relative);
      if (!file.startsWith(path.resolve(root))) {
        response.writeHead(403);
        response.end("Forbidden");
        return;
      }
      const fileStat = await stat(file);
      if (!fileStat.isFile()) {
        response.writeHead(404);
        response.end("Not found");
        return;
      }
      response.setHeader("Content-Type", contentType(file));
      response.setHeader("Content-Length", fileStat.size);
      response.setHeader("Cache-Control", cacheControl(file));
      response.end(await readFile(file));
    } catch {
      response.writeHead(404);
      response.end("Not found");
    }
  });
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve());
  });
  const address = server.address();
  if (typeof address !== "object" || address === null) {
    throw new Error("Static server did not expose a TCP address");
  }
  return {
    url: `http://127.0.0.1:${address.port}`,
    close: () => closeServer(server),
  };
}

async function closeServer(server: Server): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.close(error => error ? reject(error) : resolve());
  });
}

function contentType(file: string): string {
  const ext = path.extname(file).toLowerCase();
  switch (ext) {
    case ".html": return "text/html; charset=utf-8";
    case ".js": return "application/javascript; charset=utf-8";
    case ".css": return "text/css; charset=utf-8";
    case ".wasm": return "application/wasm";
    case ".json": return "application/json; charset=utf-8";
    case ".yaml":
    case ".yml":
    case ".txt":
    case ".md": return "text/plain; charset=utf-8";
    default: return "application/octet-stream";
  }
}

function cacheControl(file: string): string {
  if (/index\.html$/i.test(file)) {
    return "no-cache";
  }
  return "public, max-age=31536000, immutable";
}

async function assertDistExists(dir: string, label: string): Promise<void> {
  try {
    const file = path.join(dir, "index.html");
    const fileStat = await stat(file);
    if (fileStat.isFile()) {
      return;
    }
  } catch {
    // Report below.
  }
  throw new Error(`Missing ${label} at ${dir}. Run the yune-web production build commands first.`);
}
