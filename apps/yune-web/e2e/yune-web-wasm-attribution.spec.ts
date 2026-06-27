import { test, expect, chromium, type Page } from "@playwright/test";

import { statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import { mkdir, readFile, rm, stat, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { median, percentile, type StartupResource, type WasmMemorySnapshot } from "./startup-benchmark/metrics";
import { appSchemaId, type StartupSchema } from "./startup-benchmark/scenarios";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(__dirname, "..");
const resultRoot = path.join(
  __dirname,
  "results",
  process.env.YUNE_WEB_WASM_ATTRIBUTION_RESULT_ROOT ??
    path.join("yune-web-wasm-heap-optimization", "attribution"),
);
const phaseName = process.env.YUNE_WEB_WASM_ATTRIBUTION_PHASE ?? "baseline-attribution";
const phaseDir = path.join(resultRoot, phaseName);
const trackedDist = path.join(appRoot, "dist");
const publicDist = path.join(appRoot, "public-demo", "dist");
const readyTimeoutMs = 120_000;

type AssetFamily =
  | "luna-core"
  | "jyutping-core"
  | "jyutping-scolar"
  | "reverse-lookup"
  | "opencc"
  | "extras"
  | "full-jyutping";

interface AttributionScenario {
  family: AssetFamily;
  schema: StartupSchema;
  input: string;
}

interface AttributionSample {
  family: AssetFamily;
  build: "tracked-dist" | "public-demo-dist";
  publicDemo: boolean;
  schema: StartupSchema;
  input: string;
  sampleIndex: number;
  url: string;
  initialized: boolean;
  readyToInputMs: number;
  inputToCandidateMs?: number;
  commitMs?: number;
  firstCandidateText?: string;
  committedValue?: string;
  startup?: StartupMarker;
  candidateMemory?: WasmMemorySnapshot;
  commitMemory?: WasmMemorySnapshot;
  browserMemory: Record<string, number>;
  storageEstimate?: { usage?: number; quota?: number };
  resources: AttributionResource[];
  workerUrls: string[];
  loadedSharedAssetBytes: number;
  requestedFamilyBytes: number;
  consoleErrors: string[];
}

interface StartupMarker {
  phase?: string;
  totalMs?: number;
  markers?: Array<{ phase: string; ms: number; wasmMemory?: WasmMemorySnapshot }>;
  wasmMemory?: WasmMemorySnapshot;
  wasmAttributionAssetFamily?: string;
  loadedExplicitAssets?: string[];
  loadedSharedAssets?: string[];
}

interface AttributionResource extends StartupResource {
  context: "page" | "worker" | "synthetic-worker";
  family: string;
}

const familyScenarios: AttributionScenario[] = [
  { family: "luna-core", schema: "luna_pinyin", input: "ni" },
  { family: "jyutping-core", schema: "jyut6ping3_mobile", input: "nei" },
  { family: "jyutping-scolar", schema: "jyut6ping3_mobile", input: "nei" },
  { family: "reverse-lookup", schema: "jyut6ping3_mobile", input: "nei" },
  { family: "opencc", schema: "jyut6ping3_mobile", input: "nei" },
  { family: "extras", schema: "jyut6ping3_mobile", input: "nei" },
  { family: "full-jyutping", schema: "jyut6ping3_mobile", input: "nei" },
];

const commonAssets = [
  "default.custom.yaml",
  "common.yaml",
  "common.custom.yaml",
  "include.yaml",
  "template.yaml",
] as const;
const openccAssets = [
  "opencc/t2hkf.json",
  "opencc/HKVariantsFull.txt",
  "opencc/t2s.json",
  "opencc/t2tw.json",
  "opencc/hk2s.json",
  "opencc/HKVariantsRev.ocd2",
  "opencc/HKVariantsRevPhrases.ocd2",
  "opencc/TSCharacters.ocd2",
  "opencc/TSPhrases.ocd2",
] as const;
const lunaOpenccAssets = [
  "opencc/t2hk.json",
  "opencc/t2s.json",
  "opencc/t2tw.json",
  "opencc/HKVariants.ocd2",
  "opencc/TSCharacters.ocd2",
  "opencc/TSPhrases.ocd2",
  "opencc/TWVariants.ocd2",
] as const;
const lunaCoreAssets = [
  "default.custom.yaml",
  "pinyin.yaml",
  "key_bindings.yaml",
  "punctuation.yaml",
  "symbols.yaml",
  "essay.txt",
  "luna_pinyin.schema.yaml",
  "luna_pinyin.dict.yaml",
  "luna_pinyin.table.bin",
  "luna_pinyin.reverse.bin",
  "luna_pinyin.prism.bin",
  "stroke.schema.yaml",
  "stroke.dict.yaml",
  "stroke.table.bin",
  "stroke.reverse.bin",
  "stroke.prism.bin",
  ...lunaOpenccAssets,
] as const;
const jyutpingCoreAssets = [
  ...commonAssets,
  "jyut6ping3.schema.yaml",
  "jyut6ping3_mobile.schema.yaml",
  "jyut6ping3.table.bin",
  "jyut6ping3.reverse.bin",
  "jyut6ping3_mobile.prism.bin",
] as const;
const jyutpingScolarAssets = [
  ...jyutpingCoreAssets,
  "jyut6ping3_scolar.schema.yaml",
  "jyut6ping3_scolar.dict.yaml",
  "jyut6ping3_scolar.table.bin",
  "jyut6ping3_scolar.reverse.bin",
  "jyut6ping3_scolar.prism.bin",
] as const;
const reverseLookupAssets = [
  ...jyutpingCoreAssets,
  "loengfan.schema.yaml",
  "loengfan.dict.yaml",
  "cangjie3.schema.yaml",
  "cangjie3.dict.yaml",
  "cangjie5.schema.yaml",
  "cangjie5.dict.yaml",
  "luna_pinyin_yune_reverse.dict.yaml",
] as const;
const openccAttributionAssets = [
  ...jyutpingCoreAssets,
  ...openccAssets,
] as const;
const fullJyutpingAssets = [
  ...commonAssets,
  "jyut6ping3.schema.yaml",
  "jyut6ping3_mobile.schema.yaml",
  "jyut6ping3_scolar.schema.yaml",
  "jyut6ping3_scolar.dict.yaml",
  "loengfan.schema.yaml",
  "loengfan.dict.yaml",
  "cangjie3.schema.yaml",
  "cangjie3.dict.yaml",
  "cangjie5.schema.yaml",
  "cangjie5.dict.yaml",
  "luna_pinyin_yune_reverse.dict.yaml",
  ...openccAssets,
  "jyut6ping3.table.bin",
  "jyut6ping3.reverse.bin",
  "jyut6ping3_mobile.prism.bin",
  "jyut6ping3_scolar.table.bin",
  "jyut6ping3_scolar.reverse.bin",
  "jyut6ping3_scolar.prism.bin",
] as const;

test.describe("YUNE WEB WASM ATTRIBUTION benchmark", () => {
  test.skip(process.env.YUNE_WEB_WASM_ATTRIBUTION !== "1", "Set YUNE_WEB_WASM_ATTRIBUTION=1 to run this opt-in benchmark.");
  test.setTimeout(60 * 60 * 1000);

  test("YUNE WEB WASM ATTRIBUTION asset-family baseline", async () => {
    await assertDistExists(trackedDist, "tracked apps/yune-web dist");
    await assertDistExists(publicDist, "public-demo dist");
    const trackedServer = await startStaticServer(trackedDist);
    const publicServer = await startStaticServer(publicDist);
    const samples: AttributionSample[] = [];
    try {
      for (const scenario of familyScenarios) {
        samples.push(await runAttributionSample(scenario, 0, "tracked-dist", trackedServer.url, trackedDist));
        samples.push(await runAttributionSample(scenario, 0, "public-demo-dist", publicServer.url, publicDist));
      }
    } finally {
      await trackedServer.close();
      await publicServer.close();
    }
    await writeAttributionEvidence(phaseDir, samples);
    expect(samples.some(sample => sample.family === "full-jyutping" && sample.initialized)).toBe(true);
    expect(samples.some(sample => sample.family === "luna-core" && sample.initialized)).toBe(true);
  });
});

async function runAttributionSample(
  scenario: AttributionScenario,
  sampleIndex: number,
  build: "tracked-dist" | "public-demo-dist",
  baseUrl: string,
  distRoot: string,
): Promise<AttributionSample> {
  const userDataDir = await freshUserDataDir(scenario, build, sampleIndex);
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
    const page = await context.newPage();
    const consoleErrors = captureConsoleErrors(page);
    const url = `${baseUrl}/?benchmark=wasm-attribution&schema=${encodeURIComponent(scenario.schema)}&wasmAttributionFamily=${encodeURIComponent(scenario.family)}&sample=${sampleIndex}`;
    const startedAt = Date.now();
    await page.goto(url, { waitUntil: "domcontentloaded" });
    await page.waitForFunction(
      () => {
        const initialized = document.documentElement.dataset.yuneInitialized;
        return initialized === "true" || initialized === "false";
      },
      undefined,
      { timeout: readyTimeoutMs },
    ).catch(() => undefined);
    const readyAt = Date.now();
    const initialized = await page.evaluate(() => document.documentElement.dataset.yuneInitialized === "true");
    const startup = await startupMarker(page);
    let candidateMemory: WasmMemorySnapshot | undefined;
    let commitMemory: WasmMemorySnapshot | undefined;
    let inputToCandidateMs: number | undefined;
    let commitMs: number | undefined;
    let firstCandidateText: string | undefined;
    let committedValue: string | undefined;
    if (initialized) {
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
      inputToCandidateMs = Date.now() - inputStartedAt;
      const candidatePerf = await latestYunePerf(page);
      candidateMemory = yuneWasmFromPerf(candidatePerf);
      firstCandidateText = candidatePerf?.firstCandidateText;
      const commitStartedAt = Date.now();
      await page.keyboard.press("Space");
      await expect.poll(async () => {
        const value = await input.inputValue();
        return value.length > 0 && value !== scenario.input ? value : "";
      }, { timeout: 30_000 }).not.toBe("");
      commitMs = Date.now() - commitStartedAt;
      committedValue = await input.inputValue();
      commitMemory = yuneWasmFromPerf(await latestYunePerf(page));
    }
    let resources = [
      ...await collectPageResources(page),
      ...await collectWorkerResources(page),
    ];
    resources = appendYuneSyntheticResources(resources, startup, distRoot, url);
    resources = resources.map(resource => ({ ...resource, family: assetFamilyForResource(resource.name) }));
    const loadedSharedAssets = startup?.loadedSharedAssets ?? [];
    return {
      family: scenario.family,
      build,
      publicDemo: build === "public-demo-dist",
      schema: scenario.schema,
      input: scenario.input,
      sampleIndex,
      url,
      initialized,
      readyToInputMs: readyAt - startedAt,
      inputToCandidateMs,
      commitMs,
      firstCandidateText,
      committedValue,
      startup,
      candidateMemory,
      commitMemory,
      browserMemory: await collectBrowserMemory(page),
      storageEstimate: await storageEstimate(page),
      resources,
      workerUrls: page.workers().map(worker => worker.url()),
      loadedSharedAssetBytes: loadedSharedAssets.reduce((sum, asset) => sum + syntheticSize(path.join(distRoot, "schema", ...asset.split("/"))), 0),
      requestedFamilyBytes: requestedFamilyPaths(scenario.family).reduce((sum, asset) => sum + syntheticSize(path.join(distRoot, "schema", ...asset.split("/"))), 0),
      consoleErrors,
    };
  } finally {
    await context.close();
    await rm(userDataDir, { recursive: true, force: true });
  }
}

async function writeAttributionEvidence(outputDir: string, samples: AttributionSample[]): Promise<void> {
  await mkdir(outputDir, { recursive: true });
  await writeFile(path.join(outputDir, "samples.json"), `${JSON.stringify(samples, null, 2)}\n`);
  await writeFile(path.join(outputDir, "samples.csv"), sampleCsv(samples));
  const summary = summarizeAttribution(samples);
  await writeFile(path.join(outputDir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
  await writeFile(path.join(outputDir, "summary.csv"), summaryCsv(summary));
  await writeFile(path.join(outputDir, "asset-family-summary.csv"), assetFamilyCsv(samples));
  await writeFile(path.join(outputDir, "report.md"), reportMarkdown(summary, samples));
}

function summarizeAttribution(samples: AttributionSample[]) {
  const groups = new Map<string, AttributionSample[]>();
  for (const sample of samples) {
    const key = `${sample.build}:${sample.family}`;
    const list = groups.get(key) ?? [];
    list.push(sample);
    groups.set(key, list);
  }
  return [...groups.values()].map(group => {
    const first = group[0];
    return {
      build: first?.build ?? "tracked-dist",
      publicDemo: first?.publicDemo ?? false,
      family: first?.family ?? "full-jyutping",
      schema: first?.schema ?? "jyut6ping3_mobile",
      samples: group.length,
      initialized: group.filter(sample => sample.initialized).length,
      medianReadyToInputMs: Math.round(median(group.map(sample => sample.readyToInputMs))),
      p95ReadyToInputMs: Math.round(percentile(group.map(sample => sample.readyToInputMs), 0.95)),
      medianInputToCandidateMs: Math.round(median(group.map(sample => sample.inputToCandidateMs ?? 0))),
      medianCommitMs: Math.round(median(group.map(sample => sample.commitMs ?? 0))),
      medianWasmReadyBytes: Math.round(median(group.map(sample => sample.startup?.wasmMemory?.currentBytes ?? 0))),
      medianWasmPeakBytes: Math.round(median(group.map(sample => observedPeakBytes(sample)))),
      maxWasmPeakBytes: Math.max(0, ...group.map(observedPeakBytes)),
      medianSteadyStateBytes: Math.round(median(group.map(sample => sample.commitMemory?.currentBytes ?? sample.candidateMemory?.currentBytes ?? sample.startup?.wasmMemory?.currentBytes ?? 0))),
      medianLoadedSharedAssetBytes: Math.round(median(group.map(sample => sample.loadedSharedAssetBytes))),
      medianRequestedFamilyBytes: Math.round(median(group.map(sample => sample.requestedFamilyBytes))),
      medianUniqueEncodedResourceBytes: Math.round(median(group.map(sample => uniqueEncodedBytes(sample.resources)))),
      medianJSHeapUsedBytes: Math.round(median(group.map(sample => sample.browserMemory.JSHeapUsedSize ?? sample.browserMemory.usedJSHeapSize ?? 0))),
      medianStorageUsageBytes: Math.round(median(group.map(sample => sample.storageEstimate?.usage ?? 0))),
      loadedAssetCount: Math.round(median(group.map(sample => sample.startup?.loadedSharedAssets?.length ?? 0))),
      loadedAssets: [...new Set(group.flatMap(sample => sample.startup?.loadedSharedAssets ?? []))],
      consoleErrors: [...new Set(group.flatMap(sample => sample.consoleErrors))],
    };
  });
}

function observedPeakBytes(sample: AttributionSample): number {
  return Math.max(
    0,
    sample.startup?.wasmMemory?.peakBytes ?? 0,
    sample.startup?.wasmMemory?.currentBytes ?? 0,
    sample.candidateMemory?.peakBytes ?? 0,
    sample.candidateMemory?.currentBytes ?? 0,
    sample.commitMemory?.peakBytes ?? 0,
    sample.commitMemory?.currentBytes ?? 0,
  );
}

function resourceTransferBytes(resources: AttributionResource[]): number {
  return resources.reduce((sum, resource) => sum + resource.transferSize, 0);
}

function uniqueEncodedBytes(resources: AttributionResource[]): number {
  return [...uniqueResources(resources).values()]
    .reduce((sum, resource) => sum + resource.encodedBodySize, 0);
}

function uniqueResources(resources: AttributionResource[]): Map<string, AttributionResource> {
  const unique = new Map<string, AttributionResource>();
  for (const resource of resources) {
    const key = normalizedResourceName(resource.name);
    const existing = unique.get(key);
    if (!existing || resource.encodedBodySize > existing.encodedBodySize) {
      unique.set(key, resource);
    }
  }
  return unique;
}

function sampleCsv(samples: AttributionSample[]): string {
  const header = [
    "build",
    "family",
    "schema",
    "initialized",
    "readyToInputMs",
    "inputToCandidateMs",
    "commitMs",
    "wasmReadyBytes",
    "wasmPeakBytes",
    "steadyStateBytes",
    "loadedSharedAssetBytes",
    "requestedFamilyBytes",
    "uniqueEncodedResourceBytes",
    "jsHeapUsedBytes",
    "storageUsageBytes",
    "loadedAssets",
    "consoleErrors",
  ];
  const rows = samples.map(sample => [
    sample.build,
    sample.family,
    sample.schema,
    sample.initialized,
    sample.readyToInputMs,
    sample.inputToCandidateMs ?? "",
    sample.commitMs ?? "",
    sample.startup?.wasmMemory?.currentBytes ?? "",
    observedPeakBytes(sample),
    sample.commitMemory?.currentBytes ?? sample.candidateMemory?.currentBytes ?? sample.startup?.wasmMemory?.currentBytes ?? "",
    sample.loadedSharedAssetBytes,
    sample.requestedFamilyBytes,
    uniqueEncodedBytes(sample.resources),
    sample.browserMemory.JSHeapUsedSize ?? sample.browserMemory.usedJSHeapSize ?? "",
    sample.storageEstimate?.usage ?? "",
    (sample.startup?.loadedSharedAssets ?? []).join(" "),
    sample.consoleErrors.join(" | "),
  ]);
  return [header, ...rows].map(row => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function summaryCsv(rows: ReturnType<typeof summarizeAttribution>): string {
  const header = Object.keys(rows[0] ?? {
    build: "",
    publicDemo: "",
    family: "",
    schema: "",
    samples: "",
    initialized: "",
    medianReadyToInputMs: "",
    p95ReadyToInputMs: "",
    medianInputToCandidateMs: "",
    medianCommitMs: "",
    medianWasmReadyBytes: "",
    medianWasmPeakBytes: "",
    maxWasmPeakBytes: "",
    medianSteadyStateBytes: "",
    medianLoadedSharedAssetBytes: "",
    medianRequestedFamilyBytes: "",
    medianUniqueEncodedResourceBytes: "",
    medianJSHeapUsedBytes: "",
    medianStorageUsageBytes: "",
    loadedAssetCount: "",
    loadedAssets: "",
    consoleErrors: "",
  });
  return [
    header,
    ...rows.map(row => header.map(key => {
      const value = (row as unknown as Record<string, unknown>)[key];
      return Array.isArray(value) ? JSON.stringify(value) : value;
    })),
  ].map(row => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function assetFamilyCsv(samples: AttributionSample[]): string {
  const header = ["build", "family", "assetFamily", "encodedBytes"];
  const rows = samples.flatMap(sample => {
    const byFamily = new Map<string, number>();
    for (const resource of uniqueResources(sample.resources).values()) {
      byFamily.set(resource.family, (byFamily.get(resource.family) ?? 0) + resource.encodedBodySize);
    }
    return [...byFamily.entries()].map(([family, bytes]) => [sample.build, sample.family, family, bytes]);
  });
  return [header, ...rows].map(row => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function reportMarkdown(summary: ReturnType<typeof summarizeAttribution>, samples: AttributionSample[]): string {
  const rows = summary
    .map(row => `| ${row.build} | ${row.family} | ${row.initialized}/${row.samples} | ${row.medianReadyToInputMs} | ${bytes(row.medianWasmReadyBytes)} | ${bytes(row.medianWasmPeakBytes)} | ${bytes(row.medianSteadyStateBytes)} | ${bytes(row.medianRequestedFamilyBytes)} | ${bytes(row.medianUniqueEncodedResourceBytes)} | ${bytes(row.medianJSHeapUsedBytes)} |`)
    .join("\n");
  const familyRows = assetFamilyRows(samples)
    .map(row => `| ${row.build} | ${row.family} | ${row.assetFamily} | ${bytes(row.encodedBytes)} |`)
    .join("\n");
  return `# Yune Web WASM Attribution

| Build | Family | Initialized | Ready ms | WASM ready | WASM peak | Steady state | Requested family bytes | Unique encoded resources | JS heap used |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
${rows}

## Asset Family Resource Bytes

| Build | Scenario family | Asset family | Encoded bytes |
| --- | --- | --- | ---: |
${familyRows}
`;
}

function assetFamilyRows(samples: AttributionSample[]) {
  return samples.flatMap(sample => {
    const byFamily = new Map<string, number>();
    for (const resource of uniqueResources(sample.resources).values()) {
      byFamily.set(resource.family, (byFamily.get(resource.family) ?? 0) + resource.encodedBodySize);
    }
    return [...byFamily.entries()].map(([assetFamily, encodedBytes]) => ({
      build: sample.build,
      family: sample.family,
      assetFamily,
      encodedBytes,
    }));
  });
}

async function freshUserDataDir(scenario: AttributionScenario, build: string, sampleIndex: number): Promise<string> {
  const dir = path.join(
    os.tmpdir(),
    `yune-web-attribution-${process.pid}-${build}-${scenario.family}-${sampleIndex}-${Date.now()}`,
  );
  await rm(dir, { recursive: true, force: true });
  await mkdir(dir, { recursive: true });
  return dir;
}

async function startupMarker(page: Page): Promise<StartupMarker | undefined> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]") as Array<{
      source?: string;
      marker?: StartupMarker;
    }>;
    return diagnostics
      .slice()
      .reverse()
      .find(entry => entry.source === "yune-startup" && entry.marker?.phase === "startup:complete")
      ?.marker;
  });
}

async function yunePerfCount(page: Page): Promise<number> {
  return await page.evaluate(() => (JSON.parse(document.documentElement.dataset.yunePerfDiagnostics ?? "[]") as unknown[]).length);
}

async function latestYunePerf(page: Page): Promise<{
  firstCandidateText?: string;
  wasmHeapBytes?: number;
  peakWasmHeapBytes?: number;
} | undefined> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yunePerfDiagnostics ?? "[]") as Array<{
      firstCandidateText?: string;
      wasmHeapBytes?: number;
      peakWasmHeapBytes?: number;
    }>;
    return diagnostics.at(-1);
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

async function collectPageResources(page: Page): Promise<AttributionResource[]> {
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
        family: "unknown",
      };
    })
  );
}

