import { expect, test, type Browser, type Page } from "@playwright/test";

import { createHash } from "node:crypto";
import { performance } from "node:perf_hooks";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  EMPTY_DICTIONARY_EXCLUDE_SHA256,
  EMPTY_USERDB_FIXTURE_SHA256,
  INJECTED_ERROR_VALUE_SHA256,
  SCENARIO_REGISTRY,
  WEB06_PRESSURE_PAIR_REGISTRY,
  WEB06_OBSERVER_COUNTERBALANCE,
  WEB06_THRESHOLDS,
  buildClockCalibration,
  correctWorkerTimestamp,
  evaluateAttemptSeries,
  evaluateObserverOverhead,
  expandScenarioExpectedTimeline,
  resolveScenarioRun,
} from "./web06-metric-contract.mjs";
import {
  adaptPrivateProtocolReceipt,
  advanceCadenceDeadline,
  collectFiveWithinSeven,
  combinedAttemptFacts,
  classifyWeb06HarnessFailure,
  commonEndpointSequenceDigest,
  installWeb06Sentinel,
  makeCommonSurfaceReceipt,
  mergeWeb06LearnedProtocolSegments,
  mergeWeb06LearnedSentinelSegments,
  parseAndCompactCommonReceipt,
  parseAndCompactPrivateReceipt,
  parseWeb06CollectorEnvironment,
  protocolCapabilityBlockers,
  protocolHealthBlockers,
  reserveRawEvidencePacket,
  commitRawEvidencePacket,
  rawEvidencePacketReference,
  retainedRawReceiptBehaviorErrors,
  sentinelLedgerIntegrityErrors,
  buildIncompleteObserverModeProjection,
  evaluateArtifactResponseGuardObservations,
  evaluateFinalLaneDisposition,
  observeWeb06HostEnvironment,
  verifyWeb06RunnerSource,
  validateCompletedRawDecisionShape,
  writeCollectorOutput,
  writeCompactEvidenceReceipt,
  writeSuiteAttestation,
} from "./web06-collector.mjs";
import { writeIndependentRecompute } from "./web06-independent-verifier.mjs";
import {
  buildFiveRoundEvidenceSummary,
  buildRoundEvidenceSummary,
  evaluateFiveRoundCommonPool,
  evaluateFiveRoundPool,
} from "./web06-receipt-parser.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const declaredConfig = parseWeb06CollectorEnvironment(process.env, { repoRoot });
if (declaredConfig.blockedScenarios.length > 0) {
  throw new Error(`WEB06_SOURCE_REVIEWED_SCENARIOS_BLOCKED:${declaredConfig.blockedScenarios.map((row: any) => row.scenarioId).join(",")}`);
}
const readyTimeoutMs = 120_000;
let runnerSourceBefore: Awaited<ReturnType<typeof verifyWeb06RunnerSource>>;
let runnerSourceAfter: Awaited<ReturnType<typeof verifyWeb06RunnerSource>>;
let observedEnvironment: Awaited<ReturnType<typeof observeWeb06HostEnvironment>>;

type CollectorEnvironment = ReturnType<typeof parseWeb06CollectorEnvironment>;
type CollectorTarget = CollectorEnvironment["targets"][string];

interface AttemptResult {
  receipt: Record<string, unknown>;
  commonReceipt?: Record<string, unknown>;
  parsed: {
    status: string;
    cadence?: string;
    setupErrors: string[];
    behaviorErrors: string[];
    thresholdViolations: string[];
    frameRed: boolean;
    longTaskRed: boolean;
    metrics?: Record<string, unknown>;
  };
  commonParsed?: AttemptResult["parsed"];
  publicReceipts: Record<string, unknown>[];
  overhead?: Record<string, unknown>;
  rawPacket?: { relativePath: string; bytes: number; sha256: string };
  measurementStarted?: boolean;
  measurementCompleted?: boolean;
  runnerSummaries?: Record<string, unknown>;
}

test.describe("WEB-06 source-bound smoothness", () => {
  test.describe.configure({ mode: "serial", retries: 0 });
  test.setTimeout(6 * 60 * 60 * 1000);
  test.beforeAll(async ({ browser }) => {
    runnerSourceBefore = await verifyWeb06RunnerSource(declaredConfig);
    observedEnvironment = await observeWeb06HostEnvironment(declaredConfig, {
      browserVersion: browser.version(),
      browserExecutablePath: browser.browserType().executablePath(),
    });
  });
  test.afterAll(async () => {
    runnerSourceAfter = await verifyWeb06RunnerSource(declaredConfig);
    expect(runnerSourceAfter.observationSha256, "runner source/tooling must remain exact throughout the suite")
      .toBe(runnerSourceBefore.observationSha256);
  });

  if (["full", "observer-overhead"].includes(declaredConfig.runKind)) test("complete source-bound lane @web06-full", async ({ browser }) => {
    const config = declaredConfig;
    if (config.runKind === "observer-overhead") {
      const result = await collectObserverOverhead(browser, config);
      runnerSourceAfter = await verifyWeb06RunnerSource(config);
      const collector = await writeCollectorOutput({
        config,
        scenarioResults: [],
        observerTriplets: result.attempts,
        observerEvaluation: result.evaluation,
        runnerSourceBefore,
        runnerSourceAfter,
      });
      const independent = await writeIndependentRecompute({
        evidenceRoot: config.evidenceRoot,
        collectorOutputPath: collector.artifact.path,
        outputPath: config.outputPaths.independent,
      });
      await writeSuiteAttestation({ config, collectorOutputArtifact: collector.artifact,
        independentRecomputeArtifact: independent, observerTriplets: result.attempts,
        verdict: result.evaluation.status, runnerSourceBefore, runnerSourceAfter });
      expect(result.evaluation.status).toBe("PASS");
      return;
    }

    const completed = [];
    const laneFailures: string[] = [];
    const finalDispositionScenarios: Array<{
      scenarioRunId: string;
      seriesStatus: string;
      validLatencyFrameRoundCount: number;
      internalPoolPass?: boolean;
      commonPoolPass?: boolean;
    }> = [];
    for (const targetId of config.targetOrder) {
      const target = config.targets[targetId];
      for (const scenarioRunId of config.scenarioIds) {
        const series = await collectFiveWithinSeven(({ attemptId, attemptNumber }) =>
          collectAttempt(browser, config, target, scenarioRunId, attemptId, attemptNumber));
        if (series.series.measuredCount !== 5) {
          laneFailures.push(`${targetId}/${scenarioRunId}:SETUP_NO_GO:${series.series.status}`);
        }
        const internalReceipts = series.validLatencyFrameAttempts.map((attempt) => attempt.receipt);
        const commonReceipts = series.validLatencyFrameAttempts.map((attempt) => attempt.commonReceipt).filter(Boolean);
        const internalPool = internalReceipts.length === 5 ? evaluateFiveRoundPool(internalReceipts) : undefined;
        const commonPool = commonReceipts.length === 5 ? evaluateFiveRoundCommonPool(commonReceipts) : undefined;
        if (config.expectation === "FINAL") {
          finalDispositionScenarios.push({
            scenarioRunId: `${targetId}/${scenarioRunId}`,
            seriesStatus: series.series.status,
            validLatencyFrameRoundCount: series.series.validLatencyFrameCount,
            internalPoolPass: internalPool?.pass,
            commonPoolPass: commonPool?.pass,
          });
        }
        const run = resolveScenarioRun(scenarioRunId);
        const attempts = series.attempts.map((attempt, index) => projectAttemptResult(
          attempt,
          series.series.retained[index],
        ));
        completed.push({
          targetId,
          scenarioRunId,
          scenarioId: run.scenarioId,
          schemaId: run.schema,
          measuredRoundCount: series.series.measuredCount,
          validLatencyFrameRoundCount: series.series.validLatencyFrameCount,
          verdict: series.series.status === "COMPLETE_GREEN" ? "PASS"
            : series.series.status === "COMPLETE_WITH_RED" ? "RED" : "SETUP_NO_GO",
          preservedHardRedAttemptIds: series.series.preservedHardRedAttemptIds,
          preservedHardRedObserved: series.series.preservedHardRedCount > 0,
          attempts,
          runnerFiveRoundSummaries: internalPool && commonPool ? {
            internal: buildFiveRoundEvidenceSummary(internalReceipts, { surface: "internal" }),
            common: buildFiveRoundEvidenceSummary(commonReceipts, { surface: "common" }),
          } : {},
        });
      }
    }
    if (config.expectation === "FINAL") {
      const finalDisposition = evaluateFinalLaneDisposition({
        disposition: config.disposition,
        scenarios: finalDispositionScenarios,
      });
      laneFailures.push(...finalDisposition.violations);
    }
    expect(completed).toHaveLength(config.targetOrder.length * config.scenarioIds.length);
    runnerSourceAfter = await verifyWeb06RunnerSource(config);
    const collector = await writeCollectorOutput({ config, scenarioResults: completed, observerTriplets: [],
      runnerSourceBefore, runnerSourceAfter });
    const independent = await writeIndependentRecompute({ evidenceRoot: config.evidenceRoot,
      collectorOutputPath: collector.artifact.path,
      outputPath: config.outputPaths.independent });
    const verdict = completed.some((scenario) => scenario.verdict === "SETUP_NO_GO") ? "SETUP_NO_GO"
      : completed.some((scenario) => scenario.verdict !== "PASS") ? "RED" : "PASS";
    await writeSuiteAttestation({ config, collectorOutputArtifact: collector.artifact,
      independentRecomputeArtifact: independent, scenarioResults: completed, verdict,
      runnerSourceBefore, runnerSourceAfter });
    expect(laneFailures, `WEB-06 lane failures were preserved in external evidence: ${laneFailures.join(",")}`)
      .toEqual([]);
  });

  if (declaredConfig.runKind === "preview-canary") test("production-default normal and rapid canary @web06-preview-canary", async ({ browser }) => {
    const config = declaredConfig;
    expect(config.targetOrder).toHaveLength(1);
    const targetId = config.targetOrder[0];
    const target = config.targets[targetId];
    const completed = [];
    for (const scenarioRunId of config.scenarioIds) {
      const result = await collectAttempt(browser, config, target, scenarioRunId, "attempt-1", 1);
      const facts = combinedAttemptFacts({ internalParsed: result.parsed,
        commonParsed: result.commonParsed, attemptId: "attempt-1",
        measurementStarted: result.measurementStarted === true,
        measurementCompleted: result.measurementCompleted === true });
      const retained = evaluateAttemptSeries([facts]).retained[0];
      const run = resolveScenarioRun(scenarioRunId);
      completed.push({
        targetId,
        scenarioRunId,
        scenarioId: run.scenarioId,
        schemaId: run.schema,
        measuredRoundCount: Number(retained.retainedLogicalRound === true),
        validLatencyFrameRoundCount: Number(retained.validForLatencyFrame === true),
        verdict: retained.retainedLogicalRound
          ? (retained.retainedHardRed ? "RED" : "PASS") : "SETUP_INVALID",
        preservedHardRedAttemptIds: retained.retainedHardRed ? ["attempt-1"] : [],
        preservedHardRedObserved: retained.retainedHardRed === true,
        attempts: [projectAttemptResult(result, retained)],
        runnerFiveRoundSummaries: {},
      });
    }
    runnerSourceAfter = await verifyWeb06RunnerSource(config);
    const collector = await writeCollectorOutput({ config, scenarioResults: completed, observerTriplets: [],
      runnerSourceBefore, runnerSourceAfter });
    const independent = await writeIndependentRecompute({ evidenceRoot: config.evidenceRoot,
      collectorOutputPath: collector.artifact.path, outputPath: config.outputPaths.independent });
    const verdict = completed.every((scenario) => scenario.verdict === "PASS") ? "PASS"
      : completed.some((scenario) => scenario.verdict === "RED") ? "RED" : "SETUP_INVALID";
    await writeSuiteAttestation({ config, collectorOutputArtifact: collector.artifact,
      independentRecomputeArtifact: independent, scenarioResults: completed, verdict,
      runnerSourceBefore, runnerSourceAfter });
    expect(completed.map((scenario) => scenario.verdict), "preview canary verdicts")
      .toEqual(completed.map(() => "PASS"));
  });
});

async function collectObserverOverhead(browser: Browser, config: CollectorEnvironment) {
  const retained: Record<string, unknown>[] = [];
  let validSlot = 1;
  for (let attemptNumber = 1; attemptNumber <= WEB06_THRESHOLDS.observer.maximumTripletAttempts; attemptNumber += 1) {
    const attemptId = `triplet-attempt-${attemptNumber}`;
    const modeOrder = WEB06_OBSERVER_COUNTERBALANCE[validSlot as keyof typeof WEB06_OBSERVER_COUNTERBALANCE];
    const modes: Record<string, AttemptResult> = {};
    const contextIds: string[] = [];
    for (const targetId of modeOrder) {
      const result = await collectAttempt(
        browser,
        config,
        config.targets[targetId],
        "rapid-long-jyutping",
        attemptId,
        attemptNumber,
      );
      modes[targetId] = result;
      contextIds.push(String(result.overhead?.contextId ?? ""));
    }
    const product = overheadMode(modes.PRODUCT);
    const minimal = overheadMode(modes.BASE_MINIMAL);
    const full = overheadMode(modes.BASE_FULL);
    classifyObserverLongTasks({ product, minimal, full });
    const valid = [product, minimal, full].every((mode) => mode.measurementValid === true);
    retained.push({
      attemptId,
      valid,
      counterbalanceSlot: validSlot,
      freshContextId: contextIds.join("+"),
      modeContextIds: contextIds,
      modeOrder,
      modeFixedBeforePageLoad: true,
      product,
      minimal,
      full,
    });
    if (valid) validSlot += 1;
    if (validSlot > WEB06_THRESHOLDS.observer.requiredTriplets) break;
  }
  return { attempts: retained, evaluation: evaluateObserverOverhead(retained) };
}

