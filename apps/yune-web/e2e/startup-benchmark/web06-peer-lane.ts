import { createHash } from "node:crypto";

import {
  comparatorBindingRoundCount,
  comparatorEndpointContractVersion,
  comparatorPeerCadenceMs,
  comparatorPeerPageSize,
  comparatorSelectorManifest,
  evaluatePackageAlignment,
  validateCandidateObservation,
  validateCommitObservation,
  type ComparatorApp,
  type ComparatorEventBoundary,
  type ComparatorIdentityManifest,
  type ComparatorStableObservation,
} from "./comparator-endpoint.ts";

export const web06PeerLaneVersion = "web06-phase4-peer-lane-v1" as const;
export const web06PeerRawVersion = "web06-phase4-peer-raw-v1" as const;
export const web06PeerCompactVersion = "web06-phase4-peer-compact-v1" as const;
export const web06PeerAttestationVersion = "web06-phase4-peer-attestation-v1" as const;
export const web06PeerVerifierVersion = "web06-phase4-peer-independent-verifier-v1" as const;
export const web06PeerViewport = { width: 1365, height: 900 } as const;
export const web06PeerLocale = "zh-HK" as const;
export const web06PeerCacheRegime = "fresh-persistent-profile-no-store-v1" as const;
export const web06PeerMaxAttempts = 7 as const;
export const web06PeerValidRounds = comparatorBindingRoundCount;
export const web06PeerCadenceRangeMs = { minimum: 48, maximum: 75 } as const;
export const web06PeerInteractiveMaxMs = 67 as const;
export const web06PeerSchemaId = "luna_pinyin" as const;
export const web06PeerPinnedArchiveSha256 =
  "a5eea5ebffa1f62e3f4d058117c1405137fddc02c1046e725ee7b4e7c47420ba" as const;
export const web06PeerPinnedManifestSha256 =
  "bfe733f1c190898a10c22afc53b237232c4d9b3c594c056d832dfc253dd6e1b6" as const;
export const web06PeerPinnedTreeSha256 =
  "d0dde476677373f24c5cfd335780a0bac753932d0eb52049950f874f0e37e7b3" as const;
export const web06PeerPinnedDataManifestSha256 =
  "6458fef3d0710a9a1db6571b5e83552fda8e14a26902922cd9cecb8ff7d5db17" as const;
export const web06PeerPinnedProjectionSha256 =
  "37fe9303712ee2c0bca7a548fd7c77587cb18378b5945f67f0675da30ae9f949" as const;
export const web06PeerPinnedPacketChecksumsSha256 =
  "cf0aecec0a0663f329a115b7f31d65be9ba0c994d2cb9a3a10e1b6972cd27cce" as const;
export const web06PeerPinnedUpstreamCommit =
  "c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3" as const;

const stableObservationContractReasons = new Set([
  "missing-stable-observation",
  "event-boundary-invalid",
  "dom-observation-boundary-invalid",
  "endpoint-contract-version-mismatch",
  "selector-manifest-changed-during-observation",
  "dom-revision-changed-during-double-raf",
  "dom-digest-changed-during-double-raf",
  "dom-digest-does-not-match-atomic-tuple",
  "accepted-dom-revision-is-not-after-event",
  "accepted-dom-observation-precedes-event",
  "double-raf-observation-order-invalid",
]);

export const web06PeerMeasurementFailureCodes = [
  "CANDIDATE_ENDPOINT_TIMEOUT",
  "CANDIDATE_ENDPOINT_SUPERSEDED",
  "COMMIT_ENDPOINT_TIMEOUT",
  "COMMIT_ENDPOINT_SUPERSEDED",
  "ENDPOINT_OBSERVER_MISSING",
  "ENDPOINT_OBSERVER_WRONG_APP",
  "ENDPOINT_METRIC_CONTRACT_FAILURE",
  "EVENT_STREAM_METRIC_CONTRACT_FAILURE",
  "FRAME_OBSERVER_MISSING",
  "FRAME_WINDOW_NOT_STARTED",
  "FRAME_WINDOW_END_MISSING",
  "FRAME_WINDOW_DURATION_INVALID",
  "FRAME_IDLE_CONTROL_TIMEOUT",
  "FRAME_BOUNDARY_CONTRACT_FAILURE",
  "FRAME_RECEIPT_CONTRACT_FAILURE",
  "FRAME_EVALUATION_FAILURE",
  "PAGE_ENVIRONMENT_LOSS",
  "POST_DISPATCH_METRIC_CONTRACT_FAILURE",
] as const;

export type Web06PeerMeasurementFailureCode =
  (typeof web06PeerMeasurementFailureCodes)[number];
export type Web06PeerMeasurementFailureDisposition = "SETUP_INVALID" | "BEHAVIOR_RED";
export type Web06PeerMeasurementFailureStage = "candidate" | "commit" | "frame" | "post-dispatch";

export interface Web06PeerMeasurementFailure {
  stage: Web06PeerMeasurementFailureStage;
  prefixOrdinal: number | null;
  code: Web06PeerMeasurementFailureCode;
  disposition: Web06PeerMeasurementFailureDisposition;
}

const behaviorMeasurementFailureCodes = new Set<Web06PeerMeasurementFailureCode>([
  "CANDIDATE_ENDPOINT_TIMEOUT",
  "CANDIDATE_ENDPOINT_SUPERSEDED",
  "COMMIT_ENDPOINT_TIMEOUT",
  "COMMIT_ENDPOINT_SUPERSEDED",
]);

export class Web06PeerMeasurementError extends Error {
  readonly code: Web06PeerMeasurementFailureCode;

  constructor(
    code: Web06PeerMeasurementFailureCode,
    message: string,
  ) {
    super(`${code}:${message}`);
    this.code = code;
    this.name = "Web06PeerMeasurementError";
  }
}

export function measurementFailureDisposition(
  code: Web06PeerMeasurementFailureCode,
): Web06PeerMeasurementFailureDisposition {
  return behaviorMeasurementFailureCodes.has(code) ? "BEHAVIOR_RED" : "SETUP_INVALID";
}

export function measurementFailureReason(failure: Web06PeerMeasurementFailure): string {
  return [
    "measurement-failure",
    failure.stage,
    failure.prefixOrdinal ?? "none",
    failure.code,
  ].join(":");
}

export function retainPeerMeasurementFailure(
  target: Pick<
    Web06PeerAttempt,
    "measurementFailures" | "setupInvalidReasons" | "behaviorRedReasons"
  >,
  failure: Omit<Web06PeerMeasurementFailure, "disposition">,
): void {
  const complete: Web06PeerMeasurementFailure = {
    ...failure,
    disposition: measurementFailureDisposition(failure.code),
  };
  const key = canonicalJson(complete);
  if (!target.measurementFailures.some(candidate => canonicalJson(candidate) === key)) {
    target.measurementFailures.push(complete);
  }
  const reasons = complete.disposition === "SETUP_INVALID"
    ? target.setupInvalidReasons
    : target.behaviorRedReasons;
  const reason = measurementFailureReason(complete);
  if (!reasons.includes(reason)) reasons.push(reason);
}

export function isStableObservationContractReason(reason: string): boolean {
  return stableObservationContractReasons.has(reason);
}

export const web06PeerRows = [
  {
    id: "phase4-peer/luna-short-ni",
    input: "ni",
    expectedPrefixCount: 2,
    role: "binding-short" as const,
  },
  {
    id: "phase4-peer/luna-sustained-59",
    input: "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
    expectedPrefixCount: 59,
    role: "informational-sustained" as const,
  },
] as const;

export const web06PeerFinalCandidateMembership = {
  "phase4-peer/luna-short-ni": {
    "yune-web": {
      kind: "sealed-source-bound-browser-membership",
      evidenceSha256: "c0ae16bcaac3c6abb46cb5e80b9f6d1ed0ce857ce4a106f0ea4ca495693f4c19",
      candidateCount: 6,
      nextDisabled: false,
      requiredTexts: ["你", "擬", "尼", "泥", "呢", "妳"],
    },
    "my-rime": {
      kind: "sealed-source-bound-browser-membership",
      evidenceSha256: "4d5dfc899ed3282729d19fce76fda48cf622e2b6c65ac92bbaa59d64c13a9c52",
      candidateCount: 6,
      nextDisabled: false,
      requiredTexts: ["你", "擬", "尼", "泥", "呢", "妳"],
    },
  },
  "phase4-peer/luna-sustained-59": {
    "yune-web": {
      kind: "upstream-membership-subset-plus-required-source-pinned-preflight-shape",
      source: "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json",
      sourceSha256: "cb0ae9ab7a5bac8396a60818ed6f58d9a92a2dafcc1333254c7212abb3174fce",
      oracleCommit: "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
      // The native fixture freezes the five required oracle-backed texts. The
      // browser contract separately freezes the shipped six-row page shape.
      candidateCount: comparatorPeerPageSize,
      nextDisabled: false,
      requiredTexts: ["這個引擎其實應該支持超長句子輸入才能用", "這個", "這歌", "這格", "這"],
    },
    "my-rime": {
      kind: "upstream-membership-subset-plus-required-source-pinned-preflight-shape",
      source: "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json",
      sourceSha256: "cb0ae9ab7a5bac8396a60818ed6f58d9a92a2dafcc1333254c7212abb3174fce",
      oracleCommit: "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
      candidateCount: 6,
      nextDisabled: false,
      requiredTexts: ["這個引擎其實應該支持超長句子輸入才能用", "這個", "這歌", "這格", "這"],
    },
  },
} as const;

export type Web06PeerRowId = (typeof web06PeerRows)[number]["id"];
export type Web06PeerApp = ComparatorApp;
export type Web06PeerAttemptStatus = "VALID_GREEN" | "VALID_RED" | "SETUP_INVALID";

export interface Web06PeerArtifactIdentity {
  archiveSha256: string;
  completeManifestSha256: string;
  treeSha256: string;
  fileCount: number;
  sourceCommit: string;
  sourceTree: string;
  sourceTreeState: "clean";
  buildInfoSha256?: string;
  schemaManifestSha256?: string;
  wasmSha256: string;
}

export interface Web06PeerToolchainIdentity {
  runnerSourceCommit: string;
  runnerSourceTree: string;
  runnerSourceTreeState: "clean" | "provisional-setup-only";
  nodeVersion: string;
  npmVersion: string;
  playwrightVersion: string;
  playwrightPackageLockSha256: string;
  chromiumVersion: string;
  chromiumExecutableSha256: string;
  endpointSourceSha256: string;
  browserEndpointSourceSha256: string;
  laneSourceSha256: string;
  artifactAttestorSourceSha256: string;
  launcherSourceSha256: string;
  independentLogicSourceSha256: string;
  specSourceSha256: string;
  configSourceSha256: string;
  verifierSourceSha256: string;
}

