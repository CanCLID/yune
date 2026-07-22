import { expect, test } from "@playwright/test";

import { execFileSync } from "node:child_process";
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  comparatorDomTupleDigest,
  comparatorPeerPageSize,
  comparatorPeerLogicalInputIds,
  type ComparatorDomTuple,
  type ComparatorIdentityManifest,
  type ComparatorStableObservation,
} from "./startup-benchmark/comparator-endpoint";
import {
  assertAttestedSourceFileHashes,
  resolveCreateNewExternalOutputRoot,
  resolvePinnedPeerPacketFile,
  safeExtractTar,
  verifyArtifactSourceTree,
  Web06ArtifactServingGuard,
} from "./startup-benchmark/web06-peer-artifacts";
import {
  installComparatorEndpointObserver,
  readComparatorEmptyPosture,
  waitForStableCommitEndpoint,
} from "./startup-benchmark/comparator-browser-endpoint";
import { independentStableObservationFailures } from "./startup-benchmark/web06-peer-independent";
import {
  assertDataConfoundedRatioOmission,
  assertPublicEvidencePrivacy,
  buildPeerCompactReceipt,
  canonicalJson,
  classifyPeerAttempt,
  measurementFailureDisposition,
  peerCadenceSetupReasons,
  retainFinalizedPeerResultAcrossCleanup,
  retainPeerMeasurementFailure,
  sha256,
  validatePeerAttempt,
  validatePeerFrameReceipt,
  web06PeerInputRegistrySha256,
  web06PeerFinalCandidateMembership,
  web06PeerMeasurementFailureCodes,
  web06PeerRows,
  type Web06PeerAttestation,
  type Web06PeerAttempt,
  type Web06PeerRawPacket,
} from "./startup-benchmark/web06-peer-lane";
import {
  validateAttemptIndependently,
  verifyPreflightCandidateSnapshots,
  verifyWeb06PeerEvidence,
} from "./verify-web06-peer-evidence";