function projectAttemptResult(attempt: any, retained: any) {
  if (!attempt.rawPacket) throw new Error(`WEB06_RAW_REFERENCE_MISSING:${retained.attemptId}`);
  const classification = retained.verdict?.startsWith("RED") ? "RED" : retained.verdict;
  return {
    attemptId: retained.attemptId,
    measurementStarted: attempt.measurementStarted === true,
    measurementCompleted: attempt.measurementCompleted === true,
    classification,
    retainedMeasured: retained.retainedLogicalRound === true,
    retainedLogicalRound: retained.retainedLogicalRound === true,
    validForLatencyFrame: retained.validForLatencyFrame === true,
    retainedHardRed: retained.retainedHardRed === true,
    retryEligible: ["SETUP_INVALID", "NO_VERDICT_INVALID_CADENCE"].includes(retained.verdict),
    validRedObserved: retained.retainedHardRed === true,
    rawPacket: attempt.rawPacket,
    runnerSummaries: attempt.runnerSummaries ?? {},
  };
}

function observerCanonicalDiagnostics(values: unknown[]) {
  const aliases: Record<string, string> = {
    BEHAVIOR_PAGE_VISIBLE_COUNT: "BEHAVIOR_VISIBLE_COUNT",
    BEHAVIOR_VISIBLECOUNT: "BEHAVIOR_VISIBLE_COUNT",
    BEHAVIOR_PAGE_PREVIOUS: "BEHAVIOR_PREVIOUS_DISABLED",
    BEHAVIOR_PREVIOUSDISABLED: "BEHAVIOR_PREVIOUS_DISABLED",
    BEHAVIOR_PAGE_NEXT: "BEHAVIOR_NEXT_DISABLED",
    BEHAVIOR_NEXTDISABLED: "BEHAVIOR_NEXT_DISABLED",
    BEHAVIOR_TEXTAREA_VALUE: "BEHAVIOR_TEXTAREA_VALUE",
    BEHAVIOR_TEXTAREAVALUE: "BEHAVIOR_TEXTAREA_VALUE",
    BEHAVIOR_SELECTION_START: "BEHAVIOR_SELECTION_START",
    BEHAVIOR_SELECTIONSTART: "BEHAVIOR_SELECTION_START",
    BEHAVIOR_SELECTION_END: "BEHAVIOR_SELECTION_END",
    BEHAVIOR_SELECTIONEND: "BEHAVIOR_SELECTION_END",
  };
  const canonical = (raw: unknown) => {
    const value = String(raw);
    const [code, instance] = value.split(":");
    const family = ["BURST_RECOVERY_INVALID", "BURST_RECOVERY_IDENTITY"].includes(code)
      ? "BURST_RECOVERY_IDENTITY"
      : Object.prototype.hasOwnProperty.call(aliases, code) ? aliases[code] : undefined;
    return family === undefined ? value : instance === undefined ? family : `${family}:${instance}`;
  };
  const trueSynonymCodes = new Set([
    "BURST_RECOVERY_INVALID", "BURST_RECOVERY_IDENTITY",
    "BEHAVIOR_PAGE_VISIBLE_COUNT", "BEHAVIOR_VISIBLECOUNT",
    "BEHAVIOR_PAGE_PREVIOUS", "BEHAVIOR_PREVIOUSDISABLED",
    "BEHAVIOR_PAGE_NEXT", "BEHAVIOR_NEXTDISABLED",
    "BEHAVIOR_TEXTAREA_VALUE", "BEHAVIOR_TEXTAREAVALUE",
    "BEHAVIOR_SELECTION_START", "BEHAVIOR_SELECTIONSTART",
    "BEHAVIOR_SELECTION_END", "BEHAVIOR_SELECTIONEND",
  ]);
  const grouped = new Map<string, { raw: string; code: string }[]>();
  for (const rawValue of values) {
    const raw = String(rawValue);
    const key = canonical(raw);
    const records = grouped.get(key) ?? [];
    records.push({ raw, code: raw.split(":")[0] });
    grouped.set(key, records);
  }
  return [...grouped.entries()].sort(([left], [right]) => left.localeCompare(right))
    .flatMap(([key, records]) => {
      const codes = new Set(records.map((record) => record.code));
      if (codes.size <= 1 || [...codes].some((code) => !trueSynonymCodes.has(code))) {
        return Array.from({ length: records.length }, () => key);
      }
      const counts = new Map<string, number>();
      for (const record of records) counts.set(record.raw, (counts.get(record.raw) ?? 0) + 1);
      return Array.from({ length: Math.max(...counts.values()) }, () => key);
    });
}

function observerModeHardRedExpected(mode: any) {
  const atOrAbove = (values: unknown, threshold: number) => Array.isArray(values)
    && values.some((value) => Number.isFinite(value) && value >= threshold);
  const parserRed = [mode?.commonVerdict, mode?.internalVerdict]
    .some((verdict) => ["RED", "RED_BEHAVIOR"].includes(verdict));
  const localRed = atOrAbove(
    mode?.sentinelCallbacksMs,
    WEB06_THRESHOLDS.observer.sentinelCallbackExclusiveMaxMs,
  ) || atOrAbove(
    mode?.sentinelTotalPerEventMs,
    WEB06_THRESHOLDS.observer.sentinelTotalPerEventExclusiveMaxMs,
  ) || atOrAbove(
    mode?.collectorCallbacksMs,
    WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs,
  ) || atOrAbove(
    mode?.mainObserverCallbacksMs,
    WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs,
  ) || atOrAbove(
    mode?.workerCollectorCallbacksMs,
    WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs,
  ) || atOrAbove(
    mode?.instrumentationAddedLongTasksMs,
    WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs,
  );
  return mode?.behaviorRedObserved === true
    || (mode?.hardRedBindingValid === true && (parserRed || localRed));
}

function overheadMode(result: AttemptResult) {
  const commonMetrics = (result.commonParsed?.metrics as { samples?: { eventToObservationMs: number }[] } | undefined)?.samples ?? [];
  const internalMetrics = (result.parsed.metrics as { covering?: unknown[]; terminal?: unknown[] } | undefined) ?? {};
  const common = result.commonReceipt as any;
  const hasInternalSurface = result.receipt !== result.commonReceipt;
  const internal = hasInternalSurface ? result.receipt as any : undefined;
  const commonParsed = result.commonParsed;
  const cadenceValid = ["IN_RANGE", "NOT_APPLICABLE"].includes(commonParsed?.cadence ?? "")
    && ["IN_RANGE", "NOT_APPLICABLE"].includes(result.parsed.cadence ?? "");
  const hardRedCadenceValid = [commonParsed?.cadence, result.parsed.cadence]
    .every((cadence) => ["IN_RANGE", "NOT_APPLICABLE", "TOO_LONG"].includes(cadence ?? ""));
  const setupClean = (commonParsed?.setupErrors?.length ?? 0) === 0 && result.parsed.setupErrors.length === 0;
  const behaviorRedObserved = (commonParsed?.behaviorErrors?.length ?? 0) > 0
    || result.parsed.behaviorErrors.length > 0;
  if (result.measurementCompleted !== true) {
    return buildIncompleteObserverModeProjection({
      rawPacket: result.rawPacket,
      measurementStarted: result.measurementStarted === true,
      behaviorRedObserved,
    });
  }
  const commonEquivalence = {
    scenarioId: common?.scenarioId,
    events: common?.events?.map((event: any) => [event.stepId, event.type, event.key, event.code]),
    samples: common?.commonSamples?.map((sample: any) => [
      sample.stepId,
      sample.sampleKind,
      sample.outcome,
      sample.supersededByStepId ?? null,
      sample.domFingerprintSha256,
    ]),
    candidatePageSize: common?.candidatePageSize,
    cadenceIdentity: common?.cadenceGaps?.map((gap: any) => [gap.stepId, gap.nominalGapMs]),
    interactionWindowCount: common?.interactionWindows?.length,
    interactionFrameCount: common?.interactionFrameIntervalsMs?.length,
    longTaskLoci: observerLongTaskLoci(common),
    behaviorErrors: observerCanonicalDiagnostics(commonParsed?.behaviorErrors ?? []),
    thresholdViolations: observerCanonicalDiagnostics(commonParsed?.thresholdViolations ?? []),
    frameRed: commonParsed?.frameRed,
    longTaskRed: commonParsed?.longTaskRed,
  };
  const internalEquivalence = {
    actions: internal?.actions?.map((action: any) => [
      action.stepId,
      action.kind,
      action.outcome,
      action.mainQueueDepth,
      action.workerDispatchDepth,
      action.terminalKind,
    ]),
    coveringCount: internalMetrics.covering?.length ?? 0,
    terminalCount: internalMetrics.terminal?.length ?? 0,
    behaviorErrors: observerCanonicalDiagnostics(result.parsed.behaviorErrors),
    thresholdViolations: observerCanonicalDiagnostics(result.parsed.thresholdViolations),
    frameRed: result.parsed.frameRed,
    longTaskRed: result.parsed.longTaskRed,
  };
  const mode = {
    rawPacket: result.rawPacket,
    measurementStarted: result.measurementStarted === true,
    measurementCompleted: result.measurementCompleted === true,
    measurementValid: cadenceValid && setupClean && result.overhead?.callbackAttributionComplete === true,
    behaviorRedObserved,
    hardRedBindingValid: hardRedCadenceValid && setupClean
      && result.overhead?.callbackAttributionComplete === true,
    hardRedObserved: false,
    samples: commonMetrics.map((sample) => sample.eventToObservationMs),
    commonEquivalenceDigest: digestJson(commonEquivalence),
    ...(hasInternalSurface ? { internalEquivalenceDigest: digestJson(internalEquivalence) } : {}),
    commonVerdict: result.commonParsed?.status,
    internalVerdict: result.parsed.status,
    commonEventCount: result.commonReceipt?.events instanceof Array ? result.commonReceipt.events.length : 0,
    environmentManifestSha256: common?.source?.environmentManifestSha256,
    environmentId: common?.source?.environmentId,
    interactionWindowCount: common?.interactionWindows?.length ?? 0,
    sentinelCallbacksMs: result.overhead?.sentinelCallbacksMs ?? [],
    sentinelTotalPerEventMs: result.overhead?.sentinelTotalPerEventMs ?? [],
    sentinelTotalPerWindowMs: result.overhead?.sentinelTotalPerWindowMs ?? [],
    collectorCallbacksMs: result.overhead?.collectorCallbacksMs ?? [],
    mainObserverCallbacksMs: result.overhead?.mainObserverCallbacksMs ?? [],
    workerCollectorCallbacksMs: result.overhead?.workerCollectorCallbacksMs ?? [],
    callbackLedgerCount: result.overhead?.callbackLedgerCount,
    callbackLedgerCapacity: result.overhead?.callbackLedgerCapacity,
    sentinelAccountedCallbackCount: result.overhead?.sentinelAccountedCallbackCount,
    callbackLedgerOverflowCount: result.overhead?.callbackLedgerOverflowCount,
    callbackAttributionComplete: result.overhead?.callbackAttributionComplete === true,
    ...(hasInternalSurface ? {
      mainObserverCallbackCount: result.overhead?.mainObserverCallbackCount,
      mainObserverCallbackCapacity: result.overhead?.mainObserverCallbackCapacity,
      mainObserverCallbackOverflowCount: result.overhead?.mainObserverCallbackOverflowCount,
    } : {}),
    callbackIntervals: result.overhead?.callbackIntervals ?? [],
    rawLongTasks: result.overhead?.rawLongTasks ?? [],
    underlyingLongTasksMs: [] as number[],
    instrumentationAddedLongTasksMs: [] as number[],
  };
  mode.hardRedObserved = observerModeHardRedExpected(mode);
  return mode;
}

function observerLongTaskLoci(receipt: any) {
  return (receipt?.longTasks ?? [])
    .filter((task: any) => task.overlapsInteractionWindow === true)
    .map((task: any) => {
    const windowIndex = (receipt.interactionWindows ?? []).findIndex((window: any) =>
      window.pageInstanceId === task.pageInstanceId
      && task.startTime < window.endedAt
      && task.startTime + task.durationMs > window.startedAt);
    const event = [...(receipt.events ?? [])]
      .filter((candidate: any) => (task.pageInstanceId === undefined
        || candidate.pageInstanceId === task.pageInstanceId)
        && candidate.normalizedEventAt <= task.startTime + task.durationMs)
      .at(-1);
      return `${windowIndex}:${event?.stepId ?? "no-event"}`;
    });
}

function intervalUnionOverlapMs(start: number, end: number, intervals: any[]) {
  const clipped = intervals
    .map((interval) => [Math.max(start, interval.startedAt), Math.min(end, interval.finishedAt)])
    .filter(([left, right]) => Number.isFinite(left) && Number.isFinite(right) && right > left)
    .sort((left, right) => left[0] - right[0]);
  let total = 0;
  let cursorStart: number | undefined;
  let cursorEnd: number | undefined;
  for (const [left, right] of clipped) {
    if (cursorStart === undefined || left > (cursorEnd as number)) {
      if (cursorStart !== undefined) total += (cursorEnd as number) - cursorStart;
      cursorStart = left;
      cursorEnd = right;
    } else cursorEnd = Math.max(cursorEnd as number, right);
  }
  if (cursorStart !== undefined) total += (cursorEnd as number) - cursorStart;
  return total;
}