export interface Web06PeerHostIdentity {
  platform: string;
  release: string;
  architecture: string;
  cpuModel: string;
  logicalCoreCount: number;
  totalMemoryBytes: number;
  powerSource: "AC Power";
  lowPowerMode: false;
  display: {
    width: number;
    height: number;
    refreshRateHz: 60;
  };
}

export interface Web06PeerFinalRunEnvironment {
  version: "web06-final-run-environment-v1";
  environmentId: string;
  browserMode: "headed-foreground";
  nodeVersion: string;
  playwrightVersion: string;
  chromiumVersion: string;
  chromiumExecutableSha256: string;
  host: Web06PeerHostIdentity;
  viewport: typeof web06PeerViewport;
  locale: typeof web06PeerLocale;
  cacheRegime: typeof web06PeerCacheRegime;
}

export interface Web06PeerAttestation {
  version: typeof web06PeerAttestationVersion;
  phase: "phase4-peer";
  mode: "binding" | "setup-only";
  benchmarkAttempt: boolean;
  createdAt: string;
  runnerRoot: string;
  outputRoot: string;
  yuneArchivePath: string;
  peerArchivePath: string;
  yuneRoot: string;
  peerRoot: string;
  peerPacketRoot: string;
  yune: Web06PeerArtifactIdentity;
  peer: Web06PeerArtifactIdentity;
  toolchain: Web06PeerToolchainIdentity;
  host: Web06PeerHostIdentity;
  finalRunEnvironment: Web06PeerFinalRunEnvironment;
  finalRunEnvironmentSha256: string;
  identityManifest: ComparatorIdentityManifest;
  identityManifestSha256: string;
  negativeEssayControlSha256: string;
  configSha256: string;
  inputRegistrySha256: string;
  selectorManifestSha256: string;
  extraction: {
    yune: "SAFE_EXTRACTED_AND_FULLY_RECONCILED";
    peer: "SAFE_EXTRACTED_AND_FULLY_RECONCILED";
  };
}

export interface Web06PeerPostRunIntegrity {
  version: "web06-phase4-peer-post-run-integrity-v1";
  benchmarkAttempt: true;
  attestationSha256: string;
  runnerSourceCommit: string;
  runnerSourceTree: string;
  runnerSourceTreeState: "clean";
  toolchainIdentitySha256: string;
  yuneArtifactIdentitySha256: string;
  peerArtifactIdentitySha256: string;
  finalRunEnvironmentId: string;
}

export interface Web06PeerPrefixSample {
  prefixOrdinal: number;
  expectedPrefix: string;
  requestedDispatchAt: number;
  actualDriverDispatchAt: number;
  observation: ComparatorStableObservation;
  eventToStableCandidateMs: number;
}

export interface Web06PeerDriverDispatch {
  prefixOrdinal: number;
  key: string;
  phaseDeadlineAt: number;
  requestedDispatchAt: number;
  actualDriverDispatchAt: number;
  cadenceRebased: boolean;
  normalizedEventAt: number | null;
  eventDeliveredAt: number | null;
}

export interface Web06PeerFrameReceipt {
  longTaskObserverSupported: true;
  idleRafTimestamps: number[];
  idleIntervalsMs: number[];
  idleMedianMs: number;
  idleControlWindowStartAt: number;
  idleControlWindowEndAt: number;
  idleControlRafTimestamps: number[];
  observedLongTasks: Array<{ startTime: number; duration: number }>;
  idleLongTasks: Array<{ startTime: number; duration: number }>;
  interactionWindowStartAt: number;
  interactionWindowEndAt: number;
  interactionRafTimestamps: number[];
  interactionFrameIntervalsMs: number[];
  interactionLongTasks: Array<{ startTime: number; duration: number }>;
  outsideWindowLongTasks: Array<{ startTime: number; duration: number }>;
  visibilityOrFocusLost: boolean;
}

export interface Web06PeerAttempt {
  version: typeof web06PeerRawVersion;
  benchmarkAttempt: true;
  rowId: Web06PeerRowId;
  app: Web06PeerApp;
  attempt: number;
  runOrderOrdinal: number;
  freshProfile: true;
  firstKeyIncluded: true;
  warmupKeyCount: 0;
  viewport: typeof web06PeerViewport;
  locale: typeof web06PeerLocale;
  cacheRegime: typeof web06PeerCacheRegime;
  cadenceMs: typeof comparatorPeerCadenceMs;
  pageSize: typeof comparatorPeerPageSize;
  measurementStarted: boolean;
  networkRouteInstalled: boolean;
  pretypingIdleMedianMs: number | null;
  initialEventOrdinal: 0 | null;
  observedEvents: ComparatorEventBoundary[] | null;
  driverDispatches: Web06PeerDriverDispatch[];
  commitDriverDispatch: Web06PeerDriverDispatch | null;
  frame: Web06PeerFrameReceipt | null;
  prefixSamples: Web06PeerPrefixSample[];
  commit: ComparatorStableObservation | null;
  committedValue: string;
  actualDispatchGapsMs: number[];
  consoleErrors: string[];
  blockedSetupNetworkRequests: string[];
  timedAssetRequests: string[];
  unexpectedNetworkRequests: string[];
  foregroundAndFocused: boolean;
  measurementFailures: Web06PeerMeasurementFailure[];
  setupInvalidReasons: string[];
  behaviorRedReasons: string[];
  latencyRedReasons: string[];
  retainedHardRed: boolean;
  validForLatencyFrame: boolean;
  status: Web06PeerAttemptStatus;
}

export interface Web06PeerRawPacket {
  version: typeof web06PeerRawVersion;
  laneVersion: typeof web06PeerLaneVersion;
  benchmarkAttempt: true;
  phase: "phase4-peer";
  attestationSha256: string;
  configSha256: string;
  identityManifestSha256: string;
  packageAlignment: "DATA_CONFOUNDED";
  attempts: Web06PeerAttempt[];
}

interface CompactMetricSet {
  samples: number;
  medianMs: number;
  p95Ms: number;
  p99Ms: number;
  maxMs: number;
}

interface CompactFrameMetricSet extends CompactMetricSet {
  atLeast50MsCount: number;
}

interface CompactCadenceCounts {
  driverGapSamples: number;
  inRangeGapSamples: number;
  tooShortGapSamples: number;
  delayedHostGapSamples: number;
}

interface CompactLongTaskSet {
  frameWindows: number;
  interactionWindowDurationTotalMs: number;
  observedCount: number;
  observedDurationTotalMs: number;
  interactionOverlapCount: number;
  interactionTaskDurationTotalMs: number;
  interactionTaskDurationMaxMs: number;
  idleOverlapCount: number;
  outsideWindowCount: number;
}

interface CompactRoundSummary {
  attempt: number;
  status: Web06PeerAttemptStatus;
  retainedHardRed: boolean;
  verdictBearingRed: boolean;
  validForLatencyFrame: boolean;
  finalPrefix: CompactMetricSet | null;
  allPrefixes: CompactMetricSet | null;
  commit: CompactMetricSet | null;
  cadence: CompactCadenceCounts;
  frameIntervals: CompactFrameMetricSet | null;
  longTasks: CompactLongTaskSet | null;
  rawAttemptPacketSha256: string;
}

export interface Web06PeerCompactRow {
  rowId: Web06PeerRowId;
  app: Web06PeerApp;
  packageAlignment: "DATA_CONFOUNDED";
  attemptsRetained: number;
  validRounds: number;
  validGreenRounds: number;
  validRedRounds: number;
  retainedHardRedAttempts: number;
  verdictBearingRedAttempts: number;
  invalidHardRedAttempts: number;
  setupInvalidAttempts: number;
  unpairedValidAttemptsRetained: number;
  cadence: CompactCadenceCounts;
  pairedValidAttemptOrdinals: number[];
  verdict: "PASS" | "RED" | "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS";
  finalPrefix: CompactMetricSet | null;
  allPrefixes: CompactMetricSet | null;
  commit: CompactMetricSet | null;
  frameIntervals: CompactFrameMetricSet | null;
  longTasks: CompactLongTaskSet | null;
  perRound: CompactRoundSummary[];
  rawAttemptPacketSha256s: string[];
}

export interface Web06PeerCompactReceipt {
  version: typeof web06PeerCompactVersion;
  laneVersion: typeof web06PeerLaneVersion;
  phase: "phase4-peer";
  source: {
    yuneCommit: string;
    yuneTree: string;
    yuneArchiveSha256: string;
    peerUpstreamCommit: string;
    peerArtifactSourceCommit: string;
    peerArtifactSourceTree: string;
    peerArchiveSha256: string;
  };
  contracts: {
    endpoint: typeof comparatorEndpointContractVersion;
    inputRegistrySha256: string;
    selectorManifestSha256: string;
    configSha256: string;
    attestationSha256: string;
    toolchainIdentitySha256: string;
    negativeEssayControlSha256: string;
    identityManifestSha256: string;
    finalRunEnvironmentSha256: string;
    rawPacketSha256: string;
  };
  environment: {
    viewport: typeof web06PeerViewport;
    locale: typeof web06PeerLocale;
    cacheRegime: typeof web06PeerCacheRegime;
    cadenceMs: typeof comparatorPeerCadenceMs;
    nodeVersion: string;
    npmVersion: string;
    playwrightVersion: string;
    chromiumVersion: string;
    chromiumExecutableSha256: string;
    platform: string;
    architecture: string;
    environmentId: string;
  };
  packageAlignment: "DATA_CONFOUNDED";
  ratio: {
    status: "OMITTED";
    reasons: string[];
  };
  rows: Web06PeerCompactRow[];
  overallVerdict: "PASS_INFORMATIONAL_NO_RATIO" | "RED" | "SETUP_NO_GO";
}

export function peerRow(rowId: Web06PeerRowId): (typeof web06PeerRows)[number] {
  const row = web06PeerRows.find(candidate => candidate.id === rowId);
  if (!row) {
    throw new Error(`Unknown WEB-06 peer row: ${rowId}`);
  }
  return row;
}

export function peerObservationSchemaBehaviorReasons(
  observation: ComparatorStableObservation,
): string[] {
  return [observation.initial, observation.firstRaf, observation.secondRaf]
    .every(tuple => tuple.status.schemaId === web06PeerSchemaId)
    ? []
    : ["active-schema-is-not-luna-pinyin"];
}

export function peerFinalCandidateMembershipBehaviorReasons(
  rowId: Web06PeerRowId,
  app: Web06PeerApp,
  observation: ComparatorStableObservation,
): string[] {
  const observed = new Set(observation.secondRaf.candidates.map(candidate => candidate.text));
  const required = web06PeerFinalCandidateMembership[rowId][app].requiredTexts;
  return required.every(text => observed.has(text))
    ? []
    : ["frozen-final-candidate-membership-mismatch"];
}

