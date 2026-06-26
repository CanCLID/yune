import { test, expect, chromium, type BrowserContext, type Page } from "@playwright/test";

import { execFile } from "node:child_process";
import { createServer, type Server } from "node:http";
import { mkdir, readFile, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  markerDurations,
  writeEvidence,
  type FirstKeyToPaintSample,
  type StartupResource,
  type StartupSample,
} from "./startup-benchmark/metrics";
import {
  appSchemaId,
  scenarioSamples,
  startupScenarios,
  type StartupScenario,
} from "./startup-benchmark/scenarios";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(__dirname, "..");
const resultRoot = path.join(__dirname, "results", "m41-yune-web-startup-optimization");
const phaseName = process.env.M41_PHASE ?? "phase-0-baseline";
const phaseDir = path.join(resultRoot, phaseName);
const trackedDist = path.join(appRoot, "dist");
const publicDist = path.join(appRoot, "public-demo", "dist");
const readyTimeoutMs = 120_000;

test.describe("M41 STARTUP benchmark", () => {
  test.setTimeout(60 * 60 * 1000);

  test("M41 STARTUP production harness baseline", async () => {
    await assertDistExists(trackedDist, "tracked apps/yune-web dist");
    await assertDistExists(publicDist, "public-demo dist");
    const trackedServer = await startStaticServer(trackedDist);
    const publicServer = await startStaticServer(publicDist);
    const samples: StartupSample[] = [];
    try {
      for (const scenario of startupScenarios) {
        const count = scenarioSamples(scenario);
        for (let index = 0; index < count; index += 1) {
          samples.push(await runScenarioSample(
            scenario,
            index,
            scenario.publicDemo ? publicServer.url : trackedServer.url,
            scenario.publicDemo ? publicDist : trackedDist,
          ));
        }
      }
    } finally {
      await trackedServer.close();
      await publicServer.close();
    }
    await writeEvidence(phaseDir, samples);
    expect(samples.length).toBeGreaterThan(0);
  });
});

async function runScenarioSample(
  scenario: StartupScenario,
  sampleIndex: number,
  baseUrl: string,
  distRoot: string,
): Promise<StartupSample> {
  const userDataDir = path.join(
    os.tmpdir(),
    `yune-m41-${process.pid}-${scenario.id}-${sampleIndex}-${Date.now()}`,
  );
  await rm(userDataDir, { recursive: true, force: true });
  await mkdir(userDataDir, { recursive: true });
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
    }, { schema: appSchemaId(scenario.schema) });
    if (scenario.mode.startsWith("mock-worker")) {
      await installMockWorker(context);
    }
    const url = `${baseUrl}/?m41Schema=${encodeURIComponent(scenario.schema)}&m41Scenario=${encodeURIComponent(scenario.id)}`;
    const page = await context.newPage();
    if (scenario.mode === "real-worker-warm-reload" || scenario.mode === "mock-worker-warm") {
      await loadAndWaitReady(page, url, scenario);
      await page.reload({ waitUntil: "domcontentloaded" });
    } else if (scenario.mode === "real-worker-warm-new-page") {
      await loadAndWaitReady(page, url, scenario);
      await page.close();
      const next = await context.newPage();
      return await collectReadySample(next, url, scenario, sampleIndex, userDataDir, distRoot);
    }
    return await collectReadySample(page, url, scenario, sampleIndex, userDataDir, distRoot);
  } finally {
    await context.close();
    await rm(userDataDir, { recursive: true, force: true });
  }
}