async function collectWorkerResources(page: Page): Promise<AttributionResource[]> {
  const resources: AttributionResource[] = [];
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
    resources.push(...entries.map(entry => ({ ...entry, context: "worker" as const, family: "unknown" })));
  }
  return resources;
}

function appendYuneSyntheticResources(
  resources: AttributionResource[],
  startup: StartupMarker | undefined,
  distRoot: string,
  pageUrl: string,
): AttributionResource[] {
  const existing = new Set(resources.map(resource => normalizedResourceName(resource.name)));
  const names = new Set<string>();
  for (const asset of startup?.loadedExplicitAssets ?? []) {
    names.add(`schema/${asset}`);
  }
  for (const asset of startup?.loadedSharedAssets ?? []) {
    names.add(`schema/${asset}`);
  }
  if (startup?.phase === "startup:complete") {
    names.add("yune-web.js");
    names.add("yune-web.wasm");
  }
  return [
    ...resources,
    ...[...names].flatMap(name => {
      const url = new URL(name, pageUrl).toString();
      if (existing.has(normalizedResourceName(url))) {
        return [];
      }
      const file = path.join(distRoot, ...name.split("/"));
      const encodedBodySize = syntheticSize(file);
      return [{
        context: "synthetic-worker" as const,
        name: url,
        initiatorType: "worker",
        transferSize: encodedBodySize,
        encodedBodySize,
        decodedBodySize: encodedBodySize,
        duration: 0,
        family: assetFamilyForResource(url),
      }];
    }),
  ];
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

function requestedFamilyPaths(family: AssetFamily): readonly string[] {
  switch (family) {
    case "luna-core":
      return unique(lunaCoreAssets);
    case "jyutping-core":
      return unique(jyutpingCoreAssets);
    case "jyutping-scolar":
      return unique(jyutpingScolarAssets);
    case "reverse-lookup":
      return unique(reverseLookupAssets);
    case "opencc":
      return unique(openccAttributionAssets);
    case "extras":
      return [];
    case "full-jyutping":
      return unique(fullJyutpingAssets);
  }
}

function assetFamilyForResource(name: string): string {
  const pathName = normalizedResourcePath(name);
  if (pathName.endsWith("yune-web.wasm") || pathName.endsWith("yune-web.js")) {
    return "wasm-runtime";
  }
  if (pathName.includes("/opencc/")) {
    return "opencc";
  }
  if (/jyut6ping3_scolar\./.test(pathName)) {
    return "jyutping-scolar";
  }
  if (/loengfan\.|cangjie3\.|cangjie5\.|luna_pinyin_yune_reverse/.test(pathName)) {
    return "reverse-lookup";
  }
  if (/jyut6ping3|common\.|include\.|template\.|default\.custom/.test(pathName)) {
    return "jyutping-core";
  }
  if (/luna_pinyin|stroke\.|essay\.|pinyin\.|punctuation\.|symbols\.|key_bindings/.test(pathName)) {
    return "luna-core";
  }
  return "extras";
}

function normalizedResourcePath(name: string): string {
  try {
    const url = new URL(name);
    return url.pathname;
  } catch {
    return name.split("?")[0] ?? name;
  }
}

function normalizedResourceName(name: string): string {
  try {
    const url = new URL(name);
    url.search = "";
    url.hash = "";
    return url.toString();
  } catch {
    return name.split("?")[0] ?? name;
  }
}

function unique(values: readonly string[]): string[] {
  return [...new Set(values)];
}

function syntheticSize(file: string): number {
  try {
    return Number(statSync(file).size);
  } catch {
    return 0;
  }
}

function csvEscape(value: unknown): string {
  const text = String(value ?? "");
  return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function bytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0 B";
  }
  const units = ["B", "KiB", "MiB", "GiB"];
  let current = value;
  let index = 0;
  while (current >= 1024 && index < units.length - 1) {
    current /= 1024;
    index += 1;
  }
  return `${current.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
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