export function peerCandidateCommitBehaviorReasons(
  finalPrefix: Web06PeerPrefixSample,
  commit: ComparatorStableObservation,
  committedValue: string,
): string[] {
  const reasons: string[] = [];
  const finalTuple = finalPrefix.observation.secondRaf;
  const selected = finalTuple.candidates[finalTuple.highlightedIndex]?.text;
  if (commit.event.ordinal !== finalPrefix.observation.event.ordinal + 1) {
    reasons.push("commit-event-does-not-immediately-follow-final-prefix");
  }
  if (commit.event.timeStamp < finalTuple.observedAt) {
    reasons.push("commit-event-precedes-coherent-final-prefix");
  }
  if (commit.event.revisionBeforeEvent < finalTuple.revision) {
    reasons.push("commit-event-revision-precedes-final-prefix");
  }
  if (!selected || committedValue !== selected || commit.secondRaf.caret.value !== selected) {
    reasons.push("commit-value-does-not-match-highlighted-final-candidate");
  }
  return reasons;
}

export function peerEventBehaviorRedReasons(
  events: ComparatorEventBoundary[],
  expectedInput: string,
): string[] {
  const reasons: string[] = [];
  if (events.length !== expectedInput.length + 1) reasons.push("event-count-mismatch");
  if (events[0]?.ordinal !== 1 || events[0]?.key !== expectedInput[0]) {
    reasons.push("first-production-event-not-counted");
  }
  return reasons;
}

export function canonicalJson(value: unknown): string {
  return `${JSON.stringify(sortJson(value), null, 2)}\n`;
}

export function sha256(value: string | Uint8Array): string {
  return createHash("sha256").update(value).digest("hex");
}

export async function retainFinalizedPeerResultAcrossCleanup<T>(
  finalized: T,
  steps: ReadonlyArray<{
    id: "browser-context-close" | "profile-remove";
    run: () => Promise<void>;
  }>,
): Promise<{ finalized: T; warnings: string[] }> {
  const warnings: string[] = [];
  for (const step of steps) {
    try {
      await step.run();
    } catch {
      warnings.push(`post-measurement-cleanup-failed:${step.id}`);
    }
  }
  return { finalized, warnings };
}

export function web06PeerInputRegistrySha256(): string {
  return sha256(canonicalJson({
    cadenceMs: comparatorPeerCadenceMs,
    cadenceRangeMs: web06PeerCadenceRangeMs,
    pageSize: comparatorPeerPageSize,
    schemaId: web06PeerSchemaId,
    rows: web06PeerRows,
    finalCandidateMembership: web06PeerFinalCandidateMembership,
    validRounds: web06PeerValidRounds,
    maxAttempts: web06PeerMaxAttempts,
    firstKeyIncluded: true,
    warmupKeyCount: 0,
  }));
}

export function web06PeerSelectorManifestSha256(): string {
  return sha256(canonicalJson(comparatorSelectorManifest));
}

export function classifyPeerAttempt(
  attempt: Omit<Web06PeerAttempt, "status" | "retainedHardRed" | "validForLatencyFrame">,
): Pick<Web06PeerAttempt, "status" | "retainedHardRed" | "validForLatencyFrame"> {
  const retainedHardRed = attempt.behaviorRedReasons.length > 0
    || attempt.latencyRedReasons.length > 0;
  const cadencePrecedenceHardRed = retainedHardRed
    && peerAttemptHasRawCadencePrecedenceHardRed(attempt);
  const blockingSetupReasons = cadencePrecedenceHardRed
    ? attempt.setupInvalidReasons.filter(reason => reason !== "invalid-cadence-too-long")
    : attempt.setupInvalidReasons;
  const validForLatencyFrame = blockingSetupReasons.length === 0;
  return {
    retainedHardRed,
    validForLatencyFrame,
    status: validForLatencyFrame
      ? retainedHardRed ? "VALID_RED" : "VALID_GREEN"
      : "SETUP_INVALID",
  };
}

export function peerAttemptHasRawCadencePrecedenceHardRed(
  attempt: Omit<Web06PeerAttempt, "status" | "retainedHardRed" | "validForLatencyFrame">,
): boolean {
  const row = peerRow(attempt.rowId);
  if (attempt.measurementFailures.some(failure => failure.disposition === "BEHAVIOR_RED")
      || attempt.consoleErrors.length > 0
      || (attempt.observedEvents !== null
        && peerEventBehaviorRedReasons(attempt.observedEvents, row.input).length > 0)) {
    return true;
  }
  const behaviorEndpointMiss = attempt.measurementFailures.some(failure =>
    failure.stage === "candidate" && failure.disposition === "BEHAVIOR_RED"
  );
  if (attempt.prefixSamples.length !== row.expectedPrefixCount && behaviorEndpointMiss) return true;
  for (const sample of attempt.prefixSamples) {
    const candidateReasons = validateCandidateObservation(
      sample.observation,
      sample.expectedPrefix,
      {
        requireYuneDiagnostic: false,
        expectedPageShape: sample.prefixOrdinal === row.expectedPrefixCount
          ? web06PeerFinalCandidateMembership[attempt.rowId][attempt.app]
          : undefined,
      },
    ).filter(reason => !isStableObservationContractReason(reason));
    if (candidateReasons.length > 0
        || peerObservationSchemaBehaviorReasons(sample.observation).length > 0
        || (sample.prefixOrdinal === row.expectedPrefixCount
          && peerFinalCandidateMembershipBehaviorReasons(
            attempt.rowId,
            attempt.app,
            sample.observation,
          ).length > 0)) {
      return true;
    }
  }
  const finalPrefix = attempt.prefixSamples.find(sample =>
    sample.prefixOrdinal === row.expectedPrefixCount
  );
  if (finalPrefix) {
    const selected = finalPrefix.observation.secondRaf.candidates[
      finalPrefix.observation.secondRaf.highlightedIndex
    ]?.text;
    if (!selected) return true;
  }
  if (attempt.commit) {
    if (validateCommitObservation(attempt.commit, attempt.committedValue)
      .some(reason => !isStableObservationContractReason(reason))
        || peerObservationSchemaBehaviorReasons(attempt.commit).length > 0
        || (finalPrefix && peerCandidateCommitBehaviorReasons(
          finalPrefix,
          attempt.commit,
          attempt.committedValue,
        ).length > 0)) {
      return true;
    }
  }
  if (attempt.app === "yune-web" && attempt.rowId === "phase4-peer/luna-short-ni") {
    if (attempt.prefixSamples.some(sample =>
      sample.eventToStableCandidateMs > web06PeerInteractiveMaxMs
    )) return true;
    if (attempt.commit
        && attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp
          > web06PeerInteractiveMaxMs) return true;
    if (attempt.frame?.interactionFrameIntervalsMs.some(value => value >= 50)
        || (attempt.frame
          && nearestRankValue(attempt.frame.interactionFrameIntervalsMs, 0.99) > 35.4)
        || attempt.frame?.interactionLongTasks.some(entry => entry.duration >= 50)) {
      return true;
    }
  }
  return false;
}

export function derivePeerSetupInvalidReasons(
  attempt: Pick<
    Web06PeerAttempt,
    | "measurementStarted"
    | "networkRouteInstalled"
    | "pretypingIdleMedianMs"
    | "frame"
    | "actualDispatchGapsMs"
    | "blockedSetupNetworkRequests"
    | "timedAssetRequests"
    | "unexpectedNetworkRequests"
    | "foregroundAndFocused"
    | "measurementFailures"
  >,
): string[] {
  const reasons: string[] = [];
  if (!attempt.measurementStarted) reasons.push("setup-before-measurement");
  if (!attempt.networkRouteInstalled) reasons.push("network-route-not-installed-before-navigation");
  if (attempt.pretypingIdleMedianMs !== null
      && (!Number.isFinite(attempt.pretypingIdleMedianMs)
        || attempt.pretypingIdleMedianMs < 15 || attempt.pretypingIdleMedianMs > 18)) {
    reasons.push("pretyping-idle-raf-median-outside-15-to-18ms");
  }
  if (attempt.frame) {
    if (!Number.isFinite(attempt.frame.idleMedianMs)
        || attempt.frame.idleMedianMs < 15 || attempt.frame.idleMedianMs > 18) {
      reasons.push("final-idle-raf-median-outside-15-to-18ms");
    }
    if (attempt.frame.visibilityOrFocusLost) {
      reasons.push("visibility-or-focus-lost-during-window");
    }
  }
  if (attempt.blockedSetupNetworkRequests.length > 0) {
    reasons.push("blocked-setup-network-request");
  }
  if (attempt.timedAssetRequests.length > 0) {
    reasons.push("asset-request-during-timed-window");
  }
  if (attempt.unexpectedNetworkRequests.length > 0) {
    reasons.push("unexpected-network-request");
  }
  reasons.push(...peerCadenceSetupReasons(attempt.actualDispatchGapsMs));
  if (!attempt.foregroundAndFocused) reasons.push("foreground-or-focus-lost");
  reasons.push(...attempt.measurementFailures
    .filter(failure => failure.disposition === "SETUP_INVALID")
    .map(measurementFailureReason));
  return [...new Set(reasons)];
}

export function peerCadenceSetupReasons(actualDriverDispatchGapsMs: number[]): string[] {
  const reasons: string[] = [];
  if (actualDriverDispatchGapsMs.some(gap =>
    !Number.isFinite(gap) || gap < web06PeerCadenceRangeMs.minimum
  )) reasons.push("invalid-cadence-too-short");
  if (actualDriverDispatchGapsMs.some(gap =>
    Number.isFinite(gap) && gap > web06PeerCadenceRangeMs.maximum
  )) reasons.push("invalid-cadence-too-long");
  return reasons;
}

