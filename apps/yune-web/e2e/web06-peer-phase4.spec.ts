import { chromium, expect, test, type BrowserContext, type Page } from "@playwright/test";

import { createServer, type Server } from "node:http";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { performance } from "node:perf_hooks";

import {
  loadAttestedServingMembers,
  Web06ArtifactServingGuard,
} from "./startup-benchmark/web06-peer-artifacts";

import {
  comparatorEventCount,
  comparatorEventsSince,
  ensureYuneComparatorMeasurementPageSize,
  installComparatorEndpointObserver,
  readComparatorEmptyPosture,
  waitForStableCandidateEndpoint,
  waitForStableCommitEndpoint,
} from "./startup-benchmark/comparator-browser-endpoint";
import {
  comparatorSelectorManifest,
  validateCandidateObservation,
  validateCommitObservation,
  type ComparatorApp,
  type ComparatorStableObservation,
} from "./startup-benchmark/comparator-endpoint";
import {
  attemptKey,
  buildPeerCompactReceipt,
  canonicalJson,
  classifyPeerAttempt,
  derivePeerSetupInvalidReasons,
  isStableObservationContractReason,
  medianValue,
  nearestRankValue,
  peerCandidateCommitBehaviorReasons,
  peerEventBehaviorRedReasons,
  peerFinalCandidateMembershipBehaviorReasons,
  peerObservationSchemaBehaviorReasons,
  peerRow,
  retainFinalizedPeerResultAcrossCleanup,
  sha256,
  retainPeerMeasurementFailure,
  validatePeerAttempt,
  validatePeerFrameReceipt,
  Web06PeerMeasurementError,
  web06PeerCacheRegime,
  web06PeerFinalCandidateMembership,
  web06PeerInteractiveMaxMs,
  web06PeerLaneVersion,
  web06PeerLocale,
  web06PeerMaxAttempts,
  web06PeerRawVersion,
  web06PeerRows,
  web06PeerValidRounds,
  web06PeerViewport,
  type Web06PeerAttestation,
  type Web06PeerAttempt,
  type Web06PeerMeasurementFailureCode,
  type Web06PeerRawPacket,
  type Web06PeerRowId,
} from "./startup-benchmark/web06-peer-lane";

const mode = process.env.YUNE_WEB06_PEER_MODE;
const attestationPath = process.env.YUNE_WEB06_PEER_ATTESTATION;
const outputRoot = process.env.YUNE_WEB06_PEER_OUTPUT_ROOT;
const readyTimeoutMs = 120_000;

test.describe("WEB-06 Phase-4 source-pinned peer lane", () => {
  test.describe.configure({ mode: "serial", timeout: 60 * 60 * 1000 });
  test.skip(!mode || !attestationPath || !outputRoot, "Run through run-web06-peer-lane.ts");

  test("setup-only dual-artifact endpoint smoke", async () => {
    test.skip(mode !== "setup-only", "Only the explicit setup-only lane runs this smoke");
    const { attestation, attestationText } = await loadAttestation();
    expect(attestation.benchmarkAttempt).toBe(false);
    const yuneServer = await startStaticServer(
      attestation.yuneRoot,
      await loadAttestedServingMembers(attestation, "yune-web"),
    );
    const peerServer = await startStaticServer(
      attestation.peerRoot,
      await loadAttestedServingMembers(attestation, "my-rime"),
    );
    try {
      const probes = [];
      for (const app of ["yune-web", "my-rime"] as const) {
        probes.push(await setupOnlyProbe(
          app,
          app === "yune-web" ? yuneServer.url : peerServer.url,
          attestation,
        ));
      }
      const receipt = {
        version: "web06-phase4-peer-setup-only-v1",
        benchmarkAttempt: false,
        attestationSha256: sha256(attestationText),
        yuneArchiveSha256: attestation.yune.archiveSha256,
        peerArchiveSha256: attestation.peer.archiveSha256,
        environmentId: attestation.finalRunEnvironment.environmentId,
        endpointContract: "web06-comparator-endpoint-v1",
        probes,
        status: "PASS",
      };
      await writeCreateNew(path.join(outputRoot!, "setup-only-receipt.json"), canonicalJson(receipt));
    } finally {
      await Promise.all([yuneServer.close(), peerServer.close()]);
    }
  });

  test("binding dual-artifact five-of-seven matrix", async () => {
    test.skip(mode !== "binding", "Binding measurement requires explicit --mode binding");
    const { attestation, attestationText } = await loadAttestation();
    expect(attestation.benchmarkAttempt).toBe(true);
    expect(attestation.toolchain.runnerSourceTreeState).toBe("clean");
    const yuneServer = await startStaticServer(
      attestation.yuneRoot,
      await loadAttestedServingMembers(attestation, "yune-web"),
    );
    const peerServer = await startStaticServer(
      attestation.peerRoot,
      await loadAttestedServingMembers(attestation, "my-rime"),
    );
    const attempts: Web06PeerAttempt[] = [];
    const attemptDigests = new Map<string, string>();
    let runOrderOrdinal = 0;
    try {
      const pageSizePreflight = [];
      for (const app of ["yune-web", "my-rime"] as const) {
        pageSizePreflight.push(await setupOnlyProbe(
          app,
          app === "yune-web" ? yuneServer.url : peerServer.url,
          attestation,
        ));
      }
      await writeCreateNew(path.join(outputRoot!, "raw", "page-size-preflight.json"), canonicalJson({
        version: "web06-phase4-peer-page-size-preflight-v1",
        benchmarkAttempt: false,
        attestationSha256: sha256(attestationText),
        yuneArchiveSha256: attestation.yune.archiveSha256,
        peerArchiveSha256: attestation.peer.archiveSha256,
        environmentId: attestation.finalRunEnvironment.environmentId,
        sourceKey: "pageSize",
        pinnedPeerSelector: "my-rime-c73ea17-public-dom-v1",
        probes: pageSizePreflight,
        status: "PASS",
      }));
      for (const row of web06PeerRows) {
        let pairOrdinal = 0;
        let pairedValidRounds = 0;
        while (pairedValidRounds < web06PeerValidRounds && pairOrdinal < web06PeerMaxAttempts) {
          pairOrdinal += 1;
          const order: ComparatorApp[] = pairOrdinal % 2 === 1
            ? ["yune-web", "my-rime"]
            : ["my-rime", "yune-web"];
          const retainedPair: Web06PeerAttempt[] = [];
          for (const app of order) {
            runOrderOrdinal += 1;
            const attempt = await runBindingAttempt({
              app,
              rowId: row.id,
              attempt: pairOrdinal,
              runOrderOrdinal,
              baseUrl: app === "yune-web" ? yuneServer.url : peerServer.url,
              attestation,
            });
            const failures = validatePeerAttempt(attempt);
            const text = canonicalJson(attempt);
            const digest = sha256(text);
            await writeCreateNew(
              path.join(outputRoot!, "raw", "attempts", `${attemptKey(attempt)}.json`),
              text,
            );
            attemptDigests.set(attemptKey(attempt), digest);
            attempts.push(attempt);
            retainedPair.push(attempt);
            expect(failures, `${attemptKey(attempt)} raw contract`).toEqual([]);
          }
          if (retainedPair.every(attempt => attempt.validForLatencyFrame)) pairedValidRounds += 1;
        }
      }
      const packet: Web06PeerRawPacket = {
        version: web06PeerRawVersion,
        laneVersion: web06PeerLaneVersion,
        benchmarkAttempt: true,
        phase: "phase4-peer",
        attestationSha256: sha256(attestationText),
        configSha256: attestation.configSha256,
        identityManifestSha256: attestation.identityManifestSha256,
        packageAlignment: "DATA_CONFOUNDED",
        attempts,
      };
      const rawText = canonicalJson(packet);
      const rawSha256 = sha256(rawText);
      await writeCreateNew(path.join(outputRoot!, "raw", "peer-raw-packet.json"), rawText);
      const compact = buildPeerCompactReceipt(packet, attestation, rawSha256, attemptDigests);
      await writeCreateNew(path.join(outputRoot!, "compact", "peer-summary.json"), canonicalJson(compact));
    } finally {
      await Promise.all([yuneServer.close(), peerServer.close()]);
    }
  });
});

