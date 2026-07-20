import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

import { median, percentile, type StartupResource, type WasmMemorySnapshot } from "./metrics";
import {
  comparatorBindingRoundCount,
  comparatorEndpointContractVersion,
  comparatorPeerCadenceMs,
  comparatorPeerPageSize,
  comparatorPinnedMyRimeCommit,
  comparatorSelectorManifest,
  evaluatePackageAlignment,
  validateEndpointEvidence,
  type ComparatorEndpointEvidence,
  type ComparatorIdentityManifest,
  type ComparatorPageSizeSetup,
  type PackageAlignment,
} from "./comparator-endpoint";

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
  cadenceMs: number;
  inputToCandidateMs: number;
  commitMs: number;
  firstCandidateText?: string;
  committedValue?: string;
  endpoint?: ComparatorEndpointEvidence;
  pageSizeSetup?: ComparatorPageSizeSetup;
  identity?: ComparatorIdentityManifest;
  identityManifestSha256?: string;
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
  comparisonLane: "peer candidate" | "data-confounded" | "guard";
  input: string;
  samples: number;
  cadenceMs: number;
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
  endpointContractVersion: string;
  endpointVerdict: "PASS" | "FAIL";
  endpointFailures: string[];
  selectorManifestIds: string[];
  packageIdentityPresent: boolean;
  identityManifestSha256s: string[];
  committedValues: string[];
  topResources: Array<{
    context: string;
    name: string;
    encodedBodySize: number;
    transferSize: number;
  }>;
}

export interface ComparatorRatioRow {
  schema: "luna_pinyin" | "jyutping";
  input: string;
  yuneScenarioId: string;
  yuneBuild: string;
  peerScenarioId: string;
  peerBuild: string;
  packageAlignment: PackageAlignment;
  endpointAlignment: "PROVED" | "INVALID";
  ratioStatus: "PUBLISHED" | "OMITTED";
  reasons: string[];
  p95InputToCandidateRatio?: number;
  p95CommitRatio?: number;
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
    const endpointFailures = uniqueStrings(group.flatMap(sample =>
      validateEndpointEvidence(sample.endpoint, sample.schemaInput, sample.committedValue ?? "", sample.app)
    ));
    return {
      scenarioId: first?.scenarioId ?? "",
      app: first?.app ?? "yune-web",
      build: first?.build ?? "",
      schema: first?.schema ?? "luna_pinyin",
      comparisonLane: comparisonLane(first?.schema ?? "luna_pinyin", group.every(sample => sample.identity !== undefined)),
      input: first?.schemaInput ?? "",
      samples: group.length,
      cadenceMs: singleNumericValue(group.map(sample => sample.cadenceMs)),
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
      endpointContractVersion: comparatorEndpointContractVersion,
      endpointVerdict: endpointFailures.length === 0 ? "PASS" : "FAIL",
      endpointFailures,
      selectorManifestIds: uniqueStrings(group.flatMap(sample => sample.endpoint
        ? [sample.endpoint.candidate.secondRaf.selectorManifestId, sample.endpoint.commit.secondRaf.selectorManifestId]
        : [])),
      packageIdentityPresent: group.every(sample => sample.identity !== undefined),
      identityManifestSha256s: uniqueStrings(group.flatMap(sample => sample.identityManifestSha256
        ? [sample.identityManifestSha256]
        : [])),
      committedValues: [...new Set(group.map(sample => sample.committedValue ?? "").filter(Boolean))],
      topResources: topResources(group),
    };
  });
}