export function validatePeerFrameReceipt(frame: Web06PeerFrameReceipt): string[] {
  const failures: string[] = [];
  const idleIntervals = intervalsFromTimestamps(frame.idleRafTimestamps);
  const interactionIntervals = intervalsFromTimestamps(frame.interactionRafTimestamps);
  const interactionDuration = frame.interactionWindowEndAt - frame.interactionWindowStartAt;
  const idleControlDuration = frame.idleControlWindowEndAt - frame.idleControlWindowStartAt;
  const finiteSeries = [
    ...frame.idleRafTimestamps,
    ...frame.idleIntervalsMs,
    ...frame.idleControlRafTimestamps,
    ...frame.interactionRafTimestamps,
    ...frame.interactionFrameIntervalsMs,
  ].every(value => Number.isFinite(value) && value >= 0);
  if (frame.longTaskObserverSupported !== true
      || !finiteSeries
      || frame.idleRafTimestamps.length !== 121
      || frame.idleIntervalsMs.length !== 120
      || frame.interactionRafTimestamps.length < 2
      || frame.interactionFrameIntervalsMs.length !== frame.interactionRafTimestamps.length - 1
      || frame.idleControlRafTimestamps.length < 2
      || interactionDuration <= 0 || idleControlDuration <= 0
      || Math.abs(interactionDuration - idleControlDuration) > 0.000_001
      || frame.idleRafTimestamps.at(-1) !== frame.interactionWindowStartAt
      || frame.interactionRafTimestamps[0] !== frame.interactionWindowStartAt
      || frame.interactionRafTimestamps.at(-1) !== frame.interactionWindowEndAt
      || frame.idleControlWindowStartAt !== frame.interactionWindowEndAt
      || frame.idleControlRafTimestamps[0] !== frame.idleControlWindowStartAt
      || (frame.idleControlRafTimestamps.at(-1) ?? -1) < frame.idleControlWindowEndAt
      || (frame.idleControlRafTimestamps.at(-2) ?? Number.POSITIVE_INFINITY) >= frame.idleControlWindowEndAt
      || !sameNumberSeries(idleIntervals, frame.idleIntervalsMs)
      || !sameNumberSeries(interactionIntervals, frame.interactionFrameIntervalsMs)
      || Math.abs(medianValue(frame.idleIntervalsMs) - frame.idleMedianMs) > 0.000_001) {
    failures.push("frame-window-boundary-or-duration-invalid");
  }
  const invalidTask = frame.observedLongTasks.some(entry =>
    !Number.isFinite(entry.startTime) || entry.startTime < 0
    || !Number.isFinite(entry.duration) || entry.duration < 0
  );
  if (invalidTask) failures.push("long-task-entry-invalid");
  const overlaps = (
    entry: { startTime: number; duration: number },
    start: number,
    end: number,
  ) => entry.startTime < end && entry.startTime + entry.duration > start;
  const expectedInteraction = frame.observedLongTasks.filter(entry => overlaps(
    entry, frame.interactionWindowStartAt, frame.interactionWindowEndAt,
  ));
  const expectedIdle = frame.observedLongTasks.filter(entry => overlaps(
    entry, frame.idleControlWindowStartAt, frame.idleControlWindowEndAt,
  ));
  const expectedOutside = frame.observedLongTasks.filter(entry =>
    !overlaps(entry, frame.interactionWindowStartAt, frame.interactionWindowEndAt)
    && !overlaps(entry, frame.idleControlWindowStartAt, frame.idleControlWindowEndAt)
  );
  if (!sameLongTaskMultiset(expectedInteraction, frame.interactionLongTasks)
      || !sameLongTaskMultiset(expectedIdle, frame.idleLongTasks)
      || !sameLongTaskMultiset(expectedOutside, frame.outsideWindowLongTasks)) {
    failures.push("long-task-window-classification-invalid");
  }
  return failures;
}

