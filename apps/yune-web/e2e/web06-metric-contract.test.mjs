import assert from "node:assert/strict";
import test from "node:test";

import {
  ACTION_REGISTRY,
  ALL_ACTION_NAMES,
  EVENT_ACTION_RULES,
  SCENARIO_REGISTRY,
  SHIFT_TAP_LIVE_OPTION_ACTIONS,
  WEB06_OBSERVER_COUNTERBALANCE,
  WEB06_THRESHOLDS,
  buildClockCalibration,
  browserCodeForKey,
  classifyAttempt,
  computeDriverPageExchange,
  computeMainWorkerExchange,
  distributionSummary,
  evaluateAttemptSeries,
  evaluateObserverOverhead,
  evaluateThresholdDistribution,
  expandScenarioExpectedTimeline,
  interpolateClockCalibration,
  percentileNearestRank,
  validateFrozenContract,
} from "./web06-metric-contract.mjs";

function driverExchange(base, transit = 0.1, service = 0.02) {
  return {
    d0: base,
    m1: base + transit,
    m2: base + transit + service,
    d3: base + transit * 2 + service,
  };
}

function workerExchange(base, transit = 0.1, service = 0.02) {
  return {
    m0: base,
    w1: base + transit,
    w2: base + transit + service,
    m3: base + transit * 2 + service,
  };
}