async function setupOnlyProbe(
  app: ComparatorApp,
  baseUrl: string,
  attestation: Web06PeerAttestation,
): Promise<Record<string, unknown>> {
  const profile = await mkdtemp(path.join(os.tmpdir(), `web06-peer-setup-${app}-`));
  const context = await freshContext(profile, app, attestation);
  try {
    const page = await onlyPage(context);
    const network = await enforceLocalNetwork(page, baseUrl);
    const pageSizeSetup = await readyPage(page, app, baseUrl);
    await installComparatorEndpointObserver(page, app);
    const input = page.locator(app === "yune-web" ? "textarea.yd-input-area" : "#container textarea");
    await input.click();
    const observations: ComparatorStableObservation[] = [];
    for (const prefix of ["n", "ni"]) {
      const before = await comparatorEventCount(page);
      await page.keyboard.press(prefix.at(-1) ?? "");
      observations.push(await waitForStableCandidateEndpoint(
        page, app, prefix, before, 0, { requireYuneDiagnostic: false },
      ));
    }
    const shortObservation = observations[1];
    if (!shortObservation) throw new Error(`${app} setup-only short endpoint is missing`);
    const shortEventCount = await comparatorEventCount(page);
    if (shortEventCount !== 2) {
      throw new Error(`${app} setup-only short endpoint observed ${shortEventCount} input events`);
    }
    const selected = observations.at(-1)?.secondRaf.candidates[
      observations.at(-1)?.secondRaf.highlightedIndex ?? -1
    ]?.text;
    if (!selected) throw new Error(`${app} setup-only endpoint has no selected candidate`);
    const beforeCommit = await comparatorEventCount(page);
    await page.keyboard.press("Space");
    const commit = await waitForStableCommitEndpoint(page, app, selected, beforeCommit);
    expect(validateCandidateObservation(shortObservation, "ni", {
      requireYuneDiagnostic: false,
      expectedPageShape: web06PeerFinalCandidateMembership["phase4-peer/luna-short-ni"][app],
    })).toEqual([]);
    expect(validateCommitObservation(commit, selected)).toEqual([]);
    expect(peerObservationSchemaBehaviorReasons(shortObservation)).toEqual([]);
    expect(peerFinalCandidateMembershipBehaviorReasons(
      "phase4-peer/luna-short-ni",
      app,
      shortObservation,
    )).toEqual([]);

    const sustainedRow = peerRow("phase4-peer/luna-sustained-59");
    await input.click();
    await installComparatorEndpointObserver(page, app);
    for (const key of sustainedRow.input) await page.keyboard.press(key);
    const sustainedEventCount = await comparatorEventCount(page);
    if (sustainedEventCount !== sustainedRow.expectedPrefixCount) {
      throw new Error(`${app} setup-only sustained endpoint observed ${sustainedEventCount} input events`);
    }
    const sustainedObservation = await waitForStableCandidateEndpoint(
      page,
      app,
      sustainedRow.input,
      sustainedRow.expectedPrefixCount - 1,
      0,
      {
        requireYuneDiagnostic: false,
        expectedPageShape: web06PeerFinalCandidateMembership[sustainedRow.id][app],
      },
    );
    expect(validateCandidateObservation(sustainedObservation, sustainedRow.input, {
      requireYuneDiagnostic: false,
      expectedPageShape: web06PeerFinalCandidateMembership[sustainedRow.id][app],
    })).toEqual([]);
    expect(peerObservationSchemaBehaviorReasons(sustainedObservation)).toEqual([]);
    expect(peerFinalCandidateMembershipBehaviorReasons(
      sustainedRow.id,
      app,
      sustainedObservation,
    )).toEqual([]);
    expect(network.unexpected).toEqual([]);
    return {
      app,
      benchmarkAttempt: false,
      sourceArchiveSha256: app === "yune-web"
        ? attestation.yune.archiveSha256
        : attestation.peer.archiveSha256,
      pageSizeSetup,
      selectorManifestId: shortObservation.secondRaf.selectorManifestId,
      prefixesObserved: observations.length,
      finalCandidateRows: shortObservation.secondRaf.candidates.length,
      finalCandidateSnapshots: [
        setupCandidateSnapshot(
          "phase4-peer/luna-short-ni",
          shortObservation,
          shortEventCount,
        ),
        setupCandidateSnapshot(
          sustainedRow.id,
          sustainedObservation,
          sustainedEventCount,
        ),
      ],
      commitClearedComposition: commit.secondRaf.composition === "",
      committedValuePresent: commit.secondRaf.caret.value.length > 0,
      pageVisibleAndFocused: await visibleAndFocused(page),
      networkRouteInstalled: network.routeInstalled,
      blockedSetupNetworkRequests: network.blockedBeforeTimed,
      unexpectedNetworkRequestCount: network.unexpected.length,
    };
  } finally {
    await context.close();
    await rm(profile, { recursive: true, force: true });
  }
}