export function validatePeerAttempt(attempt: Web06PeerAttempt): string[] {
  const row = peerRow(attempt.rowId);
  const failures: string[] = [];
  if (attempt.attempt < 1 || attempt.attempt > web06PeerMaxAttempts) failures.push("attempt-out-of-range");
  if (!attempt.firstKeyIncluded || attempt.warmupKeyCount !== 0) failures.push("first-key-or-warmup-contract");
  if (attempt.cadenceMs !== comparatorPeerCadenceMs) failures.push("cadence-contract");
  if (attempt.pageSize !== comparatorPeerPageSize) failures.push("page-size-contract");
  if (canonicalJson(attempt.viewport) !== canonicalJson(web06PeerViewport)) failures.push("viewport-contract");
  if (attempt.locale !== web06PeerLocale || attempt.cacheRegime !== web06PeerCacheRegime) failures.push("environment-contract");
  if (typeof attempt.networkRouteInstalled !== "boolean") failures.push("network-route-installation-contract");
  if ((attempt.measurementStarted && (attempt.pretypingIdleMedianMs === null
      || !Number.isFinite(attempt.pretypingIdleMedianMs) || attempt.pretypingIdleMedianMs <= 0))
      || (attempt.pretypingIdleMedianMs !== null
        && (!Number.isFinite(attempt.pretypingIdleMedianMs) || attempt.pretypingIdleMedianMs <= 0))) {
    failures.push("pretyping-idle-median-invalid");
  }
  failures.push(...validateMeasurementFailures(attempt));
  const eventStreamFailure = attempt.measurementFailures.some(failure =>
    failure.stage === "post-dispatch" && failure.code === "EVENT_STREAM_METRIC_CONTRACT_FAILURE"
  );
  if (attempt.observedEvents === null) {
    if (attempt.measurementStarted && !eventStreamFailure
        && !attempt.measurementFailures.some(failure => failure.code === "PAGE_ENVIRONMENT_LOSS")) {
      failures.push("observed-event-stream-missing-without-failure");
    }
  } else {
    if (eventStreamFailure) failures.push("observed-event-stream-contradicts-failure");
    const expectedEventReasons = new Set(peerEventBehaviorRedReasons(attempt.observedEvents, row.input));
    for (const reason of ["event-count-mismatch", "first-production-event-not-counted"] as const) {
      if (attempt.behaviorRedReasons.includes(reason) !== expectedEventReasons.has(reason)) {
        failures.push(`${reason}-recompute`);
      }
    }
  }
  if (attempt.prefixSamples.length !== row.expectedPrefixCount) {
    const behaviorEndpointMiss = attempt.measurementFailures.some(failure =>
      failure.stage === "candidate" && failure.disposition === "BEHAVIOR_RED"
    );
    if ((attempt.status !== "SETUP_INVALID" || behaviorEndpointMiss)
        && !attempt.behaviorRedReasons.includes("exact-prefix-count")) {
      failures.push("exact-prefix-count-red-missing");
    }
  }
  if (attempt.status !== "SETUP_INVALID"
      && (!attempt.measurementStarted || attempt.initialEventOrdinal !== 0)) {
    failures.push("first-production-event-arm-contract");
  }
  if (attempt.status !== "SETUP_INVALID" && attempt.driverDispatches.length !== row.expectedPrefixCount
      && !attempt.behaviorRedReasons.includes("exact-driver-dispatch-count")) {
    failures.push("exact-driver-dispatch-count-red-missing");
  }
  for (const [index, dispatch] of attempt.driverDispatches.entries()) {
    if (dispatch.prefixOrdinal !== index + 1 || dispatch.key !== row.input[index]
        || !Number.isFinite(dispatch.phaseDeadlineAt)
        || !Number.isFinite(dispatch.requestedDispatchAt)
        || !Number.isFinite(dispatch.actualDriverDispatchAt)
        || dispatch.requestedDispatchAt < dispatch.phaseDeadlineAt
        || dispatch.actualDriverDispatchAt < dispatch.requestedDispatchAt
        || dispatch.cadenceRebased !== (
          dispatch.requestedDispatchAt > dispatch.phaseDeadlineAt
          || dispatch.actualDriverDispatchAt - dispatch.phaseDeadlineAt > 1
        )) {
      failures.push(`driver-dispatch-${index + 1}-invalid`);
    }
    if (index > 0) {
      const previous = attempt.driverDispatches[index - 1];
      const actualGap = dispatch.actualDriverDispatchAt
        - (previous?.actualDriverDispatchAt ?? dispatch.actualDriverDispatchAt);
      const expectedPhase = Math.max(
        previous?.phaseDeadlineAt ?? dispatch.phaseDeadlineAt,
        previous?.actualDriverDispatchAt ?? dispatch.phaseDeadlineAt,
      ) + comparatorPeerCadenceMs;
      if (Math.abs(dispatch.phaseDeadlineAt - expectedPhase) > 0.000_001
          && actualGap >= web06PeerCadenceRangeMs.minimum) {
        failures.push(`driver-dispatch-${index + 1}-phase-rebase`);
      }
    }
    const deliveryMissing = dispatch.normalizedEventAt === null || !Number.isFinite(dispatch.normalizedEventAt)
      || dispatch.eventDeliveredAt === null || !Number.isFinite(dispatch.eventDeliveredAt);
    if (deliveryMissing) {
      const retainedFailure = attempt.measurementFailures.some(failure =>
        failure.stage === "candidate" && failure.prefixOrdinal === index + 1
      );
      if (!retainedFailure && attempt.status !== "SETUP_INVALID") {
        failures.push(`driver-delivery-${index + 1}-red-missing`);
      }
    } else if ((dispatch.eventDeliveredAt ?? 0) < (dispatch.normalizedEventAt ?? 0)
        && !hasEndpointMetricFailure(attempt, "candidate", index + 1)) {
      failures.push(`driver-delivery-${index + 1}-invalid`);
    }
  }
  if (attempt.commitDriverDispatch) {
    const dispatch = attempt.commitDriverDispatch;
    if (dispatch.prefixOrdinal !== row.input.length + 1 || dispatch.key !== " "
        || !Number.isFinite(dispatch.phaseDeadlineAt)
        || !Number.isFinite(dispatch.requestedDispatchAt)
        || !Number.isFinite(dispatch.actualDriverDispatchAt)
        || dispatch.phaseDeadlineAt !== dispatch.requestedDispatchAt
        || dispatch.requestedDispatchAt !== dispatch.actualDriverDispatchAt
        || dispatch.cadenceRebased
        || ((dispatch.normalizedEventAt === null || dispatch.eventDeliveredAt === null)
          && !attempt.measurementFailures.some(failure => failure.stage === "commit"))
        || (dispatch.normalizedEventAt !== null && dispatch.eventDeliveredAt !== null
          && dispatch.eventDeliveredAt < dispatch.normalizedEventAt
          && !hasEndpointMetricFailure(attempt, "commit", null))) {
      failures.push("commit-driver-dispatch-invalid");
    }
  } else if (attempt.status !== "SETUP_INVALID"
      && !attempt.measurementFailures.some(failure =>
        failure.stage === "commit"
        || (failure.stage === "candidate"
          && failure.prefixOrdinal === row.expectedPrefixCount
          && failure.disposition === "BEHAVIOR_RED")
      )) {
    failures.push("commit-driver-dispatch-red-missing");
  }
  if (attempt.status !== "SETUP_INVALID" || attempt.measurementStarted) {
    if (!attempt.frame) {
      const retainedFrameFailure = attempt.measurementFailures.some(failure =>
        failure.stage === "frame" || failure.stage === "post-dispatch"
      );
      const environmentLoss = attempt.setupInvalidReasons.some(reason =>
        reason.startsWith("browser-or-foreground-loss-after-dispatch:")
      );
      if (!retainedFrameFailure && !environmentLoss) {
        failures.push("frame-window-setup-failure-missing");
      }
    } else {
      failures.push(...validatePeerFrameReceipt(attempt.frame));
    }
  }
  const seenPrefixOrdinals = new Set<number>();
  for (const sample of attempt.prefixSamples) {
    const expectedPrefix = row.input.slice(0, sample.prefixOrdinal);
    if (sample.prefixOrdinal < 1 || sample.prefixOrdinal > row.expectedPrefixCount
        || seenPrefixOrdinals.has(sample.prefixOrdinal)
        || sample.expectedPrefix !== expectedPrefix) {
      failures.push(`prefix-${sample.prefixOrdinal}-identity`);
    }
    seenPrefixOrdinals.add(sample.prefixOrdinal);
    const dispatch = attempt.driverDispatches[sample.prefixOrdinal - 1];
    if (sample.observation.event.ordinal !== sample.prefixOrdinal
        || sample.requestedDispatchAt !== dispatch?.requestedDispatchAt
        || sample.actualDriverDispatchAt !== dispatch.actualDriverDispatchAt
        || dispatch.normalizedEventAt !== sample.observation.event.timeStamp
        || dispatch.eventDeliveredAt !== (sample.observation.event.deliveredAt ?? null)) {
      if (!hasEndpointMetricFailure(attempt, "candidate", sample.prefixOrdinal)) {
        failures.push(`prefix-${sample.prefixOrdinal}-event-identity`);
      }
    }
    const expectedSelector = comparatorSelectorManifest[attempt.app].id;
    if ([sample.observation.initial, sample.observation.firstRaf, sample.observation.secondRaf]
      .some(tuple => tuple.selectorManifestId !== expectedSelector)) {
      const retainedFailure = attempt.measurementFailures.some(failure =>
        failure.stage === "candidate"
        && failure.prefixOrdinal === sample.prefixOrdinal
        && failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
      );
      if (!retainedFailure) failures.push(`prefix-${sample.prefixOrdinal}-selector-identity`);
    }
    for (const reason of validateCandidateObservation(sample.observation, expectedPrefix, {
      requireYuneDiagnostic: false,
      expectedPageShape: sample.prefixOrdinal === row.expectedPrefixCount
        ? web06PeerFinalCandidateMembership[attempt.rowId][attempt.app]
        : undefined,
    })) {
      const retainedReason = `prefix-${sample.prefixOrdinal}:${reason}`;
      if (stableObservationContractReasons.has(reason)) {
        const retainedFailure = attempt.measurementFailures.some(failure =>
          failure.stage === "candidate"
          && failure.prefixOrdinal === sample.prefixOrdinal
          && failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
          && failure.disposition === "SETUP_INVALID"
        );
        if (!retainedFailure || attempt.behaviorRedReasons.includes(retainedReason)) {
          failures.push(`${retainedReason}-measurement-contract`);
        }
      } else if (!attempt.behaviorRedReasons.includes(retainedReason)) {
        failures.push(`${retainedReason}-red-missing`);
      }
    }
    for (const reason of peerObservationSchemaBehaviorReasons(sample.observation)) {
      const retainedReason = `prefix-${sample.prefixOrdinal}:${reason}`;
      if (!attempt.behaviorRedReasons.includes(retainedReason)) {
        failures.push(`${retainedReason}-red-missing`);
      }
    }
    if (sample.prefixOrdinal === row.expectedPrefixCount) {
      for (const reason of peerFinalCandidateMembershipBehaviorReasons(
        attempt.rowId,
        attempt.app,
        sample.observation,
      )) {
        const retainedReason = `prefix-${sample.prefixOrdinal}:${reason}`;
        if (!attempt.behaviorRedReasons.includes(retainedReason)) {
          failures.push(`${retainedReason}-red-missing`);
        }
      }
    }
    const recomputed = sample.observation.secondRaf.observedAt - sample.observation.event.timeStamp;
    if (!Number.isFinite(recomputed) || recomputed < 0
        || !Number.isFinite(sample.eventToStableCandidateMs) || sample.eventToStableCandidateMs < 0
        || Math.abs(recomputed - sample.eventToStableCandidateMs) > 0.000_001) {
      if (!hasEndpointMetricFailure(attempt, "candidate", sample.prefixOrdinal)) {
        failures.push(`prefix-${sample.prefixOrdinal}-latency-recompute`);
      }
    }
  }
  if (!attempt.commit) {
    const retainedCommitFailure = attempt.measurementFailures.some(failure =>
      failure.stage === "commit"
      || (failure.stage === "candidate"
        && failure.prefixOrdinal === row.expectedPrefixCount
        && failure.disposition === "BEHAVIOR_RED")
    );
    if (attempt.status !== "SETUP_INVALID" && !retainedCommitFailure) {
      failures.push("commit-missing-red-missing");
    }
  } else {
      if (!attempt.prefixSamples.some(sample => sample.prefixOrdinal === row.expectedPrefixCount)) {
        failures.push("commit-without-coherent-final-prefix");
      }
      const expectedSelector = comparatorSelectorManifest[attempt.app].id;
      if ([attempt.commit.initial, attempt.commit.firstRaf, attempt.commit.secondRaf]
        .some(tuple => tuple.selectorManifestId !== expectedSelector)) {
        const retainedFailure = attempt.measurementFailures.some(failure =>
          failure.stage === "commit" && failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
        );
        if (!retainedFailure) failures.push("commit-selector-identity");
      }
      for (const reason of validateCommitObservation(attempt.commit, attempt.committedValue)) {
        const retainedReason = `commit:${reason}`;
        if (stableObservationContractReasons.has(reason)) {
          const retainedFailure = attempt.measurementFailures.some(failure =>
            failure.stage === "commit"
            && failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
            && failure.disposition === "SETUP_INVALID"
          );
          if (!retainedFailure || attempt.behaviorRedReasons.includes(retainedReason)) {
            failures.push(`${retainedReason}-measurement-contract`);
          }
        } else if (!attempt.behaviorRedReasons.includes(retainedReason)) {
          failures.push(`${retainedReason}-red-missing`);
        }
      }
      for (const reason of peerObservationSchemaBehaviorReasons(attempt.commit)) {
        const retainedReason = `commit:${reason}`;
        if (!attempt.behaviorRedReasons.includes(retainedReason)) {
          failures.push(`${retainedReason}-red-missing`);
        }
      }
      const finalPrefix = attempt.prefixSamples.find(sample =>
        sample.prefixOrdinal === row.expectedPrefixCount
      );
      if (finalPrefix) {
        for (const reason of peerCandidateCommitBehaviorReasons(
          finalPrefix,
          attempt.commit,
          attempt.committedValue,
        )) {
          const retainedReason = `commit-sequence:${reason}`;
          if (!attempt.behaviorRedReasons.includes(retainedReason)) {
            failures.push(`${retainedReason}-red-missing`);
          }
        }
      }
      if (attempt.commit.event.ordinal !== row.input.length + 1
          || attempt.commitDriverDispatch?.normalizedEventAt !== attempt.commit.event.timeStamp
          || attempt.commitDriverDispatch.eventDeliveredAt !== (attempt.commit.event.deliveredAt ?? null)) {
        if (!hasEndpointMetricFailure(attempt, "commit", null)) failures.push("commit-event-identity");
      }
      const commitMs = attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp;
      if ((!Number.isFinite(commitMs) || commitMs < 0)
          && !hasEndpointMetricFailure(attempt, "commit", null)) {
        failures.push("commit-latency-invalid");
      }
  }
  const recomputedDispatchGaps = attempt.driverDispatches.slice(1).map((dispatch, index) =>
    dispatch.actualDriverDispatchAt
      - (attempt.driverDispatches[index]?.actualDriverDispatchAt ?? dispatch.actualDriverDispatchAt)
  );
  if (!sameNumberSeries(attempt.actualDispatchGapsMs, recomputedDispatchGaps)) {
    failures.push("dispatch-gap-recompute");
  }
  const expectedCadenceReasons = peerCadenceSetupReasons(recomputedDispatchGaps);
  const declaredCadenceReasons = attempt.setupInvalidReasons.filter(reason =>
    reason === "invalid-cadence-too-short" || reason === "invalid-cadence-too-long"
  );
  if (canonicalJson(expectedCadenceReasons) !== canonicalJson(declaredCadenceReasons)) {
    failures.push("cadence-setup-reason-recompute");
  }
  const expectedSetupReasons = derivePeerSetupInvalidReasons(attempt);
  if (new Set(attempt.setupInvalidReasons).size !== attempt.setupInvalidReasons.length
      || canonicalJson([...expectedSetupReasons].sort())
        !== canonicalJson([...attempt.setupInvalidReasons].sort())) {
    failures.push("setup-invalid-reasons-exact-recompute");
  }
  if ((attempt.consoleErrors.length > 0)
      !== attempt.behaviorRedReasons.includes("console-or-page-error")) {
    failures.push("console-error-behavior-reason-recompute");
  }
  if (attempt.measurementStarted && attempt.app === "yune-web"
      && attempt.rowId === "phase4-peer/luna-short-ni") {
    const expectedLatencyReds = new Set<string>();
    for (const sample of attempt.prefixSamples) {
      if (sample.eventToStableCandidateMs > web06PeerInteractiveMaxMs) {
        expectedLatencyReds.add(`short-prefix-${sample.prefixOrdinal}-over-67ms`);
      }
    }
    const commitMs = attempt.commit
      ? attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp
      : undefined;
    if (commitMs !== undefined && commitMs > web06PeerInteractiveMaxMs) {
      expectedLatencyReds.add("short-commit-over-67ms");
    }
    if (attempt.frame) {
      if (attempt.frame.interactionFrameIntervalsMs.some(value => value >= 50)) {
        expectedLatencyReds.add("short-frame-interval-at-least-50ms");
      }
      if (nearestRankValue(attempt.frame.interactionFrameIntervalsMs, 0.99) > 35.4) {
        expectedLatencyReds.add("short-frame-p99-over-35.4ms");
      }
      if (attempt.frame.interactionLongTasks.some(entry => entry.duration >= 50)) {
        expectedLatencyReds.add("short-long-task-at-least-50ms");
      }
    }
    if (canonicalJson([...expectedLatencyReds].sort()) !== canonicalJson([...attempt.latencyRedReasons].sort())) {
      failures.push("short-latency-red-recompute");
    }
  }
  const classified = classifyPeerAttempt(attempt);
  if (attempt.status !== classified.status
      || attempt.retainedHardRed !== classified.retainedHardRed
      || attempt.validForLatencyFrame !== classified.validForLatencyFrame) {
    failures.push("attempt-dimensions-not-recomputed");
  }
  return [...new Set(failures)];
}