async function collectReadySample(
  page: Page,
  url: string,
  scenario: StartupScenario,
  sampleIndex: number,
  userDataDir: string,
  distRoot: string,
): Promise<StartupSample> {
  const startedAt = Date.now();
  await loadAndWaitReady(page, url, scenario);
  const readyAt = Date.now();
  const startupActions = await collectActionDiagnostics(page);
  const firstKeyToPaint = await collectFirstKeyToPaint(page, scenario.inputs);
  const startupDiagnostic = await startupMarker(page);
  const resources = [
    ...await collectResources(page),
    ...await collectWorkerResources(startupDiagnostic, distRoot, url, scenario),
  ];
  const memory = await collectBrowserMemory(page, userDataDir);
  const navigation = await page.evaluate(() => {
    const nav = performance.getEntriesByType("navigation")[0] as PerformanceNavigationTiming | undefined;
    if (!nav) {
      return {};
    }
    return {
      domContentLoaded: Math.round(nav.domContentLoadedEventEnd),
      load: Math.round(nav.loadEventEnd),
      responseEnd: Math.round(nav.responseEnd),
      requestStart: Math.round(nav.requestStart),
      transfer: Math.round(nav.responseEnd - nav.requestStart),
    };
  });
  const cache = {
    hits: Number(startupDiagnostic?.assetCache?.hits ?? 0),
    misses: Number(startupDiagnostic?.assetCache?.misses ?? 0),
    errors: await cacheErrors(page),
  };
  const storageEstimate = await page.evaluate(async () => {
    if (!navigator.storage?.estimate) {
      return undefined;
    }
    return navigator.storage.estimate();
  });
  return {
    scenarioId: scenario.id,
    sampleIndex,
    url,
    schema: scenario.schema,
    appSchema: appSchemaId(scenario.schema),
    mode: scenario.mode,
    publicDemo: scenario.publicDemo,
    readyToInputMs: readyAt - startedAt,
    startupCompleteMs: typeof startupDiagnostic?.totalMs === "number" ? startupDiagnostic.totalMs : undefined,
    phases: markerDurations(startupDiagnostic?.markers),
    navigation,
    resources,
    cache,
    storageEstimate,
    browserMemory: memory,
    startupActions,
    firstKeyToPaint,
  };
}

async function loadAndWaitReady(page: Page, url: string, scenario: StartupScenario): Promise<void> {
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    ({ schema, appSchema }) => {
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
      const expectedActive = schema === "jyut6ping3_mobile"
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
    { schema: scenario.schema, appSchema: appSchemaId(scenario.schema) },
    { timeout: readyTimeoutMs },
  );
}

async function collectFirstKeyToPaint(page: Page, inputs: string[]): Promise<FirstKeyToPaintSample[]> {
  const results: FirstKeyToPaintSample[] = [];
  const inputField = page.locator("input[type='text'], textarea").first();
  for (const input of inputs) {
    await page.keyboard.press("Escape").catch(() => undefined);
    await inputField.fill("");
    const beforeCount = await page.evaluate(() => {
      const raw = document.documentElement.dataset.yunePerfDiagnostics ?? "[]";
      return (JSON.parse(raw) as unknown[]).length;
    });
    await inputField.focus();
    await inputField.pressSequentially(input, { delay: 0 });
    const diagnostic = await page.waitForFunction(
      ({ expectedInput, minCount }) => {
        const raw = document.documentElement.dataset.yunePerfDiagnostics ?? "[]";
        const diagnostics = JSON.parse(raw) as Array<{
          input?: string;
          totalKeydownToPaintMs?: number;
          candidateCount?: number;
          totalCandidateCount?: number;
          workerQueueWaitMs?: number;
          workerProcessMs?: number;
          workerRoundtripMs?: number;
          reactUpdateMs?: number;
          paintProxyMs?: number;
          firstCandidateText?: string;
        }>;
        const newDiagnostics = diagnostics.slice(minCount);
        return newDiagnostics.slice().reverse().find(entry => entry.input === expectedInput)
          ?? newDiagnostics.at(-1)
          ?? null;
      },
      { expectedInput: input, minCount: beforeCount },
      { timeout: 30_000 },
    );
    const value = await diagnostic.jsonValue() as {
      input?: string;
      totalKeydownToPaintMs?: number;
      candidateCount?: number;
      totalCandidateCount?: number;
      workerQueueWaitMs?: number;
      workerProcessMs?: number;
      workerRoundtripMs?: number;
      reactUpdateMs?: number;
      paintProxyMs?: number;
      firstCandidateText?: string;
    } | null;
    results.push({
      input,
      ms: Number(value?.totalKeydownToPaintMs ?? 0),
      candidateCount: value?.candidateCount,
      totalCandidateCount: value?.totalCandidateCount,
      workerQueueWaitMs: value?.workerQueueWaitMs,
      workerProcessMs: value?.workerProcessMs,
      workerRoundtripMs: value?.workerRoundtripMs,
      reactUpdateMs: value?.reactUpdateMs,
      paintProxyMs: value?.paintProxyMs,
      firstCandidateText: value?.firstCandidateText,
    });
  }
  return results;
}