function observerModeLedgerValid(mode: any, modeName: "product" | "minimal" | "full") {
  const intervals = mode.callbackIntervals;
  if (!Array.isArray(intervals)) return false;
  const sentinel = intervals.filter((callback: any) => callback.sourceClass === "common-sentinel");
  const privateCallbacks = intervals.filter((callback: any) => callback.sourceClass !== "common-sentinel");
  const validRows = intervals.every((callback: any) =>
    typeof callback.callbackId === "string" && callback.callbackId.length > 0
    && Number.isSafeInteger(callback.sequenceId) && callback.sequenceId > 0
    && (callback.sourceClass === "common-sentinel"
      ? typeof callback.kind === "string" && callback.kind.length > 0
        && Number.isSafeInteger(callback.windowIndex)
        && callback.windowIndex >= -1
        && callback.windowIndex < mode.interactionWindowCount
        && (callback.eventSequenceId === undefined
          || (Number.isSafeInteger(callback.eventSequenceId)
            && callback.eventSequenceId > 0 && callback.eventSequenceId <= mode.commonEventCount))
      : typeof callback.operation === "string" && callback.operation.length > 0)
    && Number.isFinite(callback.startedAt) && Number.isFinite(callback.finishedAt)
    && callback.startedAt >= 0 && callback.finishedAt >= callback.startedAt
    && Number.isFinite(callback.durationMs) && callback.durationMs >= 0
    && callback.durationMs === callback.finishedAt - callback.startedAt);
  const orderedUnique = (rows: any[]) => new Set(rows.map((row) => row.callbackId)).size === rows.length
    && new Set(rows.map((row) => row.sequenceId)).size === rows.length
    && rows.every((row, index) => index === 0
      || (row.sequenceId > rows[index - 1].sequenceId && row.finishedAt >= rows[index - 1].finishedAt));
  const exactNumbers = (left: unknown, right: unknown) =>
    JSON.stringify(left) === JSON.stringify(right);
  const finiteDurations = (value: unknown) => Array.isArray(value)
    && value.every((duration) => Number.isFinite(duration) && duration >= 0);
  const sentinelValid = Number.isSafeInteger(mode.callbackLedgerCount)
    && mode.callbackLedgerCount === sentinel.length
    && Number.isSafeInteger(mode.callbackLedgerCapacity)
    && mode.callbackLedgerCount <= mode.callbackLedgerCapacity
    && mode.sentinelAccountedCallbackCount === sentinel.length
    && mode.callbackLedgerOverflowCount === 0
    && (mode.commonEventCount === 0 || sentinel.length > 0)
    && finiteDurations(mode.sentinelCallbacksMs)
    && exactNumbers(mode.sentinelCallbacksMs, sentinel.map((row: any) => row.durationMs));
  const privateValid = modeName === "product"
    ? privateCallbacks.length === 0
      && Array.isArray(mode.mainObserverCallbacksMs) && mode.mainObserverCallbacksMs.length === 0
      && Array.isArray(mode.workerCollectorCallbacksMs) && mode.workerCollectorCallbacksMs.length === 0
    : Number.isSafeInteger(mode.mainObserverCallbackCount)
      && Number.isSafeInteger(mode.mainObserverCallbackCapacity)
      && mode.mainObserverCallbackCount === privateCallbacks.length
      && mode.mainObserverCallbackCount <= mode.mainObserverCallbackCapacity
      && mode.mainObserverCallbackOverflowCount === 0
      && finiteDurations(mode.mainObserverCallbacksMs)
      && exactNumbers(mode.mainObserverCallbacksMs, privateCallbacks.map((row: any) => row.durationMs));
  if (!finiteDurations(mode.workerCollectorCallbacksMs) || !finiteDurations(mode.collectorCallbacksMs)
    || !finiteDurations(mode.sentinelTotalPerEventMs) || !finiteDurations(mode.sentinelTotalPerWindowMs)
    || !Array.isArray(mode.rawLongTasks)
    || mode.rawLongTasks.some((task: any) => !Number.isFinite(task?.startTime) || task.startTime < 0
      || !Number.isFinite(task?.durationMs) || task.durationMs < 0)
    || !Number.isSafeInteger(mode.commonEventCount) || mode.commonEventCount < 0
    || !Number.isSafeInteger(mode.interactionWindowCount) || mode.interactionWindowCount < 0) return false;
  const combinedCollectorDurations = [...mode.mainObserverCallbacksMs, ...mode.workerCollectorCallbacksMs]
    .sort((left: number, right: number) => left - right);
  const eventTotals = Array.from({ length: mode.commonEventCount }, (_, index) => sentinel
    .filter((row: any) => row.eventSequenceId === index + 1)
    .reduce((sum: number, row: any) => sum + row.durationMs, 0));
  const windowTotals = Array.from({ length: mode.interactionWindowCount }, (_, index) => sentinel
    .filter((row: any) => row.windowIndex === index)
    .reduce((sum: number, row: any) => sum + row.durationMs, 0));
  return validRows
    && new Set(intervals.map((row: any) => row.callbackId)).size === intervals.length
    && orderedUnique(sentinel) && orderedUnique(privateCallbacks)
    && sentinelValid && privateValid
    && exactNumbers(combinedCollectorDurations,
      [...mode.collectorCallbacksMs].sort((left: number, right: number) => left - right))
    && exactNumbers(mode.sentinelTotalPerEventMs, eventTotals)
    && exactNumbers(mode.sentinelTotalPerWindowMs, windowTotals);
}

function classifyObserverLongTasks(modes: Record<"product" | "minimal" | "full", any>) {
  const threshold = WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs;
  const thresholdTasks = Object.fromEntries(Object.entries(modes).map(([name, mode]) => [
    name,
    (() => {
      const occurrences = new Map<string, number>();
      return (mode.rawLongTasks ?? []).filter((task: any) =>
        task.overlapsInteractionWindow === true).map((task: any, index: number) => {
        const locus = task.locus ?? `unknown:${index}`;
        const occurrence = (occurrences.get(locus) ?? 0) + 1;
        occurrences.set(locus, occurrence);
        return { ...task, locus, locusOccurrence: occurrence, locusKey: `${locus}#${occurrence}` };
      }).filter((task: any) => task.durationMs >= threshold);
    })(),
  ])) as Record<string, any[]>;
  const locusKeys = Object.fromEntries(Object.entries(thresholdTasks)
    .map(([name, tasks]) => [name, new Set(tasks.map((task) => task.locusKey))]));
  for (const [modeName, mode] of Object.entries(modes)) {
    const intervals = mode.callbackIntervals ?? [];
    const intervalLedgerValid = observerModeLedgerValid(
      mode,
      modeName as "product" | "minimal" | "full",
    );
    mode.callbackAttributionComplete = intervalLedgerValid;
    mode.hardRedBindingValid = mode.hardRedBindingValid === true && intervalLedgerValid;
    if (!intervalLedgerValid) {
      mode.measurementValid = false;
      mode.commonVerdict = mode.behaviorRedObserved === true ? "RED_BEHAVIOR" : "SETUP_INVALID";
      mode.internalVerdict = mode.behaviorRedObserved === true ? "RED_BEHAVIOR" : "SETUP_INVALID";
    }
    for (const task of thresholdTasks[modeName]) {
      const callbacks = modeName === "product" ? [] : (mode.callbackIntervals ?? [])
        .filter((callback: any) => callback.sourceClass !== "common-sentinel");
      const complete = intervalLedgerValid;
      const overlap = complete
        ? intervalUnionOverlapMs(task.startTime, task.startTime + task.durationMs, callbacks)
        : task.durationMs;
      const residualLowerBoundMs = task.durationMs - overlap;
      const matchedAllModes = locusKeys.product.has(task.locusKey)
        && locusKeys.minimal.has(task.locusKey)
        && locusKeys.full.has(task.locusKey);
      if (modeName === "product"
        || (complete && residualLowerBoundMs >= threshold && matchedAllModes)) {
        mode.underlyingLongTasksMs.push(task.durationMs);
      } else if (modeName !== "product") {
        mode.instrumentationAddedLongTasksMs.push(task.durationMs);
      }
    }
  }
  // PRODUCT is the no-instrumentation authority. It cannot claim that its own
  // task was added by WEB-06. Its real RED and measurement validity remain
  // attached to PRODUCT; the evaluator rejects the retained triplet when the
  // three common surfaces disagree.
  modes.product.instrumentationAddedLongTasksMs = [];
  for (const mode of Object.values(modes)) {
    mode.hardRedObserved = observerModeHardRedExpected(mode);
  }
}

