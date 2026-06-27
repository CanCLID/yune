import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

import { median, percentile, type StartupResource, type WasmMemorySnapshot } from "./metrics";

export interface ComparatorResource extends StartupResource {
  context: "page" | "worker" | "synthetic-worker";
}

export interface ComparatorWorkerMemory {
  heapBytes?: number;
  moduleHeapBytes?: number;
  globalHeapBytes?: number;
  wasmMemoryBytes?: number;
  exportedKeys?: string[];
}

export interface ComparatorSample {
  scenarioId: string;
  app: "yune-web" | "my-rime";
  build: string;
  schema: "luna_pinyin" | "jyutping";
  schemaInput: string;
  sampleIndex: number;
  url: string;
  readyToInputMs: number;
  inputToCandidateMs: number;
  commitMs: number;
  firstCandidateText?: string;
  committedValue?: string;
  wasmMemory?: {
    ready?: WasmMemorySnapshot;
    candidate?: WasmMemorySnapshot;
    commit?: WasmMemorySnapshot;
    worker?: ComparatorWorkerMemory;
  };
  yunePerf?: {
    internalKeydownToPaintMs?: number;
    workerProcessMs?: number;
    workerRoundtripMs?: number;
    firstCandidateText?: string;
  };
  browserMemory?: Record<string, number>;
  resources: ComparatorResource[];
  storageEstimate?: { usage?: number; quota?: number };
  workerUrls: string[];
  consoleErrors: string[];
}

export interface ComparatorSummaryRow {
  scenarioId: string;
  app: "yune-web" | "my-rime";
  build: string;
  schema: "luna_pinyin" | "jyutping";
  input: string;
  samples: number;
  medianReadyToInputMs: number;
  p95ReadyToInputMs: number;
  medianInputToCandidateMs: number;
  p95InputToCandidateMs: number;
  medianCommitMs: number;
  p95CommitMs: number;
  medianWasmReadyBytes: number;
  medianWasmPeakBytes: number;
  maxWasmPeakBytes: number;
  medianJSHeapUsedBytes: number;
  medianResourceTransferBytes: number;
  medianResourceUniqueEncodedBytes: number;
  medianStorageUsageBytes: number;
  yuneMedianInternalKeydownToPaintMs: number;
  yuneMedianWorkerProcessMs: number;
  committedValues: string[];
  topResources: Array<{
    context: string;
    name: string;
    encodedBodySize: number;
    transferSize: number;
  }>;
}

export function summarizeComparatorSamples(samples: ComparatorSample[]): ComparatorSummaryRow[] {
  const groups = new Map<string, ComparatorSample[]>();
  for (const sample of samples) {
    const key = `${sample.scenarioId}:${sample.schema}`;
    const existing = groups.get(key) ?? [];
    existing.push(sample);
    groups.set(key, existing);
  }
  return [...groups.values()].map(group => {
    const first = group[0];
    const peakValues = group.map(sample => observedPeakBytes(sample));
    return {
      scenarioId: first?.scenarioId ?? "",
      app: first?.app ?? "yune-web",
      build: first?.build ?? "",
      schema: first?.schema ?? "luna_pinyin",
      input: first?.schemaInput ?? "",
      samples: group.length,
      medianReadyToInputMs: median(group.map(sample => sample.readyToInputMs)),
      p95ReadyToInputMs: percentile(group.map(sample => sample.readyToInputMs), 0.95),
      medianInputToCandidateMs: median(group.map(sample => sample.inputToCandidateMs)),
      p95InputToCandidateMs: percentile(group.map(sample => sample.inputToCandidateMs), 0.95),
      medianCommitMs: median(group.map(sample => sample.commitMs)),
      p95CommitMs: percentile(group.map(sample => sample.commitMs), 0.95),
      medianWasmReadyBytes: Math.round(median(group.map(sample => sample.wasmMemory?.ready?.currentBytes ?? sample.wasmMemory?.worker?.heapBytes ?? 0))),
      medianWasmPeakBytes: Math.round(median(peakValues)),
      maxWasmPeakBytes: Math.max(0, ...peakValues),
      medianJSHeapUsedBytes: Math.round(median(group.map(sample => sample.browserMemory?.["JSHeapUsedSize"] ?? sample.browserMemory?.["usedJSHeapSize"] ?? 0))),
      medianResourceTransferBytes: Math.round(median(group.map(sample => resourceTransferBytes(sample.resources)))),
      medianResourceUniqueEncodedBytes: Math.round(median(group.map(sample => uniqueEncodedBytes(sample.resources)))),
      medianStorageUsageBytes: Math.round(median(group.map(sample => sample.storageEstimate?.usage ?? 0))),
      yuneMedianInternalKeydownToPaintMs: Math.round(median(group.map(sample => sample.yunePerf?.internalKeydownToPaintMs ?? 0))),
      yuneMedianWorkerProcessMs: Math.round(median(group.map(sample => sample.yunePerf?.workerProcessMs ?? 0))),
      committedValues: [...new Set(group.map(sample => sample.committedValue ?? "").filter(Boolean))],
      topResources: topResources(group),
    };
  });
}