function validateMeasurementFailures(attempt: Web06PeerAttempt): string[] {
  const failures: string[] = [];
  const knownCodes = new Set<string>(web06PeerMeasurementFailureCodes);
  const canonicalReasons = new Set<string>();
  const seen = new Set<string>();
  for (const failure of attempt.measurementFailures) {
    const identity = canonicalJson(failure);
    if (seen.has(identity)) failures.push("measurement-failure-duplicate");
    seen.add(identity);
    if (!knownCodes.has(failure.code)) failures.push("measurement-failure-code-invalid");
    if (failure.disposition !== measurementFailureDisposition(failure.code)) {
      failures.push("measurement-failure-disposition-invalid");
    }
    const candidateCode = failure.code.startsWith("CANDIDATE_")
      || failure.code === "ENDPOINT_OBSERVER_MISSING"
      || failure.code === "ENDPOINT_OBSERVER_WRONG_APP"
      || failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
      || failure.code === "PAGE_ENVIRONMENT_LOSS";
    const commitCode = failure.code.startsWith("COMMIT_");
    const frameCode = failure.code.startsWith("FRAME_");
    if (failure.stage === "candidate") {
      if (!candidateCode || !Number.isInteger(failure.prefixOrdinal)
          || (failure.prefixOrdinal ?? 0) < 1
          || (failure.prefixOrdinal ?? 0) > peerRow(attempt.rowId).expectedPrefixCount) {
        failures.push("measurement-failure-candidate-identity-invalid");
      }
      if ((failure.code === "CANDIDATE_ENDPOINT_TIMEOUT"
          || failure.code === "CANDIDATE_ENDPOINT_SUPERSEDED"
          || failure.code === "ENDPOINT_OBSERVER_MISSING"
          || failure.code === "ENDPOINT_OBSERVER_WRONG_APP"
          || failure.code === "PAGE_ENVIRONMENT_LOSS")
          && attempt.prefixSamples.some(sample => sample.prefixOrdinal === failure.prefixOrdinal)) {
        failures.push("measurement-failure-candidate-contradicts-sample");
      }
      if (failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE") {
        const sample = attempt.prefixSamples.find(item => item.prefixOrdinal === failure.prefixOrdinal);
        if (sample) {
          const expectedSelector = comparatorSelectorManifest[attempt.app].id;
          const detectable = validateCandidateObservation(
            sample.observation,
            sample.expectedPrefix,
            { requireYuneDiagnostic: false },
          ).some(isStableObservationContractReason)
            || [sample.observation.initial, sample.observation.firstRaf, sample.observation.secondRaf]
              .some(tuple => tuple.selectorManifestId !== expectedSelector);
          if (!detectable) failures.push("measurement-failure-candidate-metric-unproved");
        }
      }
    } else if (failure.stage === "commit") {
      if ((!commitCode && failure.code !== "ENDPOINT_OBSERVER_MISSING"
          && failure.code !== "ENDPOINT_OBSERVER_WRONG_APP"
          && failure.code !== "ENDPOINT_METRIC_CONTRACT_FAILURE"
          && failure.code !== "PAGE_ENVIRONMENT_LOSS")
          || failure.prefixOrdinal !== null) {
        failures.push("measurement-failure-commit-identity-invalid");
      }
      if ((failure.code === "COMMIT_ENDPOINT_TIMEOUT"
          || failure.code === "COMMIT_ENDPOINT_SUPERSEDED"
          || failure.code === "ENDPOINT_OBSERVER_MISSING"
          || failure.code === "ENDPOINT_OBSERVER_WRONG_APP"
          || failure.code === "PAGE_ENVIRONMENT_LOSS") && attempt.commit) {
        failures.push("measurement-failure-commit-contradicts-observation");
      }
      if (failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE" && attempt.commit) {
        const expectedSelector = comparatorSelectorManifest[attempt.app].id;
        const detectable = validateCommitObservation(attempt.commit, attempt.committedValue)
          .some(isStableObservationContractReason)
          || [attempt.commit.initial, attempt.commit.firstRaf, attempt.commit.secondRaf]
            .some(tuple => tuple.selectorManifestId !== expectedSelector);
        if (!detectable) failures.push("measurement-failure-commit-metric-unproved");
      }
    } else if (failure.stage === "frame") {
      if (!frameCode || failure.prefixOrdinal !== null || attempt.frame !== null) {
        failures.push("measurement-failure-frame-identity-invalid");
      }
    } else if (failure.stage === "post-dispatch") {
      if ((failure.code !== "POST_DISPATCH_METRIC_CONTRACT_FAILURE"
          && failure.code !== "EVENT_STREAM_METRIC_CONTRACT_FAILURE"
          && failure.code !== "PAGE_ENVIRONMENT_LOSS")
          || failure.prefixOrdinal !== null) {
        failures.push("measurement-failure-post-dispatch-identity-invalid");
      }
      if (failure.code === "POST_DISPATCH_METRIC_CONTRACT_FAILURE"
          && attempt.frame && attempt.commit
          && attempt.prefixSamples.length === peerRow(attempt.rowId).expectedPrefixCount) {
        failures.push("measurement-failure-post-dispatch-unproved");
      }
    } else {
      failures.push("measurement-failure-stage-invalid");
    }
    const reason = measurementFailureReason(failure);
    canonicalReasons.add(reason);
    const owningReasons = failure.disposition === "SETUP_INVALID"
      ? attempt.setupInvalidReasons
      : attempt.behaviorRedReasons;
    const otherReasons = failure.disposition === "SETUP_INVALID"
      ? attempt.behaviorRedReasons
      : attempt.setupInvalidReasons;
    if (!owningReasons.includes(reason) || otherReasons.includes(reason)) {
      failures.push("measurement-failure-reason-dimension-invalid");
    }
  }
  if (attempt.measurementFailures.some(failure => failure.code === "PAGE_ENVIRONMENT_LOSS")
      && attempt.foregroundAndFocused) {
    failures.push("measurement-failure-environment-loss-contradicts-foreground");
  }
  for (const reason of [...attempt.setupInvalidReasons, ...attempt.behaviorRedReasons]) {
    if (reason.startsWith("measurement-failure:") && !canonicalReasons.has(reason)) {
      failures.push("measurement-failure-reason-orphaned");
    }
    if (reason.startsWith("post-dispatch-contract-exception")
        || reason === "frame-window-missing") {
      failures.push("legacy-post-dispatch-failure-classification");
    }
  }
  return failures;
}

function hasEndpointMetricFailure(
  attempt: Web06PeerAttempt,
  stage: "candidate" | "commit",
  prefixOrdinal: number | null,
): boolean {
  return attempt.measurementFailures.some(failure =>
    failure.stage === stage && failure.prefixOrdinal === prefixOrdinal
    && failure.code === "ENDPOINT_METRIC_CONTRACT_FAILURE"
    && failure.disposition === "SETUP_INVALID"
  );
}

export function buildPeerCompactReceipt(
  packet: Web06PeerRawPacket,
  attestation: Web06PeerAttestation,
  rawPacketSha256: string,
  attemptPacketSha256s: Map<string, string>,
): Web06PeerCompactReceipt {
  if (packet.packageAlignment !== "DATA_CONFOUNDED") {
    throw new Error("The pinned WEB-06 peer packet must remain DATA_CONFOUNDED");
  }
  const alignment = evaluatePackageAlignment(attestation.identityManifest);
  if (alignment.packageAlignment !== "DATA_CONFOUNDED") {
    throw new Error("The pinned peer identity unexpectedly became ratio-eligible");
  }
  if (packet.configSha256 !== attestation.configSha256
      || packet.identityManifestSha256 !== attestation.identityManifestSha256) {
    throw new Error("Raw peer packet is not bound to its config/identity attestation");
  }
  validateGlobalRunOrder(packet.attempts);
  const expectedGroups = web06PeerRows.flatMap(row =>
    (["yune-web", "my-rime"] as const).map(app => ({ rowId: row.id, app }))
  );
  const pairedValidByRow = new Map(web06PeerRows.map(row => [
    row.id,
    pairedValidAttemptOrdinals(packet.attempts, row.id),
  ]));
  const rows = expectedGroups.map(({ rowId, app }) => {
    const attempts = packet.attempts.filter(attempt => attempt.rowId === rowId && attempt.app === app);
    const pairedValidAttemptOrdinals = pairedValidByRow.get(rowId) ?? [];
    const pairedSet = new Set(pairedValidAttemptOrdinals);
    const valid = attempts.filter(attempt => pairedSet.has(attempt.attempt));
    const greens = valid.filter(attempt => !attempt.retainedHardRed);
    const reds = valid.filter(attempt => attempt.retainedHardRed);
    const retainedHardReds = attempts.filter(attempt => attempt.retainedHardRed);
    const verdictBearingReds = attempts.filter(peerAttemptHasVerdictBearingRed);
    const finalPrefixValues = valid.flatMap(attempt => {
      const expectedPrefixCount = peerRow(rowId).expectedPrefixCount;
      const sample = attempt.prefixSamples.find(item => item.prefixOrdinal === expectedPrefixCount);
      return sample ? [sample.eventToStableCandidateMs] : [];
    });
    const allPrefixValues = valid.flatMap(attempt =>
      attempt.prefixSamples.map(sample => sample.eventToStableCandidateMs)
    );
    const commitValues = valid.flatMap(attempt => attempt.commit
      ? [attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp]
      : []);
    const driverGaps = attempts.flatMap(attempt => attempt.actualDispatchGapsMs);
    const validFrames = valid.flatMap(attempt => attempt.frame ? [attempt.frame] : []);
    const rawAttemptPacketSha256s = attempts.map(attempt => {
      const key = attemptKey(attempt);
      const digest = attemptPacketSha256s.get(key);
      if (!digest) throw new Error(`Missing raw attempt checksum: ${key}`);
      return digest;
    });
    const verdict = valid.length < web06PeerValidRounds
      ? "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS" as const
      : verdictBearingReds.length > 0 ? "RED" as const : "PASS" as const;
    return {
      rowId,
      app,
      packageAlignment: "DATA_CONFOUNDED" as const,
      attemptsRetained: attempts.length,
      validRounds: valid.length,
      validGreenRounds: greens.length,
      validRedRounds: reds.length,
      retainedHardRedAttempts: retainedHardReds.length,
      verdictBearingRedAttempts: verdictBearingReds.length,
      invalidHardRedAttempts: attempts.filter(attempt =>
        attempt.retainedHardRed && !attempt.validForLatencyFrame
      ).length,
      setupInvalidAttempts: attempts.filter(attempt => !attempt.validForLatencyFrame).length,
      unpairedValidAttemptsRetained: attempts.filter(attempt =>
        attempt.validForLatencyFrame && !pairedSet.has(attempt.attempt)
      ).length,
      cadence: compactCadenceCounts(driverGaps),
      pairedValidAttemptOrdinals,
      verdict,
      finalPrefix: metricSet(finalPrefixValues),
      allPrefixes: metricSet(allPrefixValues),
      commit: metricSet(commitValues),
      frameIntervals: frameMetricSet(validFrames.flatMap(frame =>
        frame.interactionFrameIntervalsMs
      )),
      longTasks: longTaskSet(validFrames),
      perRound: attempts.map((attempt, index) => {
        const finalSample = attempt.prefixSamples.find(sample =>
          sample.prefixOrdinal === peerRow(rowId).expectedPrefixCount
        );
        return {
          attempt: attempt.attempt,
          status: attempt.status,
          retainedHardRed: attempt.retainedHardRed,
          verdictBearingRed: peerAttemptHasVerdictBearingRed(attempt),
          validForLatencyFrame: attempt.validForLatencyFrame,
          finalPrefix: metricSet(finalSample ? [finalSample.eventToStableCandidateMs] : []),
          allPrefixes: metricSet(attempt.prefixSamples.map(sample => sample.eventToStableCandidateMs)),
          commit: metricSet(attempt.commit
            ? [attempt.commit.secondRaf.observedAt - attempt.commit.event.timeStamp]
            : []),
          cadence: compactCadenceCounts(attempt.actualDispatchGapsMs),
          frameIntervals: frameMetricSet(attempt.frame?.interactionFrameIntervalsMs ?? []),
          longTasks: longTaskSet(attempt.frame ? [attempt.frame] : []),
          rawAttemptPacketSha256: rawAttemptPacketSha256s[index] ?? "",
        };
      }),
      rawAttemptPacketSha256s,
    };
  });
  const overallVerdict = rows.some(row => row.verdict === "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS")
    ? "SETUP_NO_GO" as const
    : rows.some(row => row.verdict === "RED")
      ? "RED" as const
      : "PASS_INFORMATIONAL_NO_RATIO" as const;
  const receipt: Web06PeerCompactReceipt = {
    version: web06PeerCompactVersion,
    laneVersion: web06PeerLaneVersion,
    phase: "phase4-peer",
    source: {
      yuneCommit: attestation.yune.sourceCommit,
      yuneTree: attestation.yune.sourceTree,
      yuneArchiveSha256: attestation.yune.archiveSha256,
      peerUpstreamCommit: attestation.identityManifest.peer.upstreamPinnedCommit,
      peerArtifactSourceCommit: attestation.identityManifest.peer.artifactSourceCommit,
      peerArtifactSourceTree: attestation.identityManifest.peer.artifactSourceTree,
      peerArchiveSha256: attestation.peer.archiveSha256,
    },
    contracts: {
      endpoint: comparatorEndpointContractVersion,
      inputRegistrySha256: attestation.inputRegistrySha256,
      selectorManifestSha256: attestation.selectorManifestSha256,
      configSha256: attestation.configSha256,
      attestationSha256: packet.attestationSha256,
      toolchainIdentitySha256: sha256(canonicalJson(attestation.toolchain)),
      negativeEssayControlSha256: attestation.negativeEssayControlSha256,
      identityManifestSha256: attestation.identityManifestSha256,
      finalRunEnvironmentSha256: attestation.finalRunEnvironmentSha256,
      rawPacketSha256,
    },
    environment: {
      viewport: web06PeerViewport,
      locale: web06PeerLocale,
      cacheRegime: web06PeerCacheRegime,
      cadenceMs: comparatorPeerCadenceMs,
      nodeVersion: attestation.toolchain.nodeVersion,
      npmVersion: attestation.toolchain.npmVersion,
      playwrightVersion: attestation.toolchain.playwrightVersion,
      chromiumVersion: attestation.toolchain.chromiumVersion,
      chromiumExecutableSha256: attestation.toolchain.chromiumExecutableSha256,
      platform: attestation.host.platform,
      architecture: attestation.host.architecture,
      environmentId: attestation.finalRunEnvironment.environmentId,
    },
    packageAlignment: "DATA_CONFOUNDED",
    ratio: {
      status: "OMITTED",
      reasons: [...new Set([
        ...alignment.reasons,
        "pinned-peer-packet-declares-data-confounded",
        "numeric-ratio-fields-structurally-omitted",
      ])].sort(),
    },
    rows,
    overallVerdict,
  };
  assertDataConfoundedRatioOmission(receipt);
  assertCompactIdentity(receipt);
  assertPublicEvidencePrivacy(receipt);
  return receipt;
}

