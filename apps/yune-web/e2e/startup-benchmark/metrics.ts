import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

export interface StartupPhase {
  phase: string;
  ms: number;
}

export interface WasmMemorySnapshot {
  currentBytes: number;
  peakBytes: number;
}

export interface StartupWasmMemoryMarker extends StartupPhase {
  wasmMemory: WasmMemorySnapshot;
}

export interface StartupResource {
  name: string;
  initiatorType: string;
  transferSize: number;
  encodedBodySize: number;
  decodedBodySize: number;
  duration: number;
}

export interface FirstKeyToPaintSample {
  input: string;
  ms: number;
  candidateCount?: number;
  totalCandidateCount?: number;
  workerQueueWaitMs?: number;
  workerProcessMs?: number;
  workerRoundtripMs?: number;
  reactUpdateMs?: number;
  paintProxyMs?: number;
  firstCandidateText?: string;
  wasmHeapBytes?: number;
  peakWasmHeapBytes?: number;
}

export interface StartupActionDiagnostic {
  action: string;
  input?: string;
  queueWaitMs?: number;
  workerRoundtripMs?: number;
  workerMs?: number;
  totalMs?: number;
}

export interface StartupSample {
  scenarioId: string;
  sampleIndex: number;
  url: string;
  schema: string;
  appSchema: string;
  mode: string;
  publicDemo: boolean;
  readyToInputMs: number;
  startupCompleteMs?: number;
  phases: StartupPhase[];
  navigation: Record<string, number>;
  resources: StartupResource[];
  cache: { hits: number; misses: number; errors: string[] };
  storageEstimate?: { usage?: number; quota?: number };
  browserMemory?: Record<string, number>;
  wasmMemory?: WasmMemorySnapshot;
  wasmMemoryMarkers: StartupWasmMemoryMarker[];
  startupActions: StartupActionDiagnostic[];
  firstKeyToPaint: FirstKeyToPaintSample[];
}

interface SummaryRow {
  scenarioId: string;
  schema: string;
  mode: string;
  publicDemo: boolean;
  samples: number;
  medianReadyToInputMs: number;
  p95ReadyToInputMs: number;
  medianStartupCompleteMs: number;
  p95StartupCompleteMs: number;
  medianFirstKeyToPaintMs: number;
  p95FirstKeyToPaintMs: number;
  transferBytes: number;
  encodedBytes: number;
  medianWasmHeapBytes: number;
  p95WasmHeapBytes: number;
  medianPeakWasmHeapBytes: number;
  p95PeakWasmHeapBytes: number;
  maxPeakWasmHeapBytes: number;
  cacheHits: number;
  cacheMisses: number;
  cacheErrors: number;
  topOwner: string;
  topOwnerMedianMs: number;
}

export function median(values: number[]): number {
  return percentile(values, 0.5);
}

export function percentile(values: number[], p: number): number {
  const clean = values.filter(Number.isFinite).slice().sort((left, right) => left - right);
  if (clean.length === 0) {
    return 0;
  }
  const index = Math.min(clean.length - 1, Math.max(0, Math.ceil(clean.length * p) - 1));
  return clean[index];
}

export function markerDurations(markers: StartupPhase[] | undefined): StartupPhase[] {
  if (!markers || markers.length === 0) {
    return [];
  }
  const byPhase = new Map(markers.map(marker => [marker.phase, marker.ms]));
  const spans: Array<[string, string, string]> = [
    ["worker:start", "wasm-glue:loaded", "worker script/glue load"],
    ["wasm:module:create:start", "wasm:module:create:finish", "wasm instantiate/runtime create"],
    ["filesystem:mount:start", "filesystem:mount:finish", "filesystem mount"],
    ["assets:load:start", "assets:load:finish", "schema/shared asset load"],
    ["schema:select:start", "schema:select:finish", "schema select/init"],
  ];
  const result = spans.flatMap(([start, finish, phase]) => {
    const startedAt = byPhase.get(start);
    const finishedAt = byPhase.get(finish);
    return startedAt === undefined || finishedAt === undefined
      ? []
      : [{ phase, ms: Math.max(0, finishedAt - startedAt) }];
  });
  const total = markers.find(marker => marker.phase === "runtime:initialized")?.ms;
  if (total !== undefined) {
    result.push({ phase: "worker total to initialized", ms: total });
  }
  return result;
}