export async function writeComparatorEvidence(outputDir: string, samples: ComparatorSample[]): Promise<void> {
  await mkdir(outputDir, { recursive: true });
  await writeFile(path.join(outputDir, "samples.json"), `${JSON.stringify(samples, null, 2)}\n`);
  await writeFile(path.join(outputDir, "samples.csv"), sampleCsv(samples));
  const summary = summarizeComparatorSamples(samples);
  await writeFile(path.join(outputDir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
  await writeFile(path.join(outputDir, "summary.csv"), summaryCsv(summary));
  await writeFile(path.join(outputDir, "report.md"), reportMarkdown(summary));
}

function observedPeakBytes(sample: ComparatorSample): number {
  return Math.max(
    0,
    sample.wasmMemory?.ready?.peakBytes ?? 0,
    sample.wasmMemory?.ready?.currentBytes ?? 0,
    sample.wasmMemory?.candidate?.peakBytes ?? 0,
    sample.wasmMemory?.candidate?.currentBytes ?? 0,
    sample.wasmMemory?.commit?.peakBytes ?? 0,
    sample.wasmMemory?.commit?.currentBytes ?? 0,
    sample.wasmMemory?.worker?.heapBytes ?? 0,
  );
}

function resourceTransferBytes(resources: ComparatorResource[]): number {
  return resources.reduce((sum, resource) => sum + resource.transferSize, 0);
}

function uniqueEncodedBytes(resources: ComparatorResource[]): number {
  return [...uniqueResources(resources).values()]
    .reduce((sum, resource) => sum + resource.encodedBodySize, 0);
}

function uniqueResources(resources: ComparatorResource[]): Map<string, ComparatorResource> {
  const unique = new Map<string, ComparatorResource>();
  for (const resource of resources) {
    const key = normalizedResourceName(resource.name);
    const existing = unique.get(key);
    if (!existing || resource.encodedBodySize > existing.encodedBodySize) {
      unique.set(key, resource);
    }
  }
  return unique;
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

function topResources(group: ComparatorSample[]): ComparatorSummaryRow["topResources"] {
  const names = new Set<string>();
  const bySample = group.map(sample => uniqueResources(sample.resources));
  for (const resources of bySample) {
    for (const name of resources.keys()) {
      names.add(name);
    }
  }
  return [...names].map(name => {
    const resources = bySample
      .map(sample => sample.get(name))
      .filter((resource): resource is ComparatorResource => resource !== undefined);
    const representative = resources[0];
    return {
      context: representative?.context ?? "page",
      name,
      encodedBodySize: Math.round(median(resources.map(resource => resource.encodedBodySize))),
      transferSize: Math.round(median(resources.map(resource => resource.transferSize))),
    };
  }).sort((left, right) => right.encodedBodySize - left.encodedBodySize).slice(0, 8);
}

function csvEscape(value: unknown): string {
  const text = String(value ?? "");
  return /[",\n]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
}

function sampleCsv(samples: ComparatorSample[]): string {
  const header = [
    "scenarioId",
    "app",
    "build",
    "schema",
    "sampleIndex",
    "readyToInputMs",
    "inputToCandidateMs",
    "commitMs",
    "firstCandidateText",
    "committedValue",
    "wasmReadyBytes",
    "wasmPeakBytes",
    "resourceTransferBytes",
    "resourceUniqueEncodedBytes",
    "storageUsageBytes",
    "workerUrls",
    "consoleErrors",
  ];
  const rows = samples.map(sample => [
    sample.scenarioId,
    sample.app,
    sample.build,
    sample.schema,
    sample.sampleIndex,
    sample.readyToInputMs,
    sample.inputToCandidateMs,
    sample.commitMs,
    sample.firstCandidateText ?? "",
    sample.committedValue ?? "",
    sample.wasmMemory?.ready?.currentBytes ?? sample.wasmMemory?.worker?.heapBytes ?? "",
    observedPeakBytes(sample),
    resourceTransferBytes(sample.resources),
    uniqueEncodedBytes(sample.resources),
    sample.storageEstimate?.usage ?? "",
    sample.workerUrls.join(" "),
    sample.consoleErrors.join(" | "),
  ]);
  return [header, ...rows].map(row => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function summaryCsv(rows: ComparatorSummaryRow[]): string {
  const header = Object.keys(rows[0] ?? {
    scenarioId: "",
    app: "",
    build: "",
    schema: "",
    input: "",
    samples: "",
    medianReadyToInputMs: "",
    p95ReadyToInputMs: "",
    medianInputToCandidateMs: "",
    p95InputToCandidateMs: "",
    medianCommitMs: "",
    p95CommitMs: "",
    medianWasmReadyBytes: "",
    medianWasmPeakBytes: "",
    maxWasmPeakBytes: "",
    medianJSHeapUsedBytes: "",
    medianResourceTransferBytes: "",
    medianResourceUniqueEncodedBytes: "",
    medianStorageUsageBytes: "",
    yuneMedianInternalKeydownToPaintMs: "",
    yuneMedianWorkerProcessMs: "",
    committedValues: "",
    topResources: "",
  });
  return [
    header,
    ...rows.map(row => header.map(key => {
      const value = (row as unknown as Record<string, unknown>)[key];
      return Array.isArray(value) ? JSON.stringify(value) : value;
    })),
  ].map(row => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function reportMarkdown(rows: ComparatorSummaryRow[]): string {
  const tableRows = rows
    .map(row => `| ${row.scenarioId} | ${row.schema} | ${row.samples} | ${row.medianReadyToInputMs.toFixed(0)} | ${row.medianInputToCandidateMs.toFixed(0)} | ${row.medianCommitMs.toFixed(0)} | ${bytes(row.medianWasmReadyBytes)} | ${bytes(row.medianWasmPeakBytes)} | ${bytes(row.medianResourceUniqueEncodedBytes)} | ${row.committedValues.map(value => `\`${value}\``).join(", ")} |`)
    .join("\n");
  const resourceSections = rows.map(row => [
    `### ${row.scenarioId} ${row.schema}`,
    "",
    "| Resource | Context | Encoded | Transfer |",
    "| --- | --- | ---: | ---: |",
    ...row.topResources.map(resource => `| ${resource.name} | ${resource.context} | ${bytes(resource.encodedBodySize)} | ${bytes(resource.transferSize)} |`),
    "",
  ].join("\n")).join("\n");
  return `# Yune Web Comparator Benchmark

| Scenario | Schema | Samples | Ready ms | Input ms | Commit ms | WASM ready | WASM peak | Unique encoded resources | Commit |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
${tableRows}

## Top Resources

${resourceSections}
`;
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