function validateGlobalRunOrder(attempts: Web06PeerAttempt[]): void {
  const seenKeys = new Set<string>();
  attempts.forEach((attempt, index) => {
    const key = attemptKey(attempt);
    if (seenKeys.has(key)) throw new Error(`raw-attempt-identity-duplicate:${key}`);
    seenKeys.add(key);
    if (!Number.isInteger(attempt.runOrderOrdinal) || attempt.runOrderOrdinal !== index + 1) {
      throw new Error(`raw-run-order-not-contiguous:index-${index + 1}:ordinal-${attempt.runOrderOrdinal}`);
    }
  });
  let cursor = 0;
  for (const row of web06PeerRows) {
    let expectedAttempt = 1;
    while (cursor < attempts.length && attempts[cursor]?.rowId === row.id) {
      const first = attempts[cursor];
      const second = attempts[cursor + 1];
      const expectedApps: Web06PeerApp[] = expectedAttempt % 2 === 1
        ? ["yune-web", "my-rime"]
        : ["my-rime", "yune-web"];
      if (!first || !second || first.rowId !== row.id || second.rowId !== row.id
          || first.attempt !== expectedAttempt || second.attempt !== expectedAttempt
          || first.app !== expectedApps[0] || second.app !== expectedApps[1]) {
        throw new Error(`raw-frozen-row-pair-order:${row.id}:attempt-${expectedAttempt}`);
      }
      cursor += 2;
      expectedAttempt += 1;
    }
    if (expectedAttempt === 1) throw new Error(`raw-frozen-row-missing:${row.id}`);
  }
  if (cursor !== attempts.length) {
    throw new Error(`raw-frozen-row-order-extra:index-${cursor + 1}`);
  }
}

export function assertCompactIdentity(receipt: Web06PeerCompactReceipt): void {
  const sha40 = /^[0-9a-f]{40}$/;
  const sha64 = /^[0-9a-f]{64}$/;
  const source40 = [
    receipt.source.yuneCommit,
    receipt.source.yuneTree,
    receipt.source.peerUpstreamCommit,
    receipt.source.peerArtifactSourceCommit,
    receipt.source.peerArtifactSourceTree,
  ];
  const source64 = [receipt.source.yuneArchiveSha256, receipt.source.peerArchiveSha256];
  const contract64 = Object.entries(receipt.contracts)
    .filter(([name]) => name !== "endpoint")
    .map(([, value]) => value);
  if (source40.some(value => !sha40.test(value))
      || [...source64, ...contract64, receipt.environment.chromiumExecutableSha256,
        receipt.environment.environmentId].some(value => !sha64.test(value))) {
    throw new Error("Compact peer receipt contains a malformed source/artifact/toolchain/config hash");
  }
  const safeVersion = /^[A-Za-z0-9][A-Za-z0-9 .()+_-]{0,119}$/;
  if (![receipt.environment.nodeVersion, receipt.environment.npmVersion,
    receipt.environment.playwrightVersion, receipt.environment.chromiumVersion]
    .every(value => safeVersion.test(value))
      || !/^(darwin|linux|win32)$/.test(receipt.environment.platform)
      || !/^(arm64|x64)$/.test(receipt.environment.architecture)) {
    throw new Error("Compact peer receipt contains an unsafe or malformed toolchain/host identity");
  }
  for (const row of receipt.rows) {
    if (row.packageAlignment !== "DATA_CONFOUNDED"
        || row.rawAttemptPacketSha256s.length !== row.attemptsRetained
        || row.rawAttemptPacketSha256s.some(value => !sha64.test(value))
        || row.pairedValidAttemptOrdinals.some(value => !Number.isInteger(value) || value < 1 || value > 7)
        || [row.attemptsRetained, row.validRounds, row.validGreenRounds, row.validRedRounds,
          row.retainedHardRedAttempts, row.invalidHardRedAttempts,
          row.verdictBearingRedAttempts,
          row.setupInvalidAttempts, row.unpairedValidAttemptsRetained,
          row.cadence.driverGapSamples, row.cadence.inRangeGapSamples,
          row.cadence.tooShortGapSamples, row.cadence.delayedHostGapSamples]
          .some(value => !Number.isInteger(value) || value < 0)
        || row.verdictBearingRedAttempts > row.retainedHardRedAttempts
        || row.validRedRounds > row.verdictBearingRedAttempts
        || row.verdict !== (row.validRounds < web06PeerValidRounds
          ? "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS"
          : row.verdictBearingRedAttempts > 0 ? "RED" : "PASS")
        || !compactCadenceCountsValid(row.cadence)
        || [row.finalPrefix, row.allPrefixes, row.commit]
          .some(metric => !compactMetricSetValid(metric))
        || !compactFrameMetricSetValid(row.frameIntervals)
        || !compactLongTaskSetValid(row.longTasks)
        || row.perRound.length !== row.attemptsRetained
        || row.perRound.some((round, index) =>
          round.attempt !== index + 1
          || !["VALID_GREEN", "VALID_RED", "SETUP_INVALID"].includes(round.status)
          || typeof round.retainedHardRed !== "boolean"
          || typeof round.verdictBearingRed !== "boolean"
          || typeof round.validForLatencyFrame !== "boolean"
          || !compactMetricSetValid(round.finalPrefix)
          || !compactMetricSetValid(round.allPrefixes)
          || !compactMetricSetValid(round.commit)
          || !compactCadenceCountsValid(round.cadence)
          || !compactFrameMetricSetValid(round.frameIntervals)
          || !compactLongTaskSetValid(round.longTasks)
          || (round.frameIntervals === null) !== (round.longTasks === null)
          || !sha64.test(round.rawAttemptPacketSha256)
          || round.rawAttemptPacketSha256 !== row.rawAttemptPacketSha256s[index])) {
      throw new Error("Compact peer row identity/hash contract failed");
    }
  }
  if (receipt.ratio.reasons.some(reason => !/^[a-z0-9:-]+$/.test(reason))) {
    throw new Error("Compact peer receipt contains an unsafe alignment reason");
  }
}

function peerAttemptHasVerdictBearingRed(attempt: Web06PeerAttempt): boolean {
  return attempt.measurementStarted && (
    attempt.behaviorRedReasons.length > 0
    || (attempt.validForLatencyFrame && attempt.latencyRedReasons.length > 0)
  );
}

function compactMetricSetValid(metric: CompactMetricSet | null): boolean {
  return metric === null || (
    Number.isInteger(metric.samples) && metric.samples >= 1
    && [metric.medianMs, metric.p95Ms, metric.p99Ms, metric.maxMs]
      .every(value => Number.isFinite(value) && value >= 0)
    && metric.medianMs <= metric.p95Ms
    && metric.p95Ms <= metric.p99Ms
    && metric.p99Ms <= metric.maxMs
  );
}

