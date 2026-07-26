import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  ACTION_REGISTRY,
  ALL_ACTION_NAMES,
  EVENT_ACTION_RULES,
  SCENARIO_REGISTRY,
  SCENARIO_RUN_REGISTRY,
  SHIFT_TAP_LIVE_OPTION_ACTIONS,
  WEB06_BEHAVIOR_PREDICATES,
  WEB06_BEHAVIOR_PREDICATE_VERSION,
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
  resolveScenarioRun,
  validateEventActionRuleMap,
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

test("scenario registry entry points reject inherited object names", () => {
  for (const value of ["toString", "__proto__", "constructor"]) {
    assert.throws(() => resolveScenarioRun(value), /WEB06_UNKNOWN_SCENARIO_RUN/);
    assert.throws(() => expandScenarioExpectedTimeline(value), /WEB06_UNKNOWN_SCENARIO/);
  }
});

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
    "selection-paging": [4, 3, 7],
    "selection-paging-jyutping": [27, 4, 31],
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
  assert.deepEqual(SCENARIO_REGISTRY["burst-action-map"].owningSchemas, ["jyut6ping3"]);
  assert.deepEqual(
    Object.values(SCENARIO_RUN_REGISTRY).filter((row) => row.scenarioId === "fifo-pressure-barriers"),
    [
      { runId: "fifo-pressure-barriers@jyut6ping3", scenarioId: "fifo-pressure-barriers", schema: "jyut6ping3" },
      { runId: "fifo-pressure-barriers@luna_pinyin", scenarioId: "fifo-pressure-barriers", schema: "luna_pinyin" },
    ],
  );
  assert.deepEqual(
    Object.values(SCENARIO_RUN_REGISTRY).filter((row) => row.scenarioId === "extended-scheduler-barriers").map((row) => row.schema),
    ["jyut6ping3", "luna_pinyin"],
  );
  assert.equal(SCENARIO_REGISTRY.correction.authorityCase.variant, "correction_enabled");
  assert.equal(SCENARIO_REGISTRY.correction.authorityCase.capturedInput, "nri");
  assert.deepEqual(
    SCENARIO_REGISTRY["selection-paging"].authorityCases.map((item) => item.fixtureCase),
    [
      "luna-pinyin-actions.json::select_ni_second",
      "luna-pinyin-actions.json::paging_ni",
    ],
  );
  assert.equal(WEB06_BEHAVIOR_PREDICATE_VERSION, "web06-behavior-predicates-v1");
  assert.ok(Object.isFrozen(WEB06_BEHAVIOR_PREDICATES));
  const covered = new Set(EVENT_ACTION_RULES.flatMap((rule) => rule.actions));
  assert.deepEqual([...ALL_ACTION_NAMES].sort(), [...covered].sort());
  assert.deepEqual(Object.keys(ACTION_REGISTRY).sort(), [...ALL_ACTION_NAMES].sort());
  assert.equal(ACTION_REGISTRY.stageAi.classification, "adapter-only");
  const focusLoss = EVENT_ACTION_RULES.find((rule) => rule.id === "focus-loss-blur");
  assert.deepEqual(focusLoss, {
    id: "focus-loss-blur",
    event: "blur",
    classification: "frontend-consumed",
    condition: "window loses focus",
    actions: [],
    compositionEpochBoundary: true,
    supersessionSubRunBoundary: true,
  });
  const wrongFocusLoss = EVENT_ACTION_RULES.map((rule) => rule.id === "focus-loss-blur"
    ? { ...rule, compositionEpochBoundary: false }
    : rule);
  assert.deepEqual(validateEventActionRuleMap(wrongFocusLoss), {
    ok: false,
    errors: ["FOCUS_LOSS_RULE_INVALID"],
  });
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
  assert.deepEqual(schemaTarget.actions[1].args[0].dictionaryExclude, {
    kind: "web06-redacted:dictionary-exclude",
    count: 0,
    sha256: "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945",
  });
  assert.equal(schemaTarget.actions.length, 15);
  assert.deepEqual(
    schemaTarget.actions.filter((action) => action.kind === "setOption").map((action) => action.args),
    [
      ["soft_cursor", true],
      ["ascii_mode", false],
      ["full_shape", false],
      ["traditionalization", false],
      ["variants_hk", false],
      ["trad_tw", false],
      ["simplification", false],
      ["zh_hans", false],
      ["zh_hant_hk", true],
      ["zh_hant_tw", false],
      ["extended_charset", false],
      ["disabled", false],
    ],
  );
  assert.equal(extended.steps.find((step) => step.id === "extended-deploy-target").publicDemoAvailability, "blocked-hidden-control");
  const errorTarget = extended.steps.find((step) => step.id === "extended-error-target");
  assert.match(errorTarget.actions[0].args[2], /^sha256:[0-9a-f]{64}$/);
  assert.equal(errorTarget.publicDemoAvailability, "blocked-hidden-control");
  assert.deepEqual(
    { event: errorTarget.domEventType, control: errorTarget.control },
    { event: "click", control: "[data-yune-freeform-customize-submit]" },
  );
  const customizeValueRule = EVENT_ACTION_RULES.find((rule) => rule.actions.includes("customizeValue"));
  assert.equal(customizeValueRule.event, "click");
  assert.notEqual(customizeValueRule.event, "submit");

  const deleteRules = EVENT_ACTION_RULES.filter((rule) => rule.actions.includes("deleteCandidate"));
  assert.deepEqual(deleteRules.map(({ event, condition }) => ({ event, condition })), [
    { event: "mousedown", condition: "800ms timer; mouseup or mouseleave cancels; mousemove does not cancel" },
    { event: "touchstart", condition: "800ms timer; touchend or touchcancel cancels; touchmove does not cancel" },
  ]);
  assert.ok(deleteRules.every((rule) => !["pointerdown", "contextmenu"].includes(rule.event)));
});