test("frozen scenario counts, first-key rule, and action coverage are exact", () => {
  assert.deepEqual(validateFrozenContract(), { ok: true, errors: [] });
  const counts = Object.fromEntries(Object.entries(SCENARIO_REGISTRY).map(([id, row]) => [
    id,
    [row.expectedCoveringSamples, row.expectedTerminalSamples, row.expectedActionCount],
  ]));
  assert.deepEqual(counts, {
    "existing-normal-guard": [47, 0, 47],
    "rapid-jyutping": [141, 2, 143],
    "rapid-long-jyutping": [156, 2, 158],
    "rapid-luna": [177, 2, 179],
    "burst-jyutping": [141, 2, 143],
    "burst-luna": [177, 2, 179],
    correction: [5, 3, 8],
    "selection-paging": [10, 3, 13],
    "burst-action-map": [3, 5, 19],
    "fifo-pressure-barriers": [16, 7, 23],
    "learned-row": [11, 1, 13],
    "fair-peer-short": [2, 1, 3],
    "peer-sustained": [59, 1, 60],
    "extended-scheduler-barriers": [5, 10, 40],
  });
  for (const row of Object.values(SCENARIO_REGISTRY)) {
    assert.equal(row.firstKeyIncluded, true);
    assert.equal(row.warmupExcluded, false);
    assert.doesNotMatch(JSON.stringify(row), /TBD/);
  }
  assert.equal(SCENARIO_REGISTRY["burst-jyutping"].expectedCadenceGapCount, 138);
  assert.equal(SCENARIO_REGISTRY["burst-jyutping"].expectedBurstRecoveryCount, 33);
  assert.equal(SCENARIO_REGISTRY["burst-luna"].expectedCadenceGapCount, 174);
  assert.equal(SCENARIO_REGISTRY["burst-luna"].expectedBurstRecoveryCount, 42);
  assert.equal(SCENARIO_REGISTRY["burst-action-map"].expectedCadenceGapCount, 7);
  assert.equal(SCENARIO_REGISTRY["burst-action-map"].expectedBurstRecoveryCount, 1);
  assert.equal(SCENARIO_REGISTRY.correction.authorityCase.variant, "correction_enabled");
  assert.equal(SCENARIO_REGISTRY.correction.authorityCase.capturedInput, "nri");
  assert.deepEqual(
    SCENARIO_REGISTRY["selection-paging"].authorityCases.map((item) => item.fixtureCase),
    [
      "luna-pinyin-actions.json::select_ni_second",
      "luna-pinyin-lattice.json::sentence_lattice_zhongguo",
    ],
  );
  const covered = new Set(EVENT_ACTION_RULES.flatMap((rule) => rule.actions));
  assert.deepEqual([...ALL_ACTION_NAMES].sort(), [...covered].sort());
  assert.deepEqual(Object.keys(ACTION_REGISTRY).sort(), [...ALL_ACTION_NAMES].sort());
  assert.equal(ACTION_REGISTRY.stageAi.classification, "adapter-only");
  const peerTimeline = expandScenarioExpectedTimeline("fair-peer-short");
  assert.deepEqual(peerTimeline.actions.map((item) => item.args), [["{n}"], ["{i}"], ["{space}"]]);
  assert.deepEqual(
    peerTimeline.events.filter((item) => item.stepId === "peer-short-commit").map(({ type, key, code }) => ({ type, key, code })),
    [
      { type: "keydown", key: " ", code: "Space" },
      { type: "keyup", key: " ", code: "Space" },
    ],
  );
  const pagingTimeline = expandScenarioExpectedTimeline("selection-paging");
  assert.ok(pagingTimeline.actions.some((item) => item.args[0] === "{Page_Down}"));
  assert.ok(pagingTimeline.actions.some((item) => item.args[0] === "{Page_Up}"));
  assert.deepEqual(expandScenarioExpectedTimeline("peer-sustained").actions.at(-1).args, ["{space}"]);

  const fifo = SCENARIO_REGISTRY["fifo-pressure-barriers"];
  assert.deepEqual(
    fifo.steps.filter((step) => step.id.endsWith("-reset")).map((step) => step.id),
    ["fifo-commit-reset", "fifo-select-reset", "fifo-page-reset"],
  );
  const fifoImport = fifo.steps.find((step) => step.id === "fifo-userdb-import");
  assert.equal(fifoImport.domEventType, "change");
  assert.match(fifoImport.actions[0].args[0], /^sha256:[0-9a-f]{64}$/);

  const extended = SCENARIO_REGISTRY["extended-scheduler-barriers"];
  const optionTarget = extended.steps.find((step) => step.id === "extended-option-target");
  assert.equal(optionTarget.domEventType, "change");
  assert.equal(optionTarget.actions.length, 12);
  assert.deepEqual(optionTarget.actions.at(-2).args, ["extended_charset", true]);
  const schemaTarget = extended.steps.find((step) => step.id === "extended-schema-target");
  assert.deepEqual(schemaTarget.actions.slice(0, 4).map((action) => action.kind), ["selectSchema", "customize", "setOption", "deploy"]);
  assert.deepEqual(schemaTarget.actions[2].args, ["soft_cursor", true]);
  assert.equal(schemaTarget.actions.length, 15);
  assert.equal(extended.steps.find((step) => step.id === "extended-deploy-target").publicDemoAvailability, "blocked-hidden-control");
  assert.equal(extended.steps.find((step) => step.id === "extended-error-target").publicDemoAvailability, "blocked-hidden-control");

  const deleteRule = EVENT_ACTION_RULES.find((rule) => rule.actions.includes("deleteCandidate"));
  assert.deepEqual(
    { event: deleteRule.event, condition: deleteRule.condition },
    { event: "pointerdown", condition: "800ms hold without move/cancel" },
  );
});

test("event expansion preserves exact zero/one/many mappings and Shift fan-out", () => {
  const timeline = expandScenarioExpectedTimeline("burst-action-map");
  assert.equal(timeline.events.length, 16);
  assert.equal(timeline.actions.length, 19);
  const shiftUp = timeline.events.find((event) => event.stepId === "action-map-shift-tap" && event.type === "keyup");
  const shiftDown = timeline.events.find((event) => event.stepId === "action-map-shift-tap" && event.type === "keydown");
  assert.deepEqual(
    { classification: shiftDown.classification, reason: shiftDown.reason },
    { classification: "frontend-consumed", reason: "ascii-mode-shift-keydown" },
  );
  assert.deepEqual(shiftDown.mappedActionIds, []);
  assert.equal(shiftUp.classification, "mapped-action(s)");
  assert.equal(shiftUp.reason, "ascii-mode-shift-tap");
  assert.equal(shiftUp.mappedActionIds.length, 12);
  assert.deepEqual(
    timeline.actions.filter((action) => action.eventSequenceId === shiftUp.eventSequenceId).map((action) => action.kind),
    Array(12).fill("setOption"),
  );
  assert.deepEqual(SHIFT_TAP_LIVE_OPTION_ACTIONS, [
    ["soft_cursor", true],
    ["ascii_mode", true],
    ["full_shape", false],
    ["traditionalization", false],
    ["variants_hk", true],
    ["trad_tw", false],
    ["simplification", false],
    ["zh_hans", false],
    ["zh_hant_hk", false],
    ["zh_hant_tw", false],
    ["extended_charset", false],
    ["disabled", false],
  ]);
});