function compactFrameMetricSetValid(metric: CompactFrameMetricSet | null): boolean {
  return metric === null || (
    compactMetricSetValid(metric)
    && Number.isInteger(metric.atLeast50MsCount)
    && metric.atLeast50MsCount >= 0
    && metric.atLeast50MsCount <= metric.samples
  );
}

function compactCadenceCountsValid(cadence: CompactCadenceCounts): boolean {
  return [cadence.driverGapSamples, cadence.inRangeGapSamples,
    cadence.tooShortGapSamples, cadence.delayedHostGapSamples]
    .every(value => Number.isInteger(value) && value >= 0)
    && cadence.driverGapSamples === cadence.inRangeGapSamples
      + cadence.tooShortGapSamples + cadence.delayedHostGapSamples;
}

function compactLongTaskSetValid(tasks: CompactLongTaskSet | null): boolean {
  if (tasks === null) return true;
  const counts = [tasks.frameWindows, tasks.observedCount, tasks.interactionOverlapCount,
    tasks.idleOverlapCount, tasks.outsideWindowCount];
  const durations = [tasks.interactionWindowDurationTotalMs, tasks.observedDurationTotalMs,
    tasks.interactionTaskDurationTotalMs, tasks.interactionTaskDurationMaxMs];
  return counts.every(value => Number.isInteger(value) && value >= 0)
    && tasks.frameWindows >= 1
    && durations.every(value => Number.isFinite(value) && value >= 0)
    && tasks.interactionTaskDurationMaxMs <= tasks.interactionTaskDurationTotalMs
    && (tasks.interactionOverlapCount !== 0
      || (tasks.interactionTaskDurationTotalMs === 0
        && tasks.interactionTaskDurationMaxMs === 0));
}

export function assertDataConfoundedRatioOmission(receipt: Web06PeerCompactReceipt): void {
  if (receipt.packageAlignment !== "DATA_CONFOUNDED" || receipt.ratio.status !== "OMITTED") {
    throw new Error("DATA_CONFOUNDED must force ratioStatus=OMITTED");
  }
  const keys = objectKeysDeep(receipt.ratio);
  const numericRatioKeys = keys.filter(key => /(?:p\d+.*ratio|ratio.*(?:value|numeric|milliseconds|ms))/i.test(key));
  if (numericRatioKeys.length > 0) {
    throw new Error(`DATA_CONFOUNDED ratio contains forbidden fields: ${numericRatioKeys.join(", ")}`);
  }
  if (containsNumber(receipt.ratio)) {
    throw new Error("DATA_CONFOUNDED ratio object contains a forbidden numeric value");
  }
}

export function assertPublicEvidencePrivacy(value: unknown): void {
  for (const item of stringValues(value)) {
    const forbidden = item.startsWith("/")
      || /^[A-Za-z]:[\\/]/.test(item)
      || /^\\\\/.test(item)
      || /^\/\/[^/]/.test(item)
      || /^file:\/\//i.test(item)
      || /YUNE_WEB06_[A-Z0-9_]+/.test(item)
      || /(?:token|password|secret|authorization)["'=:\s]+\S+/i.test(item);
    if (forbidden) throw new Error("Public WEB-06 peer receipt contains an absolute path, env name, or secret-shaped value");
  }
}

function stringValues(value: unknown): string[] {
  if (typeof value === "string") return [value];
  if (Array.isArray(value)) return value.flatMap(stringValues);
  return value !== null && typeof value === "object"
    ? Object.values(value as Record<string, unknown>).flatMap(stringValues)
    : [];
}

export function attemptKey(attempt: Pick<Web06PeerAttempt, "rowId" | "app" | "attempt">): string {
  const rowFileId = attempt.rowId.replaceAll("/", "--");
  return `${rowFileId}--${attempt.app}--attempt-${String(attempt.attempt).padStart(2, "0")}`;
}

function validateAttemptSeries(
  attempts: Web06PeerAttempt[],
  rowId: Web06PeerRowId,
  app: Web06PeerApp,
): void {
  if (attempts.length > web06PeerMaxAttempts) throw new Error(`${rowId}/${app}: more than seven attempts`);
  attempts.forEach((attempt, index) => {
    if (attempt.attempt !== index + 1) throw new Error(`${rowId}/${app}: attempt numbers are not contiguous`);
    const failures = validatePeerAttempt(attempt);
    if (failures.length > 0) throw new Error(`${rowId}/${app}/attempt-${attempt.attempt}: ${failures.join("; ")}`);
  });
}

function pairedValidAttemptOrdinals(
  allAttempts: Web06PeerAttempt[],
  rowId: Web06PeerRowId,
): number[] {
  const yune = allAttempts.filter(attempt => attempt.rowId === rowId && attempt.app === "yune-web");
  const peer = allAttempts.filter(attempt => attempt.rowId === rowId && attempt.app === "my-rime");
  validateAttemptSeries(yune, rowId, "yune-web");
  validateAttemptSeries(peer, rowId, "my-rime");
  if (yune.length !== peer.length) throw new Error(`${rowId}: retained peer pairs are incomplete`);
  const paired: number[] = [];
  for (let index = 0; index < yune.length; index += 1) {
    const yuneAttempt = yune[index];
    const peerAttempt = peer[index];
    if (!yuneAttempt || !peerAttempt || yuneAttempt.attempt !== peerAttempt.attempt) {
      throw new Error(`${rowId}: retained peer pair identity is misaligned`);
    }
    if (paired.length >= web06PeerValidRounds) {
      throw new Error(`${rowId}: pair ${index + 1} ran after five paired valid rounds`);
    }
    const yuneFirst = yuneAttempt.runOrderOrdinal < peerAttempt.runOrderOrdinal;
    if (yuneFirst !== (yuneAttempt.attempt % 2 === 1)) {
      throw new Error(`${rowId}: pair ${index + 1} violates frozen counterbalancing`);
    }
    if (yuneAttempt.validForLatencyFrame && peerAttempt.validForLatencyFrame) {
      paired.push(yuneAttempt.attempt);
    }
  }
  if (paired.length < web06PeerValidRounds && yune.length !== web06PeerMaxAttempts) {
    throw new Error(`${rowId}: lane ended before five paired valid rounds or seven retained pairs`);
  }
  return paired;
}

function metricSet(values: number[]): CompactMetricSet | null {
  if (values.length === 0) return null;
  if (values.some(value => !Number.isFinite(value) || value < 0)) {
    throw new Error("Compact peer metric input contains a negative or nonfinite duration");
  }
  const sorted = values.slice().sort((a, b) => a - b);
  return {
    samples: sorted.length,
    medianMs: nearestRank(sorted, 0.5),
    p95Ms: nearestRank(sorted, 0.95),
    p99Ms: nearestRank(sorted, 0.99),
    maxMs: sorted.at(-1) ?? 0,
  };
}

function frameMetricSet(values: number[]): CompactFrameMetricSet | null {
  const metrics = metricSet(values);
  return metrics ? {
    ...metrics,
    atLeast50MsCount: values.filter(value => value >= 50).length,
  } : null;
}

function compactCadenceCounts(values: number[]): CompactCadenceCounts {
  return {
    driverGapSamples: values.length,
    inRangeGapSamples: values.filter(value =>
      value >= web06PeerCadenceRangeMs.minimum
      && value <= web06PeerCadenceRangeMs.maximum
    ).length,
    tooShortGapSamples: values.filter(value =>
      value < web06PeerCadenceRangeMs.minimum
    ).length,
    delayedHostGapSamples: values.filter(value =>
      value > web06PeerCadenceRangeMs.maximum
    ).length,
  };
}

function longTaskSet(frames: Web06PeerFrameReceipt[]): CompactLongTaskSet | null {
  if (frames.length === 0) return null;
  const observed = frames.flatMap(frame => frame.observedLongTasks);
  const interaction = frames.flatMap(frame => frame.interactionLongTasks);
  return {
    frameWindows: frames.length,
    interactionWindowDurationTotalMs: frames.reduce(
      (sum, frame) => sum + frame.interactionWindowEndAt - frame.interactionWindowStartAt,
      0,
    ),
    observedCount: observed.length,
    observedDurationTotalMs: observed.reduce((sum, entry) => sum + entry.duration, 0),
    interactionOverlapCount: interaction.length,
    interactionTaskDurationTotalMs: interaction.reduce((sum, entry) => sum + entry.duration, 0),
    interactionTaskDurationMaxMs: Math.max(0, ...interaction.map(entry => entry.duration)),
    idleOverlapCount: frames.reduce((sum, frame) => sum + frame.idleLongTasks.length, 0),
    outsideWindowCount: frames.reduce((sum, frame) => sum + frame.outsideWindowLongTasks.length, 0),
  };
}

function intervalsFromTimestamps(timestamps: number[]): number[] {
  return timestamps.slice(1).map((timestamp, index) => timestamp - (timestamps[index] ?? timestamp));
}

function sameNumberSeries(left: number[], right: number[]): boolean {
  return left.length === right.length && left.every((value, index) =>
    Number.isFinite(value) && Number.isFinite(right[index]) && Math.abs(value - (right[index] ?? value)) <= 0.000_001
  );
}

function sameLongTaskMultiset(
  left: Array<{ startTime: number; duration: number }>,
  right: Array<{ startTime: number; duration: number }>,
): boolean {
  const normalized = (values: Array<{ startTime: number; duration: number }>) => values
    .map(value => `${value.startTime}\0${value.duration}`)
    .sort();
  return canonicalJson(normalized(left)) === canonicalJson(normalized(right));
}

function nearestRank(sorted: number[], percentile: number): number {
  const index = Math.ceil((sorted.length - 1) * percentile);
  return sorted[index] ?? 0;
}

export function medianValue(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = values.slice().sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0
    ? ((sorted[middle - 1] ?? 0) + (sorted[middle] ?? 0)) / 2
    : sorted[middle] ?? 0;
}

export function nearestRankValue(values: number[], percentile: number): number {
  if (values.length === 0) return 0;
  return nearestRank(values.slice().sort((a, b) => a - b), percentile);
}

function sortJson(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortJson);
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, child]) => [key, sortJson(child)]));
  }
  return value;
}

function objectKeysDeep(value: unknown, prefix = "ratio"): string[] {
  if (value === null || typeof value !== "object") return [];
  return Object.entries(value as Record<string, unknown>).flatMap(([key, child]) => {
    const qualified = `${prefix}.${key}`;
    return [qualified, ...objectKeysDeep(child, qualified)];
  });
}

function containsNumber(value: unknown): boolean {
  if (typeof value === "number") return true;
  if (Array.isArray(value)) return value.some(containsNumber);
  return value !== null && typeof value === "object"
    ? Object.values(value as Record<string, unknown>).some(containsNumber)
    : false;
}