export function resourceGroup(name: string): string {
  const normalized = name.split("?")[0] ?? name;
  if (/\/assets\/.*\.js$/i.test(normalized)) return "app js";
  if (/\/assets\/.*\.css$/i.test(normalized)) return "app css";
  if (/\/worker\.js$/i.test(normalized)) return "worker script";
  if (/\/yune-(?:web|typeduck)\.js$/i.test(normalized)) return "wasm glue";
  if (/\/yune-(?:web|typeduck)\.wasm$/i.test(normalized)) return "wasm binary";
  if (/\/schema\/opencc\//i.test(normalized)) return "opencc";
  if (/\/schema\/.*\.(?:bin|ocd2)$/i.test(normalized)) return "schema binary";
  if (/\/schema\/.*\.ya?ml$/i.test(normalized)) return "schema yaml";
  if (/schema-asset-manifest\.json$/i.test(normalized)) return "asset manifest";
  return "other";
}

export function ownerBuckets(sample: StartupSample): Record<string, number> {
  const buckets: Record<string, number> = {};
  for (const phase of sample.phases) {
    buckets[phase.phase] = (buckets[phase.phase] ?? 0) + phase.ms;
  }
  const startup = sample.startupCompleteMs ?? 0;
  let startupActionMs = 0;
  for (const action of sample.startupActions) {
    const owner = `app action worker: ${action.action}`;
    const duration = action.workerMs ?? action.totalMs ?? 0;
    buckets[owner] = (buckets[owner] ?? 0) + duration;
    startupActionMs += duration;
  }
  buckets["React/browser ready residual"] = Math.max(0, sample.readyToInputMs - startup - startupActionMs);
  const transferGroups = new Map<string, number>();
  for (const resource of sample.resources) {
    transferGroups.set(
      resourceGroup(resource.name),
      (transferGroups.get(resourceGroup(resource.name)) ?? 0) + resource.duration,
    );
  }
  for (const [group, duration] of transferGroups) {
    buckets[`resource duration: ${group}`] = duration;
  }
  return buckets;
}

export function summarizeSamples(samples: StartupSample[]): SummaryRow[] {
  const groups = new Map<string, StartupSample[]>();
  for (const sample of samples) {
    const existing = groups.get(sample.scenarioId) ?? [];
    existing.push(sample);
    groups.set(sample.scenarioId, existing);
  }
  return [...groups.entries()].map(([scenarioId, group]) => {
    const firstKey = group.flatMap(sample => sample.firstKeyToPaint.map(key => key.ms));
    const ownerNames = new Set(group.flatMap(sample => Object.keys(ownerBuckets(sample))));
    const ownerMedians = [...ownerNames].map(owner => ({
      owner,
      median: median(group.map(sample => ownerBuckets(sample)[owner] ?? 0)),
    })).sort((left, right) => right.median - left.median);
    const top = ownerMedians[0] ?? { owner: "unknown", median: 0 };
    return {
      scenarioId,
      schema: group[0]?.schema ?? "",
      mode: group[0]?.mode ?? "",
      publicDemo: group[0]?.publicDemo ?? false,
      samples: group.length,
      medianReadyToInputMs: median(group.map(sample => sample.readyToInputMs)),
      p95ReadyToInputMs: percentile(group.map(sample => sample.readyToInputMs), 0.95),
      medianStartupCompleteMs: median(group.map(sample => sample.startupCompleteMs ?? 0)),
      p95StartupCompleteMs: percentile(group.map(sample => sample.startupCompleteMs ?? 0), 0.95),
      medianFirstKeyToPaintMs: median(firstKey),
      p95FirstKeyToPaintMs: percentile(firstKey, 0.95),
      transferBytes: Math.round(median(group.map(sample => sample.resources.reduce((sum, resource) => sum + resource.transferSize, 0)))),
      encodedBytes: Math.round(median(group.map(sample => sample.resources.reduce((sum, resource) => sum + resource.encodedBodySize, 0)))),
      medianWasmHeapBytes: Math.round(median(group.map(sample => sample.wasmMemory?.currentBytes ?? 0))),
      p95WasmHeapBytes: Math.round(percentile(group.map(sample => sample.wasmMemory?.currentBytes ?? 0), 0.95)),
      medianPeakWasmHeapBytes: Math.round(median(group.map(sample => sample.wasmMemory?.peakBytes ?? 0))),
      p95PeakWasmHeapBytes: Math.round(percentile(group.map(sample => sample.wasmMemory?.peakBytes ?? 0), 0.95)),
      maxPeakWasmHeapBytes: Math.max(0, ...group.map(sample => sample.wasmMemory?.peakBytes ?? 0)),
      cacheHits: Math.round(median(group.map(sample => sample.cache.hits))),
      cacheMisses: Math.round(median(group.map(sample => sample.cache.misses))),
      cacheErrors: Math.round(median(group.map(sample => sample.cache.errors.length))),
      topOwner: top.owner,
      topOwnerMedianMs: top.median,
    };
  });
}

export async function writeEvidence(outputDir: string, samples: StartupSample[]): Promise<void> {
  await mkdir(outputDir, { recursive: true });
  await writeFile(path.join(outputDir, "samples.json"), `${JSON.stringify(samples, null, 2)}\n`);
  await writeFile(path.join(outputDir, "samples.csv"), csvRows(samples));
  const summary = summarizeSamples(samples);
  await writeFile(path.join(outputDir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
  await writeFile(path.join(outputDir, "summary.csv"), summaryCsv(summary));
  await writeFile(path.join(outputDir, "dashboard.md"), dashboardMarkdown(samples, summary));
}

function csvEscape(value: unknown): string {
  const text = String(value ?? "");
  return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function csvRows(samples: StartupSample[]): string {
  const header = [
    "scenarioId",
    "sampleIndex",
    "schema",
    "appSchema",
    "mode",
    "publicDemo",
    "readyToInputMs",
    "startupCompleteMs",
    "cacheHits",
    "cacheMisses",
    "cacheErrors",
    "transferBytes",
    "encodedBytes",
    "wasmHeapBytes",
    "peakWasmHeapBytes",
    "wasmMemoryMarkerCount",
    "firstKeyMedianMs",
    "firstKeyP95Ms",
    "firstKeyPeakWasmHeapBytes",
  ];
  const rows = samples.map(sample => [
    sample.scenarioId,
    sample.sampleIndex,
    sample.schema,
    sample.appSchema,
    sample.mode,
    sample.publicDemo,
    sample.readyToInputMs,
    sample.startupCompleteMs ?? "",
    sample.cache.hits,
    sample.cache.misses,
    sample.cache.errors.length,
    sample.resources.reduce((sum, resource) => sum + resource.transferSize, 0),
    sample.resources.reduce((sum, resource) => sum + resource.encodedBodySize, 0),
    sample.wasmMemory?.currentBytes ?? "",
    sample.wasmMemory?.peakBytes ?? "",
    sample.wasmMemoryMarkers.length,
    median(sample.firstKeyToPaint.map(key => key.ms)),
    percentile(sample.firstKeyToPaint.map(key => key.ms), 0.95),
    Math.max(0, ...sample.firstKeyToPaint.map(key => key.peakWasmHeapBytes ?? 0)),
  ]);
  return [header, ...rows].map(row => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function summaryCsv(rows: SummaryRow[]): string {
  const header = Object.keys(rows[0] ?? {
    scenarioId: "",
    schema: "",
    mode: "",
    publicDemo: "",
    samples: "",
    medianReadyToInputMs: "",
    p95ReadyToInputMs: "",
    medianStartupCompleteMs: "",
    p95StartupCompleteMs: "",
    medianFirstKeyToPaintMs: "",
    p95FirstKeyToPaintMs: "",
    transferBytes: "",
    encodedBytes: "",
    medianWasmHeapBytes: "",
    p95WasmHeapBytes: "",
    medianPeakWasmHeapBytes: "",
    p95PeakWasmHeapBytes: "",
    maxPeakWasmHeapBytes: "",
    cacheHits: "",
    cacheMisses: "",
    cacheErrors: "",
    topOwner: "",
    topOwnerMedianMs: "",
  });
  return [
    header,
    ...rows.map(row => header.map(key => (row as unknown as Record<string, unknown>)[key])),
  ].map(row => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function dashboardMarkdown(samples: StartupSample[], summary: SummaryRow[]): string {
  const ownerRows = summary
    .map(row => `| ${row.scenarioId} | ${row.topOwner} | ${row.topOwnerMedianMs.toFixed(1)} | ${row.medianReadyToInputMs.toFixed(1)} | ${row.p95ReadyToInputMs.toFixed(1)} |`)
    .join("\n");
  const summaryRows = summary
    .map(row => `| ${row.scenarioId} | ${row.samples} | ${row.schema} | ${row.mode} | ${row.publicDemo ? "yes" : "no"} | ${row.medianReadyToInputMs.toFixed(1)} | ${row.p95ReadyToInputMs.toFixed(1)} | ${row.medianFirstKeyToPaintMs.toFixed(1)} | ${row.transferBytes} | ${row.encodedBytes} | ${row.medianWasmHeapBytes} | ${row.medianPeakWasmHeapBytes} | ${row.cacheHits}/${row.cacheMisses}/${row.cacheErrors} |`)
    .join("\n");
  const resourceRows = resourceSummary(samples)
    .map(row => `| ${row.group} | ${row.transferBytes} | ${row.encodedBytes} | ${row.durationMs.toFixed(1)} |`)
    .join("\n");
  const memoryRows = summaryMemory(samples)
    .map(row => `| ${row.scenarioId} | ${row.wasmHeapBytes ?? ""} | ${row.peakWasmHeapBytes ?? ""} | ${row.jsHeapUsedSize ?? ""} | ${row.jsHeapTotalSize ?? ""} | ${row.domNodes ?? ""} | ${row.workingSetBytes ?? ""} |`)
    .join("\n");

  return `# Yune Web Startup Benchmark Dashboard

## Summary

| Scenario | Samples | Schema | Mode | Public | Median ready ms | p95 ready ms | Median first key ms | Transfer bytes | Encoded bytes | WASM heap bytes | Peak WASM heap bytes | Cache h/m/e |
| --- | ---: | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
${summaryRows}

## Startup Owner Map

| Scenario | Top owner | Owner median ms | Ready median ms | Ready p95 ms |
| --- | --- | ---: | ---: | ---: |
${ownerRows}

## Asset Transfer By Group

| Group | Transfer bytes | Encoded bytes | Duration ms |
| --- | ---: | ---: | ---: |
${resourceRows}

## Browser Memory

| Scenario | WASM heap | Peak WASM heap | JS heap used | JS heap total | DOM nodes | Windows working set |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
${memoryRows}
`;
}

function resourceSummary(samples: StartupSample[]): Array<{ group: string; transferBytes: number; encodedBytes: number; durationMs: number }> {
  const groups = new Map<string, { transferBytes: number[]; encodedBytes: number[]; durationMs: number[] }>();
  for (const sample of samples) {
    const byGroup = new Map<string, { transferBytes: number; encodedBytes: number; durationMs: number }>();
    for (const resource of sample.resources) {
      const group = resourceGroup(resource.name);
      const current = byGroup.get(group) ?? { transferBytes: 0, encodedBytes: 0, durationMs: 0 };
      current.transferBytes += resource.transferSize;
      current.encodedBytes += resource.encodedBodySize;
      current.durationMs += resource.duration;
      byGroup.set(group, current);
    }
    for (const [group, values] of byGroup) {
      const current = groups.get(group) ?? { transferBytes: [], encodedBytes: [], durationMs: [] };
      current.transferBytes.push(values.transferBytes);
      current.encodedBytes.push(values.encodedBytes);
      current.durationMs.push(values.durationMs);
      groups.set(group, current);
    }
  }
  return [...groups.entries()].map(([group, values]) => ({
    group,
    transferBytes: Math.round(median(values.transferBytes)),
    encodedBytes: Math.round(median(values.encodedBytes)),
    durationMs: median(values.durationMs),
  })).sort((left, right) => right.encodedBytes - left.encodedBytes);
}

function summaryMemory(samples: StartupSample[]): Array<Record<string, number | string | undefined>> {
  const byScenario = new Map<string, StartupSample[]>();
  for (const sample of samples) {
    const list = byScenario.get(sample.scenarioId) ?? [];
    list.push(sample);
    byScenario.set(sample.scenarioId, list);
  }
  return [...byScenario.entries()].map(([scenarioId, list]) => ({
    scenarioId,
    wasmHeapBytes: Math.round(median(list.map(sample => sample.wasmMemory?.currentBytes ?? 0))),
    peakWasmHeapBytes: Math.round(median(list.map(sample => sample.wasmMemory?.peakBytes ?? 0))),
    jsHeapUsedSize: Math.round(median(list.map(sample => sample.browserMemory?.["JSHeapUsedSize"] ?? 0))),
    jsHeapTotalSize: Math.round(median(list.map(sample => sample.browserMemory?.["JSHeapTotalSize"] ?? 0))),
    domNodes: Math.round(median(list.map(sample => sample.browserMemory?.["Nodes"] ?? 0))),
    workingSetBytes: Math.round(median(list.map(sample => sample.browserMemory?.["windowsWorkingSetBytes"] ?? 0))),
  }));
}