test("observer counterbalance slots match the independently reviewed five-slot mapping", () => {
  assert.deepEqual(WEB06_OBSERVER_COUNTERBALANCE, {
    1: ["PRODUCT", "BASE_MINIMAL", "BASE_FULL"],
    2: ["BASE_FULL", "BASE_MINIMAL", "PRODUCT"],
    3: ["BASE_MINIMAL", "PRODUCT", "BASE_FULL"],
    4: ["BASE_FULL", "PRODUCT", "BASE_MINIMAL"],
    5: ["PRODUCT", "BASE_FULL", "BASE_MINIMAL"],
  });
});

test("selection, paging, and learned predicates are grounded in external fixture bytes", () => {
  const luna = JSON.parse(readFileSync(new URL(
    "../../../crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
    import.meta.url,
  ), "utf8"));
  const lunaSnapshots = (scenario) => luna.snapshots.filter((row) => row.scenario === scenario);
  const paging = lunaSnapshots("paging_ni");
  const globalNi = paging.slice(0, 2).flatMap((row) => row.selected_candidates.map((candidate) => candidate.text));
  assert.deepEqual(
    WEB06_BEHAVIOR_PREDICATES["selection-paging:paging-ni-2"].expected.candidateTextsExact,
    globalNi.slice(0, 6),
  );
  assert.deepEqual(
    WEB06_BEHAVIOR_PREDICATES["selection-paging:paging-page-down"].expected.candidateTextsPrefix,
    globalNi.slice(6),
  );
  const selected = lunaSnapshots("select_ni_second").find((row) => row.label === "after_select_2");
  assert.equal(
    WEB06_BEHAVIOR_PREDICATES["selection-paging:selection-ni-digit-2"].expected.textareaValue,
    selected.commit_text,
  );

  const jyut = JSON.parse(readFileSync(new URL(
    "../../../crates/yune-core/tests/fixtures/upstream-jyutping/canonical-rime-cantonese/jyutping-m59-being-whole-input.json",
    import.meta.url,
  ), "utf8"));
  const being = jyut.cases.find((row) => row.input === "being");
  const globalBeing = being.pages.flatMap((page) => page.candidates.map((candidate) => candidate.text));
  assert.deepEqual(
    WEB06_BEHAVIOR_PREDICATES["selection-paging-jyutping:jyut-paging-being-5"].expected.candidateTextsExact,
    globalBeing.slice(0, 6),
  );
  assert.deepEqual(
    WEB06_BEHAVIOR_PREDICATES["selection-paging-jyutping:jyut-paging-page-down"].expected.candidateTextsExact,
    globalBeing.slice(6, 12),
  );

  const partial = JSON.parse(readFileSync(new URL(
    "../../../crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json",
    import.meta.url,
  ), "utf8"));
  assert.deepEqual(
    WEB06_BEHAVIOR_PREDICATES["selection-paging-jyutping:jyut-selection-partial-digit-2"].expected.candidateTextsExact,
    partial.captured_next_candidates.slice(0, 6).map((candidate) => candidate.text),
  );
  assert.equal(
    WEB06_BEHAVIOR_PREDICATES["selection-paging-jyutping:jyut-selection-partial-digit-2"].expected.textareaValue,
    partial.selection_request.requested_candidate_text,
  );
  for (const [identity, predicate] of Object.entries(WEB06_BEHAVIOR_PREDICATES)) {
    if (predicate.authorityClass.startsWith("oracle-exact")) {
      assert.doesNotMatch(predicate.fixture, /^apps\/yune-web\/e2e\/results\//, `${identity} must not use Yune browser bytes as its oracle`);
    }
  }
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
  assert.equal(series.preservedHardRedCount, 1);
  assert.throws(
    () => evaluateAttemptSeries(Array.from({ length: 8 }, (_, index) => ({ attemptId: `a${index}`, cadence: "IN_RANGE" }))),
    /CAP_EXCEEDED/,
  );

  assert.throws(() => evaluateAttemptSeries([
    { attemptId: "a1", cadence: "IN_RANGE" },
    { attemptId: "a2", cadence: "IN_RANGE" },
    { attemptId: "a3", cadence: "IN_RANGE" },
    { attemptId: "a4", cadence: "IN_RANGE" },
    { attemptId: "a5", cadence: "TOO_SHORT", behaviorRed: true },
    { attemptId: "a6", cadence: "TOO_LONG", latencyRed: true },
  ]), /MEASURED_ROUND_RETRY_FORBIDDEN/);
  const dimensionSplit = evaluateAttemptSeries([
    { attemptId: "a1", cadence: "IN_RANGE" },
    { attemptId: "a2", cadence: "IN_RANGE" },
    { attemptId: "a3", cadence: "IN_RANGE" },
    { attemptId: "a4", cadence: "TOO_SHORT", behaviorRed: true },
    { attemptId: "a5", cadence: "TOO_SHORT" },
    { attemptId: "a6", setupInvalid: true },
    { attemptId: "a7", cadence: "TOO_LONG" },
  ]);
  assert.equal(dimensionSplit.measuredCount, 4);
  assert.equal(dimensionSplit.preservedHardRedCount, 1);
  assert.deepEqual(dimensionSplit.preservedHardRedAttemptIds, ["a4"]);
  assert.equal(dimensionSplit.status, "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS_WITH_PRESERVED_RED");
  assert.equal(dimensionSplit.retained[3].validForLatencyFrame, false);
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
  assert.throws(
    () => buildClockCalibration({ length: 9 }, post, "driver-page"),
    /SETUP_INVALID_CLOCK_CALIBRATION:exchange-count/,
  );
  const nullExchange = [...pre];
  nullExchange[0] = null;
  assert.throws(
    () => buildClockCalibration(nullExchange, post, "driver-page"),
    /SETUP_INVALID_CLOCK_CALIBRATION:d0/,
  );
  const malformedExchange = [...pre];
  malformedExchange[0] = [];
  assert.throws(
    () => buildClockCalibration(malformedExchange, post, "driver-page"),
    /SETUP_INVALID_CLOCK_CALIBRATION:d0/,
  );
});

function observerTriplet(attemptId, delta = 0) {
  const mode = (samples) => ({
    samples,
    measurementCompleted: true,
    measurementValid: true,
    behaviorRedObserved: false,
    hardRedBindingValid: true,
    hardRedObserved: false,
    commonVerdict: "PASS",
    internalVerdict: "PASS",
    sentinelCallbacksMs: [0.1, 0.1, 0.1],
    sentinelTotalPerEventMs: [0.2, 0.2, 0.2],
    sentinelTotalPerWindowMs: [0.2],
    collectorCallbacksMs: [0.3],
    underlyingLongTasksMs: [],
    instrumentationAddedLongTasksMs: [],
    commonEventCount: 3,
    interactionWindowCount: 1,
    callbackLedgerCount: 4,
    sentinelAccountedCallbackCount: 4,
    callbackLedgerOverflowCount: 0,
    commonEquivalenceDigest: "0".repeat(64),
    internalEquivalenceDigest: "1".repeat(64),
    environmentManifestSha256: "2".repeat(64),
    environmentId: "3".repeat(64),
  });
  const slot = Number(attemptId.replace(/\D/g, "")) + 1;
  return {
    attemptId,
    counterbalanceSlot: slot,
    modeOrder: WEB06_OBSERVER_COUNTERBALANCE[slot],
    freshContextId: `context-${attemptId}`,
    modeContextIds: [`context-${attemptId}-product`, `context-${attemptId}-minimal`, `context-${attemptId}-full`],
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
  assert.ok(red.violations.includes("t2:minimal-sentinel-callback"));
  const noGo = evaluateObserverOverhead([
    ...Array.from({ length: 4 }, (_, index) => observerTriplet(`t${index}`, 0)),
    { ...observerTriplet("bad-1", 0), valid: false },
    { ...observerTriplet("bad-2", 0), valid: false },
    { ...observerTriplet("bad-3", 0), valid: false },
  ]);
  assert.equal(noGo.status, "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS");

  const missing = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  delete missing[0].full.collectorCallbacksMs;
  assert.ok(evaluateObserverOverhead(missing).violations.includes("t0:full-collector-callback-shape"));

  const nonfinite = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  nonfinite[0].product.samples[0] = Number.NaN;
  const nonfiniteResult = evaluateObserverOverhead(nonfinite);
  assert.equal(nonfiniteResult.status, "RED");
  assert.ok(nonfiniteResult.violations.includes("t0:product-samples-shape"));

  const duplicateContext = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  duplicateContext[1].modeContextIds[0] = duplicateContext[0].modeContextIds[0];
  assert.ok(evaluateObserverOverhead(duplicateContext).violations.includes("fresh-mode-context-identities"));

  const badHash = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  badHash[0].minimal.internalEquivalenceDigest = "not-a-hash";
  assert.ok(evaluateObserverOverhead(badHash).violations.includes("t0:internal-equivalence-digest"));
  const environmentIdDrift = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  environmentIdDrift[0].minimal.environmentId = "4".repeat(64);
  assert.ok(evaluateObserverOverhead(environmentIdDrift).violations.includes("t0:environment-id-drift"));

  const matchingRed = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  for (const triplet of matchingRed) {
    for (const mode of [triplet.product, triplet.minimal, triplet.full]) {
      mode.commonVerdict = "RED";
      mode.hardRedObserved = true;
    }
    triplet.minimal.internalVerdict = "RED";
    triplet.full.internalVerdict = "RED";
  }
  const redWorkload = evaluateObserverOverhead(matchingRed);
  assert.equal(redWorkload.pass, true);
  assert.deepEqual(redWorkload.comparisons.map((row) => row.pair), ["product-vs-minimal", "minimal-vs-full"]);

  const differingCause = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  differingCause[0].full.commonEquivalenceDigest = "3".repeat(64);
  assert.ok(evaluateObserverOverhead(differingCause).violations.includes("t0:common-equivalence-digest"));

  const largeWindowAccounting = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  largeWindowAccounting[0].minimal.sentinelTotalPerWindowMs = [25];
  assert.equal(evaluateObserverOverhead(largeWindowAccounting).pass, true);

  const agreedUnderlyingLongTask = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  for (const triplet of agreedUnderlyingLongTask) {
    for (const mode of [triplet.product, triplet.minimal, triplet.full]) {
      mode.underlyingLongTasksMs = [55];
      mode.commonVerdict = "RED";
      mode.hardRedObserved = true;
    }
    triplet.minimal.internalVerdict = "RED";
    triplet.full.internalVerdict = "RED";
  }
  assert.equal(evaluateObserverOverhead(agreedUnderlyingLongTask).pass, true);

  const addedInstrumentationLongTask = Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  addedInstrumentationLongTask[0].minimal.instrumentationAddedLongTasksMs = [50];
  assert.ok(evaluateObserverOverhead(addedInstrumentationLongTask).violations
    .includes("t0:minimal-instrumentation-added-long-task"));

  const partialRed = observerTriplet("partial-1", 0);
  partialRed.valid = false;
  partialRed.product.commonVerdict = "RED";
  partialRed.product.behaviorRedObserved = true;
  partialRed.product.hardRedBindingValid = false;
  partialRed.product.hardRedObserved = true;
  delete partialRed.minimal;
  delete partialRed.full;
  const withPartialRed = evaluateObserverOverhead([
    partialRed,
    ...Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0)),
  ]);
  assert.ok(withPartialRed.violations.includes("partial-1:product-unpaired-valid-red"));
  assert.equal(withPartialRed.status, "RED");
  assert.equal(withPartialRed.pass, false);
  assert.equal(withPartialRed.comparisons.length, 2,
    "five complete triplets remain evaluated after a preserved partial attempt");

  const fiveGreen = () =>
    Array.from({ length: 5 }, (_, index) => observerTriplet(`t${index}`, 0));
  const invalidNumeric = (attemptId, {
    hardRedBindingValid,
    verdict,
    hardRedObserved,
    behaviorRedObserved = false,
  }) => {
    const triplet = observerTriplet(attemptId, 0);
    triplet.valid = false;
    triplet.minimal.measurementValid = false;
    triplet.minimal.sentinelCallbacksMs = [0.5];
    triplet.minimal.behaviorRedObserved = behaviorRedObserved;
    triplet.minimal.hardRedBindingValid = hardRedBindingValid;
    triplet.minimal.hardRedObserved = hardRedObserved;
    triplet.minimal.commonVerdict = verdict;
    triplet.minimal.internalVerdict = verdict;
    return triplet;
  };
  const tooLongNumeric = invalidNumeric("too-long", {
    hardRedBindingValid: true,
    verdict: "RED",
    hardRedObserved: true,
  });
  const tooLongResult = evaluateObserverOverhead([tooLongNumeric, ...fiveGreen()]);
  assert.equal(tooLongResult.status, "RED");
  assert.ok(tooLongResult.violations.includes("too-long:minimal-unpaired-valid-red"));

  for (const [label, attempt] of [
    ["too-short", invalidNumeric("too-short", {
      hardRedBindingValid: false,
      verdict: "NO_VERDICT_INVALID_CADENCE",
      hardRedObserved: false,
    })],
    ["setup-invalid", invalidNumeric("setup-invalid", {
      hardRedBindingValid: false,
      verdict: "SETUP_INVALID",
      hardRedObserved: false,
    })],
    ["callback-invalid", invalidNumeric("callback-invalid", {
      hardRedBindingValid: false,
      verdict: "SETUP_INVALID",
      hardRedObserved: false,
    })],
  ]) {
    const result = evaluateObserverOverhead([attempt, ...fiveGreen()]);
    assert.equal(result.violations.includes(`${attempt.attemptId}:minimal-unpaired-valid-red`), false, label);
  }

  const behaviorWithInvalidBinding = invalidNumeric("behavior-invalid-binding", {
    hardRedBindingValid: false,
    verdict: "RED_BEHAVIOR",
    hardRedObserved: true,
    behaviorRedObserved: true,
  });
  const behaviorResult = evaluateObserverOverhead([behaviorWithInvalidBinding, ...fiveGreen()]);
  assert.equal(behaviorResult.status, "RED");
  assert.ok(behaviorResult.violations
    .includes("behavior-invalid-binding:minimal-unpaired-valid-red"));

  const productOnlyLongTask = fiveGreen();
  productOnlyLongTask[0].product.underlyingLongTasksMs = [50];
  productOnlyLongTask[0].product.commonVerdict = "RED";
  productOnlyLongTask[0].product.hardRedObserved = true;
  productOnlyLongTask[0].product.commonEquivalenceDigest = "3".repeat(64);
  const productOnlyResult = evaluateObserverOverhead(productOnlyLongTask);
  assert.ok(productOnlyResult.violations.includes("t0:common-verdict-disagreement"));
  assert.ok(productOnlyResult.violations.includes("t0:common-equivalence-digest"));
});