function setupCandidateSnapshot(
  rowId: Web06PeerRowId,
  observation: ComparatorStableObservation,
  observedInputEventCount: number,
): Record<string, unknown> {
  const row = peerRow(rowId);
  const tuple = observation.secondRaf;
  return {
    rowId,
    expectedPrefixCount: row.expectedPrefixCount,
    observedInputEventCount,
    selectorManifestId: tuple.selectorManifestId,
    composition: tuple.composition,
    schemaId: tuple.status.schemaId,
    candidates: tuple.candidates,
    candidateCount: tuple.candidates.length,
    candidateSurfaceCount: tuple.candidateSurfaceCount,
    page: tuple.page,
    highlightedIndex: tuple.highlightedIndex,
  };
}

async function runBindingAttempt(options: {
  app: ComparatorApp;
  rowId: Web06PeerRowId;
  attempt: number;
  runOrderOrdinal: number;
  baseUrl: string;
  attestation: Web06PeerAttestation;
}): Promise<Web06PeerAttempt> {
  const row = peerRow(options.rowId);
  const profile = await mkdtemp(path.join(os.tmpdir(), `web06-peer-${options.rowId}-${options.app}-`));
  const prefixSamples: Web06PeerAttempt["prefixSamples"] = [];
  const driverDispatches: Web06PeerAttempt["driverDispatches"] = [];
  let commit: ComparatorStableObservation | null = null;
  let commitDriverDispatch: Web06PeerAttempt["commitDriverDispatch"] = null;
  let committedValue = "";
  const setupInvalidReasons: string[] = [];
  const behaviorRedReasons: string[] = [];
  const latencyRedReasons: string[] = [];
  const consoleErrors: string[] = [];
  const blockedSetupNetworkRequests: string[] = [];
  const timedAssetRequests: string[] = [];
  const unexpectedNetworkRequests: string[] = [];
  const measurementFailures: Web06PeerAttempt["measurementFailures"] = [];
  const retainMeasurementFailure = (
    stage: Web06PeerAttempt["measurementFailures"][number]["stage"],
    prefixOrdinal: number | null,
    code: Web06PeerMeasurementFailureCode,
  ) => retainPeerMeasurementFailure(
    { measurementFailures, setupInvalidReasons, behaviorRedReasons },
    { stage, prefixOrdinal, code },
  );
  let foregroundAndFocused = false;
  let environmentLossObserved = false;
  let context: BrowserContext | undefined;
  let page: Page | undefined;
  let measurementStarted = false;
  let networkRouteInstalled = false;
  let pretypingIdleMedianMs: number | null = null;
  let initialEventOrdinal: 0 | null = null;
  let observedEvents: Web06PeerAttempt["observedEvents"] = null;
  let frame: Web06PeerAttempt["frame"] = null;
  let network: LocalNetworkGuard | undefined;
  let finalizedAttempt: Web06PeerAttempt;

  const finalizeAttempt = async (): Promise<Web06PeerAttempt> => {
    if (measurementStarted && page && !page.isClosed()) {
      try {
        observedEvents = await comparatorEventsSince(page, initialEventOrdinal ?? 0);
        for (const dispatch of driverDispatches) {
          const event = observedEvents.find(candidate => candidate.ordinal === dispatch.prefixOrdinal);
          if (event) {
            dispatch.normalizedEventAt = event.timeStamp;
            dispatch.eventDeliveredAt = event.deliveredAt ?? null;
          }
        }
        if (commitDriverDispatch) {
          const event = observedEvents.find(candidate =>
            candidate.ordinal === commitDriverDispatch?.prefixOrdinal
          );
          if (event) {
            commitDriverDispatch.normalizedEventAt = event.timeStamp;
            commitDriverDispatch.eventDeliveredAt = event.deliveredAt ?? null;
          }
        }
      } catch {
        observedEvents = null;
        retainMeasurementFailure(
          "post-dispatch",
          null,
          "EVENT_STREAM_METRIC_CONTRACT_FAILURE",
        );
      }
    }
    if (observedEvents) {
      behaviorRedReasons.push(...peerEventBehaviorRedReasons(observedEvents, row.input));
    }
    if (prefixSamples.length !== row.expectedPrefixCount
        && measurementFailures.some(failure =>
          failure.stage === "candidate" && failure.disposition === "BEHAVIOR_RED"
        )) {
      behaviorRedReasons.push("exact-prefix-count");
    }
    for (const sample of prefixSamples) {
      const expectedSelector = comparatorSelectorManifest[options.app].id;
      if ([sample.observation.initial, sample.observation.firstRaf, sample.observation.secondRaf]
        .some(tuple => tuple.selectorManifestId !== expectedSelector)) {
        retainMeasurementFailure(
          "candidate",
          sample.prefixOrdinal,
          "ENDPOINT_METRIC_CONTRACT_FAILURE",
        );
      }
      for (const reason of validateCandidateObservation(
        sample.observation,
        sample.expectedPrefix,
        {
          requireYuneDiagnostic: false,
          expectedPageShape: sample.prefixOrdinal === row.expectedPrefixCount
            ? web06PeerFinalCandidateMembership[options.rowId][options.app]
            : undefined,
        },
      )) {
        if (isStableObservationContractReason(reason)) {
          retainMeasurementFailure(
            "candidate",
            sample.prefixOrdinal,
            "ENDPOINT_METRIC_CONTRACT_FAILURE",
          );
        } else {
          behaviorRedReasons.push(`prefix-${sample.prefixOrdinal}:${reason}`);
        }
      }
      for (const reason of peerObservationSchemaBehaviorReasons(sample.observation)) {
        behaviorRedReasons.push(`prefix-${sample.prefixOrdinal}:${reason}`);
      }
      if (sample.prefixOrdinal === row.expectedPrefixCount) {
        for (const reason of peerFinalCandidateMembershipBehaviorReasons(
          options.rowId,
          options.app,
          sample.observation,
        )) {
          behaviorRedReasons.push(`prefix-${sample.prefixOrdinal}:${reason}`);
        }
      }
    }
    if (commit) {
      const expectedSelector = comparatorSelectorManifest[options.app].id;
      if ([commit.initial, commit.firstRaf, commit.secondRaf]
        .some(tuple => tuple.selectorManifestId !== expectedSelector)) {
        retainMeasurementFailure("commit", null, "ENDPOINT_METRIC_CONTRACT_FAILURE");
      }
      for (const reason of validateCommitObservation(commit, committedValue)) {
        if (isStableObservationContractReason(reason)) {
          retainMeasurementFailure("commit", null, "ENDPOINT_METRIC_CONTRACT_FAILURE");
        } else {
          behaviorRedReasons.push(`commit:${reason}`);
        }
      }
      for (const reason of peerObservationSchemaBehaviorReasons(commit)) {
        behaviorRedReasons.push(`commit:${reason}`);
      }
      const finalPrefix = prefixSamples.find(sample =>
        sample.prefixOrdinal === row.expectedPrefixCount
      );
      if (finalPrefix) {
        for (const reason of peerCandidateCommitBehaviorReasons(
          finalPrefix,
          commit,
          committedValue,
        )) {
          behaviorRedReasons.push(`commit-sequence:${reason}`);
        }
      }
    }
    if (options.app === "yune-web" && options.rowId === "phase4-peer/luna-short-ni") {
      for (const sample of prefixSamples) {
        if (sample.eventToStableCandidateMs > web06PeerInteractiveMaxMs) {
          latencyRedReasons.push(`short-prefix-${sample.prefixOrdinal}-over-67ms`);
        }
      }
      const commitMs = commit ? commit.secondRaf.observedAt - commit.event.timeStamp : undefined;
      if (commitMs !== undefined && commitMs > web06PeerInteractiveMaxMs) {
        latencyRedReasons.push("short-commit-over-67ms");
      }
      if (frame?.interactionFrameIntervalsMs.some(value => value >= 50)) {
        latencyRedReasons.push("short-frame-interval-at-least-50ms");
      }
      if (frame && nearestRankValue(frame.interactionFrameIntervalsMs, 0.99) > 35.4) {
        latencyRedReasons.push("short-frame-p99-over-35.4ms");
      }
      if (frame?.interactionLongTasks.some(entry => entry.duration >= 50)) {
        latencyRedReasons.push("short-long-task-at-least-50ms");
      }
    }
    if (network) {
      retainNetworkEvidence(
        network,
        blockedSetupNetworkRequests,
        timedAssetRequests,
        unexpectedNetworkRequests,
        setupInvalidReasons,
      );
    }
    if (measurementStarted && consoleErrors.length > 0) {
      behaviorRedReasons.push("console-or-page-error");
    }
    const live = page ? await visibleAndFocused(page).catch(() => false) : false;
    if (measurementStarted && (page?.isClosed() || !live)) {
      environmentLossObserved = true;
      retainMeasurementFailure("post-dispatch", null, "PAGE_ENVIRONMENT_LOSS");
    }
    foregroundAndFocused = live && !environmentLossObserved;
    const actualDispatchGapsMs = driverDispatches.slice(1).map((dispatch, index) =>
      dispatch.actualDriverDispatchAt
        - (driverDispatches[index]?.actualDriverDispatchAt ?? dispatch.actualDriverDispatchAt)
    );
    const exactSetupInvalidReasons = derivePeerSetupInvalidReasons({
      measurementStarted,
      networkRouteInstalled,
      pretypingIdleMedianMs,
      frame,
      actualDispatchGapsMs,
      blockedSetupNetworkRequests,
      timedAssetRequests,
      unexpectedNetworkRequests,
      foregroundAndFocused,
      measurementFailures,
    });
    const partial = {
      version: web06PeerRawVersion,
      benchmarkAttempt: true as const,
      rowId: options.rowId,
      app: options.app,
      attempt: options.attempt,
      runOrderOrdinal: options.runOrderOrdinal,
      freshProfile: true as const,
      firstKeyIncluded: true as const,
      warmupKeyCount: 0 as const,
      viewport: web06PeerViewport,
      locale: web06PeerLocale,
      cacheRegime: web06PeerCacheRegime,
      cadenceMs: 60 as const,
      pageSize: 6 as const,
      measurementStarted,
      networkRouteInstalled,
      pretypingIdleMedianMs,
      initialEventOrdinal,
      observedEvents,
      driverDispatches,
      commitDriverDispatch,
      frame,
      prefixSamples,
      commit,
      committedValue,
      actualDispatchGapsMs,
      consoleErrors,
      blockedSetupNetworkRequests,
      timedAssetRequests,
      unexpectedNetworkRequests,
      foregroundAndFocused,
      measurementFailures,
      setupInvalidReasons: exactSetupInvalidReasons,
      behaviorRedReasons: [...new Set(behaviorRedReasons)],
      latencyRedReasons: [...new Set(latencyRedReasons)],
    };
    return { ...partial, ...classifyPeerAttempt(partial) };
  };

  try {
    context = await freshContext(profile, options.app, options.attestation);
    page = await onlyPage(context);
    network = await enforceLocalNetwork(page, options.baseUrl);
    networkRouteInstalled = network.routeInstalled;
    capturePageFailures(page, consoleErrors);
    await readyPage(page, options.app, options.baseUrl);
    await page.waitForLoadState("networkidle");
    const idleCalibration = await installAndCalibrateFrameObserver(page);
    pretypingIdleMedianMs = idleCalibration.idleMedianMs;
    if (idleCalibration.idleMedianMs < 15 || idleCalibration.idleMedianMs > 18) {
      setupInvalidReasons.push("idle-raf-median-outside-15-to-18ms");
    }
    const input = page.locator(options.app === "yune-web" ? "textarea.yd-input-area" : "#container textarea");
    await input.click();
    await installComparatorEndpointObserver(page, options.app);
    const firstEventOrdinal = await comparatorEventCount(page);
    if (firstEventOrdinal !== 0) throw new Error(`observer armed after ${firstEventOrdinal} undeclared setup events`);
    initialEventOrdinal = 0;
    await beginPeerInteractionWindow(page);
    network.startTimed();
    const pendingEndpoints: Array<{
      index: number;
      expectedPrefix: string;
      result: Promise<
        { observation: ComparatorStableObservation; error?: never }
        | { observation?: never; error: unknown }
      >;
    }> = [];
    let cadencePhaseBaseAt = performance.now();
    for (let index = 0; index < row.input.length; index += 1) {
      const phaseDeadlineAt = index === 0
        ? cadencePhaseBaseAt
        : cadencePhaseBaseAt + 60;
      const requestedDispatchAt = Math.max(phaseDeadlineAt, performance.now());
      await waitUntil(requestedDispatchAt);
      measurementStarted = true;
      const actualDriverDispatchAt = performance.now();
      const cadenceRebased = requestedDispatchAt > phaseDeadlineAt
        || actualDriverDispatchAt - phaseDeadlineAt > 1;
      cadencePhaseBaseAt = Math.max(phaseDeadlineAt, actualDriverDispatchAt);
      await page.keyboard.press(row.input[index] ?? "");
      driverDispatches.push({
        prefixOrdinal: index + 1,
        key: row.input[index] ?? "",
        phaseDeadlineAt,
        requestedDispatchAt,
        actualDriverDispatchAt,
        cadenceRebased,
        normalizedEventAt: null,
        eventDeliveredAt: null,
      });
      const expectedPrefix = row.input.slice(0, index + 1);
      pendingEndpoints.push({
        index,
        expectedPrefix,
        result: waitForStableCandidateEndpoint(
          page,
          options.app,
          expectedPrefix,
          index,
          0,
          {
            requireYuneDiagnostic: false,
            failOnLaterEvent: true,
            expectedPageShape: index + 1 === row.expectedPrefixCount
              ? web06PeerFinalCandidateMembership[options.rowId][options.app]
              : undefined,
          },
        ).then(
          observation => ({ observation }),
          error => ({ error }),
        ),
      });
    }
    const endpointResults = await Promise.all(pendingEndpoints.map(async pending => ({
      ...pending,
      ...await pending.result,
    })));
    for (const endpoint of endpointResults) {
      const event = endpoint.observation?.event;
      const dispatch = driverDispatches[endpoint.index];
      if (dispatch) {
        dispatch.normalizedEventAt = event?.timeStamp ?? null;
        dispatch.eventDeliveredAt = event?.deliveredAt ?? null;
      }
      if (endpoint.observation) {
        const observation = endpoint.observation;
        prefixSamples.push({
          prefixOrdinal: endpoint.index + 1,
          expectedPrefix: endpoint.expectedPrefix,
          requestedDispatchAt: dispatch?.requestedDispatchAt ?? 0,
          actualDriverDispatchAt: dispatch?.actualDriverDispatchAt ?? 0,
          observation,
          eventToStableCandidateMs: observation.secondRaf.observedAt - observation.event.timeStamp,
        });
      } else {
        const live = await visibleAndFocused(page).catch(() => false);
        if (page.isClosed() || !live) {
          environmentLossObserved = true;
          retainMeasurementFailure("candidate", endpoint.index + 1, "PAGE_ENVIRONMENT_LOSS");
        } else {
          retainMeasurementFailure(
            "candidate",
            endpoint.index + 1,
            measurementErrorCode(endpoint.error, "ENDPOINT_METRIC_CONTRACT_FAILURE"),
          );
        }
      }
    }
    prefixSamples.sort((left, right) => left.prefixOrdinal - right.prefixOrdinal);
    const finalPrefixSample = prefixSamples.find(sample => sample.prefixOrdinal === row.expectedPrefixCount);
    if (finalPrefixSample) {
      const selected = finalPrefixSample.observation.secondRaf.candidates[
        finalPrefixSample.observation.secondRaf.highlightedIndex
      ]?.text;
      if (!selected) {
        behaviorRedReasons.push("selected-candidate-missing");
      } else {
        const beforeCommit = await comparatorEventCount(page);
        if (beforeCommit !== row.input.length) {
          behaviorRedReasons.push(`event-ordinal-before-commit:${beforeCommit}`);
        }
        const actualDriverDispatchAt = performance.now();
        await page.keyboard.press("Space");
        try {
          commit = await waitForStableCommitEndpoint(page, options.app, selected, beforeCommit);
          committedValue = commit.secondRaf.caret.value;
          commitDriverDispatch = {
            prefixOrdinal: row.input.length + 1,
            key: " ",
            phaseDeadlineAt: actualDriverDispatchAt,
            requestedDispatchAt: actualDriverDispatchAt,
            actualDriverDispatchAt,
            cadenceRebased: false,
            normalizedEventAt: commit.event.timeStamp,
            eventDeliveredAt: commit.event.deliveredAt ?? null,
          };
        } catch (error) {
          const delivered = (await comparatorEventsSince(page, beforeCommit).catch(() => [])).at(0);
          commitDriverDispatch = {
            prefixOrdinal: row.input.length + 1,
            key: " ",
            phaseDeadlineAt: actualDriverDispatchAt,
            requestedDispatchAt: actualDriverDispatchAt,
            actualDriverDispatchAt,
            cadenceRebased: false,
            normalizedEventAt: delivered?.timeStamp ?? null,
            eventDeliveredAt: delivered?.deliveredAt ?? null,
          };
          const live = await visibleAndFocused(page).catch(() => false);
          if (page.isClosed() || !live) {
            environmentLossObserved = true;
            retainMeasurementFailure("commit", null, "PAGE_ENVIRONMENT_LOSS");
          }
          else retainMeasurementFailure(
            "commit",
            null,
            measurementErrorCode(error, "ENDPOINT_METRIC_CONTRACT_FAILURE"),
          );
        }
      }
    }
    frame = await finishPeerInteractionWindow(page);
    finalizedAttempt = await finalizeAttempt();
  } catch (error) {
    const live = page ? await visibleAndFocused(page).catch(() => false) : false;
    foregroundAndFocused = live && !environmentLossObserved;
    if (!measurementStarted) {
      // The exact setup taxonomy derives this from measurementStarted=false.
    } else if (page?.isClosed() || !live) {
      environmentLossObserved = true;
      foregroundAndFocused = false;
      retainMeasurementFailure("post-dispatch", null, "PAGE_ENVIRONMENT_LOSS");
    } else {
      const frameFailure = error instanceof Web06PeerMeasurementError
        && error.code.startsWith("FRAME_");
      retainMeasurementFailure(
        frameFailure ? "frame" : "post-dispatch",
        null,
        frameFailure ? error.code : "POST_DISPATCH_METRIC_CONTRACT_FAILURE",
      );
    }
    if (measurementStarted && !frame && page && !page.isClosed()
        && !measurementFailures.some(failure => failure.stage === "frame")) {
      try {
        frame = await finishPeerInteractionWindow(page);
      } catch (frameError) {
        retainMeasurementFailure(
          "frame",
          null,
          measurementErrorCode(frameError, "FRAME_EVALUATION_FAILURE"),
        );
      }
    }
    finalizedAttempt = await finalizeAttempt();
  }
  const cleanup = await retainFinalizedPeerResultAcrossCleanup(finalizedAttempt, [
    {
      id: "browser-context-close",
      run: async () => { await context?.close(); },
    },
    {
      id: "profile-remove",
      run: async () => { await rm(profile, { recursive: true, force: true }); },
    },
  ]);
  if (cleanup.warnings.length > 0) {
    try {
      process.stderr.write(`${JSON.stringify({
        benchmarkAttempt: true,
        rowId: options.rowId,
        app: options.app,
        attempt: options.attempt,
        warnings: cleanup.warnings,
      })}\n`);
    } catch {
      // A logging failure must not erase a finalized measurement packet either.
    }
  }
  return cleanup.finalized;
}