test("browser key/code identity is frozen for letters, digits, Space, and specials", () => {
  assert.equal(browserCodeForKey("n"), "KeyN");
  assert.equal(browserCodeForKey("2"), "Digit2");
  assert.equal(browserCodeForKey(" "), "Space");
  assert.equal(browserCodeForKey("Backspace"), "Backspace");
  assert.equal(browserCodeForKey("PageDown"), "PageDown");
  assert.throws(() => browserCodeForKey("UnreviewedKey"), /UNFROZEN_BROWSER_KEY_CODE/);

  const selection = expandScenarioExpectedTimeline("selection-paging");
  const digit = selection.events.find((event) => event.stepId === "selection-ni-digit-2" && event.type === "keydown");
  assert.deepEqual(
    { key: digit.key, code: digit.code, classification: digit.classification, reason: digit.reason },
    { key: "2", code: "Digit2", classification: "mapped-action(s)", reason: "composition-digit-selection" },
  );
  const correction = expandScenarioExpectedTimeline("correction");
  const space = correction.events.find((event) => event.stepId === "correction-commit" && event.type === "keydown");
  assert.deepEqual({ key: space.key, code: space.code }, { key: " ", code: "Space" });
});

test("nearest-rank arithmetic uses ceil((n - 1) * p) without interpolation", () => {
  const twenty = Array.from({ length: 20 }, (_, index) => index + 1);
  assert.equal(percentileNearestRank(twenty, 0.95), 20);
  const oneFortyOne = Array.from({ length: 141 }, (_, index) => index + 1);
  assert.equal(percentileNearestRank(oneFortyOne, 0.99), 140);
  assert.deepEqual(distributionSummary([3, 1, 2]), {
    count: 3,
    median: 2,
    p95: 3,
    p99: 3,
    max: 3,
  });
  assert.throws(() => percentileNearestRank([1, Number.NaN], 0.95), /NONFINITE/);
  assert.throws(() => percentileNearestRank([], 0.95), /EMPTY/);
});

test("threshold arithmetic is unrounded and every value above 67 ms is RED", () => {
  const green = evaluateThresholdDistribution(
    Array(140).fill(50).concat(67),
    WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs,
    "covering",
  );
  assert.equal(green.pass, true);
  const red = evaluateThresholdDistribution(
    Array(140).fill(50).concat(67.000001),
    WEB06_THRESHOLDS.sustained.eventToCoveringPaintMs,
    "covering",
  );
  assert.equal(red.pass, false);
  assert.ok(red.violations.some((violation) => violation.includes("max")));
});