export function summarizeComparatorRatios(samples: ComparatorSample[]): ComparatorRatioRow[] {
  const groups = sampleGroups(samples);
  const yuneGroups = [...groups.values()].filter(group => group[0]?.app === "yune-web");
  const peerGroups = [...groups.values()].filter(group => group[0]?.app === "my-rime");
  const rows: ComparatorRatioRow[] = [];
  for (const yuneGroup of yuneGroups) {
    const yuneFirst = yuneGroup[0];
    if (!yuneFirst) {
      continue;
    }
    for (const peerGroup of peerGroups) {
      const peerFirst = peerGroup[0];
      if (!peerFirst || yuneFirst.schema !== peerFirst.schema || yuneFirst.schemaInput !== peerFirst.schemaInput) {
        continue;
      }
      const yuneIdentity = singleIdentity(yuneGroup);
      const peerIdentity = singleIdentity(peerGroup);
      const pairedIdentity = pairIdentity(yuneIdentity.identity, peerIdentity.identity);
      const identityHashReasons = pairedIdentityHashReasons(yuneGroup, peerGroup);
      const roundReasons = [
        ...bindingRoundFailures("yune", yuneGroup),
        ...bindingRoundFailures("peer", peerGroup),
      ];
      const alignment = evaluatePackageAlignment(pairedIdentity.identity);
      const endpointReasons = uniqueStrings([
        ...yuneGroup.flatMap(sample => validateEndpointEvidence(sample.endpoint, sample.schemaInput, sample.committedValue ?? "", sample.app)),
        ...peerGroup.flatMap(sample => validateEndpointEvidence(sample.endpoint, sample.schemaInput, sample.committedValue ?? "", sample.app)),
        ...yuneGroup.flatMap(endpointIdentityFailures),
        ...peerGroup.flatMap(endpointIdentityFailures),
      ]);
      const yuneCandidateP95 = percentile(yuneGroup.map(sample => sample.inputToCandidateMs), 0.95);
      const peerCandidateP95 = percentile(peerGroup.map(sample => sample.inputToCandidateMs), 0.95);
      const yuneCommitP95 = percentile(yuneGroup.map(sample => sample.commitMs), 0.95);
      const peerCommitP95 = percentile(peerGroup.map(sample => sample.commitMs), 0.95);
      const ratioReasons = [
        ...(!Number.isFinite(yuneCandidateP95) || !Number.isFinite(peerCandidateP95) || peerCandidateP95 <= 0
          ? ["candidate-ratio-input-invalid"]
          : []),
        ...(!Number.isFinite(yuneCommitP95) || !Number.isFinite(peerCommitP95) || peerCommitP95 <= 0
          ? ["commit-ratio-input-invalid"]
          : []),
      ];
      const reasons = uniqueStrings([
        ...alignment.reasons,
        ...yuneIdentity.reasons,
        ...peerIdentity.reasons,
        ...pairedIdentity.reasons,
        ...identityHashReasons,
        ...roundReasons,
        ...endpointReasons.map(reason => "endpoint:" + reason),
        ...ratioReasons,
        ...(yuneFirst.schema === "luna_pinyin" ? [] : ["jyutping-is-a-guard-not-a-fair-peer-lane"]),
      ]);
      const publish = alignment.packageAlignment === "PROVED"
        && endpointReasons.length === 0
        && yuneIdentity.reasons.length === 0
        && peerIdentity.reasons.length === 0
        && pairedIdentity.reasons.length === 0
        && identityHashReasons.length === 0
        && roundReasons.length === 0
        && ratioReasons.length === 0
        && yuneFirst.schema === "luna_pinyin";
      rows.push({
        schema: yuneFirst.schema,
        input: yuneFirst.schemaInput,
        yuneScenarioId: yuneFirst.scenarioId,
        yuneBuild: yuneFirst.build,
        peerScenarioId: peerFirst.scenarioId,
        peerBuild: peerFirst.build,
        packageAlignment: alignment.packageAlignment,
        endpointAlignment: endpointReasons.length === 0 ? "PROVED" : "INVALID",
        ratioStatus: publish ? "PUBLISHED" : "OMITTED",
        reasons,
        ...(publish ? {
          p95InputToCandidateRatio: ratio(
            yuneCandidateP95,
            peerCandidateP95,
          ),
          p95CommitRatio: ratio(
            yuneCommitP95,
            peerCommitP95,
          ),
        } : {}),
      });
    }
  }
  return rows;
}