async function installAndCalibrateFrameObserver(page: Page): Promise<{
  idleMedianMs: number;
}> {
  const idle = await page.evaluate(async () => {
    type LongTask = { startTime: number; duration: number };
    type FrameState = {
      running: boolean;
      rafTimestamps: number[];
      longTasks: LongTask[];
      observer: PerformanceObserver;
      startIndex: number | null;
      interactionStartAt: number | null;
      visibilityOrFocusLost: boolean;
      visibilityListener: () => void;
      blurListener: () => void;
    };
    const scope = window as typeof window & { __web06PeerFrame?: FrameState };
    scope.__web06PeerFrame?.observer.disconnect();
    if (!PerformanceObserver.supportedEntryTypes.includes("longtask")) {
      throw new Error("Chromium Long Task observer is unavailable");
    }
    const longTasks: LongTask[] = [];
    const observer = new PerformanceObserver(list => {
      for (const entry of list.getEntries()) {
        longTasks.push({ startTime: entry.startTime, duration: entry.duration });
      }
    });
    observer.observe({ type: "longtask", buffered: true });
    const state: FrameState = {
      running: true,
      rafTimestamps: [],
      longTasks,
      observer,
      startIndex: null,
      interactionStartAt: null,
      visibilityOrFocusLost: document.visibilityState !== "visible" || !document.hasFocus(),
      visibilityListener: () => undefined,
      blurListener: () => undefined,
    };
    state.visibilityListener = () => {
      if (document.visibilityState !== "visible") state.visibilityOrFocusLost = true;
    };
    state.blurListener = () => { state.visibilityOrFocusLost = true; };
    document.addEventListener("visibilitychange", state.visibilityListener);
    window.addEventListener("blur", state.blurListener);
    const tick = (timestamp: number) => {
      state.rafTimestamps.push(timestamp);
      if (state.running) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
    const deadline = performance.now() + 10_000;
    while (state.rafTimestamps.length < 121 && performance.now() < deadline) {
      await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    }
    if (state.rafTimestamps.length < 121) throw new Error("Could not collect 120 idle rAF intervals");
    scope.__web06PeerFrame = state;
    const timestamps = state.rafTimestamps.slice(-121);
    return {
      timestamps,
      intervals: timestamps.slice(1).map((value, index) => value - (timestamps[index] ?? value)),
      visibilityOrFocusLost: state.visibilityOrFocusLost,
    };
  });
  if (idle.visibilityOrFocusLost) throw new Error("Page lost foreground/focus during idle rAF calibration");
  return { idleMedianMs: medianValue(idle.intervals) };
}

async function beginPeerInteractionWindow(page: Page): Promise<void> {
  await page.evaluate(async () => {
    type FrameState = {
      rafTimestamps: number[];
      startIndex: number | null;
      interactionStartAt: number | null;
    };
    const state = (window as typeof window & { __web06PeerFrame?: FrameState }).__web06PeerFrame;
    if (!state) throw new Error("Peer frame observer is not installed");
    await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    state.startIndex = state.rafTimestamps.length - 1;
    state.interactionStartAt = state.rafTimestamps[state.startIndex] ?? null;
    if (state.startIndex < 0 || state.interactionStartAt === null) {
      throw new Error("Peer interaction window has no preceding rAF boundary");
    }
  });
}

async function finishPeerInteractionWindow(page: Page): Promise<NonNullable<Web06PeerAttempt["frame"]>> {
  let outcome: FrameEvaluationOutcome;
  try {
    outcome = await page.evaluate(async () => {
    type LongTask = { startTime: number; duration: number };
    type FrameState = {
      running: boolean;
      rafTimestamps: number[];
      longTasks: LongTask[];
      observer: PerformanceObserver;
      startIndex: number | null;
      interactionStartAt: number | null;
      visibilityOrFocusLost: boolean;
      visibilityListener: () => void;
      blurListener: () => void;
    };
    const frameFailure = (
      code: Web06PeerMeasurementFailureCode,
    ): FrameEvaluationOutcome => ({ ok: false, code });
    const state = (window as typeof window & { __web06PeerFrame?: FrameState }).__web06PeerFrame;
    if (!state) return frameFailure("FRAME_OBSERVER_MISSING");
    if (state.startIndex === null || state.interactionStartAt === null) {
      return frameFailure("FRAME_WINDOW_NOT_STARTED");
    }
    await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    const interactionEndIndex = state.rafTimestamps.length - 1;
    const interactionWindowEndAt = state.rafTimestamps[interactionEndIndex];
    if (interactionWindowEndAt === undefined) {
      return frameFailure("FRAME_WINDOW_END_MISSING");
    }
    const interactionDuration = interactionWindowEndAt - state.interactionStartAt;
    if (!Number.isFinite(interactionDuration) || interactionDuration <= 0) {
      return frameFailure("FRAME_WINDOW_DURATION_INVALID");
    }
    const idleControlWindowStartAt = interactionWindowEndAt;
    const idleControlWindowEndAt = idleControlWindowStartAt + interactionDuration;
    const idleDeadline = performance.now() + interactionDuration + 10_000;
    while ((state.rafTimestamps.at(-1) ?? -1) < idleControlWindowEndAt
        && performance.now() < idleDeadline) {
      await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    }
    if ((state.rafTimestamps.at(-1) ?? -1) < idleControlWindowEndAt) {
      return frameFailure("FRAME_IDLE_CONTROL_TIMEOUT");
    }
    state.running = false;
    state.observer.takeRecords().forEach(entry => {
      state.longTasks.push({ startTime: entry.startTime, duration: entry.duration });
    });
    state.observer.disconnect();
    document.removeEventListener("visibilitychange", state.visibilityListener);
    window.removeEventListener("blur", state.blurListener);
    const idleRafTimestamps = state.rafTimestamps.slice(state.startIndex - 120, state.startIndex + 1);
    const interactionRafTimestamps = state.rafTimestamps.slice(state.startIndex, interactionEndIndex + 1);
    const idleControlRafTimestamps = state.rafTimestamps.slice(interactionEndIndex);
    if (idleRafTimestamps.length !== 121 || interactionRafTimestamps.length < 2
        || idleControlRafTimestamps.length < 2) {
      return frameFailure("FRAME_BOUNDARY_CONTRACT_FAILURE");
    }
    return {
      ok: true,
      raw: {
        idleRafTimestamps,
        idleControlRafTimestamps,
        idleControlWindowStartAt,
        idleControlWindowEndAt,
        interactionRafTimestamps,
        interactionWindowStartAt: state.interactionStartAt,
        interactionWindowEndAt,
        longTasks: state.longTasks,
        visibilityOrFocusLost: state.visibilityOrFocusLost
          || document.visibilityState !== "visible" || !document.hasFocus(),
      },
    };
    });
  } catch (error) {
    throw new Web06PeerMeasurementError(
      "FRAME_EVALUATION_FAILURE",
      `Frame window evaluation failed: ${errorMessage(error)}`,
    );
  }
  if (!outcome.ok) throw new Web06PeerMeasurementError(outcome.code, outcome.code);
  const raw = outcome.raw;
  const idleIntervalsMs = raw.idleRafTimestamps.slice(1).map((value, index) =>
    value - (raw.idleRafTimestamps[index] ?? value)
  );
  const interactionFrameIntervalsMs = raw.interactionRafTimestamps.slice(1).map((value, index) =>
    value - (raw.interactionRafTimestamps[index] ?? value)
  );
  const overlaps = (entry: { startTime: number; duration: number }, start: number, end: number) =>
    entry.startTime < end && entry.startTime + entry.duration > start;
  const interactionLongTasks = raw.longTasks.filter(entry => overlaps(
    entry,
    raw.interactionWindowStartAt,
    raw.interactionWindowEndAt,
  ));
  const idleLongTasks = raw.longTasks.filter(entry => overlaps(
    entry,
    raw.idleControlWindowStartAt,
    raw.idleControlWindowEndAt,
  ));
  const outsideWindowLongTasks = raw.longTasks.filter(entry =>
    !overlaps(entry, raw.interactionWindowStartAt, raw.interactionWindowEndAt)
    && !overlaps(entry, raw.idleControlWindowStartAt, raw.idleControlWindowEndAt)
  );
  const receipt: NonNullable<Web06PeerAttempt["frame"]> = {
    longTaskObserverSupported: true,
    idleRafTimestamps: raw.idleRafTimestamps,
    idleIntervalsMs,
    idleMedianMs: medianValue(idleIntervalsMs),
    idleControlWindowStartAt: raw.idleControlWindowStartAt,
    idleControlWindowEndAt: raw.idleControlWindowEndAt,
    idleControlRafTimestamps: raw.idleControlRafTimestamps,
    observedLongTasks: raw.longTasks,
    idleLongTasks,
    interactionWindowStartAt: raw.interactionWindowStartAt,
    interactionWindowEndAt: raw.interactionWindowEndAt,
    interactionRafTimestamps: raw.interactionRafTimestamps,
    interactionFrameIntervalsMs,
    interactionLongTasks,
    outsideWindowLongTasks,
    visibilityOrFocusLost: raw.visibilityOrFocusLost,
  };
  const receiptFailures = validatePeerFrameReceipt(receipt);
  if (receiptFailures.length > 0) {
    throw new Web06PeerMeasurementError(
      "FRAME_RECEIPT_CONTRACT_FAILURE",
      `Frame receipt failed its exact contract: ${receiptFailures.join(",")}`,
    );
  }
  return receipt;
}

interface RawFrameWindow {
  idleRafTimestamps: number[];
  idleControlRafTimestamps: number[];
  idleControlWindowStartAt: number;
  idleControlWindowEndAt: number;
  interactionRafTimestamps: number[];
  interactionWindowStartAt: number;
  interactionWindowEndAt: number;
  longTasks: Array<{ startTime: number; duration: number }>;
  visibilityOrFocusLost: boolean;
}

type FrameEvaluationOutcome =
  | { ok: true; raw: RawFrameWindow }
  | { ok: false; code: Web06PeerMeasurementFailureCode };

async function freshContext(
  profile: string,
  app: ComparatorApp,
  attestation: Web06PeerAttestation,
): Promise<BrowserContext> {
  if (attestation.finalRunEnvironment.browserMode !== "headed-foreground") {
    throw new Error("Peer lane requires the FINAL headed foreground Chromium mode");
  }
  const context = await chromium.launchPersistentContext(profile, {
    headless: false,
    viewport: web06PeerViewport,
    locale: web06PeerLocale,
    serviceWorkers: "block",
  });
  await context.addInitScript(({ targetApp }) => {
    localStorage.clear();
    if (targetApp === "yune-web") {
      localStorage.setItem("activeSchema", "luna_pinyin");
      localStorage.setItem("uiLanguage", "en");
      localStorage.setItem("enableAI", "false");
    }
    localStorage.setItem("pageSize", "6");
  }, { targetApp: app });
  return context;
}

async function onlyPage(context: BrowserContext): Promise<Page> {
  const pages = context.pages();
  const page = pages[0] ?? await context.newPage();
  for (const extra of context.pages().filter(candidate => candidate !== page)) await extra.close();
  await page.bringToFront();
  if (context.pages().length !== 1) throw new Error("Peer lane requires exactly one foreground page");
  return page;
}

async function readyPage(
  page: Page,
  app: ComparatorApp,
  baseUrl: string,
): Promise<Record<string, unknown>> {
  let returnAfterPosture: Record<string, unknown> | undefined;
  const url = new URL(baseUrl);
  if (app === "yune-web") {
    url.searchParams.set("schema", "luna_pinyin");
    url.searchParams.set("web06Peer", "1");
  } else {
    url.searchParams.set("schemaId", "luna_pinyin");
    url.searchParams.set("web06Peer", "1");
  }
  await page.goto(url.toString(), { waitUntil: "domcontentloaded" });
  if (app === "yune-web") {
    await page.waitForFunction(() => {
      const root = document.documentElement;
      const textarea = document.querySelector("textarea.yd-input-area") as HTMLTextAreaElement | null;
      return root.dataset.yuneInitialized === "true"
        && root.dataset.yuneLoading !== "true"
        && (root.dataset.yuneActiveSchema === "luna_pinyin")
        && textarea !== null
        && !textarea.disabled;
    }, undefined, { timeout: readyTimeoutMs });
    const pageSizeSetup = await ensureYuneComparatorMeasurementPageSize(page);
    if (pageSizeSetup.final.uiValue !== "6"
        || pageSizeSetup.final.localStorageValue !== "6"
        || pageSizeSetup.final.persistedConfigValue !== "6"
        || pageSizeSetup.final.deployStatus !== "success"
        || !pageSizeSetup.final.loadingComplete) {
      throw new Error("Yune peer page-size transition did not bind UI/storage/config/runtime to six");
    }
    const control = page.getByLabel(/No\. of Candidates Per Page|Candidates Per Page/).last();
    await expect(control).toHaveValue("6");
    returnAfterPosture = { sourceKey: "pageSize", yuneMeasurementPage: pageSizeSetup };
  } else {
    await page.waitForFunction(schemaSelector => {
      const textarea = document.querySelector("#container textarea") as HTMLTextAreaElement | null;
      const copy = [...document.querySelectorAll("button")]
        .find(button => button.getAttribute("title") === "Copy link for current IME") as HTMLButtonElement | undefined;
      const schema = [...document.querySelectorAll(schemaSelector)];
      return textarea !== null && !textarea.disabled && copy !== undefined && !copy.disabled
        && schema.length === 1 && schema[0]?.getAttribute("title") === "朙月拼音";
    }, comparatorSelectorManifest["my-rime"].schemaStatus, { timeout: readyTimeoutMs });
  }
  const input = page.locator(app === "yune-web" ? "textarea.yd-input-area" : "#container textarea");
  await expect(input).toHaveCount(1);
  await expect(input).toBeVisible();
  await expect(input).toHaveValue("");
  const initialPosture = await readComparatorEmptyPosture(page, app);
  if (initialPosture.localStoragePageSize !== "6" || initialPosture.visibleCandidateSurfaces !== 0
      || initialPosture.schemaEvidenceMatches !== 1
      || initialPosture.schemaEvidenceValue !== (app === "yune-web" ? "luna_pinyin" : "朙月拼音")
      || (app === "yune-web" && initialPosture.yuneComposing === "true")) {
    throw new Error(`${app} fresh-profile empty/page-size posture is not exact: ${JSON.stringify(initialPosture)}`);
  }
  return returnAfterPosture ?? {
    sourceKey: "pageSize",
    myRimeLocalStorageValue: initialPosture.localStoragePageSize,
    proof: "actual measured candidate endpoints require exactly six visible rows",
  };
}

interface LocalNetworkGuard {
  routeInstalled: true;
  unexpected: string[];
  blockedBeforeTimed: string[];
  timedRequests: string[];
  startTimed: () => void;
}

async function enforceLocalNetwork(page: Page, baseUrl: string): Promise<LocalNetworkGuard> {
  const allowedOrigin = new URL(baseUrl).origin;
  const unexpected: string[] = [];
  const blockedBeforeTimed: string[] = [];
  const timedRequests: string[] = [];
  let timed = false;
  await page.route("**/*", async route => {
    const url = new URL(route.request().url());
    if (url.origin !== allowedOrigin) {
      const identity = `${url.protocol}//${url.host}${url.pathname}`;
      (timed ? unexpected : blockedBeforeTimed).push(identity);
      await route.abort("blockedbyclient");
      return;
    }
    if (timed && route.request().resourceType() !== "document") timedRequests.push(url.pathname);
    await route.continue();
  });
  return {
    routeInstalled: true,
    unexpected,
    blockedBeforeTimed,
    timedRequests,
    startTimed: () => { timed = true; },
  };
}

function retainNetworkEvidence(
  network: LocalNetworkGuard,
  blockedSetup: string[],
  timedAssets: string[],
  unexpected: string[],
  setupReasons: string[],
): void {
  for (const value of network.blockedBeforeTimed) if (!blockedSetup.includes(value)) blockedSetup.push(value);
  for (const value of network.timedRequests) if (!timedAssets.includes(value)) timedAssets.push(value);
  for (const value of network.unexpected) if (!unexpected.includes(value)) unexpected.push(value);
  if (timedAssets.length > 0) setupReasons.push("asset-request-during-timed-window");
  if (unexpected.length > 0) setupReasons.push("unexpected-network-request");
}

function capturePageFailures(page: Page, errors: string[]): void {
  page.on("pageerror", error => errors.push(`pageerror:${error.message}`));
  page.on("console", message => {
    if (message.type() === "error") errors.push(`console:error:${message.text()}`);
  });
  page.on("response", response => {
    if (response.status() >= 400) errors.push(`response:${response.status()}:${new URL(response.url()).pathname}`);
  });
}

async function visibleAndFocused(page: Page): Promise<boolean> {
  return page.evaluate(() => document.visibilityState === "visible" && document.hasFocus());
}

async function loadAttestation(): Promise<{ attestation: Web06PeerAttestation; attestationText: string }> {
  const attestationText = await readFile(attestationPath!, "utf8");
  const value = JSON.parse(attestationText) as Web06PeerAttestation & {
    liveEnvironmentDrift?: string[];
    sourceMismatch?: string[];
  };
  if (value.version !== "web06-phase4-peer-attestation-v1") throw new Error("Wrong peer attestation version");
  if (value.phase !== "phase4-peer") throw new Error("Peer lane rejects BASELINE/FINAL phase aliases");
  if (mode === "binding" && ((value.liveEnvironmentDrift?.length ?? 0) > 0
      || (value.sourceMismatch?.length ?? 0) > 0)) {
    throw new Error("Binding attestation contains FINAL environment/source drift");
  }
  return { attestation: value, attestationText };
}

async function waitUntil(target: number): Promise<void> {
  while (performance.now() < target) {
    const remaining = target - performance.now();
    await new Promise(resolve => setTimeout(resolve, Math.max(0, Math.min(remaining, 10))));
  }
}

async function writeCreateNew(file: string, text: string): Promise<void> {
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, text, { flag: "wx", mode: 0o600 });
}