async function collectAttempt(
  browser: Browser,
  config: CollectorEnvironment,
  target: CollectorTarget,
  scenarioRunId: string,
  attemptId: string,
  attemptNumber: number,
): Promise<AttemptResult> {
  const run = resolveScenarioRun(scenarioRunId);
  const { scenarioId, schema } = run;
  const contextId = `${target.id}-${scenarioRunId}-${attemptId}-${Date.now()}`;
  let context: Awaited<ReturnType<Browser["newContext"]>> | undefined;
  let page: Page | undefined;
  let responseGuard: Awaited<ReturnType<typeof installArtifactResponseGuard>> | undefined;
  let reservation: Awaited<ReturnType<typeof reserveRawEvidencePacket>> | undefined;
  let measurementStarted = false;
  let attemptSourceBefore: Awaited<ReturnType<typeof verifyWeb06RunnerSource>> | undefined;
  const partialAttempt: Record<string, any> = {
    version: "web06-partial-attempt-v1",
    phase: "created",
    measurementStarted: false,
    driverEvents: [],
    cadenceGaps: [],
    burstRecoveries: [],
    pressureProofs: [],
    argumentCommitments: {},
  };
  const rawEnvelope: Record<string, unknown> = {
    version: "web06-raw-attempt-v1",
    contextId,
    target,
    scenarioId,
    scenarioRunId,
    schemaId: schema,
    attemptId,
    attemptNumber,
    expectation: config.expectation,
    identityManifestSha256: config.identityManifestSha256,
    runnerSourceManifestSha256: config.runnerSourceManifestSha256,
    runnerSourceBefore,
    attemptSourceBefore,
    observedEnvironment,
    scenarioIdsSha256: config.scenarioIdsSha256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    partialAttempt,
  };
  let rawPersisted = false;
  let persistedRawSha256: string | undefined;
  try {
    reservation = await reserveRawEvidencePacket({
      evidenceRoot: config.evidenceRoot,
      repoRoot: config.repoRoot,
      runId: config.runId,
      sourceCommit: target.sourceCommit,
      scenarioId: scenarioRunId,
      attemptId,
      mode: target.id,
    });
    partialAttempt.phase = "raw-reserved";
    attemptSourceBefore = await verifyWeb06RunnerSource(config);
    rawEnvelope.attemptSourceBefore = attemptSourceBefore;
    const setup = await prepareDisposableSetup(browser, target, scenarioId, schema);
    rawEnvelope.disposableSetup = setup.receipt;
    context = await browser.newContext({
      viewport: config.environmentManifest.browser.viewport,
      locale: config.environmentManifest.browser.locale,
      serviceWorkers: "block",
    });
    await context.addInitScript(({ activeSchema }) => {
      localStorage.setItem("activeSchema", activeSchema);
      localStorage.setItem("uiLanguage", "en");
      localStorage.setItem("enableAI", "false");
      localStorage.setItem("pageSize", "6");
    }, { activeSchema: schema });
    await context.addInitScript(installWeb06Sentinel);
    page = await context.newPage();
    responseGuard = await installArtifactResponseGuard(page, target);
    const url = targetUrl(target, schema, scenarioId);
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: readyTimeoutMs });
    await waitForReady(page);
    rawEnvelope.measuredUiPosture = await verifyMeasuredUiPosture(page);
    rawEnvelope.sourceProof = await verifyServedSource(page, target, responseGuard);
    const pageSizeSetup = {
      ...setup.receipt.pageSizeSetup,
      measuredUiPageSize: await page.locator("[data-yune-page-size] input[type='range']").inputValue(),
    };
    if (pageSizeSetup.measuredUiPageSize !== "6") throw new Error("WEB06_MEASURED_UI_PAGE_SIZE_NOT_SIX");
    rawEnvelope.pageSizeSetup = pageSizeSetup;
    if (scenarioId === "extended-scheduler-barriers") await prepareExtendedErrorControl(page);
    if (scenarioId === "correction") await enableCorrectionThroughUi(page, target.protocolMode !== "off");
    await page.locator(".yd-input-area").focus();

    const uiCapabilities = await readUiCapabilities(page, scenarioId, target.protocolMode !== "off");
    rawEnvelope.measuredRealmSetup = {
      configurationRecipeReappliedBeforeLoad: true,
      persistenceTransferred: false,
      correctionEnabled: scenarioId === "correction"
        ? await page.locator("[data-yune-section='active'] label").filter({ hasText: "Auto-correction" }).locator("input").isChecked()
        : false,
      uiCapabilities,
    };
    if (target.protocolMode !== "off") await waitForProtocolIdle(page);
    const protocolBeforeReset = await readProtocol(page);
    if (target.protocolMode !== "off") {
      const setupHealth = protocolHealthBlockers(protocolBeforeReset);
      if (setupHealth.length) throw new Error(`WEB06_SETUP_PROTOCOL_DIRTY:${setupHealth.join(",")}`);
    }
    const idleFrameIntervalsMs = await page.evaluate(
      (count) => (window as any).__YUNE_WEB06_SENTINEL__.takeIdleIntervals(count),
      WEB06_THRESHOLDS.frame.requiredIdleIntervals,
    );
    const eventClockProbe = await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.probeEventClock());
    const preCalibration = await collectCalibration(page, target.protocolMode !== "off");
    if (target.protocolMode !== "off") {
      await page.evaluate(() => (window as any).__YUNE_WEB06__.resetReceipts());
      await waitForProtocolIdle(page);
    }
    const protocolAfterReset = await readProtocol(page);
    const blockers = protocolCapabilityBlockers({
      mode: target.protocolMode,
      scenarioId: run.scenarioId,
      protocol: protocolAfterReset?.header,
      status: protocolAfterReset?.status,
      invalidations: protocolAfterReset?.invalidations ?? [],
      uiCapabilities,
      selectedBranch: config.branch,
    });
    rawEnvelope.preflight = { protocolBeforeReset, protocolAfterReset, uiCapabilities, blockers };
    if (blockers.length) throw new Error(`WEB06_SETUP_PREFLIGHT:${blockers.join(",")}`);
    await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.reset());
    measurementStarted = true;
    partialAttempt.measurementStarted = true;
    partialAttempt.phase = "measurement-started";
    const drive = await driveScenario(page, scenarioId, target.protocolMode !== "off", target, responseGuard, (progress) => {
      Object.assign(partialAttempt, structuredClone(progress));
    });
    Object.assign(partialAttempt, structuredClone({ ...drive, phase: "drive-complete" }));
    const finalSegmentExpectedActions = drive.learned?.postRun.segmentActionCount
      ?? SCENARIO_REGISTRY[scenarioId].expectedActionCount;
    const finalSegmentCompletion = await waitForExactCompletion(
      page,
      target.protocolMode !== "off",
      finalSegmentExpectedActions,
    );
    const completion = drive.learned
      ? {
        ...finalSegmentCompletion,
        expectedActionCount: SCENARIO_REGISTRY[scenarioId].expectedActionCount,
        observedActionCount: drive.learned.preRun.segmentActionCount
          + (finalSegmentCompletion.observedActionCount ?? 0),
      }
      : finalSegmentCompletion;
    partialAttempt.completion = structuredClone(completion);
    partialAttempt.phase = "completion-observed";
    const postCalibration = await collectCalibration(page, target.protocolMode !== "off");
    const finalSentinel = await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.snapshot());
    const finalProtocolExport = await readProtocol(page, true);
    rawEnvelope.artifactResponseGuard = await responseGuard.assertComplete("post-measurement");
    partialAttempt.finalSentinel = structuredClone(finalSentinel);
    partialAttempt.finalProtocolExport = structuredClone(finalProtocolExport);
    partialAttempt.phase = "final-snapshots-captured";
    const sentinel = drive.learned
      ? mergeWeb06LearnedSentinelSegments(
        drive.learned.preSegment.sentinel,
        finalSentinel,
        drive.learned.lifecycleMarker,
      )
      : finalSentinel;
    const protocolExport = drive.learned
      ? mergeWeb06LearnedProtocolSegments(
        drive.learned.preSegment.protocol,
        finalProtocolExport,
        drive.learned.lifecycleMarker,
      )
      : finalProtocolExport;
    const measurementProtocolBlockers = target.protocolMode === "off"
      ? []
      : [
        ...(completion.timedOut ? ["PROTOCOL_COMPLETION_TIMEOUT"] : []),
        ...protocolHealthBlockers(protocolExport, { requireCallbackLedger: true }),
      ];
    const commonSurface = {
      ...sentinel,
      candidatePageSize: Number(pageSizeSetup.measuredUiPageSize),
      pageSizeSetup,
      initialDomObserved: drive.initialDomObserved,
      eventClockProbe,
      idleFrameIntervalsMs: drive.learned
        ? [...idleFrameIntervalsMs, ...drive.learned.postReloadIdleFrameIntervalsMs]
        : idleFrameIntervalsMs,
      idleFrameSegments: drive.learned ? [
        {
          pageInstanceId: drive.learned.preSegment.sentinel.longTaskObserver.pageInstanceId,
          intervalsMs: idleFrameIntervalsMs,
        },
        {
          pageInstanceId: finalSentinel.longTaskObserver.pageInstanceId,
          intervalsMs: drive.learned.postReloadIdleFrameIntervalsMs,
        },
      ] : undefined,
      cadenceGaps: drive.cadenceGaps,
      burstRecoveries: drive.burstRecoveries,
      pressureProofs: drive.pressureProofs,
      lifecycleContinuity: drive.learned?.lifecycleContinuity,
      measurementProtocolBlockers,
      calibration: {
        driver: { pre: preCalibration.driver, post: postCalibration.driver },
        worker: { pre: preCalibration.worker, post: postCalibration.worker },
      },
      calibrationSegments: drive.learned ? {
        preReload: {
          driver: { pre: preCalibration.driver, post: drive.learned.preSegment.postCalibration.driver },
          worker: { pre: preCalibration.worker, post: drive.learned.preSegment.postCalibration.worker },
        },
        postReload: {
          driver: { pre: drive.learned.postReloadCalibration.driver, post: postCalibration.driver },
          worker: { pre: drive.learned.postReloadCalibration.worker, post: postCalibration.worker },
        },
      } : undefined,
      eventClockSegments: drive.learned ? {
        preReload: eventClockProbe,
        postReload: drive.learned.postReloadEventClockProbe,
      } : undefined,
    };
    rawEnvelope.measurementEvidence = {
      eventClockProbe: structuredClone(commonSurface.eventClockProbe),
      eventClockSegments: structuredClone(commonSurface.eventClockSegments),
      calibration: structuredClone(commonSurface.calibration),
      calibrationSegments: structuredClone(commonSurface.calibrationSegments),
      idleFrameIntervalsMs: structuredClone(commonSurface.idleFrameIntervalsMs),
      idleFrameSegments: structuredClone(commonSurface.idleFrameSegments),
    };
    const attemptSourceAfter = await verifyWeb06RunnerSource(config);
    rawEnvelope.attemptSourceAfter = attemptSourceAfter;
    if (attemptSourceAfter.observationSha256 !== attemptSourceBefore?.observationSha256) {
      throw new Error("WEB06_RUNNER_SOURCE_CHANGED_DURING_ATTEMPT");
    }
    const metadata = {
      scenarioId: run.scenarioId,
      scenarioRunId,
      schemaId: schema,
      mode: target.id,
      source: {
        ...sourceIdentity(config, target, rawEnvelope.artifactResponseGuard as Record<string, unknown>),
        runnerSourcePostObservationSha256: attemptSourceAfter.observationSha256,
      },
      roundId: `${scenarioId}-round-${attemptNumber}`,
      attemptId,
      measurementStarted: true,
      measurementCompleted: true,
    };
    const commonReceipt = makeCommonSurfaceReceipt({ metadata, commonSurface });
    const privateReceipt = target.protocolMode === "off" ? undefined : adaptPrivateProtocolReceipt({
      metadata,
      protocolWindow: {
        receiptWindowStartEventSequenceId: protocolAfterReset.status.receiptWindowStartEventSequenceId,
        receiptWindowStartActionSequenceId: protocolAfterReset.status.receiptWindowStartActionSequenceId,
      },
      wireEvents: Array.isArray(protocolExport?.events) ? protocolExport.events : [],
      wireActions: Array.isArray(protocolExport?.actions) ? protocolExport.actions : [],
      driverEvents: drive.driverEvents,
      commonSurface,
      argumentCommitments: drive.argumentCommitments,
      externalLifecycleEvents: protocolExport?.externalLifecycleEvents ?? [],
      protocolWindowSegments: protocolExport?.protocolWindowSegments ?? [],
    });
    if (privateReceipt && SCENARIO_REGISTRY[scenarioId].overlapRequired) {
      privateReceipt.pressureProofs = buildPressureProofs(
        privateReceipt,
        scenarioId,
        commonSurface.calibration.worker,
      );
    }
    Object.assign(rawEnvelope, { drive, completion, sentinel, protocolExport, commonReceipt, privateReceipt });
    rawEnvelope.measurementStarted = true;
    rawEnvelope.measurementCompleted = true;
    partialAttempt.phase = "complete-receipts-built";
    partialAttempt.measurementCompleted = true;
    const completedRawShape = validateCompletedRawDecisionShape(rawEnvelope);
    if (!completedRawShape.pass) throw new Error(completedRawShape.errors[0]);
    if (!reservation) throw new Error("WEB06_RAW_RESERVATION_MISSING");
    const raw = await commitRawEvidencePacket({ reservation, packet: rawEnvelope });
    rawPersisted = true;
    persistedRawSha256 = raw.rawPacketSha256;
    const rawPacket = rawEvidencePacketReference(raw);
    const common = parseAndCompactCommonReceipt(commonReceipt, raw.rawPacketSha256);
    const internal = privateReceipt
      ? parseAndCompactPrivateReceipt(privateReceipt, raw.rawPacketSha256)
      : { parsed: common.parsed, publicReceipt: undefined };
    const publicReceipts = [common.publicReceipt, internal.publicReceipt].filter(Boolean) as Record<string, unknown>[];
    for (let index = 0; index < publicReceipts.length; index += 1) {
      await writeCompactEvidenceReceipt({
        evidenceRoot: config.evidenceRoot,
        repoRoot: config.repoRoot,
        runId: config.runId,
        sourceCommit: target.sourceCommit,
        scenarioId: scenarioRunId,
        attemptId,
        mode: `${target.id}-${index === 0 ? "COMMON" : "INTERNAL"}`,
        receipt: publicReceipts[index],
      });
    }
    return {
      receipt: (privateReceipt ?? commonReceipt) as Record<string, unknown>,
      commonReceipt,
      parsed: internal.parsed as AttemptResult["parsed"],
      commonParsed: common.parsed as AttemptResult["parsed"],
      publicReceipts,
      rawPacket,
      measurementStarted: true,
      measurementCompleted: true,
      runnerSummaries: {
        internal: buildRoundEvidenceSummary((privateReceipt ?? commonReceipt) as any, {
          surface: privateReceipt ? "internal" : "common",
        }),
        common: buildRoundEvidenceSummary(commonReceipt, { surface: "common" }),
      },
      overhead: {
        contextId,
        sentinelCallbacksMs: sentinel.sentinelCallbacksMs,
        sentinelTotalPerEventMs: sentinel.sentinelTotalPerEventMs,
        sentinelTotalPerWindowMs: sentinel.sentinelTotalPerWindowMs,
        callbackLedgerCount: sentinel.callbackLedger?.length,
        callbackLedgerCapacity: sentinel.callbackLedgerCapacity,
        sentinelAccountedCallbackCount: sentinel.sentinelAccountedCallbackCount,
        callbackLedgerOverflowCount: sentinel.callbackLedgerOverflowCount,
        collectorCallbacksMs: [
          ...(protocolExport?.mainObserverCallbacksMs ?? []),
          ...(protocolExport?.actions?.flatMap((action: any) =>
            action.worker?.collectorSpans?.map((span: any) => span.finishedAt - span.startedAt) ?? []) ?? []),
        ],
        mainObserverCallbacksMs: protocolExport?.mainObserverCallbacksMs ?? [],
        workerCollectorCallbacksMs: protocolExport?.actions?.flatMap((action: any) =>
          action.worker?.collectorSpans?.map((span: any) => span.finishedAt - span.startedAt) ?? []) ?? [],
        callbackIntervals: [
          ...(sentinel.callbackLedger ?? []).map((callback: any, index: number) => ({
            ...callback,
            callbackId: `${callback.pageInstanceId ?? "sentinel"}-sentinel-${index + 1}`,
            sequenceId: index + 1,
            sourceClass: "common-sentinel",
            windowIndex: (commonReceipt.interactionWindows ?? []).findIndex((window: any) =>
              window.pageInstanceId === callback.pageInstanceId
              && callback.startedAt >= window.startedAt && callback.startedAt <= window.endedAt),
          })),
          ...(protocolExport?.mainObserverCallbacks ?? []).map((callback: any) => ({
            ...callback,
            sourceClass: target.protocolMode === "minimal" ? "minimal-probe" : "full-collector",
          })),
        ],
        mainObserverCallbackCount: protocolExport?.status?.mainObserverCallbackCount,
        mainObserverCallbackCapacity: protocolExport?.status?.mainObserverCallbackCapacity,
        mainObserverCallbackOverflowCount: protocolExport?.status?.mainObserverCallbackOverflowCount,
        callbackAttributionComplete: sentinelLedgerIntegrityErrors(sentinel).length === 0
          && sentinel.callbackLedgerOverflowCount === 0
          && sentinel.sentinelAccountedCallbackCount === (sentinel.callbackLedger?.length ?? -1)
          && (target.protocolMode === "off"
          || (Array.isArray(protocolExport?.mainObserverCallbacks)
            && protocolExport.mainObserverCallbacks.length === protocolExport?.status?.mainObserverCallbackCount
            && protocolExport?.status?.mainObserverCallbackCount <= protocolExport?.status?.mainObserverCallbackCapacity
            && protocolExport?.status?.mainObserverCallbackOverflowCount === 0
            && protocolExport?.actions?.every((action: any) => Array.isArray(action.worker?.collectorSpans)))),
        rawLongTasks: sentinel.longTasks
          .filter((task: any) => task.overlapsInteractionWindow === true)
          .map((task: any, index: number) => ({
          ...task,
          locus: observerLongTaskLoci(commonReceipt)[index],
          })),
      },
    };
  } catch (error) {
    if (rawPersisted) {
      throw new Error(`WEB06_POST_MEASUREMENT_PACKAGING_FAILURE:${persistedRawSha256}:${error instanceof Error ? error.message : String(error)}`);
    }
    rawEnvelope.setupFailure = error instanceof Error
      ? { name: error.name, message: error.message }
      : { value: String(error) };
    rawEnvelope.measurementStarted = measurementStarted;
    rawEnvelope.measurementCompleted = false;
    partialAttempt.measurementStarted = measurementStarted;
    partialAttempt.measurementCompleted = false;
    partialAttempt.phase = "failed";
    partialAttempt.failure = classifyWeb06HarnessFailure(error);
    rawEnvelope.browserFailure = {
      pageClosed: page?.isClosed() ?? true,
      messageCode: partialAttempt.failure.code,
    };
    if (responseGuard) {
      try {
        rawEnvelope.artifactResponseGuard = await responseGuard.snapshot("failure");
      } catch {}
    }
    if (page && !page.isClosed()) {
      try {
        await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__?.flushEndpoints?.());
      } catch {}
      try {
        partialAttempt.failureSentinel = await page.evaluate(() =>
          (window as any).__YUNE_WEB06_SENTINEL__?.snapshot?.());
      } catch {}
      try {
        partialAttempt.failureProtocolExport = await readProtocol(page, true);
      } catch {}
    }
    let failureRawPacket: AttemptResult["rawPacket"];
    if (reservation && reservation.committed === false) {
      try {
        const raw = await commitRawEvidencePacket({ reservation, packet: rawEnvelope });
        failureRawPacket = rawEvidencePacketReference(raw);
      } catch (persistenceError) {
        throw new Error(`WEB06_RAW_PERSISTENCE_FATAL:${persistenceError instanceof Error ? persistenceError.message : String(persistenceError)}`);
      }
    }
    const classification = classifyWeb06HarnessFailure(error);
    if (classification.dimension === "fatal") {
      throw new Error(`${classification.code}:${failureRawPacket?.sha256 ?? "raw-unavailable"}`);
    }
    if (measurementStarted && classification.dimension === "behavior") {
      return behaviorFailureAttempt(config, target, run, attemptId, error, contextId, failureRawPacket, partialAttempt);
    }
    return setupInvalidAttempt(config, target, run, attemptId, error, contextId, failureRawPacket, partialAttempt,
      measurementStarted ? retainedRawReceiptBehaviorErrors(rawEnvelope) : []);
  } finally {
    await context?.close();
  }
}