test.describe("@contract WEB-06 Phase-4 peer lane", () => {
  test("freezes the exact Phase-4 IDs, order, inputs, and five-of-seven policy", () => {
    expect(web06PeerRows).toEqual([
      { id: "phase4-peer/luna-short-ni", input: "ni", expectedPrefixCount: 2, role: "binding-short" },
      {
        id: "phase4-peer/luna-sustained-59",
        input: "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
        expectedPrefixCount: 59,
        role: "informational-sustained",
      },
    ]);
    expect(web06PeerRows[1]?.input.length).toBe(59);
    expect(web06PeerFinalCandidateMembership["phase4-peer/luna-sustained-59"]["yune-web"])
      .toMatchObject({
        candidateCount: comparatorPeerPageSize,
        requiredTexts: ["這個引擎其實應該支持超長句子輸入才能用", "這個", "這歌", "這格", "這"],
      });
    expect(web06PeerInputRegistrySha256()).toMatch(/^[0-9a-f]{64}$/);
  });

  test("cannot erase a finalized measured attempt when post-measurement cleanup fails", async () => {
    const attempt = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    const retained = await retainFinalizedPeerResultAcrossCleanup(attempt, [
      {
        id: "browser-context-close",
        run: async () => { throw new Error("injected context cleanup failure"); },
      },
      {
        id: "profile-remove",
        run: async () => { throw new Error("injected profile cleanup failure"); },
      },
    ]);
    expect(retained.finalized).toBe(attempt);
    expect(validatePeerAttempt(retained.finalized)).toEqual([]);
    expect(retained.warnings).toEqual([
      "post-measurement-cleanup-failed:browser-context-close",
      "post-measurement-cleanup-failed:profile-remove",
    ]);
  });

  test("independently accepts the frozen six-row sustained Yune preflight and attempt", () => {
    const rowIds = [
      "phase4-peer/luna-short-ni",
      "phase4-peer/luna-sustained-59",
    ] as const;
    const attempts = rowIds.map(rowId => fixtureAttempt(rowId, "yune-web", 1));
    const snapshots = attempts.map(attempt => {
      const finalSample = attempt.prefixSamples.at(-1)!;
      const tuple = finalSample.observation.secondRaf;
      return {
        rowId: attempt.rowId,
        expectedPrefixCount: finalSample.prefixOrdinal,
        observedInputEventCount: attempt.observedEvents!.length - 1,
        selectorManifestId: tuple.selectorManifestId,
        composition: tuple.composition,
        schemaId: tuple.status.schemaId,
        candidates: tuple.candidates,
        candidateCount: tuple.candidates.length,
        candidateSurfaceCount: tuple.candidateSurfaceCount,
        page: tuple.page,
        highlightedIndex: tuple.highlightedIndex,
      };
    });

    expect(snapshots[1]?.candidateCount).toBe(comparatorPeerPageSize);
    expect(() => verifyPreflightCandidateSnapshots(
      snapshots,
      "yune-web",
      "yune-web-public-dom-v1",
    )).not.toThrow();
    expect(() => validateAttemptIndependently(attempts[1]!)).not.toThrow();
  });

  test("derives DATA_CONFOUNDED from raw attempts and omits every numeric ratio field", () => {
    const { packet, attestation, attemptDigests } = completeFixture();
    const compact = buildPeerCompactReceipt(packet, attestation, "f".repeat(64), attemptDigests);
    expect(compact.packageAlignment).toBe("DATA_CONFOUNDED");
    expect(compact.ratio).toEqual(expect.objectContaining({ status: "OMITTED" }));
    expect(Object.keys(compact.ratio)).toEqual(["status", "reasons"]);
    expect(JSON.stringify(compact.ratio)).not.toMatch(/p95|p99|numerator|denominator|value/i);
    expect(() => assertDataConfoundedRatioOmission(compact)).not.toThrow();
  });

  test("reports endpoint, cadence, frame, and Long Task statistics per round and pooled", () => {
    const { packet, attestation, attemptDigests } = completeFixture();
    const sustained = packet.attempts.find(attempt =>
      attempt.rowId === "phase4-peer/luna-sustained-59"
      && attempt.app === "yune-web" && attempt.attempt === 1
    )!;
    const overlappingTask = { startTime: 2_001, duration: 50 };
    sustained.frame!.observedLongTasks.push(overlappingTask);
    sustained.frame!.interactionLongTasks.push(overlappingTask);
    sustained.frame!.idleLongTasks.push(overlappingTask);
    expect(validatePeerAttempt(sustained)).toEqual([]);
    attemptDigests.set(key(sustained), sha256(canonicalJson(sustained)));

    const compact = buildPeerCompactReceipt(packet, attestation, "f".repeat(64), attemptDigests);
    const short = compact.rows.find(row =>
      row.rowId === "phase4-peer/luna-short-ni" && row.app === "yune-web"
    )!;
    expect(short.finalPrefix).toEqual({
      samples: 5, medianMs: 30, p95Ms: 30, p99Ms: 30, maxMs: 30,
    });
    expect(short.allPrefixes).toEqual({
      samples: 10, medianMs: 30, p95Ms: 30, p99Ms: 30, maxMs: 30,
    });
    expect(short.commit).toEqual({
      samples: 5, medianMs: 30, p95Ms: 30, p99Ms: 30, maxMs: 30,
    });
    expect(short.cadence).toEqual({
      driverGapSamples: 5,
      inRangeGapSamples: 5,
      tooShortGapSamples: 0,
      delayedHostGapSamples: 0,
    });
    expect(short.frameIntervals).toEqual({
      samples: 5,
      medianMs: 16.666,
      p95Ms: 16.666,
      p99Ms: 16.666,
      maxMs: 16.666,
      atLeast50MsCount: 0,
    });
    expect(short.longTasks).toEqual(expect.objectContaining({
      frameWindows: 5,
      observedCount: 0,
      observedDurationTotalMs: 0,
      interactionOverlapCount: 0,
      interactionTaskDurationTotalMs: 0,
      interactionTaskDurationMaxMs: 0,
      idleOverlapCount: 0,
      outsideWindowCount: 0,
    }));
    expect(short.longTasks!.interactionWindowDurationTotalMs).toBeCloseTo(83.33, 6);
    expect(short.perRound).toHaveLength(5);
    expect(short.perRound[0]).toEqual(expect.objectContaining({
      attempt: 1,
      status: "VALID_GREEN",
      validForLatencyFrame: true,
      finalPrefix: { samples: 1, medianMs: 30, p95Ms: 30, p99Ms: 30, maxMs: 30 },
      allPrefixes: { samples: 2, medianMs: 30, p95Ms: 30, p99Ms: 30, maxMs: 30 },
      commit: { samples: 1, medianMs: 30, p95Ms: 30, p99Ms: 30, maxMs: 30 },
      cadence: {
        driverGapSamples: 1,
        inRangeGapSamples: 1,
        tooShortGapSamples: 0,
        delayedHostGapSamples: 0,
      },
      frameIntervals: {
        samples: 1,
        medianMs: 16.666,
        p95Ms: 16.666,
        p99Ms: 16.666,
        maxMs: 16.666,
        atLeast50MsCount: 0,
      },
    }));
    expect(short.perRound[0]!.longTasks!.interactionWindowDurationTotalMs)
      .toBeCloseTo(16.666, 6);

    const sustainedSummary = compact.rows.find(row =>
      row.rowId === "phase4-peer/luna-sustained-59" && row.app === "yune-web"
    )!;
    expect(sustainedSummary.longTasks).toEqual(expect.objectContaining({
      frameWindows: 5,
      observedCount: 1,
      observedDurationTotalMs: 50,
      interactionOverlapCount: 1,
      interactionTaskDurationTotalMs: 50,
      interactionTaskDurationMaxMs: 50,
      idleOverlapCount: 1,
      outsideWindowCount: 0,
    }));
    expect(sustainedSummary.perRound[0]!.longTasks).toEqual(expect.objectContaining({
      frameWindows: 1,
      observedCount: 1,
      observedDurationTotalMs: 50,
      interactionOverlapCount: 1,
      interactionTaskDurationTotalMs: 50,
      interactionTaskDurationMaxMs: 50,
      idleOverlapCount: 1,
      outsideWindowCount: 0,
    }));
  });

  test("counts a valid RED, preserves it, and continues only until five valid attempts", () => {
    const { packet, attestation, attemptDigests } = completeFixture();
    const target = packet.attempts.find(attempt =>
      attempt.rowId === "phase4-peer/luna-short-ni" && attempt.app === "yune-web" && attempt.attempt === 1
    );
    expect(target).toBeDefined();
    const finalSample = target!.prefixSamples.at(-1)!;
    finalSample.observation.secondRaf.observedAt = finalSample.observation.event.timeStamp + 68;
    finalSample.eventToStableCandidateMs = 68;
    target!.latencyRedReasons.push("short-prefix-2-over-67ms");
    Object.assign(target!, classifyPeerAttempt(target!));
    attemptDigests.set(key(target!), sha256(canonicalJson(target!)));
    const compact = buildPeerCompactReceipt(packet, attestation, "f".repeat(64), attemptDigests);
    const row = compact.rows.find(item => item.rowId === "phase4-peer/luna-short-ni" && item.app === "yune-web");
    expect(row).toEqual(expect.objectContaining({
      attemptsRetained: 5,
      validRounds: 5,
      validRedRounds: 1,
      verdict: "RED",
    }));

    const behavior = packet.attempts.find(attempt =>
      attempt.rowId === "phase4-peer/luna-short-ni"
      && attempt.app === "my-rime" && attempt.attempt === 1
    )!;
    for (const tuple of [behavior.prefixSamples[1]!.observation.initial,
      behavior.prefixSamples[1]!.observation.firstRaf,
      behavior.prefixSamples[1]!.observation.secondRaf]) {
      tuple.page.nextDisabled = true;
      tuple.digest = comparatorDomTupleDigest(tuple);
    }
    behavior.behaviorRedReasons.push("prefix-2:candidate-page-evidence-incomplete");
    Object.assign(behavior, classifyPeerAttempt(behavior));
    expect(validatePeerAttempt(behavior)).toEqual([]);
  });

  test("rejects negative raw durations, misaligned rounds, and arbitrary smaller arrays", () => {
    const { packet, attestation, attemptDigests } = completeFixture();
    const negative = structuredClone(packet.attempts[0]!);
    negative.prefixSamples[0]!.eventToStableCandidateMs = -1;
    expect(validatePeerAttempt(negative)).toContain("prefix-1-latency-recompute");

    const swapped = structuredClone(packet);
    const group = swapped.attempts.filter(attempt => attempt.rowId === "phase4-peer/luna-short-ni" && attempt.app === "yune-web");
    group[1]!.attempt = 4;
    expect(() => buildPeerCompactReceipt(swapped, attestation, "f".repeat(64), attemptDigests))
      .toThrow(/raw-attempt-identity-duplicate/);

    const arbitrary = structuredClone(packet);
    arbitrary.attempts = arbitrary.attempts.filter(attempt =>
      !(attempt.rowId === "phase4-peer/luna-short-ni" && attempt.attempt > 3)
    );
    expect(() => buildPeerCompactReceipt(arbitrary, attestation, "f".repeat(64), attemptDigests))
      .toThrow(/raw-run-order-not-contiguous/);
  });

  test("gives hard RED precedence over a too-long driver gap without hiding other cadence/setup failures", () => {
    const attempt = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    attempt.driverDispatches[0]!.normalizedEventAt = 100;
    attempt.driverDispatches[0]!.eventDeliveredAt = 135;
    attempt.driverDispatches[1]!.normalizedEventAt = 190;
    attempt.driverDispatches[1]!.eventDeliveredAt = 230;
    attempt.driverDispatches[0]!.actualDriverDispatchAt = 100;
    attempt.driverDispatches[1]!.actualDriverDispatchAt = 160;
    attempt.actualDispatchGapsMs = [60];
    expect(peerCadenceSetupReasons(attempt.actualDispatchGapsMs)).toEqual([]);

    attempt.actualDispatchGapsMs = [80];
    attempt.setupInvalidReasons = peerCadenceSetupReasons(attempt.actualDispatchGapsMs);
    Object.assign(attempt, classifyPeerAttempt(attempt));
    expect(attempt.status).toBe("SETUP_INVALID");
    attempt.prefixSamples[1]!.observation.secondRaf.observedAt =
      attempt.prefixSamples[1]!.observation.event.timeStamp + 68;
    attempt.prefixSamples[1]!.eventToStableCandidateMs = 68;
    attempt.latencyRedReasons = ["short-prefix-2-over-67ms"];
    Object.assign(attempt, classifyPeerAttempt(attempt));
    expect(attempt).toEqual(expect.objectContaining({
      status: "VALID_RED", retainedHardRed: true, validForLatencyFrame: true,
    }));

    attempt.actualDispatchGapsMs = [40];
    attempt.setupInvalidReasons = peerCadenceSetupReasons(attempt.actualDispatchGapsMs);
    Object.assign(attempt, classifyPeerAttempt(attempt));
    expect(attempt.status).toBe("SETUP_INVALID");
    attempt.behaviorRedReasons = ["event-count-mismatch"];
    Object.assign(attempt, classifyPeerAttempt(attempt));
    expect(attempt).toEqual(expect.objectContaining({
      status: "SETUP_INVALID", retainedHardRed: true, validForLatencyFrame: false,
    }));

    const otherSetupFailure = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    otherSetupFailure.actualDispatchGapsMs = [80];
    otherSetupFailure.networkRouteInstalled = false;
    otherSetupFailure.setupInvalidReasons = [
      "network-route-not-installed-before-navigation",
      "invalid-cadence-too-long",
    ];
    otherSetupFailure.prefixSamples[1]!.observation.secondRaf.observedAt =
      otherSetupFailure.prefixSamples[1]!.observation.event.timeStamp + 68;
    otherSetupFailure.prefixSamples[1]!.eventToStableCandidateMs = 68;
    otherSetupFailure.latencyRedReasons = ["short-prefix-2-over-67ms"];
    Object.assign(otherSetupFailure, classifyPeerAttempt(otherSetupFailure));
    expect(otherSetupFailure).toEqual(expect.objectContaining({
      status: "SETUP_INVALID", retainedHardRed: true, validForLatencyFrame: false,
    }));

    const { packet, attestation, attemptDigests } = completeFixture();
    const retained = packet.attempts[0]!;
    retained.driverDispatches[1]!.requestedDispatchAt = 180;
    retained.driverDispatches[1]!.actualDriverDispatchAt = 180;
    retained.driverDispatches[1]!.cadenceRebased = true;
    retained.prefixSamples[1]!.requestedDispatchAt = 180;
    retained.prefixSamples[1]!.actualDriverDispatchAt = 180;
    retained.actualDispatchGapsMs = [80];
    retained.setupInvalidReasons = ["invalid-cadence-too-long"];
    retained.observedEvents = retained.observedEvents!.slice(0, -1);
    retained.behaviorRedReasons = ["event-count-mismatch"];
    Object.assign(retained, classifyPeerAttempt(retained));
    expect(validatePeerAttempt(retained)).toEqual([]);
    expect(() => validateAttemptIndependently(retained)).not.toThrow();
    attemptDigests.set(key(retained), sha256(canonicalJson(retained)));
    const compact = buildPeerCompactReceipt(packet, attestation, "f".repeat(64), attemptDigests);
    const retainedRow = compact.rows.find(row =>
      row.rowId === retained.rowId && row.app === retained.app
    );
    expect(retainedRow).toEqual(expect.objectContaining({
      attemptsRetained: 5,
      validRounds: 5,
      validRedRounds: 1,
      retainedHardRedAttempts: 1,
      invalidHardRedAttempts: 0,
      setupInvalidAttempts: 0,
      cadence: {
        driverGapSamples: 5,
        inRangeGapSamples: 4,
        tooShortGapSamples: 0,
        delayedHostGapSamples: 1,
      },
      verdict: "RED",
    }));

    const insertAt = packet.attempts.findIndex(candidate =>
      candidate.rowId === "phase4-peer/luna-sustained-59"
    );
    const sixthPair = (["my-rime", "yune-web"] as const).map(app =>
      fixtureAttempt("phase4-peer/luna-short-ni", app, 6)
    );
    packet.attempts.splice(insertAt, 0, ...sixthPair);
    packet.attempts.forEach((candidate, index) => { candidate.runOrderOrdinal = index + 1; });
    attemptDigests.clear();
    for (const candidate of packet.attempts) {
      attemptDigests.set(key(candidate), sha256(canonicalJson(candidate)));
    }
    expect(() => buildPeerCompactReceipt(packet, attestation, "f".repeat(64), attemptDigests))
      .toThrow(/after five paired valid rounds/);
  });

  test("separates preserved invalid latency evidence from verdict-bearing REDs", () => {
    const compactRow = (
      mutate: (attempt: Web06PeerAttempt) => void,
      addSixthPair: boolean,
    ) => {
      const { packet, attestation, attemptDigests } = completeFixture();
      const target = packet.attempts.find(attempt =>
        attempt.rowId === "phase4-peer/luna-short-ni"
        && attempt.app === "yune-web" && attempt.attempt === 1
      )!;
      mutate(target);
      Object.assign(target, classifyPeerAttempt(target));
      if (addSixthPair) appendShortFixturePair(packet, 6);
      refreshAttemptDigests(packet, attemptDigests);
      expect(validatePeerAttempt(target)).toEqual([]);
      expect(() => validateAttemptIndependently(target)).not.toThrow();
      return buildPeerCompactReceipt(packet, attestation, "f".repeat(64), attemptDigests)
        .rows.find(row => row.rowId === target.rowId && row.app === target.app)!;
    };
    const markLatencyRed = (attempt: Web06PeerAttempt) => {
      const final = attempt.prefixSamples[1]!;
      final.observation.secondRaf.observedAt = final.observation.event.timeStamp + 68;
      final.eventToStableCandidateMs = 68;
      attempt.latencyRedReasons = ["short-prefix-2-over-67ms"];
    };
    const setSecondDispatch = (attempt: Web06PeerAttempt, at: number) => {
      const dispatch = attempt.driverDispatches[1]!;
      dispatch.phaseDeadlineAt = at;
      dispatch.requestedDispatchAt = at;
      dispatch.actualDriverDispatchAt = at;
      dispatch.cadenceRebased = false;
      attempt.prefixSamples[1]!.requestedDispatchAt = at;
      attempt.prefixSamples[1]!.actualDriverDispatchAt = at;
      attempt.actualDispatchGapsMs = [at - attempt.driverDispatches[0]!.actualDriverDispatchAt];
      attempt.setupInvalidReasons = peerCadenceSetupReasons(attempt.actualDispatchGapsMs);
    };

    const tooShortLatency = compactRow(attempt => {
      setSecondDispatch(attempt, 140);
      markLatencyRed(attempt);
    }, true);
    expect(tooShortLatency).toEqual(expect.objectContaining({
      attemptsRetained: 6,
      validRounds: 5,
      validRedRounds: 0,
      retainedHardRedAttempts: 1,
      verdictBearingRedAttempts: 0,
      invalidHardRedAttempts: 1,
      setupInvalidAttempts: 1,
      cadence: {
        driverGapSamples: 6,
        inRangeGapSamples: 5,
        tooShortGapSamples: 1,
        delayedHostGapSamples: 0,
      },
      verdict: "PASS",
    }));
    expect(tooShortLatency.perRound[0]).toEqual(expect.objectContaining({
      status: "SETUP_INVALID",
      retainedHardRed: true,
      verdictBearingRed: false,
      validForLatencyFrame: false,
    }));

    const tooShortBehavior = compactRow(attempt => {
      setSecondDispatch(attempt, 140);
      attempt.observedEvents = attempt.observedEvents!.slice(0, -1);
      attempt.behaviorRedReasons = ["event-count-mismatch"];
    }, true);
    expect(tooShortBehavior).toEqual(expect.objectContaining({
      validRounds: 5,
      validRedRounds: 0,
      retainedHardRedAttempts: 1,
      verdictBearingRedAttempts: 1,
      invalidHardRedAttempts: 1,
      verdict: "RED",
    }));
    expect(tooShortBehavior.perRound[0]).toEqual(expect.objectContaining({
      status: "SETUP_INVALID",
      retainedHardRed: true,
      verdictBearingRed: true,
      validForLatencyFrame: false,
    }));

    const otherSetupLatency = compactRow(attempt => {
      attempt.networkRouteInstalled = false;
      attempt.setupInvalidReasons = ["network-route-not-installed-before-navigation"];
      markLatencyRed(attempt);
    }, true);
    expect(otherSetupLatency).toEqual(expect.objectContaining({
      validRounds: 5,
      validRedRounds: 0,
      retainedHardRedAttempts: 1,
      verdictBearingRedAttempts: 0,
      invalidHardRedAttempts: 1,
      verdict: "PASS",
    }));

    const tooLongLatency = compactRow(attempt => {
      const dispatch = attempt.driverDispatches[1]!;
      dispatch.requestedDispatchAt = 180;
      dispatch.actualDriverDispatchAt = 180;
      dispatch.cadenceRebased = true;
      attempt.prefixSamples[1]!.requestedDispatchAt = 180;
      attempt.prefixSamples[1]!.actualDriverDispatchAt = 180;
      attempt.actualDispatchGapsMs = [80];
      attempt.setupInvalidReasons = ["invalid-cadence-too-long"];
      markLatencyRed(attempt);
    }, false);
    expect(tooLongLatency).toEqual(expect.objectContaining({
      validRounds: 5,
      validRedRounds: 1,
      retainedHardRedAttempts: 1,
      verdictBearingRedAttempts: 1,
      invalidHardRedAttempts: 0,
      setupInvalidAttempts: 0,
      verdict: "RED",
    }));
    expect(tooLongLatency.perRound[0]).toEqual(expect.objectContaining({
      status: "VALID_RED",
      retainedHardRed: true,
      verdictBearingRed: true,
      validForLatencyFrame: true,
    }));
  });

  test("keeps slow and superseded intermediate prefixes as valid REDs on the absolute cadence", () => {
    const slow = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    for (const sample of slow.prefixSamples) {
      sample.observation.secondRaf.observedAt = sample.observation.event.timeStamp + 68;
      sample.eventToStableCandidateMs = 68;
      slow.latencyRedReasons.push(`short-prefix-${sample.prefixOrdinal}-over-67ms`);
    }
    Object.assign(slow, classifyPeerAttempt(slow));
    expect(slow.actualDispatchGapsMs).toEqual([60]);
    expect(peerCadenceSetupReasons(slow.actualDispatchGapsMs)).toEqual([]);
    expect(validatePeerAttempt(slow)).toEqual([]);

    const omittedThreshold = structuredClone(slow);
    omittedThreshold.latencyRedReasons = ["short-prefix-2-over-67ms"];
    Object.assign(omittedThreshold, classifyPeerAttempt(omittedThreshold));
    expect(validatePeerAttempt(omittedThreshold)).toContain("short-latency-red-recompute");

    const superseded = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    superseded.prefixSamples.splice(0, 1);
    retainPeerMeasurementFailure(superseded, {
      stage: "candidate",
      prefixOrdinal: 1,
      code: "CANDIDATE_ENDPOINT_SUPERSEDED",
    });
    superseded.behaviorRedReasons.push("exact-prefix-count");
    Object.assign(superseded, classifyPeerAttempt(superseded));
    expect(superseded.actualDispatchGapsMs).toEqual([60]);
    expect(validatePeerAttempt(superseded)).toEqual([]);

    const substituted = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    substituted.prefixSamples[0]!.observation.event.ordinal = 2;
    expect(validatePeerAttempt(substituted)).toContain("prefix-1-event-identity");

    const delayed = fixtureAttempt("phase4-peer/luna-sustained-59", "my-rime", 1);
    delayed.driverDispatches[1]!.actualDriverDispatchAt = 240;
    delayed.driverDispatches[1]!.cadenceRebased = true;
    delayed.prefixSamples[1]!.actualDriverDispatchAt = 240;
    let phaseBase = 240;
    for (let index = 2; index < delayed.driverDispatches.length; index += 1) {
      const dispatch = delayed.driverDispatches[index]!;
      const deadline = phaseBase + 60;
      dispatch.phaseDeadlineAt = deadline;
      dispatch.requestedDispatchAt = deadline;
      dispatch.actualDriverDispatchAt = deadline;
      dispatch.cadenceRebased = false;
      delayed.prefixSamples[index]!.requestedDispatchAt = deadline;
      delayed.prefixSamples[index]!.actualDriverDispatchAt = deadline;
      phaseBase = deadline;
    }
    delayed.actualDispatchGapsMs = delayed.driverDispatches.slice(1).map((dispatch, index) =>
      dispatch.actualDriverDispatchAt - delayed.driverDispatches[index]!.actualDriverDispatchAt
    );
    delayed.setupInvalidReasons = peerCadenceSetupReasons(delayed.actualDispatchGapsMs);
    Object.assign(delayed, classifyPeerAttempt(delayed));
    expect(delayed.actualDispatchGapsMs.slice(0, 3)).toEqual([140, 60, 60]);
    expect(delayed.actualDispatchGapsMs.some(gap => gap < 48)).toBe(false);
    expect(validatePeerAttempt(delayed)).toEqual([]);
  });

  test("independent tuple digest recomputation rejects field mutation with unchanged supplied digests", () => {
    const observation = candidateObservation("yune-web", "ni", 2, 160);
    expect(independentStableObservationFailures(observation)).toEqual([]);
    observation.secondRaf.candidates[0]!.text = "篡改";
    expect(independentStableObservationFailures(observation))
      .toContain("second-raf-digest-not-recomputed");
  });

  test("recomputes equal-duration idle/interaction/outside Long Task classification", () => {
    const base = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1).frame!;
    expect(validatePeerFrameReceipt(base)).toEqual([]);

    const omittedSetup = structuredClone(base);
    omittedSetup.observedLongTasks.push({ startTime: 1_000, duration: 50 });
    expect(validatePeerFrameReceipt(omittedSetup)).toContain("long-task-window-classification-invalid");
    omittedSetup.outsideWindowLongTasks.push({ startTime: 1_000, duration: 50 });
    expect(validatePeerFrameReceipt(omittedSetup)).toEqual([]);

    const straddling = structuredClone(base);
    const boundaryTask = { startTime: 2_006.666, duration: 20 };
    straddling.observedLongTasks.push(boundaryTask);
    straddling.interactionLongTasks.push(boundaryTask);
    expect(validatePeerFrameReceipt(straddling)).toContain("long-task-window-classification-invalid");
    straddling.idleLongTasks.push(boundaryTask);
    expect(validatePeerFrameReceipt(straddling)).toEqual([]);

    const unequal = structuredClone(base);
    unequal.idleControlWindowEndAt += 1;
    expect(validatePeerFrameReceipt(unequal)).toContain("frame-window-boundary-or-duration-invalid");

    const swapped = structuredClone(base);
    const interactionTask = { startTime: 2_005, duration: 5 };
    swapped.observedLongTasks.push(interactionTask);
    swapped.idleLongTasks.push(interactionTask);
    expect(validatePeerFrameReceipt(swapped)).toContain("long-task-window-classification-invalid");
  });

  test("declared commit cannot be substituted by a later event", async ({ page }) => {
    await page.setContent(`
      <div data-yune-status>
        <span data-yune-status-schema data-yune-status-schema-id="luna_pinyin"></span>
        <span data-yune-status-composing="false"></span>
      </div>
      <textarea class="yd-input-area">你</textarea>
    `);
    const editable = page.locator("textarea.yd-input-area");
    await editable.evaluate((element: HTMLTextAreaElement) => {
      element.value = "你";
      element.setSelectionRange(1, 1);
    });
    await editable.focus();
    await installComparatorEndpointObserver(page, "yune-web");
    await page.evaluate(() => {
      const editableElement = document.querySelector("textarea.yd-input-area")!;
      const status = document.querySelector("[data-yune-status]")!;
      setTimeout(() => {
        editableElement.dispatchEvent(new KeyboardEvent("keydown", {
          key: " ", code: "Space", bubbles: true,
        }));
        status.setAttribute("data-commit-revision", "1");
      }, 0);
      setTimeout(() => {
        editableElement.dispatchEvent(new KeyboardEvent("keydown", {
          key: "x", code: "KeyX", bubbles: true,
        }));
      }, 1);
    });
    await expect(waitForStableCommitEndpoint(page, "yune-web", "你", 0))
      .rejects.toThrow(/COMMIT_ENDPOINT_SUPERSEDED/);
  });

  test("uses live root schema evidence for empty Yune posture before candidate status exists", async ({ page }) => {
    await page.route("http://posture.test/**", async route => route.fulfill({
      contentType: "text/html",
      body: `
        <style>textarea { display: block; width: 200px; height: 24px }</style>
        <textarea class="yd-input-area"></textarea>
        <div class="candidate-panel"><div class="candidates" style="display: none"></div></div>
      `,
    }));
    await page.goto("http://posture.test/");
    await page.evaluate(() => {
      localStorage.setItem("pageSize", "6");
      document.documentElement.dataset.yuneActiveSchema = "luna_pinyin";
    });
    expect(await page.locator("[data-yune-status-schema]").count()).toBe(0);
    expect(await readComparatorEmptyPosture(page, "yune-web")).toEqual({
      localStoragePageSize: "6",
      visibleCandidateSurfaces: 0,
      yuneComposing: null,
      schemaEvidenceMatches: 1,
      schemaEvidenceValue: "luna_pinyin",
    });
  });

  test("binds global run order, exact final-prefix aggregation, public privacy, and verifier API", () => {
    const { packet, attestation, attemptDigests } = completeFixture();
    const reordered = structuredClone(packet);
    [reordered.attempts[0], reordered.attempts[1]] = [reordered.attempts[1]!, reordered.attempts[0]!];
    expect(() => buildPeerCompactReceipt(reordered, attestation, "f".repeat(64), attemptDigests))
      .toThrow(/raw-run-order-not-contiguous/);
    reordered.attempts.forEach((attempt, index) => { attempt.runOrderOrdinal = index + 1; });
    expect(() => buildPeerCompactReceipt(reordered, attestation, "f".repeat(64), attemptDigests))
      .toThrow(/raw-frozen-row-pair-order/);

    const interleaved = structuredClone(packet);
    const sustainedPair = interleaved.attempts.splice(10, 2);
    interleaved.attempts.splice(2, 0, ...sustainedPair);
    interleaved.attempts.forEach((attempt, index) => { attempt.runOrderOrdinal = index + 1; });
    expect(() => buildPeerCompactReceipt(interleaved, attestation, "f".repeat(64), attemptDigests))
      .toThrow(/raw-frozen-row/);

    const missingFinal = packet.attempts.find(attempt =>
      attempt.rowId === "phase4-peer/luna-short-ni"
      && attempt.app === "yune-web" && attempt.attempt === 1
    )!;
    missingFinal.prefixSamples = missingFinal.prefixSamples.filter(sample => sample.prefixOrdinal !== 2);
    missingFinal.commit = null;
    missingFinal.commitDriverDispatch = null;
    missingFinal.committedValue = "";
    retainPeerMeasurementFailure(missingFinal, {
      stage: "candidate",
      prefixOrdinal: 2,
      code: "CANDIDATE_ENDPOINT_TIMEOUT",
    });
    missingFinal.behaviorRedReasons.push("exact-prefix-count");
    Object.assign(missingFinal, classifyPeerAttempt(missingFinal));
    attemptDigests.set(key(missingFinal), sha256(canonicalJson(missingFinal)));
    const compact = buildPeerCompactReceipt(packet, attestation, "f".repeat(64), attemptDigests);
    const shortYune = compact.rows.find(row =>
      row.rowId === "phase4-peer/luna-short-ni" && row.app === "yune-web"
    );
    expect(shortYune?.finalPrefix?.samples).toBe(4);
    expect(shortYune?.commit?.samples).toBe(4);

    for (const privateValue of [
      "/Users/private/raw", "C:\\private\\raw", "\\\\server\\share\\raw",
      "//server/share/raw", "file:///private/raw", "YUNE_WEB06_SECRET", "token: abc123",
    ]) {
      expect(() => assertPublicEvidencePrivacy({ nested: [privateValue] })).toThrow(/absolute path|secret-shaped/);
    }
    expect(typeof verifyWeb06PeerEvidence).toBe("function");
  });

  test("keeps observer/metric/frame failures setup-invalid and endpoint absence/supersession RED", () => {
    expect(measurementFailureDisposition("ENDPOINT_OBSERVER_MISSING")).toBe("SETUP_INVALID");
    expect(measurementFailureDisposition("ENDPOINT_OBSERVER_WRONG_APP")).toBe("SETUP_INVALID");
    expect(measurementFailureDisposition("FRAME_IDLE_CONTROL_TIMEOUT")).toBe("SETUP_INVALID");
    expect(measurementFailureDisposition("CANDIDATE_ENDPOINT_TIMEOUT")).toBe("BEHAVIOR_RED");
    expect(measurementFailureDisposition("CANDIDATE_ENDPOINT_SUPERSEDED")).toBe("BEHAVIOR_RED");

    for (const code of ["ENDPOINT_OBSERVER_MISSING", "ENDPOINT_OBSERVER_WRONG_APP"] as const) {
      const observer = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
      observer.prefixSamples.splice(0, 1);
      retainPeerMeasurementFailure(observer, { stage: "candidate", prefixOrdinal: 1, code });
      Object.assign(observer, classifyPeerAttempt(observer));
      expect(observer).toEqual(expect.objectContaining({
        status: "SETUP_INVALID", validForLatencyFrame: false, retainedHardRed: false,
      }));
      expect(validatePeerAttempt(observer)).toEqual([]);
    }

    const frame = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    frame.frame = null;
    retainPeerMeasurementFailure(frame, {
      stage: "frame", prefixOrdinal: null, code: "FRAME_IDLE_CONTROL_TIMEOUT",
    });
    Object.assign(frame, classifyPeerAttempt(frame));
    expect(frame.status).toBe("SETUP_INVALID");
    expect(validatePeerAttempt(frame)).toEqual([]);

    const metric = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    metric.prefixSamples[0]!.observation.secondRaf.revision += 1;
    retainPeerMeasurementFailure(metric, {
      stage: "candidate", prefixOrdinal: 1, code: "ENDPOINT_METRIC_CONTRACT_FAILURE",
    });
    Object.assign(metric, classifyPeerAttempt(metric));
    expect(metric.status).toBe("SETUP_INVALID");
    expect(validatePeerAttempt(metric)).toEqual([]);

    const fakeSetup = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    fakeSetup.prefixSamples[1]!.observation.secondRaf.observedAt += 40;
    fakeSetup.prefixSamples[1]!.eventToStableCandidateMs += 40;
    fakeSetup.latencyRedReasons.push("short-prefix-2-over-67ms");
    fakeSetup.setupInvalidReasons.push("fake-setup-invalid");
    Object.assign(fakeSetup, classifyPeerAttempt(fakeSetup));
    expect(fakeSetup.retainedHardRed).toBe(true);
    expect(validatePeerAttempt(fakeSetup)).toContain("setup-invalid-reasons-exact-recompute");
  });

  test("retains endpoint and event behavior REDs when post-dispatch frame/setup evidence fails", () => {
    const slow = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    const final = slow.prefixSamples[1]!;
    final.observation.secondRaf.observedAt = final.observation.event.timeStamp + 70;
    final.eventToStableCandidateMs = 70;
    slow.latencyRedReasons.push("short-prefix-2-over-67ms");
    slow.frame = null;
    retainPeerMeasurementFailure(slow, {
      stage: "frame",
      prefixOrdinal: null,
      code: "FRAME_IDLE_CONTROL_TIMEOUT",
    });
    Object.assign(slow, classifyPeerAttempt(slow));
    expect(slow).toEqual(expect.objectContaining({
      status: "SETUP_INVALID",
      retainedHardRed: true,
      validForLatencyFrame: false,
    }));
    expect(validatePeerAttempt(slow)).toEqual([]);
    expect(() => validateAttemptIndependently(slow)).not.toThrow();

    const eventRed = fixtureAttempt("phase4-peer/luna-short-ni", "my-rime", 1);
    eventRed.observedEvents!.pop();
    eventRed.behaviorRedReasons.push("event-count-mismatch");
    eventRed.frame = null;
    retainPeerMeasurementFailure(eventRed, {
      stage: "post-dispatch",
      prefixOrdinal: null,
      code: "POST_DISPATCH_METRIC_CONTRACT_FAILURE",
    });
    Object.assign(eventRed, classifyPeerAttempt(eventRed));
    expect(eventRed).toEqual(expect.objectContaining({
      status: "SETUP_INVALID",
      retainedHardRed: true,
      validForLatencyFrame: false,
    }));
    expect(validatePeerAttempt(eventRed)).toEqual([]);
    expect(() => validateAttemptIndependently(eventRed)).not.toThrow();
  });

  test("rejects final candidate membership, active-schema, and candidate-to-commit linkage mutation", () => {
    const membership = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    for (const tuple of [membership.prefixSamples[1]!.observation.initial,
      membership.prefixSamples[1]!.observation.firstRaf,
      membership.prefixSamples[1]!.observation.secondRaf]) {
      tuple.candidates[4]!.text = "篡改";
      tuple.digest = comparatorDomTupleDigest(tuple);
    }
    expect(validatePeerAttempt(membership))
      .toContain("prefix-2:frozen-final-candidate-membership-mismatch-red-missing");
    expect(() => validateAttemptIndependently(membership))
      .toThrow(/frozen final candidate membership RED/);

    const schema = fixtureAttempt("phase4-peer/luna-short-ni", "my-rime", 1);
    for (const tuple of [schema.prefixSamples[1]!.observation.initial,
      schema.prefixSamples[1]!.observation.firstRaf,
      schema.prefixSamples[1]!.observation.secondRaf]) {
      tuple.status.schemaId = "not_luna";
      tuple.status.digest = JSON.stringify({ schemaId: "not_luna", composing: true });
      tuple.digest = comparatorDomTupleDigest(tuple);
    }
    expect(validatePeerAttempt(schema))
      .toContain("prefix-2:active-schema-is-not-luna-pinyin-red-missing");
    expect(() => validateAttemptIndependently(schema))
      .toThrow(/active-schema-is-not-luna-pinyin/);

    const temporal = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    const finalEndpointAt = temporal.prefixSamples[1]!.observation.secondRaf.observedAt;
    temporal.commit!.event.timeStamp = finalEndpointAt - 1;
    temporal.commit!.event.deliveredAt = finalEndpointAt;
    temporal.commitDriverDispatch!.normalizedEventAt = finalEndpointAt - 1;
    temporal.commitDriverDispatch!.eventDeliveredAt = finalEndpointAt;
    temporal.observedEvents!.at(-1)!.timeStamp = finalEndpointAt - 1;
    temporal.observedEvents!.at(-1)!.deliveredAt = finalEndpointAt;
    expect(validatePeerAttempt(temporal))
      .toContain("commit-sequence:commit-event-precedes-coherent-final-prefix-red-missing");
    expect(() => validateAttemptIndependently(temporal))
      .toThrow(/commit linkage RED commit-event-precedes-coherent-final-prefix/);

    const substituted = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    substituted.committedValue = "篡改";
    for (const tuple of [substituted.commit!.initial,
      substituted.commit!.firstRaf,
      substituted.commit!.secondRaf]) {
      tuple.caret.value = "篡改";
      tuple.caret.selectionStart = 2;
      tuple.caret.selectionEnd = 2;
      tuple.digest = comparatorDomTupleDigest(tuple);
    }
    expect(validatePeerAttempt(substituted))
      .toContain("commit-sequence:commit-value-does-not-match-highlighted-final-candidate-red-missing");
    expect(() => validateAttemptIndependently(substituted))
      .toThrow(/commit linkage RED commit-value-does-not-match-highlighted-final-candidate/);
  });

  test("keeps every typed measurement-failure code mirrored in the independent verifier", async () => {
    const verifierSource = await readFile(
      path.join(path.dirname(fileURLToPath(import.meta.url)), "verify-web06-peer-evidence.ts"),
      "utf8",
    );
    for (const code of web06PeerMeasurementFailureCodes) {
      expect(verifierSource, code).toContain(`"${code}"`);
    }

    const eventStream = fixtureAttempt("phase4-peer/luna-short-ni", "yune-web", 1);
    eventStream.observedEvents = null;
    retainPeerMeasurementFailure(eventStream, {
      stage: "post-dispatch",
      prefixOrdinal: null,
      code: "EVENT_STREAM_METRIC_CONTRACT_FAILURE",
    });
    Object.assign(eventStream, classifyPeerAttempt(eventStream));
    expect(validatePeerAttempt(eventStream)).toEqual([]);
    expect(() => validateAttemptIndependently(eventStream)).not.toThrow();
  });

  test("derives the source tree from the artifact commit and rejects forged identity", () => {
    const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
    const commit = execFileSync("git", ["rev-parse", "HEAD"], { cwd: repoRoot, encoding: "utf8" }).trim();
    const tree = execFileSync("git", ["rev-parse", "HEAD^{tree}"], { cwd: repoRoot, encoding: "utf8" }).trim();
    expect(verifyArtifactSourceTree(repoRoot, commit, tree)).toBe(tree);
    expect(() => verifyArtifactSourceTree(repoRoot, commit, "f".repeat(40)))
      .toThrow(/not derived from artifact build-info commit/);
    expect(() => verifyArtifactSourceTree(repoRoot, "0".repeat(40), tree))
      .toThrow(/build-info source commit is not present/);
  });

  test("safe extractor rejects links and traversal before extraction", async () => {
    const root = await mkdtemp(path.join(os.tmpdir(), "web06-peer-tar-contract-"));
    try {
      const source = path.join(root, "source");
      await mkdir(source);
      await writeFile(path.join(source, "safe"), "ok\n");
      const normal = path.join(root, "normal.tar");
      execFileSync("tar", ["-cf", normal, "-C", source, "safe"]);
      await safeExtractTar(normal, path.join(root, "normal-out"));
      expect(await readFile(path.join(root, "normal-out", "safe"), "utf8")).toBe("ok\n");

      await symlink("safe", path.join(source, "link"));
      const linked = path.join(root, "linked.tar");
      execFileSync("tar", ["-cf", linked, "-C", source, "link"]);
      await expect(safeExtractTar(linked, path.join(root, "linked-out"))).rejects.toThrow(/forbidden non-file/);

      const traversing = path.join(root, "traversing.tar");
      execFileSync("tar", ["-cf", traversing, "-C", source, "-s", ",^safe$,../escape,", "safe"]);
      await expect(safeExtractTar(traversing, path.join(root, "traversing-out"))).rejects.toThrow(/archive path/);

      const packet = path.join(root, "packet");
      await mkdir(packet);
      const outside = path.join(root, "outside");
      await writeFile(outside, "not sealed\n");
      await symlink(outside, path.join(packet, "README.md"));
      await expect(resolvePinnedPeerPacketFile(packet, "README.md")).rejects.toThrow(/non-link/);
      await expect(resolvePinnedPeerPacketFile(packet, "../../escape")).rejects.toThrow(/exact sealed allowlist/);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  test("rehashes every served member and keeps mutation/symlink failures sticky", async () => {
    const root = await mkdtemp(path.join(os.tmpdir(), "web06-peer-serving-contract-"));
    try {
      const member = path.join(root, "index.html");
      const original = "alpha";
      await writeFile(member, original);
      const members = [{ path: "index.html", size: original.length, sha256: sha256(original) }];
      const guard = await Web06ArtifactServingGuard.create(root, members);
      expect(Buffer.from((await guard.read("index.html"))!).toString("utf8")).toBe(original);
      await writeFile(member, "omega");
      await expect(guard.read("index.html")).rejects.toThrow(/changed from its immutable attestation/);
      await writeFile(member, original);
      expect(Buffer.from((await guard.read("index.html"))!).toString("utf8")).toBe(original);
      expect(() => guard.assertIntegrity()).toThrow(/Sticky artifact serving integrity failure/);

      const symlinkGuard = await Web06ArtifactServingGuard.create(root, members);
      const outside = path.join(root, "..", `${path.basename(root)}-outside`);
      await writeFile(outside, original);
      await rm(member);
      await symlink(outside, member);
      await expect(symlinkGuard.read("index.html")).rejects.toThrow(/link|outside/);
      expect(() => symlinkGuard.assertIntegrity()).toThrow(/Sticky artifact serving integrity failure/);
      await rm(outside, { force: true });
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  test("rejects ordinary runner-source and verifier-byte mutation after attestation", async () => {
    const root = await mkdtemp(path.join(os.tmpdir(), "web06-peer-source-contract-"));
    try {
      const names = [
        "endpoint", "browserEndpoint", "lane", "artifactAttestor", "launcher",
        "independentLogic", "spec", "config", "verifier",
      ] as const;
      const sourceFiles = Object.fromEntries(names.map(name => [name, path.join(root, `${name}.ts`)])) as
        Record<(typeof names)[number], string>;
      await Promise.all(Object.values(sourceFiles).map(file => writeFile(file, "sealed\n")));
      const digest = sha256("sealed\n");
      const expected = {
        endpointSourceSha256: digest,
        browserEndpointSourceSha256: digest,
        laneSourceSha256: digest,
        artifactAttestorSourceSha256: digest,
        launcherSourceSha256: digest,
        independentLogicSourceSha256: digest,
        specSourceSha256: digest,
        configSourceSha256: digest,
        verifierSourceSha256: digest,
      };
      await expect(assertAttestedSourceFileHashes(sourceFiles, expected)).resolves.toBeUndefined();
      await writeFile(sourceFiles.endpoint, "mutated\n");
      await expect(assertAttestedSourceFileHashes(sourceFiles, expected))
        .rejects.toThrow(/source changed after snapshot: endpoint/);
      await writeFile(sourceFiles.endpoint, "sealed\n");
      await writeFile(sourceFiles.verifier, "mutated\n");
      await expect(assertAttestedSourceFileHashes(sourceFiles, expected))
        .rejects.toThrow(/source changed after snapshot: verifier/);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  test("external output resolution rejects a symlink parent into the repository and accepts tmp aliases", async () => {
    const root = await mkdtemp(path.join(os.tmpdir(), "web06-peer-output-contract-"));
    const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
    try {
      const linkedParent = path.join(root, "repo-link");
      await symlink(repoRoot, linkedParent);
      await expect(resolveCreateNewExternalOutputRoot(repoRoot, path.join(linkedParent, "raw")))
        .rejects.toThrow(/resolves inside the tracked repository/);

      const privateTmp = await resolveCreateNewExternalOutputRoot(repoRoot, path.join("/private/tmp", "web06-output"));
      const tmpAlias = await resolveCreateNewExternalOutputRoot(repoRoot, path.join("/tmp", "web06-output"));
      expect(tmpAlias.outputRoot).toBe(privateTmp.outputRoot);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });
});

function completeFixture(): {
  packet: Web06PeerRawPacket;
  attestation: Web06PeerAttestation;
  attemptDigests: Map<string, string>;
} {
  const attempts: Web06PeerAttempt[] = [];
  let runOrderOrdinal = 0;
  for (const row of web06PeerRows) {
    for (let attempt = 1; attempt <= 5; attempt += 1) {
      const order = attempt % 2 === 1
        ? (["yune-web", "my-rime"] as const)
        : (["my-rime", "yune-web"] as const);
      for (const app of order) {
        const value = fixtureAttempt(row.id, app, attempt);
        runOrderOrdinal += 1;
        value.runOrderOrdinal = runOrderOrdinal;
        attempts.push(value);
      }
    }
  }
  const attestation = fixtureAttestation();
  const packet: Web06PeerRawPacket = {
    version: "web06-phase4-peer-raw-v1",
    laneVersion: "web06-phase4-peer-lane-v1",
    phase: "phase4-peer",
    benchmarkAttempt: true,
    attestationSha256: "a".repeat(64),
    configSha256: "b".repeat(64),
    identityManifestSha256: attestation.identityManifestSha256,
    packageAlignment: "DATA_CONFOUNDED",
    attempts,
  };
  return {
    packet,
    attestation,
    attemptDigests: new Map(attempts.map(attempt => [key(attempt), sha256(canonicalJson(attempt))])),
  };
}

function appendShortFixturePair(packet: Web06PeerRawPacket, attempt: number): void {
  const insertAt = packet.attempts.findIndex(candidate =>
    candidate.rowId === "phase4-peer/luna-sustained-59"
  );
  const order = attempt % 2 === 1
    ? (["yune-web", "my-rime"] as const)
    : (["my-rime", "yune-web"] as const);
  packet.attempts.splice(
    insertAt,
    0,
    ...order.map(app => fixtureAttempt("phase4-peer/luna-short-ni", app, attempt)),
  );
  packet.attempts.forEach((candidate, index) => { candidate.runOrderOrdinal = index + 1; });
}

function refreshAttemptDigests(
  packet: Web06PeerRawPacket,
  attemptDigests: Map<string, string>,
): void {
  attemptDigests.clear();
  for (const attempt of packet.attempts) {
    attemptDigests.set(key(attempt), sha256(canonicalJson(attempt)));
  }
}

function fixtureAttempt(
  rowId: "phase4-peer/luna-short-ni" | "phase4-peer/luna-sustained-59",
  app: "yune-web" | "my-rime",
  attempt: number,
): Web06PeerAttempt {
  const input = web06PeerRows.find(row => row.id === rowId)!.input;
  const prefixSamples = [...input].map((_, index) => {
    const prefix = input.slice(0, index + 1);
    const eventAt = 100 + index * 60;
    const observation = candidateObservation(app, prefix, index + 1, eventAt);
    return {
      prefixOrdinal: index + 1,
      expectedPrefix: prefix,
      requestedDispatchAt: eventAt,
      actualDriverDispatchAt: eventAt,
      observation,
      eventToStableCandidateMs: observation.secondRaf.observedAt - eventAt,
    };
  });
  const commitAt = 100 + input.length * 60 + 100;
  const committedValue = prefixSamples.at(-1)!.observation.secondRaf.candidates[0]!.text;
  const commit = commitObservation(app, commitAt, committedValue, input.length + 1);
  const yuneRunsFirst = attempt % 2 === 1;
  const appRunsFirst = app === (yuneRunsFirst ? "yune-web" : "my-rime");
  const idleRafTimestamps = Array.from(
    { length: 121 },
    (_, index) => 2_000 - (120 - index) * 16.666,
  );
  const idleIntervalsMs = Array.from({ length: 120 }, () => 16.666);
  return {
    version: "web06-phase4-peer-raw-v1",
    benchmarkAttempt: true,
    rowId,
    app,
    attempt,
    runOrderOrdinal: attempt * 2 - (appRunsFirst ? 1 : 0),
    freshProfile: true,
    firstKeyIncluded: true,
    warmupKeyCount: 0,
    viewport: { width: 1365, height: 900 },
    locale: "zh-HK",
    cacheRegime: "fresh-persistent-profile-no-store-v1",
    cadenceMs: 60,
    pageSize: 6,
    measurementStarted: true,
    networkRouteInstalled: true,
    pretypingIdleMedianMs: 16.666,
    initialEventOrdinal: 0,
    observedEvents: [
      ...prefixSamples.map(sample => sample.observation.event),
      commit.event,
    ],
    driverDispatches: prefixSamples.map(sample => ({
      prefixOrdinal: sample.prefixOrdinal,
      key: sample.expectedPrefix.at(-1)!,
      phaseDeadlineAt: sample.actualDriverDispatchAt,
      requestedDispatchAt: sample.actualDriverDispatchAt,
      actualDriverDispatchAt: sample.actualDriverDispatchAt,
      cadenceRebased: false,
      normalizedEventAt: sample.observation.event.timeStamp,
      eventDeliveredAt: sample.observation.event.deliveredAt ?? null,
    })),
    commitDriverDispatch: {
      prefixOrdinal: input.length + 1,
      key: " ",
      phaseDeadlineAt: commitAt,
      requestedDispatchAt: commitAt,
      actualDriverDispatchAt: commitAt,
      cadenceRebased: false,
      normalizedEventAt: commit.event.timeStamp,
      eventDeliveredAt: commit.event.deliveredAt ?? null,
    },
    frame: {
      longTaskObserverSupported: true,
      idleRafTimestamps,
      idleIntervalsMs,
      idleMedianMs: 16.666,
      idleControlWindowStartAt: 2_016.666,
      idleControlWindowEndAt: 2_033.332,
      idleControlRafTimestamps: [2_016.666, 2_033.332],
      observedLongTasks: [],
      idleLongTasks: [],
      interactionWindowStartAt: 2_000,
      interactionWindowEndAt: 2_016.666,
      interactionRafTimestamps: [2_000, 2_016.666],
      interactionFrameIntervalsMs: [16.666],
      interactionLongTasks: [],
      outsideWindowLongTasks: [],
      visibilityOrFocusLost: false,
    },
    prefixSamples,
    commit,
    committedValue,
    actualDispatchGapsMs: Array.from({ length: Math.max(0, input.length - 1) }, () => 60),
    consoleErrors: [],
    blockedSetupNetworkRequests: [],
    timedAssetRequests: [],
    unexpectedNetworkRequests: [],
    foregroundAndFocused: true,
    measurementFailures: [],
    setupInvalidReasons: [],
    behaviorRedReasons: [],
    latencyRedReasons: [],
    retainedHardRed: false,
    validForLatencyFrame: true,
    status: "VALID_GREEN",
  };
}

function candidateObservation(
  app: "yune-web" | "my-rime",
  prefix: string,
  ordinal: number,
  eventAt: number,
): ComparatorStableObservation {
  const tuple = atomicTuple(app, prefix, "", eventAt + 10);
  return {
    event: {
      ordinal,
      type: "keydown",
      key: prefix.at(-1)!,
      code: `Key${prefix.at(-1)!.toUpperCase()}`,
      timeStamp: eventAt,
      deliveredAt: eventAt + 1,
      revisionBeforeEvent: ordinal - 1,
    },
    initial: { ...tuple, observedAt: eventAt + 10 },
    firstRaf: { ...tuple, observedAt: eventAt + 20 },
    secondRaf: { ...tuple, observedAt: eventAt + 30 },
  };
}

function commitObservation(
  app: "yune-web" | "my-rime",
  eventAt: number,
  value: string,
  ordinal: number,
): ComparatorStableObservation {
  const tuple = atomicTuple(app, "", value, eventAt + 10, 100);
  return {
    event: {
      ordinal,
      type: "keydown",
      key: " ",
      code: "Space",
      timeStamp: eventAt,
      deliveredAt: eventAt + 1,
      revisionBeforeEvent: 99,
    },
    initial: { ...tuple, observedAt: eventAt + 10 },
    firstRaf: { ...tuple, observedAt: eventAt + 20 },
    secondRaf: { ...tuple, observedAt: eventAt + 30 },
  };
}

function atomicTuple(
  app: "yune-web" | "my-rime",
  composition: string,
  committedValue: string,
  observedAt: number,
  revision = 99,
): ComparatorDomTuple {
  const composing = composition !== "";
  const payload = {
    contractVersion: "web06-comparator-endpoint-v1" as const,
    selectorManifestId: app === "yune-web" ? "yune-web-public-dom-v1" : "my-rime-c73ea17-public-dom-v1",
    revision,
    observedAt,
    composition,
    candidates: composing
      ? candidateTextsForFixture(app, composition).map((text, index) => ({
        label: `${index + 1}.`,
        text,
        comment: "",
      }))
      : [],
    candidateSurfaceCount: composing ? 1 : 0,
    page: composing
      ? { index: 0, buttonCount: 2, previousDisabled: true, nextDisabled: false }
      : { index: null, buttonCount: 0, previousDisabled: null, nextDisabled: null },
    highlightedIndex: composing ? 0 : -1,
    caret: {
      selectorCount: 1,
      value: committedValue,
      selectionStart: committedValue.length,
      selectionEnd: committedValue.length,
      selectionDirection: "none",
      active: true,
      visible: true,
      disabled: false,
    },
    status: {
      schemaId: "luna_pinyin",
      composing,
      surfaceVisible: composing,
      digest: JSON.stringify({ schemaId: "luna_pinyin", composing }),
    },
    digest: "",
  };
  payload.digest = comparatorDomTupleDigest(payload);
  return payload;
}

function candidateTextsForFixture(
  _app: "yune-web" | "my-rime",
  composition: string,
): string[] {
  if (composition === "ni") return ["你", "擬", "尼", "泥", "呢", "妳"];
  if (composition === "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong") {
    const oracleSubset = ["這個引擎其實應該支持超長句子輸入才能用", "這個", "這歌", "這格", "這"];
    return [...oracleSubset, "着"];
  }
  return Array.from({ length: 6 }, (_, index) => `候選${index + 1}`);
}

function fixtureAttestation(): Web06PeerAttestation {
  const identity = fixtureIdentity();
  const host = {
    platform: "darwin",
    release: "fixture",
    architecture: "arm64",
    cpuModel: "fixture",
    logicalCoreCount: 10,
    totalMemoryBytes: 16_000_000_000,
    powerSource: "AC Power" as const,
    lowPowerMode: false as const,
    display: { width: 1470, height: 956, refreshRateHz: 60 as const },
  };
  const envWithoutId = {
    version: "web06-final-run-environment-v1" as const,
    browserMode: "headed-foreground" as const,
    nodeVersion: "v24.18.0",
    playwrightVersion: "1.61.0",
    chromiumVersion: "Chromium fixture",
    chromiumExecutableSha256: "d".repeat(64),
    host,
    viewport: { width: 1365, height: 900 } as const,
    locale: "zh-HK" as const,
    cacheRegime: "fresh-persistent-profile-no-store-v1" as const,
  };
  const environmentId = sha256(canonicalJson(envWithoutId));
  const finalRunEnvironment = { ...envWithoutId, environmentId };
  return {
    version: "web06-phase4-peer-attestation-v1",
    phase: "phase4-peer",
    mode: "binding",
    benchmarkAttempt: true,
    createdAt: "2026-07-21T00:00:00.000Z",
    runnerRoot: "/private/runner",
    outputRoot: "/private/raw-only",
    yuneArchivePath: "/private/yune.tar.gz",
    peerArchivePath: "/private/peer.tar",
    yuneRoot: "/private/yune",
    peerRoot: "/private/peer",
    peerPacketRoot: "/private/packet",
    yune: {
      archiveSha256: "1".repeat(64), completeManifestSha256: "3".repeat(64), treeSha256: "3".repeat(64),
      fileCount: 76, sourceCommit: "1".repeat(40), sourceTree: "4".repeat(40), sourceTreeState: "clean",
      buildInfoSha256: "6".repeat(64), schemaManifestSha256: "2".repeat(64), wasmSha256: "c".repeat(64),
    },
    peer: {
      archiveSha256: "a5eea5ebffa1f62e3f4d058117c1405137fddc02c1046e725ee7b4e7c47420ba",
      completeManifestSha256: "bfe733f1c190898a10c22afc53b237232c4d9b3c594c056d832dfc253dd6e1b6",
      treeSha256: "d0dde476677373f24c5cfd335780a0bac753932d0eb52049950f874f0e37e7b3",
      fileCount: 29, sourceCommit: "2".repeat(40), sourceTree: "5".repeat(40), sourceTreeState: "clean",
      wasmSha256: "c".repeat(64),
    },
    toolchain: {
      runnerSourceCommit: "1".repeat(40), runnerSourceTree: "4".repeat(40), runnerSourceTreeState: "clean",
      nodeVersion: "v24.18.0", npmVersion: "11.6.0", playwrightVersion: "1.61.0",
      playwrightPackageLockSha256: "d".repeat(64), chromiumVersion: "Chromium fixture",
      chromiumExecutableSha256: "d".repeat(64), endpointSourceSha256: "e".repeat(64),
      browserEndpointSourceSha256: "e".repeat(64), laneSourceSha256: "f".repeat(64),
      artifactAttestorSourceSha256: "f".repeat(64), launcherSourceSha256: "f".repeat(64),
      independentLogicSourceSha256: "f".repeat(64),
      specSourceSha256: "0".repeat(64), configSourceSha256: "1".repeat(64),
      verifierSourceSha256: "2".repeat(64),
    },
    host,
    finalRunEnvironment,
    finalRunEnvironmentSha256: sha256(canonicalJson(finalRunEnvironment)),
    identityManifest: identity,
    identityManifestSha256: sha256(canonicalJson(identity)),
    negativeEssayControlSha256: "3".repeat(64),
    configSha256: "b".repeat(64),
    inputRegistrySha256: web06PeerInputRegistrySha256(),
    selectorManifestSha256: "861f078303ca3619ef7bc3f0d2189555da9b87e07a03ba9201b4e98d7fcdb013",
    extraction: { yune: "SAFE_EXTRACTED_AND_FULLY_RECONCILED", peer: "SAFE_EXTRACTED_AND_FULLY_RECONCILED" },
  };
}

function fixtureIdentity(): ComparatorIdentityManifest {
  const side = (name: "yune" | "peer") => ({
    repositoryCommit: (name === "yune" ? "1" : "2").repeat(40),
    upstreamPinnedCommit: name === "yune"
      ? "1".repeat(40)
      : "c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3",
    artifactSourceCommit: (name === "yune" ? "1" : "2").repeat(40),
    artifactSourceTree: (name === "yune" ? "4" : "5").repeat(40),
    sourceTreeState: "clean" as const,
    artifactSha256: "1".repeat(64), generatedManifestSha256: "2".repeat(64), completeArtifactManifestSha256: "3".repeat(64),
    buildCommand: "sealed-no-build-input", packageManager: { name: "npm" as const, version: "1", lockSha256: "4".repeat(64), integrityManifestSha256: "5".repeat(64) },
    toolchain: { nodeVersion: "v1", emscriptenVersion: "1", emscriptenCommit: "6".repeat(40), compilerVersion: "fixture" },
    resolvedRecipes: [{ id: "luna", repository: "https://example.invalid/luna", commit: "7".repeat(40), logicalBytesSha256: "8".repeat(64) }],
    compiledHashes: { table: "9".repeat(64), prism: "a".repeat(64), reverse: "b".repeat(64), "data-model": "none", runtime: "c".repeat(64) },
  });
  return {
    version: "web06-peer-data-v1",
    yune: side("yune"),
    peer: {
      ...side("peer"),
      artifactSha256: "a5eea5ebffa1f62e3f4d058117c1405137fddc02c1046e725ee7b4e7c47420ba",
      completeArtifactManifestSha256: "bfe733f1c190898a10c22afc53b237232c4d9b3c594c056d832dfc253dd6e1b6",
      packageManager: { ...side("peer").packageManager, name: "pnpm" as const },
    },
    logicalInputs: comparatorPeerLogicalInputIds.map((id, index) => id === "grammar-model"
      ? { id, yuneSha256: "none", peerSha256: "none", explicitNone: true }
      : { id, yuneSha256: index === 0 ? "d".repeat(64) : "e".repeat(64), peerSha256: index === 0 ? "f".repeat(64) : "e".repeat(64) }),
    effectiveConfiguration: { yuneSha256: "1".repeat(64), peerSha256: "2".repeat(64) },
    freshEmptyUserdb: true,
    sameEndpointObserver: true,
  };
}

function key(attempt: Pick<Web06PeerAttempt, "rowId" | "app" | "attempt">): string {
  return `${attempt.rowId.replaceAll("/", "--")}--${attempt.app}--attempt-${String(attempt.attempt).padStart(2, "0")}`;
}