async function startStaticServer(
  root: string,
  members: Parameters<typeof Web06ArtifactServingGuard.create>[1],
): Promise<{ url: string; close: () => Promise<void> }> {
  const rootPath = path.resolve(root);
  const guard = await Web06ArtifactServingGuard.create(rootPath, members);
  const server = createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
      const raw = decodeURIComponent(requestUrl.pathname === "/" ? "/index.html" : requestUrl.pathname);
      const relative = raw.replace(/^\/+/, "");
      const file = path.resolve(rootPath, relative);
      const relation = path.relative(rootPath, file);
      if (relation.startsWith("..") || path.isAbsolute(relation)) {
        response.writeHead(403).end("Forbidden");
        return;
      }
      const bytes = await guard.read(relative);
      if (bytes === null) {
        response.writeHead(404).end("Not found");
        return;
      }
      response.setHeader("Content-Type", contentType(file));
      response.setHeader("Content-Length", bytes.byteLength);
      response.setHeader("Cache-Control", "no-store");
      response.setHeader("Cross-Origin-Opener-Policy", "same-origin");
      response.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
      response.end(bytes);
    } catch {
      response.writeHead(404).end("Not found");
    }
  });
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve());
  });
  const address = server.address();
  if (typeof address !== "object" || !address) throw new Error("Static server address missing");
  return {
    url: `http://127.0.0.1:${address.port}`,
    close: async () => {
      await closeServer(server);
      guard.assertIntegrity();
    },
  };
}

async function closeServer(server: Server): Promise<void> {
  await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
}

function contentType(file: string): string {
  switch (path.extname(file).toLowerCase()) {
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

function errorMessage(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}

function measurementErrorCode(
  value: unknown,
  fallback: Web06PeerMeasurementFailureCode,
): Web06PeerMeasurementFailureCode {
  return value instanceof Web06PeerMeasurementError ? value.code : fallback;
}