async function startupMarker(page: Page): Promise<{
  phase?: string;
  totalMs?: number;
  markers?: Array<{ phase: string; ms: number }>;
  assetCache?: { hits?: number; misses?: number };
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
        totalMs?: number;
        markers?: Array<{ phase: string; ms: number }>;
        assetCache?: { hits?: number; misses?: number };
      };
    }>;
    return diagnostics
      .slice()
      .reverse()
      .find(entry => entry.source === "yune-startup" && entry.marker?.phase === "startup:complete")
      ?.marker;
  });
}

async function collectResources(page: Page): Promise<StartupResource[]> {
  return await page.evaluate(() =>
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
  );
}

async function collectWorkerResources(
  startupDiagnostic: {
    wasmGlue?: string;
    wasmBinary?: string;
    loadedExplicitAssets?: string[];
    loadedSharedAssets?: string[];
  } | undefined,
  distRoot: string,
  pageUrl: string,
  scenario: StartupScenario,
): Promise<StartupResource[]> {
  const names = new Set<string>();
  if (startupDiagnostic?.wasmGlue) {
    names.add(startupDiagnostic.wasmGlue);
  }
  if (startupDiagnostic?.wasmBinary) {
    names.add(startupDiagnostic.wasmBinary);
  }
  for (const asset of startupDiagnostic?.loadedExplicitAssets ?? []) {
    names.add(`schema/${asset}`);
  }
  for (const asset of startupDiagnostic?.loadedSharedAssets ?? []) {
    names.add(`schema/${asset}`);
  }
  const resources: StartupResource[] = [];
  for (const name of names) {
    const file = path.join(distRoot, ...name.split("/"));
    try {
      const fileStat = await stat(file);
      if (!fileStat.isFile()) {
        continue;
      }
      resources.push({
        name: new URL(name, pageUrl).toString(),
        initiatorType: "worker",
        transferSize: scenario.mode.includes("warm") ? 0 : fileStat.size,
        encodedBodySize: fileStat.size,
        decodedBodySize: fileStat.size,
        duration: 0,
      });
    } catch {
      // Missing optional runtime assets are surfaced by worker startup itself.
    }
  }
  return resources;
}

async function collectBrowserMemory(page: Page, userDataDir: string): Promise<Record<string, number>> {
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Performance.enable");
  const metrics = await cdp.send("Performance.getMetrics");
  const values: Record<string, number> = {};
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
  values["windowsWorkingSetBytes"] = await windowsWorkingSetForUserDataDir(userDataDir);
  return values;
}

async function collectActionDiagnostics(page: Page): Promise<Array<{
  action: string;
  input?: string;
  queueWaitMs?: number;
  workerRoundtripMs?: number;
  workerMs?: number;
  totalMs?: number;
}>> {
  return await page.evaluate(() => {
    const raw = document.documentElement.dataset.yuneActionDiagnostics ?? "[]";
    return (JSON.parse(raw) as Array<{
      action?: string;
      input?: string;
      queueWaitMs?: number;
      workerRoundtripMs?: number;
      workerMs?: number;
      totalMs?: number;
    }>).map(action => ({
      action: action.action ?? "unknown",
      input: action.input,
      queueWaitMs: action.queueWaitMs,
      workerRoundtripMs: action.workerRoundtripMs,
      workerMs: action.workerMs,
      totalMs: action.totalMs,
    }));
  });
}

async function cacheErrors(page: Page): Promise<string[]> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]") as Array<{
      source?: string;
      marker?: { phase?: string; error?: string; path?: string };
    }>;
    return diagnostics
      .filter(entry => entry.marker?.phase === "asset-cache:error")
      .map(entry => `${entry.marker?.path ?? "unknown"}: ${entry.marker?.error ?? "unknown error"}`);
  });
}