function partialEvidenceSummary(partial: Record<string, any>, rawPacket?: AttemptResult["rawPacket"]) {
  return {
    version: partial.version,
    phase: partial.phase,
    measurementStarted: partial.measurementStarted === true,
    measurementCompleted: partial.measurementCompleted === true,
    driverEventCount: partial.driverEvents?.length ?? 0,
    cadenceGapCount: partial.cadenceGaps?.length ?? 0,
    sentinelEventCount: partial.failureSentinel?.events?.length ?? partial.finalSentinel?.events?.length ?? 0,
    protocolEventCount: partial.failureProtocolExport?.events?.length ?? partial.finalProtocolExport?.events?.length ?? 0,
    protocolActionCount: partial.failureProtocolExport?.actions?.length ?? partial.finalProtocolExport?.actions?.length ?? 0,
    learnedPreSegmentPreserved: partial.learnedPreSegment !== undefined,
    rawPacketSha256: rawPacket?.sha256,
  };
}

function setupInvalidAttempt(config: CollectorEnvironment, target: CollectorTarget, run: ReturnType<typeof resolveScenarioRun>, attemptId: string, error: unknown, contextId: string, rawPacket?: AttemptResult["rawPacket"], partial: Record<string, any> = {}, retainedBehaviorErrors: string[] = []): AttemptResult {
  const code = classifyWeb06HarnessFailure(error).code;
  const behaviorRedObserved = retainedBehaviorErrors.length > 0;
  const parsed: AttemptResult["parsed"] = {
    status: behaviorRedObserved ? "RED_BEHAVIOR" : "SETUP_INVALID",
    cadence: "NOT_APPLICABLE",
    setupErrors: [code],
    behaviorErrors: retainedBehaviorErrors,
    thresholdViolations: [],
    frameRed: false,
    longTaskRed: false,
  };
  return {
    receipt: {
      scenarioId: run.scenarioId,
      scenarioRunId: run.runId,
      schemaId: run.schema,
      attemptId,
      mode: target.id,
      source: sourceIdentity(config, target),
      partialEvidence: partialEvidenceSummary(partial, rawPacket),
    },
    parsed,
    commonParsed: parsed,
    publicReceipts: [],
    overhead: { contextId },
    rawPacket,
    measurementStarted: partial.measurementStarted === true,
    measurementCompleted: false,
    runnerSummaries: {},
  };
}

function behaviorFailureAttempt(config: CollectorEnvironment, target: CollectorTarget, run: ReturnType<typeof resolveScenarioRun>, attemptId: string, error: unknown, contextId: string, rawPacket?: AttemptResult["rawPacket"], partial: Record<string, any> = {}): AttemptResult {
  const code = classifyWeb06HarnessFailure(error).code;
  const parsed: AttemptResult["parsed"] = {
    status: "RED_BEHAVIOR",
    cadence: "NOT_APPLICABLE",
    setupErrors: ["SETUP_INCOMPLETE_MEASUREMENT"],
    behaviorErrors: [code],
    thresholdViolations: [],
    frameRed: false,
    longTaskRed: false,
  };
  return {
    receipt: {
      scenarioId: run.scenarioId,
      scenarioRunId: run.runId,
      schemaId: run.schema,
      attemptId,
      mode: target.id,
      source: sourceIdentity(config, target),
      partialEvidence: partialEvidenceSummary(partial, rawPacket),
    },
    parsed,
    commonParsed: parsed,
    publicReceipts: [],
    overhead: { contextId },
    rawPacket,
    measurementStarted: true,
    measurementCompleted: false,
    runnerSummaries: {},
  };
}

function sourceIdentity(config: CollectorEnvironment, target: CollectorTarget,
  responseGuardSummary?: Record<string, unknown>) {
  return {
    commit: target.sourceCommit,
    tree: target.sourceTree,
    treeState: target.treeState,
    archiveSha256: target.archiveSha256,
    buildInfoSha256: target.buildInfoSha256,
    artifactSha256: target.artifactSha256,
    artifactResponseGuardSha256: target.artifactResponseGuardSha256,
    artifactResponseGuardSummarySha256: responseGuardSummary?.summarySha256,
    identityManifestSha256: config.identityManifestSha256,
    runnerSourceManifestSha256: config.runnerSourceManifestSha256,
    runnerToolingManifestSha256: config.runnerSource.toolingManifestSha256,
    runnerSourceObservationSha256: runnerSourceBefore?.observationSha256,
    observedEnvironmentSha256: observedEnvironment?.observationSha256,
    collectorContractSha256: target.collectorContractSha256,
    scenarioIdsSha256: config.scenarioIdsSha256,
    environmentManifestSha256: config.environmentManifestSha256,
    environmentId: config.environmentId,
    selectedBranch: target.pinnedSelectedBranch,
    disposition: target.pinnedDisposition,
  };
}

function targetUrl(target: CollectorTarget, schema: string, scenarioId: string) {
  const url = new URL(target.origin);
  url.searchParams.set("schema", schema);
  url.searchParams.set("web06Scenario", scenarioId);
  // PRODUCT and preview validate selector-omitted startup; every other mode is explicit.
  if (target.selectorPolicy === "explicit") url.searchParams.set("yuneWeb06Mode", target.protocolMode);
  return url.toString();
}

function artifactGuardDigest(value: unknown) {
  return createHash("sha256")
    .update(Buffer.isBuffer(value) ? value : Buffer.from(JSON.stringify(value), "utf8"))
    .digest("hex");
}