test("attempt precedence preserves measured reds and permits only setup/cadence retries", () => {
  assert.equal(classifyAttempt({ cadence: "TOO_SHORT", latencyRed: true }), "NO_VERDICT_INVALID_CADENCE");
  assert.equal(classifyAttempt({ cadence: "TOO_SHORT", behaviorRed: true }), "RED_BEHAVIOR");
  assert.equal(classifyAttempt({ cadence: "TOO_LONG", latencyRed: true }), "RED");
  assert.equal(classifyAttempt({ cadence: "TOO_LONG" }), "NO_VERDICT_INVALID_CADENCE");
  assert.equal(classifyAttempt({ cadence: "IN_RANGE", frameRed: true }), "RED");

  const series = evaluateAttemptSeries([
    { attemptId: "a1", cadence: "IN_RANGE" },
    { attemptId: "a2", cadence: "TOO_SHORT" },
    { attemptId: "a3", cadence: "IN_RANGE", latencyRed: true },
    { attemptId: "a4", cadence: "IN_RANGE" },
    { attemptId: "a5", setupInvalid: true },
    { attemptId: "a6", cadence: "IN_RANGE" },
    { attemptId: "a7", cadence: "IN_RANGE" },
  ]);
  assert.equal(series.status, "COMPLETE_WITH_RED");
  assert.equal(series.measuredCount, 5);
  assert.equal(series.replaceableCount, 2);
  assert.throws(
    () => evaluateAttemptSeries(Array.from({ length: 8 }, (_, index) => ({ attemptId: `a${index}`, cadence: "IN_RANGE" }))),
    /CAP_EXCEEDED/,
  );
});

test("driver/page and main/worker calibration retain uncertainty and fail closed", () => {
  const driver = computeDriverPageExchange(driverExchange(0));
  const worker = computeMainWorkerExchange(workerExchange(0));
  assert.ok(Math.abs(driver.offset) < 1e-12);
  assert.ok(Math.abs(worker.offset) < 1e-12);
  assert.ok(Math.abs(driver.uncertainty - 0.1) < 1e-12);
  assert.ok(Math.abs(worker.uncertainty - 0.1) < 1e-12);

  const pre = Array.from({ length: 9 }, (_, index) => driverExchange(index, 0.1 + index * 0.01));
  const post = Array.from({ length: 9 }, (_, index) => driverExchange(1000 + index, 0.1 + index * 0.01));
  const calibration = buildClockCalibration(pre, post, "driver-page");
  const point = interpolateClockCalibration(calibration, 500);
  assert.ok(point.uncertainty >= 0 && point.uncertainty <= 2);
  assert.throws(() => interpolateClockCalibration(calibration, -1), /outside-boundary/);
  assert.throws(
    () => computeDriverPageExchange({ d0: 0, m1: 1, m2: 2, d3: 0.5 }),
    /driver-net-rtt/,
  );
  assert.throws(
    () => buildClockCalibration(pre.slice(1), post, "driver-page"),
    /exchange-count/,
  );
});

function observerTriplet(attemptId, delta = 0) {
  const mode = (samples) => ({
    samples,
    commonVerdict: "PASS",
    internalVerdict: "PASS",
    sentinelCallbacksMs: [0.1],
    sentinelTotalPerEventMs: [0.2],
    collectorCallbacksMs: [0.3],
    instrumentationLongTasksMs: [],
    commonCountDigest: "sha256:common-counts",
    internalCountDigest: "sha256:internal-counts",
  });
  const slot = Number(attemptId.replace(/\D/g, "")) + 1;
  return {
    attemptId,
    counterbalanceSlot: slot,
    modeOrder: WEB06_OBSERVER_COUNTERBALANCE[slot],
    freshContextId: `context-${attemptId}`,
    modeFixedBeforePageLoad: true,
    valid: true,
    product: mode([10, 20, 30]),
    minimal: mode([10 + delta, 20 + delta, 30 + delta]),
    full: mode([10 + delta, 20 + delta, 30 + delta]),
  };
}

test("observer overhead requires five counterbalanced triplets and strict callbacks", () => {
  const green = evaluateObserverOverhead(Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 1)));
  assert.equal(green.pass, true);
  const callbackRed = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  callbackRed[2].minimal.sentinelCallbacksMs = [0.5];
  const red = evaluateObserverOverhead(callbackRed);
  assert.equal(red.pass, false);
  assert.ok(red.violations.includes("t2:sentinel-callback"));
  const noGo = evaluateObserverOverhead([
    ...Array.from({ length: 4 }, (_, index) => observerTriplet(`t${index}`, 0)),
    { ...observerTriplet("bad-1", 0), valid: false },
    { ...observerTriplet("bad-2", 0), valid: false },
    { ...observerTriplet("bad-3", 0), valid: false },
  ]);
  assert.equal(noGo.status, "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS");
});