async function windowsWorkingSetForUserDataDir(userDataDir: string): Promise<number> {
  if (process.platform !== "win32") {
    return 0;
  }
  const literalUserDataDir = userDataDir.replace(/'/g, "''");
  const script = [
    `$dir = '${literalUserDataDir}'`,
    "$leaf = Split-Path -Leaf $dir",
    "$all = @(Get-CimInstance Win32_Process)",
    "$roots = @($all | Where-Object { $_.Name -like 'chrome*' -and $_.CommandLine -like \"*$leaf*\" })",
    "$ids = @{}",
    "$roots | ForEach-Object { $ids[[int]$_.ProcessId] = $true }",
    "$changed = $true",
    "while ($changed) { $changed = $false; foreach ($item in $all) { if ($ids.ContainsKey([int]$item.ParentProcessId) -and -not $ids.ContainsKey([int]$item.ProcessId)) { $ids[[int]$item.ProcessId] = $true; $changed = $true } } }",
    "$items = @($all | Where-Object { $ids.ContainsKey([int]$_.ProcessId) })",
    "($items | Measure-Object -Property WorkingSetSize -Sum).Sum",
  ].join("; ");
  return await new Promise(resolve => {
    execFile("powershell.exe", ["-NoProfile", "-Command", script], { timeout: 10_000 }, (error, stdout) => {
      if (error) {
        resolve(0);
        return;
      }
      const parsed = Number(stdout.trim());
      resolve(Number.isFinite(parsed) ? parsed : 0);
    });
  });
}

async function installMockWorker(context: BrowserContext): Promise<void> {
  await context.route(/\/worker\.js(?:\?.*)?$/, route => route.fulfill({
    status: 200,
    contentType: "application/javascript",
    body: mockWorkerScript,
  }));
}

const mockWorkerScript = `
const startedAt = performance.now();
function postDiagnostic() {
  postMessage({
    type: "diagnostic",
    source: "yune-startup",
    marker: {
      phase: "startup:complete",
      totalMs: Math.round(performance.now() - startedAt),
      markers: [
        { phase: "worker:start", ms: 0 },
        { phase: "runtime:initialized", ms: Math.round(performance.now() - startedAt) }
      ],
      schema: "mock",
      assetCache: { hits: 0, misses: 0, unavailable: false },
      loadedSharedAssets: []
    }
  });
  postMessage({ type: "listener", name: "schemaChanged", args: ["luna_pinyin", "Luna Pinyin"] });
  postMessage({ type: "listener", name: "initialized", args: [true] });
}
setTimeout(postDiagnostic, 0);
self.onmessage = (event) => {
  const { name, args } = event.data;
  const started = performance.timeOrigin + performance.now();
  let result = true;
  if (name === "processKey") {
    const key = String(args?.[0] ?? "").replace(/[{}]/g, "").toLowerCase();
    result = {
      success: true,
      isComposing: true,
      inputBuffer: { before: "", active: key.length === 1 ? key : "mock", after: "" },
      page: 0,
      isLastPage: true,
      highlightedIndex: 0,
      candidates: []
    };
  } else if (name === "getUserdbSnapshot") {
    result = { schemaId: "luna_pinyin", dictionaryId: "mock", path: "/rime/mock.userdb", exists: false, bytes: 0, updatedAt: null, rows: [], rawText: "", parseErrors: [] };
  } else if (name === "selectSchema") {
    postMessage({ type: "listener", name: "schemaChanged", args: [args?.[0] ?? "luna_pinyin", String(args?.[0] ?? "Luna Pinyin")] });
    result = true;
  } else if (name === "stageAi" || name === "selectCandidate" || name === "deleteCandidate" || name === "flipPage") {
    result = { success: true, isComposing: false };
  }
  const finished = performance.timeOrigin + performance.now();
  postMessage({ type: "success", result, elapsedMs: Math.round(finished - started), workerStartedAt: started, workerFinishedAt: finished });
};
`;

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
  throw new Error(`Missing ${label} at ${dir}. Run the M41 build commands first.`);
}