async function installArtifactResponseGuard(page: Page, target: CollectorTarget) {
  const expected = new Map(target.artifactResponseGuard.entries.map((entry: any) => [entry.path, entry]));
  const observations: { path: string; status: number; bytes: number; sha256: string }[] = [];
  const transportFailureCodes: string[] = [];
  const targetOrigin = new URL(target.origin).origin;
  const normalizePath = (rawUrl: string) => {
    const url = new URL(rawUrl);
    if (url.origin !== targetOrigin) return undefined;
    const decoded = decodeURIComponent(url.pathname);
    return decoded === "/" ? target.artifactResponseGuard.rootDocumentPath : decoded.replace(/^\//, "");
  };
  const observe = async (artifactPath: string, status: number, bytes: Buffer) => {
    const sha256 = artifactGuardDigest(bytes);
    observations.push({ path: artifactPath, status, bytes: bytes.length, sha256 });
    const row = expected.get(artifactPath) as any;
    return row !== undefined && status >= 200 && status < 300
      && bytes.length === row.bytes && sha256 === row.sha256;
  };
  await page.route("**/*", async (route) => {
    let artifactPath: string | undefined;
    try {
      artifactPath = normalizePath(route.request().url());
    } catch {
      transportFailureCodes.push("SAME_ORIGIN_RESPONSE_URL_INVALID");
      await route.abort("blockedbyclient");
      return;
    }
    if (artifactPath === undefined) {
      await route.continue();
      return;
    }
    try {
      const response = await route.fetch();
      const bytes = await response.body();
      const valid = await observe(artifactPath, response.status(), bytes);
      if (valid) await route.fulfill({ response, body: bytes });
      else await route.abort("blockedbyclient");
    } catch {
      transportFailureCodes.push(`RESPONSE_BODY_UNAVAILABLE:${artifactGuardDigest(artifactPath)}`);
      await route.abort("failed").catch(() => {});
    }
  });
  const snapshot = (stage: string) => evaluateArtifactResponseGuardObservations({
    guard: target.artifactResponseGuard,
    guardSha256: target.artifactResponseGuardSha256,
    observations,
    stage,
    additionalFailureCodes: transportFailureCodes,
  });
  return {
    async observeDirect(artifactPath: string, status: number, bytes: Buffer) {
      await observe(artifactPath, status, bytes);
    },
    async snapshot(stage: string) {
      return snapshot(stage);
    },
    async assertComplete(stage: string) {
      const bound = snapshot(stage);
      if (!bound.pass) throw new Error(`WEB06_ARTIFACT_RESPONSE_GUARD_FAILED:${bound.failureCodes.join(",")}`);
      return bound;
    },
  };
}

async function waitForReady(page: Page) {
  await page.locator(".yd-input-area:not([disabled])").waitFor({ state: "visible", timeout: readyTimeoutMs });
  await expect(page.locator("[data-yune-loading-indicator]")).toHaveCount(0, { timeout: readyTimeoutMs });
  await expect(page.locator(".yd-input-area")).toBeEnabled();
  await expect(page.locator("[data-yune-section='active']").getByText("AI candidates", { exact: false })).toBeVisible();
  const ai = page.locator("[data-yune-section='active'] label").filter({ hasText: "AI candidates" }).locator("input");
  await expect(ai).not.toBeChecked();
}

async function verifyMeasuredUiPosture(page: Page) {
  const posture = await page.evaluate(() => {
    const ai = [...document.querySelectorAll("[data-yune-section='active'] label")]
      .find((label) => label.textContent?.includes("AI candidates"))?.querySelector("input");
    const inspector = document.querySelector("[data-yune-inspector-toggle] input");
    return {
      locale: navigator.language,
      aiEnabled: ai instanceof HTMLInputElement ? ai.checked : undefined,
      inspectorEnabled: inspector instanceof HTMLInputElement ? inspector.checked : undefined,
      inspectorPanelCount: document.querySelectorAll("[data-yune-inspector='panel']").length,
      debugQueryPresent: new URL(location.href).searchParams.has("debug"),
      pageSize: (document.querySelector("[data-yune-page-size] input[type='range']") as HTMLInputElement | null)?.value,
      viewport: { width: innerWidth, height: innerHeight },
      focused: document.hasFocus(),
      visibilityState: document.visibilityState,
    };
  });
  if (posture.locale !== "zh-HK") throw new Error(`WEB06_SETUP_LOCALE_POSTURE:${posture.locale}`);
  if (posture.aiEnabled !== false) throw new Error("WEB06_SETUP_AI_POSTURE");
  if (posture.inspectorEnabled !== false || posture.inspectorPanelCount !== 0) {
    throw new Error("WEB06_SETUP_INSPECTOR_POSTURE");
  }
  if (posture.debugQueryPresent) throw new Error("WEB06_SETUP_DEBUG_POSTURE");
  if (posture.pageSize !== "6") throw new Error("WEB06_SETUP_PAGE_SIZE_POSTURE");
  if (posture.viewport.width !== 1365 || posture.viewport.height !== 900) {
    throw new Error("WEB06_SETUP_VIEWPORT_POSTURE");
  }
  if (!posture.focused || posture.visibilityState !== "visible") throw new Error("WEB06_SETUP_FOREGROUND_POSTURE");
  return posture;
}

async function verifyServedSource(page: Page, target: CollectorTarget,
  responseGuard?: Awaited<ReturnType<typeof installArtifactResponseGuard>>) {
  const buildInfoResponse = await page.request.get(new URL("/build-info.json", page.url()).toString(), { failOnStatusCode: false });
  expect(buildInfoResponse.ok(), "source-bound build-info.json").toBe(true);
  const buildInfoBytes = await buildInfoResponse.body();
  await responseGuard?.observeDirect("build-info.json", buildInfoResponse.status(), buildInfoBytes);
  const buildInfoSha256 = createHash("sha256").update(buildInfoBytes).digest("hex");
  expect(buildInfoSha256, "exact served build-info.json bytes").toBe(target.buildInfoSha256);
  const buildInfo = JSON.parse(buildInfoBytes.toString("utf8"));
  expect(buildInfo.sourceCommit).toBe(target.sourceCommit);
  expect(buildInfo.sourceTreeState).toBe("clean");
  expect(buildInfo.publicArtifactManifestSha256).toBe(target.artifactSha256);
  if (target.identityRole !== "PRODUCT") {
    expect(buildInfo.sourceTree).toBe(target.sourceTree);
    expect(buildInfo.web06Measurement).toEqual({
      collectorContractSha256: target.collectorContractSha256,
      metricContractVersion: "web06-metric-v1",
      scenarioRegistryVersion: "web06-scenarios-v1",
      behaviorPredicateVersion: "web06-behavior-predicates-v1",
      selectedBranch: target.pinnedSelectedBranch,
      productionDefaultProtocolMode: "minimal",
    });
  } else {
    expect(buildInfo.web06Measurement).toBeUndefined();
  }
  const manifestResponse = await page.request.get(new URL("/public-artifact-manifest.json", page.url()).toString(), { failOnStatusCode: false });
  expect(manifestResponse.ok(), "source-bound artifact manifest").toBe(true);
  const manifestBytes = await manifestResponse.body();
  await responseGuard?.observeDirect("public-artifact-manifest.json", manifestResponse.status(), manifestBytes);
  expect(createHash("sha256").update(manifestBytes).digest("hex")).toBe(target.artifactSha256);
  const manifest = JSON.parse(manifestBytes.toString("utf8"));
  expect(manifest.generatedFor).toBe("yune-web");
  expect(manifest.version).toBe("web03-public-artifact-v1");
  expect(Array.isArray(manifest.files)).toBe(true);
  const paths = new Set<string>();
  for (const file of manifest.files) {
    expect(file.path).toMatch(/^(?!\/)(?!.*(?:^|\/)\.\.?(?:\/|$))(?!.*\\).+$/);
    expect(paths.has(file.path), `duplicate artifact path ${file.path}`).toBe(false);
    paths.add(file.path);
    expect(Number.isSafeInteger(file.bytes) && file.bytes >= 0).toBe(true);
    expect(file.sha256).toMatch(/^[0-9a-f]{64}$/);
    const served = await page.request.get(new URL(`/${file.path}`, page.url()).toString(), { failOnStatusCode: false });
    expect(served.ok(), `served artifact member ${file.path}`).toBe(true);
    const bytes = await served.body();
    await responseGuard?.observeDirect(file.path, served.status(), bytes);
    expect(bytes.length, `served byte count ${file.path}`).toBe(file.bytes);
    expect(createHash("sha256").update(bytes).digest("hex"), `served hash ${file.path}`).toBe(file.sha256);
  }
  for (const required of ["index.html", "worker.js", "yune-web.js", "yune-web.wasm", "schema-asset-manifest.json"]) {
    expect(paths.has(required), `required artifact ${required}`).toBe(true);
  }
  const artifactResponseGuard = await responseGuard?.assertComplete("source-proof");
  return {
    buildInfo,
    buildInfoSha256,
    manifestSha256: target.artifactSha256,
    archiveSha256: target.archiveSha256,
    archiveIdentity: "pinned-sealed-extraction",
    servedFileCount: paths.size,
    artifactResponseGuard,
  };
}

async function prepareDisposableSetup(browser: Browser, target: CollectorTarget, scenarioId: string, schema: string) {
  const context = await browser.newContext({ viewport: { width: 1365, height: 900 }, locale: "zh-HK",
    serviceWorkers: "block" });
  try {
    await context.addInitScript(installWeb06Sentinel);
    await context.addInitScript(({ activeSchema }) => {
      localStorage.setItem("activeSchema", activeSchema);
      localStorage.setItem("uiLanguage", "en");
      localStorage.setItem("enableAI", "false");
      localStorage.setItem("pageSize", "6");
    }, { activeSchema: schema });
    const page = await context.newPage();
    const responseGuard = await installArtifactResponseGuard(page, target);
    await page.goto(targetUrl(target, schema, scenarioId), { waitUntil: "domcontentloaded", timeout: readyTimeoutMs });
    await waitForReady(page);
    const sourceProof = await verifyServedSource(page, target, responseGuard);
    const pageSizeSetup = await forceRealSixRowSetup(page, schema, target.protocolMode !== "off");
    if (scenarioId === "correction") await enableCorrectionThroughUi(page, target.protocolMode !== "off");
    const uiCapabilities = await readUiCapabilities(page, scenarioId, target.protocolMode !== "off");
    if (target.protocolMode !== "off") {
      await waitForProtocolIdle(page);
      const health = protocolHealthBlockers(await readProtocol(page));
      if (health.length) throw new Error(`WEB06_SETUP_DISPOSABLE_PROTOCOL_DIRTY:${health.join(",")}`);
    }
    const expectedConfiguration = {
      activeSchema: schema,
      uiLanguage: "en",
      enableAI: "false",
      pageSize: "6",
    };
    const localValues = await page.evaluate(() => Object.fromEntries(
      ["activeSchema", "uiLanguage", "enableAI", "pageSize"].map((name) => [name, localStorage.getItem(name)]),
    ));
    for (const [name, value] of Object.entries(expectedConfiguration)) {
      if (localValues[name] !== value) throw new Error(`WEB06_SETUP_CONFIGURATION_TRANSFER:${name}`);
    }
    return {
      receipt: {
        kind: "web06-disposable-setup-v1",
        sourceProof,
        artifactResponseGuard: await responseGuard.assertComplete("disposable-setup-complete"),
        pageSizeSetup,
        uiCapabilities,
        disposableActionsExcludedFromMeasuredRealm: true,
        measuredRealmFresh: true,
        persistenceTransferredToMeasuredRealm: false,
        configurationRecipe: {
          localStorageSha256: digestJson(expectedConfiguration),
          requiredKeys: Object.keys(expectedConfiguration),
        },
      },
    };
  } finally {
    await context.close();
  }
}

async function forceRealSixRowSetup(page: Page, schema: string, hasProtocol: boolean) {
  const range = page.locator("[data-yune-section='display'] input[type='range']");
  await expect(range).toHaveValue("6");
  await range.focus();
  await page.keyboard.press("ArrowRight");
  await waitForProtocolIdle(page, hasProtocol);
  await expect(range).toHaveValue("7");
  const sevenRows = await probeCandidateRows(page, schema, hasProtocol);
  if (sevenRows !== 7) throw new Error(`WEB06_PAGE_SIZE_ENGINE_NOT_SEVEN:${sevenRows}`);
  await range.focus();
  await page.keyboard.press("ArrowLeft");
  await waitForProtocolIdle(page, hasProtocol);
  await expect(range).toHaveValue("6");
  return {
    uiTransition: [6, 7, 6],
    configuredPageSize: 6,
    sevenRows,
    restoredControlValue: await range.inputValue(),
    realPreferencesControl: true,
  };
}

async function probeCandidateRows(page: Page, schema: string, hasProtocol: boolean) {
  const input = page.locator(".yd-input-area");
  await input.focus();
  await page.keyboard.type(schema === "luna_pinyin" ? "ni" : "nei");
  await page.waitForFunction(() => document.querySelectorAll(".candidate-panel .candidate-row").length > 0);
  await waitForProtocolIdle(page, hasProtocol);
  const count = await page.locator(".candidate-panel .candidate-row").count();
  await page.keyboard.press("Escape");
  await waitForProtocolIdle(page, hasProtocol);
  await page.waitForFunction(() => (document.querySelector(".candidate-preedit")?.textContent?.trim() ?? "") === "");
  return count;
}

async function enableCorrectionThroughUi(page: Page, hasProtocol: boolean) {
  const control = page.locator("[data-yune-section='active'] label").filter({ hasText: "Auto-correction" }).locator("input");
  if (!(await control.isChecked())) await control.click();
  await waitForProtocolIdle(page, hasProtocol);
  await expect(control).toBeChecked();
}

async function readUiCapabilities(page: Page, scenarioId: string, hasProtocol: boolean) {
  const capabilities = await page.evaluate(() => ({
    importUserdbSameTask: false,
    backgroundCausality: typeof (window as any).__YUNE_WEB06__?.status === "function",
    browserLifecycleContinuity:
      typeof (window as any).__YUNE_WEB06__?.prepareLearnedReloadContinuity === "function"
      && typeof (window as any).__YUNE_WEB06__?.bindLearnedReloadWindow === "function"
      && typeof sessionStorage !== "undefined",
    publicDeployControl: document.querySelector("[data-yune-control-redeploy]") !== null,
    publicCustomizeValueControl: document.querySelector("[data-yune-freeform-customize-submit]") !== null,
  }));
  if (!hasProtocol) {
    capabilities.backgroundCausality = false;
    capabilities.browserLifecycleContinuity = false;
  }
  if (scenarioId === "fifo-pressure-barriers" && hasProtocol) {
    capabilities.importUserdbSameTask = await probeImportContinuationMarker(page);
  }
  return capabilities;
}

async function probeImportContinuationMarker(page: Page) {
  const detail = await page.evaluate(() => new Promise<any>((resolve, reject) => {
    const timeout = window.setTimeout(() => reject(new Error("WEB06_IMPORT_CONTINUATION_MARKER_TIMEOUT")), 5000);
    document.addEventListener("yune-web06-import-enqueued", (event) => {
      window.clearTimeout(timeout);
      resolve((event as CustomEvent).detail);
    }, { once: true });
    const input = document.querySelector("[data-yune-userdb-import-input]");
    if (!(input instanceof HTMLInputElement)) {
      window.clearTimeout(timeout);
      reject(new Error("WEB06_IMPORT_CONTROL_MISSING"));
      return;
    }
    const transfer = new DataTransfer();
    transfer.items.add(new File([new Uint8Array()], "web06-capability.userdb", { type: "text/plain" }));
    input.files = transfer.files;
    input.dispatchEvent(new Event("change", { bubbles: true, composed: true }));
  }));
  await waitForProtocolIdle(page, true);
  return detail?.protocolVersion === "web06-private-v1"
    && typeof detail?.eventId === "string"
    && Number.isSafeInteger(detail?.eventSequenceId);
}

async function readProtocol(page: Page, exportReceipts = false) {
  return page.evaluate((shouldExport) => {
    const api = (window as any).__YUNE_WEB06__;
    if (api === undefined) return undefined;
    const status = api.status();
    const value: any = {
      header: {
        protocolVersion: api.protocolVersion,
        mode: api.mode,
        modeProvenance: api.modeProvenance,
        pageInstanceId: status.pageInstanceId,
        measurementId: status.measurementId,
        continuityNonce: status.continuityNonce,
        reloadContinuityPhase: status.reloadContinuityPhase,
      },
      status,
      invalidations: api.invalidations(),
    };
    if (shouldExport) {
      value.events = api.events();
      value.actions = api.actions();
      value.mainObserverCallbacksMs = typeof api.mainObserverCallbacksMs === "function"
        ? api.mainObserverCallbacksMs()
        : undefined;
      value.mainObserverCallbacks = typeof api.mainObserverCallbacks === "function"
        ? api.mainObserverCallbacks()
        : undefined;
      value.status = api.status();
      value.invalidations = api.invalidations();
    }
    return value;
  }, exportReceipts);
}

async function waitForProtocolIdle(page: Page, enabled = true) {
  if (!enabled) {
    await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    return;
  }
  await page.waitForFunction(() => {
    const api = (window as any).__YUNE_WEB06__;
    if (!api) return false;
    const status = api.status();
    return status.valid && status.queueDepth === 0 && status.runningActionId === undefined
      && status.pendingFanoutActions === 0 && status.pendingTerminalActions === 0;
  });
}

async function waitForExactCompletion(page: Page, hasProtocol: boolean, expectedActionCount: number) {
  if (!hasProtocol) {
    await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    return { timedOut: false, expectedActionCount, observedActionCount: undefined, protocolStatus: undefined };
  }
  let timedOut = false;
  try {
    await page.waitForFunction(() => {
      const api = (window as any).__YUNE_WEB06__;
      const status = api?.status();
      return status && status.queueDepth === 0
        && status.runningActionId === undefined
        && status.pendingFanoutActions === 0
        && status.pendingTerminalActions === 0;
    }, undefined, { timeout: 10_000 });
  } catch {
    timedOut = true;
  }
  const observed = await page.evaluate(() => {
    const api = (window as any).__YUNE_WEB06__;
    const observedActionCount = typeof api?.snapshot === "function"
      ? api.snapshot().actions?.length
      : api?.actions?.().length;
    return { observedActionCount, protocolStatus: api?.status?.() };
  });
  const { observedActionCount, protocolStatus } = observed;
  if (!timedOut && observedActionCount !== expectedActionCount) {
    throw new Error(`WEB06_ACTION_COMPLETION_COUNT:${observedActionCount}!=${expectedActionCount}`);
  }
  return { timedOut, expectedActionCount, observedActionCount, protocolStatus };
}

async function collectCalibration(page: Page, worker: boolean) {
  await waitForProtocolIdle(page, worker);
  const driver = [];
  const workerValues = [];
  for (let index = 0; index < WEB06_THRESHOLDS.calibration.exchangesPerBoundary; index += 1) {
    const d0 = performance.now();
    const pageTimes = await page.evaluate(() => {
      const m1 = performance.now();
      const m2 = performance.now();
      return { m1, m2 };
    });
    const d3 = performance.now();
    driver.push({ d0, ...pageTimes, d3 });
    if (worker) {
      const exchange = await page.evaluate(() => (window as any).__YUNE_WEB06__.clockPing());
      workerValues.push({
        m0: exchange.mainSentAt,
        w1: exchange.workerReceivedAt,
        w2: exchange.workerSentAt,
        m3: exchange.mainReceivedAt,
      });
    }
  }
  return { driver, worker: workerValues };
}

async function driveScenario(page: Page, scenarioId: string, hasProtocol: boolean, target: CollectorTarget,
  responseGuard: Awaited<ReturnType<typeof installArtifactResponseGuard>>,
  onProgress: (progress: Record<string, unknown>) => void = () => {}) {
  if (scenarioId === "learned-row") return driveLearnedScenario(page, hasProtocol, target, responseGuard, onProgress);
  const row = SCENARIO_REGISTRY[scenarioId];
  const expected = expandScenarioExpectedTimeline(scenarioId);
  const driverEvents: Record<string, unknown>[] = [];
  const cadenceGaps: Record<string, unknown>[] = [];
  const burstRecoveries: Record<string, unknown>[] = [];
  const pressureProofs: Record<string, unknown>[] = [];
  const argumentCommitments: Record<string, Record<string, Record<string, string>>> = {};
  let previousActualDispatchAt: number | undefined;
  let completedActions = 0;
  let windowNumber = 0;
  let windowStartedAt: number | undefined;
  let initialDomObserved: unknown;
  const progress = (phase: string) => onProgress({
    phase,
    driverEvents,
    cadenceGaps,
    burstRecoveries,
    pressureProofs,
    argumentCommitments,
    completedActionCount: completedActions,
    windowNumber,
    initialDomObserved,
  });
  const startWindow = async () => {
    windowNumber += 1;
    const started = await page.evaluate((label) =>
      (window as any).__YUNE_WEB06_SENTINEL__.startWindow(label), `${scenarioId}-window-${windowNumber}`);
    windowStartedAt = started.startedAt;
    initialDomObserved ??= started.initialDomObserved;
    progress("window-started");
  };
  const finishWindow = async () => {
    await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.flushEndpoints());
    const ended = await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.endWindow());
    if (!Number.isFinite(windowStartedAt) || ended.endedAt <= (windowStartedAt as number)) {
      throw new Error("WEB06_WINDOW_DURATION_INVALID");
    }
    const durationMs = ended.endedAt - (windowStartedAt as number);
    await page.evaluate(({ label, duration }) =>
      (window as any).__YUNE_WEB06_SENTINEL__.captureIdleControl(label, duration), {
      label: `${scenarioId}-idle-${windowNumber}`,
      duration: durationMs,
    });
    windowStartedAt = undefined;
    progress("window-and-idle-control-complete");
  };
  await startWindow();
  for (let stepIndex = 0; stepIndex < row.steps.length; stepIndex += 1) {
    const step = row.steps[stepIndex];
    const batchPartnerId = SAME_TASK_PAIRS[step.id];
    if (batchPartnerId !== undefined) {
      const later = row.steps[stepIndex + 1];
      if (later?.id !== batchPartnerId) throw new Error(`WEB06_SAME_TASK_PAIR_REORDERED:${step.id}`);
      const batch = await driveSameTaskPair(page, step, later);
      for (const item of [step, later]) {
        const dispatch = batch.dispatches[item.id];
        for (const event of expected.events.filter((candidate) => candidate.stepId === item.id)) {
          driverEvents.push({
            stepId: item.id,
            type: event.type,
            requestedDriverDispatchAt: dispatch.requestedDriverDispatchAt,
            actualDriverDispatchAt: dispatch.actualDriverDispatchAt,
          });
        }
        completedActions += item.actions.length;
      }
      Object.assign(argumentCommitments, batch.argumentCommitments);
      progress("same-task-pair-dispatched");
      await waitForExactCompletion(page, hasProtocol, completedActions);
      await page.evaluate(async (ids) => {
        for (const stepId of ids) (window as any).__YUNE_WEB06_SENTINEL__.scheduleEndpoint(stepId);
        await (window as any).__YUNE_WEB06_SENTINEL__.flushEndpoints();
      }, [step.id, later.id]);
      previousActualDispatchAt = batch.dispatches[later.id].actualDriverDispatchAt;
      const following = row.steps[stepIndex + 2];
      if (row.expectedInteractionWindowCount > 1 && following && following.subcase !== later.subcase) {
        await finishWindow();
        await startWindow();
      }
      stepIndex += 1;
      continue;
    }
    if (step.cadence === "after-exact-final-paint" || step.declaredBurstPauseAfter === true) {
      await waitForExactCompletion(page, hasProtocol, completedActions);
    }
    let requestedDriverDispatchAt = performance.now();
    let rebasedAfterLateHost = false;
    if (Number.isFinite(step.nominalGapMs) && previousActualDispatchAt !== undefined) {
      const scheduled = advanceCadenceDeadline({
        previousActualDispatchAt,
        nominalGapMs: step.nominalGapMs,
        nowMs: performance.now(),
      });
      requestedDriverDispatchAt = scheduled.requestedDispatchAt;
      rebasedAfterLateHost = scheduled.rebasedAfterLateHost;
      await waitUntil(requestedDriverDispatchAt);
    }
    const expectedTypes = step.source === "keyboard" ? ["keydown", "keyup"] : [step.domEventType ?? step.source];
    await page.evaluate((arm) => (window as any).__YUNE_WEB06_SENTINEL__.armStep(arm), {
      stepId: step.id,
      expectedTypes,
      precursorTypes: step.id === "extended-option-target" ? ["click"] : [],
    });
    const actualDriverDispatchAt = performance.now();
    if (Number.isFinite(step.nominalGapMs) && previousActualDispatchAt !== undefined) {
      cadenceGaps.push({
        stepId: step.id,
        nominalGapMs: step.nominalGapMs,
        actualDriverGapMs: actualDriverDispatchAt - previousActualDispatchAt,
        rebasedAfterLateHost,
      });
    }
    if (step.source === "keyboard") {
      await page.keyboard.press(playwrightKey(step.key));
    } else if (step.source === "control") {
      const commitment = await dispatchRealControl(page, step);
      if (commitment) argumentCommitments[step.id] = commitment;
    } else {
      throw new Error(`WEB06_UNSUPPORTED_REAL_UI_STEP:${step.id}:${step.source}`);
    }
    await page.evaluate(({ stepId, requested, actual }) =>
      (window as any).__YUNE_WEB06_SENTINEL__.bindDriverDispatch(stepId, requested, actual), {
      stepId: step.id,
      requested: requestedDriverDispatchAt,
      actual: actualDriverDispatchAt,
    });
    if (step.sample !== "none") {
      await page.evaluate((stepId) => (window as any).__YUNE_WEB06_SENTINEL__.scheduleEndpoint(stepId), step.id);
    }
    const matchingEvents = expected.events.filter((event) => event.stepId === step.id);
    for (const event of matchingEvents) {
      driverEvents.push({
        stepId: step.id,
        type: event.type,
        requestedDriverDispatchAt,
        actualDriverDispatchAt,
      });
    }
    completedActions += step.actions.length;
    progress("step-dispatched");
    if (step.sample === "terminal" || step.declaredBurstPauseAfter === true) {
      const completion = await waitForExactCompletion(page, hasProtocol, completedActions);
      const endpoint = await page.evaluate(async (stepId) => {
        await (window as any).__YUNE_WEB06_SENTINEL__.flushEndpoints();
        return (window as any).__YUNE_WEB06_SENTINEL__.latestSnapshot(stepId);
      }, step.id);
      const latestPaintAt = endpoint?.observedAt;
      if (step.declaredBurstPauseAfter === true) {
        const sentinelStatus = await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.status());
        const protocolStatus = completion.protocolStatus;
        burstRecoveries.push({
          afterStepId: step.id,
          latestPaintAt,
          expectedCompletedActionCount: completedActions,
          idleSnapshot: {
            queueDepth: protocolStatus?.queueDepth ?? 0,
            runningActionId: protocolStatus?.runningActionId ?? null,
            pendingFanoutActions: protocolStatus?.pendingFanoutActions ?? 0,
            pendingTerminalActions: protocolStatus?.pendingTerminalActions ?? 0,
            pendingSentinelCaptures: sentinelStatus.pendingCaptures,
            completedActionCount: completion.observedActionCount ?? completedActions,
          },
        });
      }
    }
    previousActualDispatchAt = actualDriverDispatchAt;
    const next = row.steps[stepIndex + 1];
    if (row.expectedInteractionWindowCount > 1 && next && next.subcase !== step.subcase) {
      await finishWindow();
      await startWindow();
    }
  }
  await finishWindow();
  if (windowNumber !== row.expectedInteractionWindowCount) {
    throw new Error(`WEB06_WINDOW_COUNT:${windowNumber}!=${row.expectedInteractionWindowCount}`);
  }
  progress("drive-complete");
  return { driverEvents, cadenceGaps, burstRecoveries, pressureProofs, argumentCommitments, initialDomObserved };
}