export async function writeComparatorEvidence(outputDir: string, samples: ComparatorSample[]): Promise<void> {
  await mkdir(outputDir, { recursive: true });
  await writeFile(path.join(outputDir, "samples.json"), `${JSON.stringify(samples, null, 2)}\n`);
  await writeFile(path.join(outputDir, "samples.csv"), sampleCsv(samples));
  const summary = summarizeComparatorSamples(samples);
  await writeFile(path.join(outputDir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
  await writeFile(path.join(outputDir, "summary.csv"), summaryCsv(summary));
  const ratios = summarizeComparatorRatios(samples);
  await writeFile(path.join(outputDir, "ratios.json"), `${JSON.stringify(ratios, null, 2)}\n`);
  await writeFile(path.join(outputDir, "ratios.csv"), ratioCsv(ratios));
  await writeFile(path.join(outputDir, "report.md"), reportMarkdown(summary, ratios));
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
    "cadenceMs",
    "inputToCandidateMs",
    "commitMs",
    "firstCandidateText",
    "committedValue",
    "endpointContractVersion",
    "inputEvents",
    "candidateEventOrdinal",
    "candidateEventRevision",
    "candidateDomRevision",
    "candidateComposition",
    "candidateDomDigest",
    "commitEventOrdinal",
    "commitEventRevision",
    "commitDomRevision",
    "commitDomDigest",
    "pageSizeSetup",
    "identityManifestSha256",
    "packageIdentity",
    "dataIdentity",
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
    sample.cadenceMs,
    sample.inputToCandidateMs,
    sample.commitMs,
    sample.firstCandidateText ?? "",
    sample.committedValue ?? "",
    sample.endpoint?.candidate.secondRaf.contractVersion ?? "",
    sample.endpoint ? JSON.stringify(sample.endpoint.inputEvents) : "",
    sample.endpoint?.candidate.event.ordinal ?? "",
    sample.endpoint?.candidate.event.revisionBeforeEvent ?? "",
    sample.endpoint?.candidate.secondRaf.revision ?? "",
    sample.endpoint?.candidate.secondRaf.composition ?? "",
    sample.endpoint?.candidate.secondRaf.digest ?? "",
    sample.endpoint?.commit.event.ordinal ?? "",
    sample.endpoint?.commit.event.revisionBeforeEvent ?? "",
    sample.endpoint?.commit.secondRaf.revision ?? "",
    sample.endpoint?.commit.secondRaf.digest ?? "",
    sample.pageSizeSetup ? JSON.stringify(sample.pageSizeSetup) : "",
    sample.identityManifestSha256 ?? "",
    sample.identity ? JSON.stringify({ yune: sample.identity.yune, peer: sample.identity.peer }) : "",
    sample.identity ? JSON.stringify({
      logicalInputs: sample.identity.logicalInputs,
      effectiveConfiguration: sample.identity.effectiveConfiguration,
      freshEmptyUserdb: sample.identity.freshEmptyUserdb,
      sameEndpointObserver: sample.identity.sameEndpointObserver,
    }) : "",
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
    comparisonLane: "",
    input: "",
    samples: "",
    cadenceMs: "",
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
    endpointContractVersion: "",
    endpointVerdict: "",
    endpointFailures: "",
    selectorManifestIds: "",
    packageIdentityPresent: "",
    identityManifestSha256s: "",
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

function ratioCsv(rows: ComparatorRatioRow[]): string {
  const header = [
    "schema",
    "input",
    "yuneScenarioId",
    "yuneBuild",
    "peerScenarioId",
    "peerBuild",
    "packageAlignment",
    "endpointAlignment",
    "ratioStatus",
    "p95InputToCandidateRatio",
    "p95CommitRatio",
    "reasons",
  ];
  return [
    header,
    ...rows.map(row => [
      row.schema,
      row.input,
      row.yuneScenarioId,
      row.yuneBuild,
      row.peerScenarioId,
      row.peerBuild,
      row.packageAlignment,
      row.endpointAlignment,
      row.ratioStatus,
      row.p95InputToCandidateRatio ?? "",
      row.p95CommitRatio ?? "",
      row.reasons.join(";"),
    ]),
  ].map(row => row.map(csvEscape).join(",")).join("\n") + "\n";
}

function reportMarkdown(rows: ComparatorSummaryRow[], ratios: ComparatorRatioRow[]): string {
  const tableRows = rows
    .map(row => `| ${row.scenarioId} | ${row.schema} | ${row.comparisonLane} | ${row.samples} | ${row.cadenceMs.toFixed(0)} | ${row.medianReadyToInputMs.toFixed(0)} | ${row.medianInputToCandidateMs.toFixed(0)} | ${row.medianCommitMs.toFixed(0)} | ${bytes(row.medianWasmReadyBytes)} | ${bytes(row.medianWasmPeakBytes)} | ${bytes(row.medianResourceUniqueEncodedBytes)} | ${row.committedValues.map(value => `\`${value}\``).join(", ")} |`)
    .join("\n");
  const resourceSections = rows.map(row => [
    `### ${row.scenarioId} ${row.schema}`,
    "",
    "| Resource | Context | Encoded | Transfer |",
    "| --- | --- | ---: | ---: |",
    ...row.topResources.map(resource => `| ${resource.name} | ${resource.context} | ${bytes(resource.encodedBodySize)} | ${bytes(resource.transferSize)} |`),
    "",
  ].join("\n")).join("\n");
  const ratioRows = ratios.length === 0
    ? "| (no paired peer rows) |  |  | OMITTED |  |  | No Yune/My RIME row pair was captured. |"
    : ratios.map(row => [
      "| " + row.schema,
      row.yuneBuild + " / " + row.peerBuild,
      row.packageAlignment,
      row.ratioStatus,
      row.p95InputToCandidateRatio?.toFixed(3) ?? "",
      row.p95CommitRatio?.toFixed(3) ?? "",
      row.reasons.join("; ") + " |",
    ].join(" | ")).join("\n");
  return `# Yune Web Comparator Benchmark

## Comparison Read

No schema name or matching candidate makes a fair comparison by itself. A ratio
is emitted only when both apps pass the same post-event atomic DOM endpoint and
their complete row-level logical-data identity produces
\`packageAlignment=PROVED\`. Jyutping remains guard evidence.

| Scenario | Schema | Lane | Samples | Cadence ms | Ready ms | Input ms | Commit ms | WASM ready | WASM peak | Unique encoded resources | Commit |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
${tableRows}

## Fail-Closed Peer Ratios

| Schema | Builds (Yune / peer) | Package alignment | Ratio status | Candidate p95 ratio | Commit p95 ratio | Reasons |
| --- | --- | --- | --- | ---: | ---: | --- |
${ratioRows}

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

function comparisonLane(
  schema: ComparatorSummaryRow["schema"],
  hasIdentity: boolean,
): ComparatorSummaryRow["comparisonLane"] {
  if (schema !== "luna_pinyin") {
    return "guard";
  }
  return hasIdentity ? "peer candidate" : "data-confounded";
}

function sampleGroups(samples: ComparatorSample[]): Map<string, ComparatorSample[]> {
  const groups = new Map<string, ComparatorSample[]>();
  for (const sample of samples) {
    const key = [sample.app, sample.scenarioId, sample.build, sample.schema, sample.schemaInput].join(":");
    const group = groups.get(key) ?? [];
    group.push(sample);
    groups.set(key, group);
  }
  return groups;
}

function singleIdentity(group: ComparatorSample[]): {
  identity?: ComparatorIdentityManifest;
  reasons: string[];
} {
  if (group.some(sample => sample.identity === undefined)) {
    return { reasons: ["identity-missing-from-one-or-more-samples"] };
  }
  const identities = new Map<string, ComparatorIdentityManifest>();
  for (const sample of group) {
    if (sample.identity) {
      identities.set(JSON.stringify(sample.identity), sample.identity);
    }
  }
  if (identities.size !== 1) {
    return { reasons: ["identity-changed-across-samples"] };
  }
  return { identity: [...identities.values()][0], reasons: [] };
}

function pairIdentity(
  yune: ComparatorIdentityManifest | undefined,
  peer: ComparatorIdentityManifest | undefined,
): { identity?: ComparatorIdentityManifest; reasons: string[] } {
  if (!yune || !peer) {
    return { reasons: ["paired-peer-data-manifest-missing"] };
  }
  if (JSON.stringify(yune) !== JSON.stringify(peer)) {
    return { reasons: ["paired-peer-data-manifest-different"] };
  }
  return { identity: yune, reasons: [] };
}

function pairedIdentityHashReasons(
  yuneGroup: ComparatorSample[],
  peerGroup: ComparatorSample[],
): string[] {
  const hashes = new Set(
    [...yuneGroup, ...peerGroup]
      .map(sample => sample.identityManifestSha256)
      .filter((hash): hash is string => hash !== undefined),
  );
  if (hashes.size !== 1
      || [...yuneGroup, ...peerGroup].some(sample => !/^[0-9a-f]{64}$/i.test(sample.identityManifestSha256 ?? ""))) {
    return ["paired-peer-data-manifest-sha256-missing-or-different"];
  }
  return [];
}

function bindingRoundFailures(side: "yune" | "peer", group: ComparatorSample[]): string[] {
  const indices = group.map(sample => sample.sampleIndex).sort((left, right) => left - right);
  if (group.length !== comparatorBindingRoundCount
      || indices.some((value, index) => value !== index)) {
    return [side + "-binding-rounds-are-not-exactly-five-contiguous-fresh-profile-samples"];
  }
  return [];
}

function ratio(numerator: number, denominator: number): number {
  return numerator / denominator;
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values)].sort();
}

function singleNumericValue(values: number[]): number {
  const unique = [...new Set(values)];
  return unique.length === 1 ? unique[0] ?? Number.NaN : Number.NaN;
}

function endpointIdentityFailures(sample: ComparatorSample): string[] {
  const reasons: string[] = [];
  if (!sample.identity) {
    return ["endpoint-package-data-identity-missing"];
  }
  if (!/^[0-9a-f]{64}$/i.test(sample.identityManifestSha256 ?? "")) {
    reasons.push("endpoint-identity-manifest-sha256-missing-or-invalid");
  }
  if (sample.cadenceMs !== comparatorPeerCadenceMs) {
    reasons.push("endpoint-cadence-is-not-the-frozen-60ms-schedule");
  }
  try {
    const hostname = new URL(sample.url).hostname;
    if (hostname !== "127.0.0.1" && hostname !== "localhost" && hostname !== "[::1]") {
      reasons.push("endpoint-is-not-the-locally-served-pinned-artifact");
    }
  } catch {
    reasons.push("endpoint-url-invalid");
  }
  if (sample.app === "my-rime") {
    if (sample.build === "unverified-live") {
      reasons.push("endpoint-peer-build-is-unverified-live");
    }
    if (sample.identity.peer.upstreamPinnedCommit !== comparatorPinnedMyRimeCommit) {
      reasons.push("endpoint-peer-source-is-not-the-pinned-my-rime-commit");
    }
  }
  if (sample.app === "yune-web" && !validYunePageSizeSetup(sample)) {
    reasons.push("endpoint-yune-six-row-setup-provenance-is-missing-or-invalid");
  }
  const candidate = sample.endpoint?.candidate.secondRaf;
  const commit = sample.endpoint?.commit.secondRaf;
  const expectedSelectorId = comparatorSelectorManifest[sample.app].id;
  if (!candidate || !commit
      || candidate.selectorManifestId !== expectedSelectorId
      || commit.selectorManifestId !== expectedSelectorId) {
    reasons.push("endpoint-frozen-selector-manifest-not-proved");
  }
  if (!candidate
      || candidate.composition !== sample.schemaInput
      || candidate.candidateSurfaceCount !== 1) {
    reasons.push("endpoint-frozen-composition-extraction-not-proved");
  }
  if (!candidate
      || candidate.candidates.length !== comparatorPeerPageSize
      || candidate.page.index !== 0
      || candidate.page.buttonCount !== 2
      || candidate.page.previousDisabled !== true
      || candidate.page.nextDisabled !== false) {
    reasons.push("endpoint-frozen-page-button-contract-not-proved");
  }
  return reasons;
}

function validYunePageSizeSetup(sample: ComparatorSample): boolean {
  const setup = sample.pageSizeSetup;
  if (!setup
      || setup.contractVersion !== "web06-page-size-setup-v1"
      || setup.requiredRows !== comparatorPeerPageSize
      || setup.initial.uiValue !== String(comparatorPeerPageSize)
      || setup.initial.localStorageValue !== String(comparatorPeerPageSize)
      || setup.actions.length !== 2) {
    return false;
  }
  const expectedTransitions = [
    { from: "6", to: "7", key: "ArrowRight", rows: 7 },
    { from: "7", to: "6", key: "ArrowLeft", rows: 6 },
  ];
  if (setup.actions.some((action, index) => {
    const expected = expectedTransitions[index];
    return !expected
      || action.ordinal !== index + 1
      || action.fromUiValue !== expected.from
      || action.targetUiValue !== expected.to
      || action.interaction.kind !== "keyboard"
      || action.interaction.key !== expected.key
      || action.interaction.control !== "preferences-page-size-range"
      || action.deployStatus !== "success"
      || !action.loadingComplete
      || action.localStorageValue !== expected.to
      || action.persistedConfigValue !== expected.to
      || !Number.isInteger(action.persistenceDiagnosticIndex)
      || action.persistenceDiagnosticIndex < setup.initial.persistenceDiagnosticCount
      || action.engineProbe.input !== "ni"
      || action.engineProbe.candidateRows !== expected.rows
      || action.engineProbe.candidates.length !== expected.rows
      || action.engineProbe.candidates.some(candidate => candidate === "")
      || action.engineProbe.pageIndex !== 0
      || action.engineProbe.buttonCount !== 2
      || action.engineProbe.previousDisabled !== true
      || action.engineProbe.nextDisabled !== false
      || action.engineProbe.resetKey !== "Escape"
      || !action.engineProbe.resetEmpty;
  })) {
    return false;
  }
  if (setup.measurementPage.initial.uiValue !== String(comparatorPeerPageSize)
      || setup.measurementPage.initial.localStorageValue !== String(comparatorPeerPageSize)
      || setup.measurementPage.actions.length !== 2
      || setup.measurementPage.actions.some((action, index) => {
        const expected = expectedTransitions[index];
        return !expected
          || action.ordinal !== index + 1
          || action.fromUiValue !== expected.from
          || action.targetUiValue !== expected.to
          || action.interaction.kind !== "keyboard"
          || action.interaction.key !== expected.key
          || action.interaction.control !== "preferences-page-size-range"
          || action.deployStatus !== "success"
          || !action.loadingComplete
          || action.localStorageValue !== expected.to
          || action.persistedConfigValue !== expected.to
          || !Number.isInteger(action.persistenceDiagnosticIndex)
          || action.persistenceDiagnosticIndex
            < setup.measurementPage.initial.persistenceDiagnosticCount;
      })) {
    return false;
  }
  return setup.final.uiValue === String(comparatorPeerPageSize)
    && setup.final.localStorageValue === String(comparatorPeerPageSize)
    && setup.final.persistedConfigValue === String(comparatorPeerPageSize)
    && setup.final.deployStatus === "success"
    && setup.final.loadingComplete
    && setup.measurementPage.final.uiValue === String(comparatorPeerPageSize)
    && setup.measurementPage.final.localStorageValue === String(comparatorPeerPageSize)
    && setup.measurementPage.final.persistedConfigValue === String(comparatorPeerPageSize)
    && setup.measurementPage.final.deployStatus === "success"
    && setup.measurementPage.final.loadingComplete
    && setup.engineProof.candidateRows === comparatorPeerPageSize
    && setup.engineProof.pageIndex === 0
    && setup.engineProof.buttonCount === 2
    && setup.engineProof.previousDisabled === true
    && setup.engineProof.nextDisabled === false;
}