async function driveLearnedScenario(page: Page, hasProtocol: boolean, target: CollectorTarget,
  responseGuard: Awaited<ReturnType<typeof installArtifactResponseGuard>>,
  onProgress: (progress: Record<string, unknown>) => void = () => {}) {
  if (!hasProtocol) throw new Error("WEB06_SETUP_LEARNED_PRIVATE_PROTOCOL_REQUIRED");
  const row = SCENARIO_REGISTRY["learned-row"];
  const expected = expandScenarioExpectedTimeline("learned-row");
  const boundaryIndex = row.steps.findIndex((step: any) => step.source === "browser-lifecycle");
  if (boundaryIndex < 1) throw new Error("WEB06_LEARNED_BOUNDARY_MISSING");
  const driverEvents: Record<string, unknown>[] = [];
  const cadenceGaps: Record<string, unknown>[] = [];
  const argumentCommitments: Record<string, Record<string, Record<string, string>>> = {};
  let initialDomObserved: unknown;
  const progress = (phase: string, extra: Record<string, unknown> = {}) => onProgress({
    phase,
    driverEvents,
    cadenceGaps,
    argumentCommitments,
    initialDomObserved,
    ...extra,
  });

  const runSegment = async (steps: any[], segmentLabel: string) => {
    const started = await page.evaluate((label) =>
      (window as any).__YUNE_WEB06_SENTINEL__.startWindow(label), segmentLabel);
    initialDomObserved ??= started.initialDomObserved;
    let previousActualDispatchAt: number | undefined;
    let segmentActionCount = 0;
    for (const step of steps) {
      if (step.source !== "keyboard") throw new Error(`WEB06_LEARNED_STEP_SOURCE:${step.id}`);
      if (step.cadence === "after-exact-final-paint") {
        await waitForExactCompletion(page, true, segmentActionCount);
      }
      let requestedDriverDispatchAt = performance.now();
      let rebasedAfterLateHost = false;
      if (Number.isFinite(step.nominalGapMs) && previousActualDispatchAt !== undefined) {
        const scheduled = advanceCadenceDeadline({
          previousActualDispatchAt,
          nominalGapMs: step.nominalGapMs,
          nowMs: performance.now(),
        });
        requestedDriverDispatchAt = scheduled.requestedDispatchAt;
        rebasedAfterLateHost = scheduled.rebasedAfterLateHost;
        await waitUntil(requestedDriverDispatchAt);
      }
      await page.evaluate((arm) => (window as any).__YUNE_WEB06_SENTINEL__.armStep(arm), {
        stepId: step.id,
        expectedTypes: ["keydown", "keyup"],
        precursorTypes: [],
      });
      const actualDriverDispatchAt = performance.now();
      if (Number.isFinite(step.nominalGapMs) && previousActualDispatchAt !== undefined) {
        cadenceGaps.push({
          stepId: step.id,
          nominalGapMs: step.nominalGapMs,
          actualDriverGapMs: actualDriverDispatchAt - previousActualDispatchAt,
          rebasedAfterLateHost,
        });
      }
      await page.keyboard.press(playwrightKey(step.key));
      await page.evaluate(({ stepId, requested, actual }) =>
        (window as any).__YUNE_WEB06_SENTINEL__.bindDriverDispatch(stepId, requested, actual), {
        stepId: step.id,
        requested: requestedDriverDispatchAt,
        actual: actualDriverDispatchAt,
      });
      await page.evaluate((stepId) => (window as any).__YUNE_WEB06_SENTINEL__.scheduleEndpoint(stepId), step.id);
      for (const event of expected.events.filter((candidate) => candidate.stepId === step.id)) {
        driverEvents.push({
          stepId: step.id,
          type: event.type,
          requestedDriverDispatchAt,
          actualDriverDispatchAt,
        });
      }
      segmentActionCount += step.actions.length;
      progress("learned-step-dispatched", { segmentLabel, segmentActionCount });
      if (step.sample === "terminal") await waitForExactCompletion(page, true, segmentActionCount);
      previousActualDispatchAt = actualDriverDispatchAt;
    }
    await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.flushEndpoints());
    const ended = await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.endWindow());
    const durationMs = ended.endedAt - started.startedAt;
    if (!(durationMs > 0)) throw new Error("WEB06_LEARNED_WINDOW_DURATION_INVALID");
    await page.evaluate(({ label, duration }) =>
      (window as any).__YUNE_WEB06_SENTINEL__.captureIdleControl(label, duration), {
      label: `${segmentLabel}-idle`,
      duration: durationMs,
    });
    return { segmentActionCount, startedAt: started.startedAt, endedAt: ended.endedAt };
  };

  const preRun = await runSegment(row.steps.slice(0, boundaryIndex), "learned-row-pre-reload");
  await waitForProtocolIdle(page, true);
  const preBoundaryCalibration = await collectCalibration(page, true);
  const continuityPrepare = await page.evaluate(() => {
    const api = (window as any).__YUNE_WEB06__;
    const measurementId = crypto.randomUUID();
    const actions = api.actions();
    const expectedTerminalActionId = actions.at(-1)?.identity?.actionId;
    return api.prepareLearnedReloadContinuity(measurementId, expectedTerminalActionId);
  });
  if (typeof continuityPrepare?.measurementId !== "string"
    || typeof continuityPrepare?.continuityNonce !== "string"
    || continuityPrepare?.phase !== "pre-reload"
    || continuityPrepare?.terminal?.persistenceCompleted !== true) {
    throw new Error("WEB06_LEARNED_PREPARE_CONTINUITY_INVALID");
  }
  const preSegment = {
    sentinel: await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.snapshot()),
    protocol: await readProtocol(page, true),
    postCalibration: preBoundaryCalibration,
  };
  const boundary = expected.events.find((event) => event.stepId === "learned-reload-boundary");
  if (!boundary) throw new Error("WEB06_LEARNED_BOUNDARY_EVENT_MISSING");
  const lifecycleClock = await page.evaluate(() => performance.now());
  const lifecycleMarker = {
    ...boundary,
    pageInstanceId: preSegment.sentinel.longTaskObserver.pageInstanceId,
    eventTimestamp: lifecycleClock,
    normalizedEventAt: lifecycleClock,
    sentinelObservedAt: lifecycleClock,
    originOwner: "harness-browser-lifecycle",
  };
  progress("learned-pre-reload-segment-preserved", {
    learnedPreSegment: preSegment,
    preRun,
    lifecycleMarker,
  });

  await page.reload({ waitUntil: "domcontentloaded", timeout: readyTimeoutMs });
  progress("learned-page-reloaded", { learnedPreSegment: preSegment, preRun });
  await waitForReady(page);
  const postReloadSourceProof = await verifyServedSource(page, target, responseGuard);
  const arrived = await readProtocol(page);
  if (arrived?.status?.reloadContinuityPhase !== "post-reload-arrived"
    || arrived?.status?.measurementId !== continuityPrepare.measurementId
    || arrived?.status?.continuityNonce !== continuityPrepare.continuityNonce) {
    throw new Error("WEB06_LEARNED_RELOAD_ARRIVAL_INVALID");
  }
  await page.evaluate(() => (window as any).__YUNE_WEB06__.resetReceipts());
  await waitForProtocolIdle(page, true);
  const postReloadCalibration = await collectCalibration(page, true);
  const continuityBind = await page.evaluate(({ measurementId, continuityNonce }) =>
    (window as any).__YUNE_WEB06__.bindLearnedReloadWindow(measurementId, continuityNonce), {
    measurementId: continuityPrepare.measurementId,
    continuityNonce: continuityPrepare.continuityNonce,
  });
  const bound = await readProtocol(page);
  if (bound?.status?.reloadContinuityPhase !== "post-reload-bound"
    || bound?.status?.measurementId !== continuityPrepare.measurementId
    || bound?.status?.continuityNonce !== continuityPrepare.continuityNonce) {
    throw new Error("WEB06_LEARNED_RELOAD_BIND_INVALID");
  }
  await page.evaluate(() => (window as any).__YUNE_WEB06_SENTINEL__.reset());
  const postReloadIdleFrameIntervalsMs = await page.evaluate(
    (count) => (window as any).__YUNE_WEB06_SENTINEL__.takeIdleIntervals(count),
    WEB06_THRESHOLDS.frame.requiredIdleIntervals,
  );
  const postReloadEventClockProbe = await page.evaluate(() =>
    (window as any).__YUNE_WEB06_SENTINEL__.probeEventClock());
  const postRun = await runSegment(row.steps.slice(boundaryIndex + 1), "learned-row-post-reload");
  progress("learned-post-reload-segment-complete", { learnedPreSegment: preSegment, preRun, postRun });
  return {
    driverEvents,
    cadenceGaps,
    burstRecoveries: [],
    pressureProofs: [],
    argumentCommitments,
    initialDomObserved,
    learned: {
      preSegment,
      preRun,
      postRun,
      lifecycleMarker,
      postReloadCalibration,
      postReloadEventClockProbe,
      postReloadIdleFrameIntervalsMs,
      postReloadSourceProof,
      lifecycleContinuity: {
        browserLifecycleEventCount: 1,
        measurementId: continuityPrepare.measurementId,
        pre: continuityPrepare,
        post: continuityBind,
      },
    },
  };
}

const SAME_TASK_PAIRS: Record<string, string> = Object.freeze({
  "fifo-commit": "fifo-commit-later-1",
  "fifo-select": "fifo-select-later-1",
  "fifo-page": "fifo-page-later-1",
  "fifo-userdb-import": "fifo-userdb-later-1",
  "extended-option-earlier-1": "extended-option-target",
  "extended-schema-earlier-1": "extended-schema-target",
  "extended-deploy-earlier-1": "extended-deploy-target",
  "extended-persistence-earlier-1": "extended-persistence-target",
  "extended-error-earlier-1": "extended-error-target",
});

async function driveSameTaskPair(page: Page, earlier: any, later: any) {
  const requestedDriverDispatchAt = performance.now();
  const arms = [earlier, later].map((step) => ({
    stepId: step.id,
    expectedTypes: step.source === "keyboard" ? ["keydown", "keyup"] : [step.domEventType],
    precursorTypes: step.id === "extended-option-target" ? ["click"] : [],
  }));
  await page.evaluate((value) => (window as any).__YUNE_WEB06_SENTINEL__.armSteps(value), arms);
  const actualDriverDispatchAt = performance.now();
  await page.evaluate(({ first, second }) => {
    const dispatchKey = (step: any) => {
      const init = {
        key: step.key,
        code: step.code,
        bubbles: true,
        cancelable: true,
        composed: true,
      };
      document.dispatchEvent(new KeyboardEvent("keydown", init));
      document.dispatchEvent(new KeyboardEvent("keyup", init));
    };
    const dispatchControl = (step: any) => {
      if (step.id === "extended-option-target") {
        const label = [...document.querySelectorAll("[data-yune-section='live'] label")]
          .find((candidate) => candidate.textContent?.includes("Extended charset"));
        const input = label?.querySelector("input");
        if (!(input instanceof HTMLInputElement)) throw new Error("WEB06_EXTENDED_OPTION_CONTROL_MISSING");
        input.click();
        return;
      }
      if (step.id === "extended-schema-target") {
        const select = document.querySelector(step.control);
        if (!(select instanceof HTMLSelectElement)) throw new Error("WEB06_SCHEMA_CONTROL_MISSING");
        select.value = "luna_pinyin";
        select.dispatchEvent(new Event("change", { bubbles: true, composed: true }));
        return;
      }
      if (["fifo-userdb-import", "extended-persistence-target"].includes(step.id)) {
        const input = document.querySelector(step.control);
        if (!(input instanceof HTMLInputElement)) throw new Error("WEB06_IMPORT_CONTROL_MISSING");
        const transfer = new DataTransfer();
        transfer.items.add(new File([new Uint8Array()], "empty.userdb", { type: "text/plain" }));
        input.files = transfer.files;
        input.dispatchEvent(new Event("change", { bubbles: true, composed: true }));
        return;
      }
      const button = document.querySelector(step.control);
      if (!(button instanceof HTMLElement)) throw new Error(`WEB06_CONTROL_MISSING:${step.id}`);
      button.click();
    };
    // Frozen pressure contract: both originating DOM events are dispatched
    // synchronously in this one page task. No Promise, await, or microtask may
    // separate an import/change barrier from the later key.
    if (first.source === "keyboard") dispatchKey(first);
    else dispatchControl(first);
    if (second.source === "keyboard") dispatchKey(second);
    else dispatchControl(second);
  }, { first: earlier, second: later });
  await page.evaluate(({ stepIds, requested, actual }) => {
    for (const stepId of stepIds) {
      (window as any).__YUNE_WEB06_SENTINEL__.bindDriverDispatch(stepId, requested, actual);
    }
  }, { stepIds: [earlier.id, later.id], requested: requestedDriverDispatchAt, actual: actualDriverDispatchAt });
  const argumentCommitments: Record<string, Record<string, Record<string, string>>> = {};
  for (const step of [earlier, later]) {
    if (["fifo-userdb-import", "extended-persistence-target"].includes(step.id)) {
      argumentCommitments[step.id] = { importUserdb: { userdbTextSha256: EMPTY_USERDB_FIXTURE_SHA256 } };
    } else if (step.id === "extended-schema-target") {
      argumentCommitments[step.id] = { customize: { dictionaryExcludeSha256: EMPTY_DICTIONARY_EXCLUDE_SHA256 } };
    } else if (step.id === "extended-error-target") {
      argumentCommitments[step.id] = { customizeValue: { customizeValueSha256: INJECTED_ERROR_VALUE_SHA256 } };
    }
  }
  return {
    dispatches: {
      [earlier.id]: { requestedDriverDispatchAt, actualDriverDispatchAt },
      [later.id]: { requestedDriverDispatchAt, actualDriverDispatchAt },
    },
    argumentCommitments,
  };
}

async function dispatchRealControl(page: Page, step: any) {
  if (step.id === "extended-option-target") {
    await page.locator("[data-yune-section='live'] label").filter({ hasText: "Extended charset" }).locator("input").click();
    return undefined;
  }
  if (step.id === "extended-schema-target") {
    await page.locator(step.control).selectOption("luna_pinyin");
    return { customize: { dictionaryExcludeSha256: EMPTY_DICTIONARY_EXCLUDE_SHA256 } };
  }
  if (["fifo-userdb-import", "extended-persistence-target"].includes(step.id)) {
    await page.locator(step.control).setInputFiles({ name: "empty.userdb", mimeType: "text/plain", buffer: Buffer.alloc(0) });
    return { importUserdb: { userdbTextSha256: EMPTY_USERDB_FIXTURE_SHA256 } };
  }
  if (step.id === "extended-error-target") {
    const inputs = page.locator("[data-yune-freeform-customize] input");
    await inputs.nth(0).fill("");
    await inputs.nth(1).fill("");
    await inputs.nth(2).fill("web06-unused-error-control-value");
    await page.locator(step.control).click();
    return { customizeValue: { customizeValueSha256: INJECTED_ERROR_VALUE_SHA256 } };
  }
  if (step.domEventType === "click") {
    await page.locator(step.control).click();
    return undefined;
  }
  throw new Error(`WEB06_UNSUPPORTED_REAL_CONTROL:${step.id}`);
}

function buildPressureProofs(receipt: any, scenarioId: string, workerRawCalibration: any) {
  const proofs = [];
  // This is an identity projection only. Parser and independent verifier each
  // recompute every timing/queue inequality from immutable raw action fields.
  buildClockCalibration(workerRawCalibration.pre, workerRawCalibration.post, "main-worker");
  for (const pair of WEB06_PRESSURE_PAIR_REGISTRY[scenarioId] ?? []) {
    const { subcase, earlierStepId, laterStepId } = pair;
    const earlierActions = receipt.actions.filter((action: any) => action.stepId === earlierStepId && action.originKind !== "background");
    const laterActions = receipt.actions.filter((action: any) => action.stepId === laterStepId && action.originKind !== "background");
    const earlier = earlierActions.at(-1);
    const later = laterActions[0];
    if (!earlier || !later) throw new Error(`WEB06_PRESSURE_ACTION_MISSING:${earlierStepId}`);
    proofs.push({
      subcase,
      earlierStepId,
      laterStepId,
      earlierSequenceId: earlier.sequenceId,
      laterSequenceId: later.sequenceId,
      dispatchContract: "single-page-task-no-await",
    });
  }
  return proofs;
}

async function prepareExtendedErrorControl(page: Page) {
  const surface = page.locator("[data-yune-freeform-customize]");
  if (await surface.count() === 0) return;
  const inputs = surface.locator("input");
  await inputs.nth(0).fill("");
  await inputs.nth(1).fill("");
  await inputs.nth(2).fill("web06-unused-error-control-value");
}

function playwrightKey(key: string) {
  if (key === " ") return "Space";
  if (key === "Shift") return "ShiftLeft";
  return key;
}

async function waitUntil(deadline: number) {
  while (true) {
    const remaining = deadline - performance.now();
    if (remaining <= 0) return;
    await new Promise((resolve) => setTimeout(resolve, Math.max(0, remaining - 0.25)));
  }
}

function primarySchema(value: string | readonly string[]) {
  return Array.isArray(value) ? value[0] : value;
}

function digestJson(value: unknown) {
  return createHash("sha256").update(JSON.stringify(value), "utf8").digest("hex");
}
