/**
 * WEB-06 frozen Phase-0 measurement contract.
 *
 * This module is intentionally independent of Playwright and the application.
 * Browser collectors emit raw receipts; this module owns the immutable
 * scenario/action registry, exact arithmetic, and fail-closed verdict rules.
 */

export const WEB06_METRIC_CONTRACT_VERSION = "web06-metric-v1";
export const WEB06_SCENARIO_REGISTRY_VERSION = "web06-scenarios-v1";
export const WEB06_BEHAVIOR_PREDICATE_VERSION = "web06-behavior-predicates-v1";
export const WEB06_SUMMARY_SEMANTIC_PROJECTION_VERSION =
  "web06-summary-semantic-projection-v1";
export const WEB06_IMPLEMENTATION_DIAGNOSTIC_BINDING_VERSION =
  "web06-implementation-diagnostic-binding-v1";

export const WEB06_PRODUCT_IDENTITY = deepFreeze({
  sourceCommit: "4e369d7109ceb97ee49cf04e8ef2caf734d8488c",
  sourceTree: "cdcba9a4997ddbe87c57dda377fc1a2c2c273468",
  archiveSha256: "b80ae71db7475454d340c47177e351c3a7f99da262c90c430fe4c638971bfa43",
  artifactManifestSha256: "13015adfb46a411520e512d8488e0dda3d0852246e5d1e095347fc322f056c49",
});

export const WEB06_SELECTED_BRANCHES = deepFreeze(["NONE", "A", "B", "C"]);
export const WEB06_DISPOSITIONS = deepFreeze([
  "DIAGNOSTIC",
  "SOURCE_CURRENT_BASELINE",
  "PRODUCTION_REDUCTION",
  "MEASURED_NO_GO",
]);

const SHA256_RE = /^[0-9a-f]{64}$/;

function deepFreeze(value) {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) deepFreeze(child);
    Object.freeze(value);
  }
  return value;
}

function hasOwn(record, key) {
  return typeof key === "string" && Object.prototype.hasOwnProperty.call(record, key);
}

export const WEB06_THRESHOLDS = deepFreeze({
  attempts: { requiredValid: 5, maximum: 7, playwrightRetries: 0 },
  sustained: {
    eventToCoveringPaintMs: { p95: 50, p99: 67, max: 67 },
    driverDispatchToCoveringPaintUpperBoundMs: { max: 67 },
    preServiceWaitUpperBoundMs: { p95: 10, max: 30 },
    supersessionSequenceLag: { max: 2 },
  },
  terminal: {
    eventToTerminalObservationMs: { max: 67 },
    driverDispatchToTerminalUpperBoundMs: { max: 67 },
    persistenceStressCompletionMs: { max: 250 },
  },
  frame: {
    requiredIdleIntervals: 120,
    idleMedianMs: { min: 15, max: 18 },
    p99Ms: { max: 35.4 },
    rejectIntervalAtOrAboveMs: 50,
    rejectLongTaskAtOrAboveMs: 50,
  },
  cadence: {
    sustained60: { requestedMs: 60, minMs: 48, maxMs: 75 },
    burst40: { requestedMs: 40, minMs: 32, maxMs: 50 },
    burst120: { requestedMs: 120, minMs: 96, maxMs: 150 },
    existing100: { requestedMs: 100, minMs: 80, maxMs: 125 },
  },
  calibration: {
    exchangesPerBoundary: 9,
    uncertaintyMinMs: 0,
    uncertaintyMaxMs: 2,
    offsetDriftMaxMs: 2,
  },
  observer: {
    requiredTriplets: 5,
    maximumTripletAttempts: 7,
    absolutePooledMedianDeltaMs: 1,
    absolutePooledP95DeltaMs: 2,
    absolutePooledMaxDeltaMs: 4,
    sentinelCallbackExclusiveMaxMs: 0.5,
    sentinelTotalPerEventExclusiveMaxMs: 1,
    collectorCallbackExclusiveMaxMs: 5,
    rejectInstrumentationLongTaskAtOrAboveMs: 50,
  },
  metric: { timelineResidualAbsoluteMaxMs: 0.1 },
  peer: { matchesOrBeatsP95RatioMax: 1 },
});

export const WEB06_OBSERVER_COUNTERBALANCE = deepFreeze({
  1: ["PRODUCT", "BASE_MINIMAL", "BASE_FULL"],
  2: ["BASE_FULL", "BASE_MINIMAL", "PRODUCT"],
  3: ["BASE_MINIMAL", "PRODUCT", "BASE_FULL"],
  4: ["BASE_FULL", "PRODUCT", "BASE_MINIMAL"],
  5: ["PRODUCT", "BASE_FULL", "BASE_MINIMAL"],
});

export const ACTION_REGISTRY = deepFreeze({
  setOption: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "lifecycle" },
  selectSchema: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "lifecycle" },
  getUserdbSnapshot: { classification: "read-only", supersedable: false, terminalStrategy: "lifecycle" },
  importUserdb: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "lifecycle" },
  processKey: { classification: "native-key", supersedable: "printable-only", terminalStrategy: "presentation" },
  stageAi: { classification: "adapter-only", supersedable: false, defaultOff: true, terminalStrategy: "presentation" },
  selectCandidate: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "presentation" },
  deleteCandidate: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "presentation" },
  flipPage: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "presentation" },
  customize: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "lifecycle" },
  customizeValue: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "lifecycle" },
  deploy: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "lifecycle" },
  deployCacheSnapshot: { classification: "read-only", supersedable: false, terminalStrategy: "lifecycle" },
  invalidateDeployCache: { classification: "stateful-barrier", supersedable: false, terminalStrategy: "lifecycle" },
  injectedAssetsManifest: { classification: "read-only", supersedable: false, terminalStrategy: "lifecycle" },
});

export const ALL_ACTION_NAMES = deepFreeze(Object.keys(ACTION_REGISTRY));

export const SHIFT_TAP_LIVE_OPTION_ACTIONS = deepFreeze([
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

function action(kind, args = [], extra = {}) {
  if (!hasOwn(ACTION_REGISTRY, kind)) throw new Error(`WEB06_UNKNOWN_ACTION:${kind}`);
  const contract = ACTION_REGISTRY[kind];
  return {
    kind,
    args,
    classification: contract.classification,
    supersedable: extra.supersedable ?? false,
    ...extra,
  };
}

export function browserCodeForKey(key) {
  if (/^[A-Za-z]$/.test(key)) return `Key${key.toUpperCase()}`;
  if (/^[0-9]$/.test(key)) return `Digit${key}`;
  if (key === " ") return "Space";
  if (["Backspace", "Delete", "Escape", "PageDown", "PageUp", "ArrowDown", "ArrowUp", "Enter", "Tab"].includes(key)) {
    return key;
  }
  throw new Error(`WEB06_UNFROZEN_BROWSER_KEY_CODE:${JSON.stringify(key)}`);
}

function keyboardStep(id, key, {
  code = browserCodeForKey(key),
  actions,
  sample = "none",
  cadence = "sustained60",
  subcase,
  nominalGapMs,
  expectedLogicalInputAfter,
} = {}) {
  return {
    id,
    source: "keyboard",
    domEventCount: 2,
    key,
    code,
    cadence,
    nominalGapMs,
    sample,
    subcase,
    ...(expectedLogicalInputAfter === undefined ? {} : { expectedLogicalInputAfter }),
    actions: actions ?? [],
  };
}

function printableSteps(prefix, text, cadence = "sustained60", subcase) {
  const nominal = cadence === "sustained60" ? 60 : cadence === "existing100" ? 100 : undefined;
  return [...text].map((key, index) => keyboardStep(`${prefix}-${index + 1}`, key, {
    cadence,
    subcase,
    nominalGapMs: index === 0 ? undefined : nominal,
    sample: "covering",
    actions: [action("processKey", [`{${key}}`], {
      supersedable: true,
      inputClass: "printable-insertion",
    })],
  }));
}

function specialKeyStep(id, key, rimeKey, {
  cadence = "sustained60",
  sample = "terminal",
  subcase,
  stressDeadline = false,
} = {}) {
  return keyboardStep(id, key, {
    cadence,
    subcase,
    sample,
    actions: [action("processKey", [`{${rimeKey}}`], {
      inputClass: stressDeadline ? "persistence-stress" : "interactive-barrier",
    })],
  });
}

function digitSelectStep(id, digit = "1", {
  cadence = "sustained60",
  subcase,
  includeUserdbRefresh = false,
  expectedLogicalInputAfter = "",
} = {}) {
  const index = digit === "0" ? 9 : Number(digit) - 1;
  const actions = [action("selectCandidate", [index], { inputClass: "digit-selection" })];
  if (includeUserdbRefresh) {
    actions.push(action("getUserdbSnapshot", [], {
      inputClass: "commit-triggered-userdb-refresh",
      background: true,
      originReason: "commit-triggered-userdb-refresh",
    }));
  }
  return keyboardStep(id, digit, {
    cadence,
    subcase,
    sample: "terminal",
    expectedLogicalInputAfter,
    actions,
  });
}

function directActionsStep(id, actions, {
  sample = "terminal",
  subcase,
  inputClass = "control-barrier",
  stressDeadline = false,
  domEventType,
  control,
  publicDemoAvailability = "available",
  expectedOutcome,
} = {}) {
  if (!["change", "click", "submit"].includes(domEventType)) {
    throw new Error(`WEB06_CONTROL_DOM_EVENT_REQUIRED:${id}`);
  }
  return {
    id,
    source: "control",
    domEventType,
    control,
    publicDemoAvailability,
    domEventCount: 1,
    cadence: "same-task-pressure",
    sample,
    subcase,
    actions: actions.map((item) => action(item.kind, item.args, {
      inputClass: item.inputClass ?? inputClass,
      stressDeadline: item.stressDeadline ?? stressDeadline,
      background: item.background,
      expectedOutcome: item.expectedOutcome ?? expectedOutcome,
    })),
  };
}


function directActionStep(id, kind, args, options) {
  return directActionsStep(id, [{ kind, args }], options);
}

function repeatPhrase(prefix, text, cadence) {
  const steps = [];
  for (let repetition = 1; repetition <= 3; repetition += 1) {
    steps.push(...printableSteps(`${prefix}-r${repetition}`, text, cadence, `repetition-${repetition}`));
    if (repetition < 3) {
      steps.push(specialKeyStep(`${prefix}-reset-${repetition}`, "Escape", "Escape", {
        cadence: "after-exact-final-paint",
        subcase: `repetition-${repetition}`,
      }));
    }
  }
  return steps;
}

function burstPhrase(prefix, text) {
  const steps = repeatPhrase(prefix, text, "burst-cycle");
  const previousPrintableByRepetition = new Map();
  const printableIndexByRepetition = new Map();
  for (const step of steps) {
    if (step.sample !== "covering") continue;
    const printableIndex = printableIndexByRepetition.get(step.subcase) ?? 0;
    const cycle = [40, 40, 40, 120];
    step.nominalGapMs = printableIndex === 0 ? undefined : cycle[(printableIndex - 1) % cycle.length];
    const previous = previousPrintableByRepetition.get(step.subcase);
    if (step.nominalGapMs === 120 && previous) previous.declaredBurstPauseAfter = true;
    previousPrintableByRepetition.set(step.subcase, step);
    printableIndexByRepetition.set(step.subcase, printableIndex + 1);
  }
  for (const previous of previousPrintableByRepetition.values()) previous.declaredScenarioSegmentEnd = true;
  return steps;
}

function scenario(id, definition) {
  const steps = definition.steps;
  return deepFreeze({
    id,
    ...definition,
    firstKeyIncluded: true,
    warmupExcluded: false,
    expectedDomEventCount: steps.reduce((sum, step) => sum + step.domEventCount, 0),
    expectedActionCount: steps.reduce((sum, step) => sum + step.actions.length, 0),
    expectedCoveringSamples: steps.filter((step) => step.sample === "covering").length,
    expectedTerminalSamples: steps.filter((step) => step.sample === "terminal").length,
    expectedCadenceGapCount: steps.filter((step) => Number.isFinite(step.nominalGapMs)).length,
    expectedBurstRecoveryCount: steps.filter((step) => step.declaredBurstPauseAfter === true).length,
    expectedInteractionWindowCount: definition.expectedInteractionWindowCount ?? 1,
  });
}

const PRIMARY_JYUTPING = "ngodeigungsijigaahaidoumaaigangeihaaijansougeoi";
const LONG_JYUTPING = "taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng";
const RAPID_LUNA = "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong";
export const EMPTY_USERDB_FIXTURE_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
export const EMPTY_DICTIONARY_EXCLUDE_SHA256 = "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945";
export const INJECTED_ERROR_VALUE_SHA256 = "cb609f7bc692db375e04888748e12cb673fe23c2a6d2c4e4249df2fc45370ace";

const CORRECTION_STEPS = [
  ...printableSteps("correction-pre", "nri", "sustained60", "pre-delete"),
  specialKeyStep("correction-backspace-1", "Backspace", "BackSpace", { subcase: "delete-1" }),
  specialKeyStep("correction-backspace-2", "Backspace", "BackSpace", { subcase: "delete-2" }),
  ...printableSteps("correction-resume", "ri", "sustained60", "resumed-input"),
  specialKeyStep("correction-commit", " ", "space", { cadence: "after-exact-final-paint", subcase: "commit" }),
];

const SELECTION_PAGING_STEPS = [
  ...printableSteps("selection-ni", "ni", "sustained60", "luna-digit-select"),
  digitSelectStep("selection-ni-digit-2", "2", { cadence: "after-exact-final-paint", subcase: "luna-digit-select" }),
  ...printableSteps("paging-ni", "ni", "sustained60", "luna-page-roundtrip"),
  specialKeyStep("paging-page-down", "PageDown", "Page_Down", { cadence: "after-exact-final-paint", subcase: "luna-page-roundtrip" }),
  specialKeyStep("paging-page-up", "PageUp", "Page_Up", { subcase: "luna-page-roundtrip" }),
];

const JYUTPING_SELECTION_PAGING_STEPS = [
  ...printableSteps(
    "jyut-selection-partial",
    "caksijathaacoenggeoizi",
    "sustained60",
    "jyutping-partial-digit-select",
  ),
  digitSelectStep("jyut-selection-partial-digit-2", "2", {
    cadence: "after-exact-final-paint",
    subcase: "jyutping-partial-digit-select",
    expectedLogicalInputAfter: "sijathaacoenggeoizi",
  }),
  specialKeyStep("jyut-selection-reset", "Escape", "Escape", {
    cadence: "after-exact-final-paint",
    subcase: "jyutping-partial-digit-select",
  }),
  ...printableSteps("jyut-paging-being", "being", "sustained60", "jyutping-page-roundtrip"),
  specialKeyStep("jyut-paging-page-down", "PageDown", "Page_Down", { cadence: "after-exact-final-paint", subcase: "jyutping-page-roundtrip" }),
  specialKeyStep("jyut-paging-page-up", "PageUp", "Page_Up", { subcase: "jyutping-page-roundtrip" }),
];

const SHIFT_TAP_ACTIONS = SHIFT_TAP_LIVE_OPTION_ACTIONS.map(([option, value]) =>
  action("setOption", [option, value], { inputClass: "modifier-release-option-barrier" }));

const BURST_ACTION_MAP_STEPS = [
  ...printableSteps("action-map-prefix", "ni", "burst-cycle", "mixed-actions"),
  specialKeyStep("action-map-backspace", "Backspace", "BackSpace", { cadence: "burst-cycle", subcase: "mixed-actions" }),
  ...printableSteps("action-map-resume", "i", "burst-cycle", "mixed-actions"),
  specialKeyStep("action-map-page-down", "PageDown", "Page_Down", { cadence: "burst-cycle", subcase: "mixed-actions" }),
  specialKeyStep("action-map-page-up", "PageUp", "Page_Up", { cadence: "burst-cycle", subcase: "mixed-actions" }),
  keyboardStep("action-map-shift-tap", "Shift", {
    code: "ShiftLeft",
    cadence: "burst-cycle",
    subcase: "mixed-actions",
    sample: "terminal",
    actions: SHIFT_TAP_ACTIONS,
  }),
  specialKeyStep("action-map-escape", "Escape", "Escape", { cadence: "burst-cycle", subcase: "mixed-actions" }),
];
let previousBurstActionStep;
let burstActionIndex = 0;
for (const step of BURST_ACTION_MAP_STEPS) {
  if (step.cadence !== "burst-cycle") continue;
  step.nominalGapMs = burstActionIndex === 0 ? undefined : [40, 40, 40, 120][(burstActionIndex - 1) % 4];
  if (step.nominalGapMs === 120 && previousBurstActionStep) previousBurstActionStep.declaredBurstPauseAfter = true;
  previousBurstActionStep = step;
  burstActionIndex += 1;
}
if (previousBurstActionStep) previousBurstActionStep.declaredScenarioSegmentEnd = true;

const FIFO_PRESSURE_STEPS = [
  ...printableSteps("fifo-commit-prefix", "ni", "same-task-pressure", "commit-then-type"),
  specialKeyStep("fifo-commit", " ", "space", { cadence: "same-task-pressure", subcase: "commit-then-type" }),
  ...printableSteps("fifo-commit-later", "h", "same-task-pressure", "commit-then-type"),
  specialKeyStep("fifo-commit-reset", "Escape", "Escape", { cadence: "after-exact-final-paint", subcase: "commit-then-type" }),
  ...printableSteps("fifo-select-prefix", "ni", "same-task-pressure", "digit-select-then-type"),
  digitSelectStep("fifo-select", "1", { cadence: "same-task-pressure", subcase: "digit-select-then-type" }),
  ...printableSteps("fifo-select-later", "h", "same-task-pressure", "digit-select-then-type"),
  specialKeyStep("fifo-select-reset", "Escape", "Escape", { cadence: "after-exact-final-paint", subcase: "digit-select-then-type" }),
  ...printableSteps("fifo-page-prefix", "zhongguo", "same-task-pressure", "page-then-type"),
  specialKeyStep("fifo-page", "PageDown", "Page_Down", { cadence: "same-task-pressure", subcase: "page-then-type" }),
  ...printableSteps("fifo-page-later", "h", "same-task-pressure", "page-then-type"),
  specialKeyStep("fifo-page-reset", "Escape", "Escape", { cadence: "after-exact-final-paint", subcase: "page-then-type" }),
  directActionStep("fifo-userdb-import", "importUserdb", [`sha256:${EMPTY_USERDB_FIXTURE_SHA256}`], {
    subcase: "persistence-then-type",
    inputClass: "persistence-stress",
    stressDeadline: true,
    domEventType: "change",
    control: "[data-yune-userdb-import-input]",
  }),
  ...printableSteps("fifo-userdb-later", "n", "same-task-pressure", "persistence-then-type"),
];

const LEARNED_STEPS = [
  ...printableSteps("learned-training", "ngohaigo", "sustained60", "learn-and-persist"),
  digitSelectStep("learned-commit", "1", {
    cadence: "after-exact-final-paint",
    subcase: "learn-and-persist",
    includeUserdbRefresh: true,
  }),
  {
    id: "learned-reload-boundary",
    source: "browser-lifecycle",
    domEventCount: 1,
    cadence: "after-persistence-complete",
    sample: "none",
    subcase: "reload",
    actions: [],
  },
  ...printableSteps("learned-probe", "ngo", "sustained60", "post-reload-rapid-probe"),
];

const DEFAULT_DEPLOY_PREFERENCES = deepFreeze({
  pageSize: 6,
  enableCompletion: true,
  enableCorrection: false,
  enableSentence: true,
  enableLearning: true,
  combineCandidates: true,
  predictionNeverFirst: true,
  predictionThreshold: 0,
  dictionaryExclude: {
    kind: "web06-redacted:dictionary-exclude",
    count: 0,
    sha256: EMPTY_DICTIONARY_EXCLUDE_SHA256,
  },
  isCangjie5: true,
});

function liveOptionDescriptors({
  schema = "jyut6ping3",
  asciiMode = false,
  extendedCharset = false,
} = {}) {
  const isLunaPinyin = schema === "luna_pinyin";
  return SHIFT_TAP_LIVE_OPTION_ACTIONS.map(([option, defaultValue]) => ({
    kind: "setOption",
    args: [
      option,
      option === "ascii_mode" ? asciiMode
        : option === "extended_charset" ? extendedCharset
          : option === "variants_hk" ? !isLunaPinyin
            : option === "zh_hant_hk" ? isLunaPinyin
          : defaultValue,
    ],
    inputClass: "live-option-fanout-barrier",
  }));
}

const EXTENDED_BARRIERS = [
  ...printableSteps("extended-option-earlier", "n", "same-task-pressure", "option-barrier"),
  directActionsStep("extended-option-target", liveOptionDescriptors({ extendedCharset: true }), {
    subcase: "option-barrier",
    domEventType: "change",
    control: "[data-yune-section='live'] input[type='checkbox']",
  }),
  specialKeyStep("extended-option-reset", "Escape", "Escape", { cadence: "after-exact-final-paint", subcase: "option-barrier" }),

  ...printableSteps("extended-schema-earlier", "n", "same-task-pressure", "schema-barrier"),
  directActionsStep("extended-schema-target", (() => {
    const liveOptions = liveOptionDescriptors({ schema: "luna_pinyin" });
    return [
      { kind: "selectSchema", args: ["luna_pinyin"], inputClass: "schema-change-barrier" },
      { kind: "customize", args: [DEFAULT_DEPLOY_PREFERENCES], inputClass: "schema-deploy-preference-barrier" },
      liveOptions[0],
      { kind: "deploy", args: [], inputClass: "schema-deploy-barrier" },
      ...liveOptions.slice(1),
    ];
  })(), {
    subcase: "schema-barrier",
    domEventType: "change",
    control: "[data-yune-schema-switcher] select",
  }),
  specialKeyStep("extended-schema-reset", "Escape", "Escape", { cadence: "after-exact-final-paint", subcase: "schema-barrier" }),

  ...printableSteps("extended-deploy-earlier", "n", "same-task-pressure", "deploy-barrier"),
  directActionStep("extended-deploy-target", "deploy", [], {
    subcase: "deploy-barrier",
    domEventType: "click",
    control: "[data-yune-control-redeploy]",
    publicDemoAvailability: "blocked-hidden-control",
  }),
  specialKeyStep("extended-deploy-reset", "Escape", "Escape", { cadence: "after-exact-final-paint", subcase: "deploy-barrier" }),

  ...printableSteps("extended-persistence-earlier", "n", "same-task-pressure", "persistence-barrier"),
  directActionStep("extended-persistence-target", "importUserdb", [`sha256:${EMPTY_USERDB_FIXTURE_SHA256}`], {
    subcase: "persistence-barrier",
    inputClass: "persistence-stress",
    stressDeadline: true,
    domEventType: "change",
    control: "[data-yune-userdb-import-input]",
  }),
  specialKeyStep("extended-persistence-reset", "Escape", "Escape", { cadence: "after-exact-final-paint", subcase: "persistence-barrier" }),

  ...printableSteps("extended-error-earlier", "n", "same-task-pressure", "error-barrier"),
  directActionStep("extended-error-target", "customizeValue", ["", "", `sha256:${INJECTED_ERROR_VALUE_SHA256}`], {
    subcase: "error-barrier",
    expectedOutcome: "failure",
    domEventType: "click",
    control: "[data-yune-freeform-customize-submit]",
    publicDemoAvailability: "blocked-hidden-control",
  }),
  specialKeyStep("extended-error-reset", "Escape", "Escape", { cadence: "after-exact-final-paint", subcase: "error-barrier" }),
];

export const SCENARIO_REGISTRY = deepFreeze({
  "existing-normal-guard": scenario("existing-normal-guard", {
    schema: "jyut6ping3",
    authority: "latency-and-page-shape-only",
    binding: false,
    gate: "WEB03-11-UNCHANGED",
    cadence: "existing100",
    steps: printableSteps("normal", PRIMARY_JYUTPING, "existing100"),
  }),
  "rapid-jyutping": scenario("rapid-jyutping", {
    schema: "jyut6ping3",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "sustained60",
    expectedInteractionWindowCount: 3,
    steps: repeatPhrase("rapid-jyutping", PRIMARY_JYUTPING, "sustained60"),
  }),
  "rapid-long-jyutping": scenario("rapid-long-jyutping", {
    schema: "jyut6ping3",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "sustained60",
    expectedInteractionWindowCount: 3,
    steps: repeatPhrase("rapid-long-jyutping", LONG_JYUTPING, "sustained60"),
  }),
  "rapid-luna": scenario("rapid-luna", {
    schema: "luna_pinyin",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "sustained60",
    expectedInteractionWindowCount: 3,
    steps: repeatPhrase("rapid-luna", RAPID_LUNA, "sustained60"),
  }),
  "burst-jyutping": scenario("burst-jyutping", {
    schema: "jyut6ping3",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "burst-cycle",
    expectedInteractionWindowCount: 3,
    steps: burstPhrase("burst-jyutping", PRIMARY_JYUTPING),
  }),
  "burst-luna": scenario("burst-luna", {
    schema: "luna_pinyin",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "burst-cycle",
    expectedInteractionWindowCount: 3,
    steps: burstPhrase("burst-luna", RAPID_LUNA),
  }),
  correction: scenario("correction", {
    schema: "jyut6ping3",
    authority: "oracle-exact",
    authorityFixture: "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m14-completion-correction.json",
    authorityCase: {
      variant: "correction_enabled",
      capturedInput: "nri",
      capturedCommitTextPreview: "你",
      browserFlow: "nri -> Backspace -> Backspace -> ri -> Space",
      provenance: "TypeDuck-HK/librime v1.1.2 correction_enabled case; intermediate deletion DOM fingerprints are browser contract observations, never Yune-derived oracle bytes",
    },
    binding: true,
    cadence: "sustained60",
    steps: CORRECTION_STEPS,
  }),
  "selection-paging": scenario("selection-paging", {
    schema: "luna_pinyin",
    authority: "oracle-exact",
    authorityFixture: "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
    authorityCases: [
      {
        fixtureCase: "luna-pinyin-actions.json::select_ni_second",
        flow: "ni -> digit 2",
        capturedCommitText: "擬",
      },
      {
        fixtureCase: "luna-pinyin-actions.json::paging_ni",
        flow: "ni -> PageDown -> PageUp",
        capturedPageSize: 5,
        publicPageSize: 6,
        transform: "preserve the captured global order; repartition into consecutive six-row pages",
        exactScope: "page-1 six rows and page-2 captured prefix; no uncaptured page-2 suffix parity claim",
      },
    ],
    binding: true,
    cadence: "sustained60",
    steps: SELECTION_PAGING_STEPS,
  }),
  "selection-paging-jyutping": scenario("selection-paging-jyutping", {
    schema: "jyut6ping3",
    authority: "oracle-exact",
    authorityFixture: [
      "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json",
      "crates/yune-core/tests/fixtures/upstream-jyutping/canonical-rime-cantonese/jyutping-m59-being-whole-input.json",
    ],
    nonAuthorityBrowserDiagnostic: "apps/yune-web/e2e/results/m28-partial-selection/browser-partial-selection.json",
    authorityCases: [
      {
        fixtureCase: "jyut6ping3-m28-partial-selection.json::candidate-index-1",
        flow: "caksijathaacoenggeoizi -> digit 2",
        capturedPartialCommitText: "測",
        capturedRemainingInput: "sijathaacoenggeoizi",
      },
      {
        fixtureCase: "jyutping-m59-being-whole-input.json::being",
        flow: "being -> PageDown -> PageUp",
        capturedPageSize: 5,
        publicPageSize: 6,
        transform: "preserve the complete captured global order; repartition into consecutive six-row pages",
      },
    ],
    binding: true,
    cadence: "sustained60",
    steps: JYUTPING_SELECTION_PAGING_STEPS,
  }),
  "burst-action-map": scenario("burst-action-map", {
    schema: "jyut6ping3",
    owningSchemas: ["jyut6ping3"],
    owningSchemaRationale: "Shift-tap live-option fan-out is owned by the frozen Jyutping production posture",
    authority: "latency-and-page-shape-only+exact-action-map",
    binding: true,
    cadence: "burst-cycle",
    actionMapOwner: "apps/yune-web/src/CandidatePanel.tsx Shift tap -> App.tsx live-options effect",
    initialOptionPosture: "production defaults: jyut6ping3, ascii_mode=false, full_shape=false, Hong Kong traditional, extended_charset=false, disabled=false",
    shiftReleaseOrderedSetOptions: SHIFT_TAP_LIVE_OPTION_ACTIONS,
    steps: BURST_ACTION_MAP_STEPS,
  }),
  "fifo-pressure-barriers": scenario("fifo-pressure-barriers", {
    schema: ["jyut6ping3", "luna_pinyin"],
    authority: "contract-exact",
    binding: true,
    cadence: "same-task-pressure",
    overlapRequired: true,
    steps: FIFO_PRESSURE_STEPS,
  }),
  "learned-row": scenario("learned-row", {
    schema: "jyut6ping3",
    authority: "contract-exact",
    authorityFixture: [
      "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m21-closeout.json",
    ],
    authorityCases: [
      {
        fixtureCase: "jyut6ping3-m21-closeout.json::ngohaigo",
        trainingInput: "ngohaigo",
        expectedCandidateText: "我係個",
        expectedCommitText: "我係個",
      },
      {
        fixtureCase: "WEB03::typeduck-learned-userdb-prefix",
        probeInput: "ngo",
        expectedClassicFirstText: "我",
        expectedLearnedCandidateText: "我係個",
        persistence: "real browser reload plus visible learned candidate",
      },
    ],
    binding: true,
    cadence: "sustained60",
    expectedInteractionWindowCount: 2,
    steps: LEARNED_STEPS,
  }),
  "fair-peer-short": scenario("fair-peer-short", {
    schema: "luna_pinyin",
    authority: "latency-and-page-shape-only",
    binding: true,
    peer: "binding-only-when-packageAlignment-PROVED",
    cadence: "sustained60",
    steps: [
      ...printableSteps("peer-short", "ni", "sustained60", "candidate"),
      specialKeyStep("peer-short-commit", " ", "space", { cadence: "after-exact-final-paint", subcase: "commit" }),
    ],
  }),
  "peer-sustained": scenario("peer-sustained", {
    schema: "luna_pinyin",
    authority: "latency-and-page-shape-only",
    binding: false,
    peer: "informational-unless-row-packageAlignment-PROVED",
    cadence: "sustained60",
    steps: [
      ...printableSteps("peer-sustained", RAPID_LUNA, "sustained60", "candidate"),
      specialKeyStep("peer-sustained-commit", " ", "space", { subcase: "commit" }),
    ],
  }),
  "extended-scheduler-barriers": scenario("extended-scheduler-barriers", {
    schema: ["jyut6ping3", "luna_pinyin"],
    authority: "contract-exact",
    binding: "branch-b-only",
    cadence: "same-task-pressure",
    overlapRequired: true,
    steps: EXTENDED_BARRIERS,
  }),
});

/**
 * Measurement rows are scenario x owning-schema rows. A single-schema row
 * keeps its historical id; multi-schema rows are deliberately expanded so a
 * caller cannot silently select array element zero.
 */
export const SCENARIO_RUN_REGISTRY = deepFreeze(Object.fromEntries(
  Object.entries(SCENARIO_REGISTRY).flatMap(([scenarioId, row]) => {
    const schemas = Array.isArray(row.schema) ? row.schema : [row.schema];
    return schemas.map((schema) => {
      const runId = schemas.length === 1 ? scenarioId : `${scenarioId}@${schema}`;
      return [runId, { runId, scenarioId, schema }];
    });
  }),
));

export const WEB06_PRESSURE_PAIR_REGISTRY = deepFreeze({
  "fifo-pressure-barriers": [
    { subcase: "commit-then-type", earlierStepId: "fifo-commit", laterStepId: "fifo-commit-later-1" },
    { subcase: "digit-select-then-type", earlierStepId: "fifo-select", laterStepId: "fifo-select-later-1" },
    { subcase: "page-then-type", earlierStepId: "fifo-page", laterStepId: "fifo-page-later-1" },
    { subcase: "persistence-then-type", earlierStepId: "fifo-userdb-import", laterStepId: "fifo-userdb-later-1" },
  ],
  "extended-scheduler-barriers": [
    { subcase: "option-barrier", earlierStepId: "extended-option-earlier-1", laterStepId: "extended-option-target" },
    { subcase: "schema-barrier", earlierStepId: "extended-schema-earlier-1", laterStepId: "extended-schema-target" },
    { subcase: "deploy-barrier", earlierStepId: "extended-deploy-earlier-1", laterStepId: "extended-deploy-target" },
    { subcase: "persistence-barrier", earlierStepId: "extended-persistence-earlier-1", laterStepId: "extended-persistence-target" },
    { subcase: "error-barrier", earlierStepId: "extended-error-earlier-1", laterStepId: "extended-error-target" },
  ],
});

export function resolveScenarioRun(runId) {
  if (!hasOwn(SCENARIO_RUN_REGISTRY, runId)) throw new Error(`WEB06_UNKNOWN_SCENARIO_RUN:${runId}`);
  const run = SCENARIO_RUN_REGISTRY[runId];
  return run;
}

const FIXTURE_SHA256 = deepFreeze({
  "docs/plans/active/web06-plan-rapid-typing-smoothness.md": "793bd4e06280e1e8f27e3192fb304a8ff19472462b6bcc2e0b0ea8fe57714340",
  "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m14-completion-correction.json": "fad9483785486fa4bd5d173bbf5d3caf0db7c6f46164442dca2b60255a98ca65",
  "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json": "42699a67070fca0db9606ccd1f50a800e9d9cae34ac7d29eed50dc4edc9406f7",
  "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json": "46b811ddfe97ace9ba56fd9735f3ebc199d5023426e07150f0a2d2ac31e8f965",
  "crates/yune-core/tests/fixtures/upstream-jyutping/canonical-rime-cantonese/jyutping-m59-being-whole-input.json": "8a8f399e44cb09e651f6580c4a4be24fdf9db2c52ada50d567324798dcdc81ac",
  "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m21-closeout.json": "9e1125159a916c3a8d7b60540b5179342fedd6687fce93bf595e9c119892728c",
  "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json": "cb0ae9ab7a5bac8396a60818ed6f58d9a92a2dafcc1333254c7212abb3174fce",
});

const ORACLE_FIELDS = new Set([
  "candidateTextsExact",
  "candidateTextsPrefix",
  "commitTextExact",
]);

function behaviorPredicate(authorityClass, fixture, fixtureCase, expected, fieldAuthorityOverrides = {}) {
  const fieldAuthority = Object.fromEntries(Object.keys(expected).map((field) => [
    field,
    fieldAuthorityOverrides[field]
      ?? (ORACLE_FIELDS.has(field)
        && (field === "commitTextExact" || !Array.isArray(expected[field]) || expected[field].length > 0)
        && authorityClass.startsWith("oracle-exact")
        ? authorityClass
        : "contract-exact"),
  ]));
  return {
    authorityClass,
    fixture,
    fixtureSha256: FIXTURE_SHA256[fixture],
    fixtureCase,
    expected,
    fieldAuthority,
  };
}

const CONTRACT_RESET_PREDICATES = Object.fromEntries(
  Object.entries(SCENARIO_REGISTRY).flatMap(([scenarioId, row]) => row.steps
    .filter((step) => step.id.endsWith("-reset"))
    .map((step) => [`${scenarioId}:${step.id}`, behaviorPredicate(
      "contract-exact",
      "docs/plans/active/web06-plan-rapid-typing-smoothness.md",
      "latest state at each exact reset is empty composition",
      { renderedInput: "", candidateTextsExact: [], visibleCount: 0 },
    )])),
);

/**
 * Frozen external/contract behavior predicates. A DOM self-hash proves only
 * coherence; these rows bind the coherent endpoint to externally owned bytes.
 */
export const WEB06_BEHAVIOR_PREDICATES = deepFreeze({
  ...CONTRACT_RESET_PREDICATES,
  "correction:correction-pre-3": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m14-completion-correction.json",
    "correction_enabled/nri",
    { candidateTextsExact: ["你", "呢", "尼", "妮", "彌", "妳"], visibleCount: 6 },
  ),
  "correction:correction-resume-2": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m14-completion-correction.json",
    "correction_enabled/nri",
    { candidateTextsExact: ["你", "呢", "尼", "妮", "彌", "妳"], visibleCount: 6 },
  ),
  "correction:correction-commit": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m14-completion-correction.json",
    "correction_enabled/nri commit_text_preview",
    {
      renderedInput: "",
      candidateTextsExact: [],
      visibleCount: 0,
      commitTextExact: "你",
      textareaValue: "你",
      selectionStart: 1,
      selectionEnd: 1,
      visibleComposition: false,
    },
  ),
  "selection-paging:selection-ni-digit-2": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
    "select_ni_second/after_select_2",
    {
      renderedInput: "",
      candidateTextsExact: [],
      visibleCount: 0,
      commitTextExact: "擬",
      textareaValue: "擬",
      selectionStart: 1,
      selectionEnd: 1,
      visibleComposition: false,
    },
  ),
  "selection-paging:paging-ni-2": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
    "paging_ni/page_1+page_2 global-order repartition to page_size=6",
    { candidateTextsExact: ["你", "擬", "尼", "泥", "呢", "妳"], visibleCount: 6, textareaValue: "擬" },
  ),
  "selection-paging:paging-page-down": behaviorPredicate(
    "oracle-exact-partial-page",
    "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
    "paging_ni/page_2 captured global indices 6..9 after page_size=6 repartition",
    { candidateTextsPrefix: ["妮", "膩", "逆", "倪"], visibleCount: 6, previousDisabled: false, textareaValue: "擬" },
  ),
  "selection-paging:paging-page-up": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
    "paging_ni/page_1_again+page_2 first row repartitioned to page_size=6",
    { candidateTextsExact: ["你", "擬", "尼", "泥", "呢", "妳"], visibleCount: 6, previousDisabled: true, textareaValue: "擬" },
  ),
  "selection-paging-jyutping:jyut-selection-partial-digit-2": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json",
    "captured_next_candidates indices 0..5 after candidate-index-1 partial selection",
    {
      candidateTextsExact: ["是日下場句子", "是日", "時日", "時", "是", "事"],
      visibleCount: 6,
      textareaValue: "測",
      selectionStart: 1,
      selectionEnd: 1,
      visibleComposition: true,
    },
  ),
  "selection-paging-jyutping:jyut-paging-being-5": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/upstream-jyutping/canonical-rime-cantonese/jyutping-m59-being-whole-input.json",
    "being/global indices 0..5 repartitioned to page_size=6",
    {
      candidateTextsExact: ["畀嗯", "畀", "比", "被", "鼻", "避"],
      visibleCount: 6,
      textareaValue: "測",
      selectionStart: 1,
      selectionEnd: 1,
      visibleComposition: true,
    },
  ),
  "selection-paging-jyutping:jyut-paging-page-down": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/upstream-jyutping/canonical-rime-cantonese/jyutping-m59-being-whole-input.json",
    "being/global indices 6..11 repartitioned to page_size=6",
    {
      candidateTextsExact: ["髀", "碑", "臂", "秘", "祕", "俾"],
      visibleCount: 6,
      previousDisabled: false,
      textareaValue: "測",
      selectionStart: 1,
      selectionEnd: 1,
      visibleComposition: true,
    },
  ),
  "selection-paging-jyutping:jyut-paging-page-up": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/upstream-jyutping/canonical-rime-cantonese/jyutping-m59-being-whole-input.json",
    "being/global indices 0..5 repartitioned to page_size=6",
    {
      candidateTextsExact: ["畀嗯", "畀", "比", "被", "鼻", "避"],
      visibleCount: 6,
      previousDisabled: true,
      textareaValue: "測",
      selectionStart: 1,
      selectionEnd: 1,
      visibleComposition: true,
    },
  ),
  "learned-row:learned-training-8": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m21-closeout.json",
    "ngohaigo/default candidate 0",
    { candidateTextsPrefix: ["我係個"] },
  ),
  "learned-row:learned-commit": behaviorPredicate(
    "oracle-exact+contract-exact-persistence",
    "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m21-closeout.json",
    "ngohaigo commit_text_preview plus WEB03 persisted learned row",
    {
      renderedInput: "",
      candidateTextsExact: [],
      visibleCount: 0,
      commitTextExact: "我係個",
      textareaValue: "我係個",
      selectionStart: 3,
      selectionEnd: 3,
      visibleComposition: false,
      persistenceCompleted: true,
    },
  ),
  "learned-row:learned-probe-3": behaviorPredicate(
    "oracle-exact+contract-exact-persistence",
    "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m21-closeout.json",
    "default_combined/ngo candidate 0 plus ngohaigo candidate 0; learned inclusion after real reload is contract-exact persistence",
    { candidateTextsPrefix: ["我"], candidateTextsInclude: ["我係個"], textareaValue: "" },
    { candidateTextsPrefix: "oracle-exact", candidateTextsInclude: "contract-exact-persistence" },
  ),
  "fair-peer-short:peer-short-2": behaviorPredicate(
    "oracle-exact-membership",
    "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
    "paging_ni page_1+page_2 first row repartitioned to page_size=6",
    { candidateTextsExact: ["你", "擬", "尼", "泥", "呢", "妳"], visibleCount: 6 },
  ),
  "fair-peer-short:peer-short-commit": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
    "commit_ni_space/after_space",
    {
      renderedInput: "",
      candidateTextsExact: [],
      visibleCount: 0,
      commitTextExact: "你",
      textareaValue: "你",
      selectionStart: 1,
      selectionEnd: 1,
      visibleComposition: false,
    },
  ),
  "peer-sustained:peer-sustained-59": behaviorPredicate(
    "oracle-exact-membership",
    "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json",
    "sentence_benchmark_59/page_1",
    { candidateTextsPrefix: ["這個引擎其實應該支持超長句子輸入才能用", "這個", "這歌", "這格", "這"], visibleCount: 6 },
  ),
  "peer-sustained:peer-sustained-commit": behaviorPredicate(
    "oracle-exact",
    "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json",
    "sentence_benchmark_59/commit_text_preview",
    {
      renderedInput: "",
      candidateTextsExact: [],
      visibleCount: 0,
      commitTextExact: "這個引擎其實應該支持超長句子輸入才能用",
      textareaValue: "這個引擎其實應該支持超長句子輸入才能用",
      selectionStart: 19,
      selectionEnd: 19,
      visibleComposition: false,
    },
  ),
});

export const EVENT_ACTION_RULES = deepFreeze([
  {
    id: "focus-loss-blur",
    event: "blur",
    classification: "frontend-consumed",
    condition: "window loses focus",
    actions: [],
    compositionEpochBoundary: true,
    supersessionSubRunBoundary: true,
  },
  { id: "printable-keydown", event: "keydown", classification: "mapped-action(s)", actions: ["processKey"] },
  { id: "printable-keyup", event: "keyup", classification: "browser-pass-through", actions: [] },
  { id: "digit-selection-keydown", event: "keydown", condition: "composition-active-and-visible-index", classification: "mapped-action(s)", actions: ["selectCandidate"] },
  { id: "digit-selection-keyup", event: "keyup", condition: "composition-active", classification: "frontend-consumed(digit-release)", actions: [] },
  { id: "backspace-delete-keydown", event: "keydown", classification: "mapped-action(s)", actions: ["processKey"] },
  { id: "arrow-page-keydown", event: "keydown", classification: "mapped-action(s)", actions: ["processKey"] },
  { id: "punctuation-commit-keydown", event: "keydown", classification: "mapped-action(s)", actions: ["processKey"] },
  { id: "modifier-tap-keydown", event: "keydown", classification: "frontend-consumed(pending-ascii-shift-tap)", actions: [] },
  { id: "modifier-tap-keyup", event: "keyup", classification: "mapped-action(s)", actions: Array(12).fill("setOption") },
  { id: "modifier-release-keyup", event: "keyup", condition: "composition-active-and-not-shift-tap", classification: "mapped-action(s)", actions: ["processKey"] },
  { id: "escape-cancel-keydown", event: "keydown", classification: "mapped-action(s)", actions: ["processKey"] },
  { id: "candidate-click", event: "click", classification: "mapped-action(s)", actions: ["selectCandidate"] },
  { id: "candidate-delete-long-press-mouse", event: "mousedown", condition: "800ms timer; mouseup or mouseleave cancels; mousemove does not cancel", classification: "mapped-action(s) after long-press", actions: ["deleteCandidate"] },
  { id: "candidate-delete-long-press-touch", event: "touchstart", condition: "800ms timer; touchend or touchcancel cancels; touchmove does not cancel", classification: "mapped-action(s) after long-press", actions: ["deleteCandidate"] },
  { id: "page-button", event: "click", classification: "mapped-action(s)", actions: ["flipPage"] },
  { id: "live-option-control", event: "change", classification: "mapped-action(s)", actions: ["setOption"] },
  { id: "schema-control", event: "change", classification: "mapped-action(s)", actions: ["selectSchema"] },
  { id: "userdb-refresh-background", event: "background", classification: "mapped-action(s)", actions: ["getUserdbSnapshot"] },
  { id: "userdb-import-control", event: "change", classification: "mapped-action(s)", actions: ["importUserdb"] },
  { id: "ai-second-pass-background", event: "background", classification: "mapped-action(s)", actions: ["stageAi"] },
  { id: "customize-control", event: "change", classification: "mapped-action(s)", actions: ["customize"] },
  { id: "customize-value-control", event: "click", classification: "mapped-action(s)", actions: ["customizeValue"] },
  { id: "deploy-control", event: "click", classification: "mapped-action(s)", actions: ["deploy"] },
  { id: "deploy-cache-read-background", event: "background", classification: "mapped-action(s)", actions: ["deployCacheSnapshot"] },
  { id: "deploy-cache-invalidate-control", event: "click", classification: "mapped-action(s)", actions: ["invalidateDeployCache"] },
  { id: "injected-assets-read-background", event: "background", classification: "mapped-action(s)", actions: ["injectedAssetsManifest"] },
]);

export function validateEventActionRuleMap(rules = EVENT_ACTION_RULES) {
  const errors = [];
  if (!Array.isArray(rules)) return { ok: false, errors: ["EVENT_ACTION_RULE_MAP_INVALID"] };
  const ids = rules.map((rule) => rule?.id);
  if (ids.some((id) => typeof id !== "string" || !id) || new Set(ids).size !== ids.length) {
    errors.push("EVENT_ACTION_RULE_ID_INVALID");
  }
  const focusRules = rules.filter((rule) => rule?.event === "blur");
  if (focusRules.length !== 1) {
    errors.push("FOCUS_LOSS_RULE_CARDINALITY");
  } else {
    const [focus] = focusRules;
    if (focus.id !== "focus-loss-blur" || focus.classification !== "frontend-consumed"
      || !Array.isArray(focus.actions) || focus.actions.length !== 0
      || focus.compositionEpochBoundary !== true || focus.supersessionSubRunBoundary !== true) {
      errors.push("FOCUS_LOSS_RULE_INVALID");
    }
  }
  return { ok: errors.length === 0, errors };
}

function keyboardEventExpectation(step, type) {
  if (step.code.startsWith("Shift")) {
    return type === "keydown"
      ? { classification: "frontend-consumed", reason: "ascii-mode-shift-keydown" }
      : { classification: "mapped-action(s)", reason: "ascii-mode-shift-tap" };
  }
  const primary = step.actions[0];
  if (type === "keyup") {
    return /^[0-9]$/.test(step.key) && primary?.kind === "selectCandidate"
      ? { classification: "frontend-consumed", reason: "composition-digit-keyup-follows-keydown" }
      : { classification: "browser-pass-through", reason: "unmapped-keyup" };
  }
  if (primary?.kind === "selectCandidate") {
    return { classification: "mapped-action(s)", reason: "composition-digit-selection" };
  }
  if (primary?.kind === "processKey") {
    if (primary.supersedable) return { classification: "mapped-action(s)", reason: "printable-key" };
    const rimeKey = String(primary.args[0] ?? "").replace(/^\{|\}$/g, "");
    const boundary = rimeKey === "BackSpace" || rimeKey === "Delete"
      ? "correction"
      : rimeKey === "Page_Down" || rimeKey === "Page_Up" || rimeKey === "Down" || rimeKey === "Up"
        ? "paging"
        : rimeKey === "space" || rimeKey === "Return"
          ? "commit"
          : rimeKey === "Escape"
            ? "cancel"
            : "none";
    return { classification: "mapped-action(s)", reason: `rime-key:${boundary}` };
  }
  return { classification: "browser-pass-through", reason: "unmapped-keydown" };
}

export function expandScenarioExpectedTimeline(scenarioId) {
  if (!hasOwn(SCENARIO_REGISTRY, scenarioId)) throw new Error(`WEB06_UNKNOWN_SCENARIO:${scenarioId}`);
  const row = SCENARIO_REGISTRY[scenarioId];
  let eventSequenceId = 0;
  let sequenceId = 0;
  const events = [];
  const actions = [];
  for (const step of row.steps) {
    const eventIds = [];
    if (step.source === "keyboard") {
      eventIds.push(++eventSequenceId, ++eventSequenceId);
      const actionEventSequenceId = step.code === "ShiftLeft" || step.code === "ShiftRight"
        ? eventIds[1]
        : eventIds[0];
      const keydown = keyboardEventExpectation(step, "keydown");
      const keyup = keyboardEventExpectation(step, "keyup");
      events.push({
        eventSequenceId: eventIds[0],
        stepId: step.id,
        type: "keydown",
        key: step.key,
        code: step.code,
        classification: keydown.classification,
        reason: keydown.reason,
        mappedActionIds: [],
      });
      events.push({
        eventSequenceId: eventIds[1],
        stepId: step.id,
        type: "keyup",
        key: step.key,
        code: step.code,
        classification: keyup.classification,
        reason: keyup.reason,
        mappedActionIds: [],
      });
      for (const expected of step.actions) {
        sequenceId += 1;
        const actionId = `a${sequenceId}`;
        const causedBy = expected.background === true
          ? [...actions].reverse().find(candidate => candidate.stepId === step.id && candidate.background !== true)
          : undefined;
        actions.push({
          ...expected,
          actionId,
          sequenceId,
          eventSequenceId: expected.background === true ? undefined : actionEventSequenceId,
          originKind: expected.background === true ? "background" : "dom-event",
          originReason: expected.background === true ? expected.originReason : events[actionEventSequenceId - 1].reason,
          causedByActionId: causedBy?.actionId,
          causedBySequenceId: causedBy?.sequenceId,
          causedByEventSequenceId: expected.background === true ? actionEventSequenceId : undefined,
          stepId: step.id,
        });
        if (expected.background !== true) events[actionEventSequenceId - 1].mappedActionIds.push(actionId);
      }
    } else {
      const id = ++eventSequenceId;
      events.push({
        eventSequenceId: id,
        stepId: step.id,
        type: step.domEventType ?? step.source,
        key: "",
        code: "",
        classification: step.actions.length ? "mapped-action(s)" : "browser-pass-through",
        reason: step.eventReason ?? step.id,
        mappedActionIds: [],
      });
      for (const expected of step.actions) {
        sequenceId += 1;
        const actionId = `a${sequenceId}`;
        const causedBy = expected.background === true
          ? [...actions].reverse().find(candidate => candidate.stepId === step.id && candidate.background !== true)
          : undefined;
        actions.push({
          ...expected,
          actionId,
          sequenceId,
          eventSequenceId: expected.background === true ? undefined : id,
          originKind: expected.background === true ? "background" : "dom-event",
          originReason: expected.background === true ? expected.originReason : events[id - 1].reason,
          causedByActionId: causedBy?.actionId,
          causedBySequenceId: causedBy?.sequenceId,
          causedByEventSequenceId: expected.background === true ? id : undefined,
          stepId: step.id,
        });
        if (expected.background !== true) events[id - 1].mappedActionIds.push(actionId);
      }
    }
  }
  return { events, actions };
}

export function validateFrozenContract() {
  const errors = [];
  errors.push(...validateEventActionRuleMap().errors);
  const coveredActions = new Set(EVENT_ACTION_RULES.flatMap((rule) => rule.actions));
  for (const name of ALL_ACTION_NAMES) {
    if (!coveredActions.has(name)) errors.push(`UNMAPPED_ACTION:${name}`);
  }
  for (const [id, row] of Object.entries(SCENARIO_REGISTRY)) {
    if (id !== row.id) errors.push(`SCENARIO_ID_MISMATCH:${id}`);
    if (!Number.isInteger(row.expectedDomEventCount) || row.expectedDomEventCount <= 0) errors.push(`INVALID_EVENT_COUNT:${id}`);
    if (!Number.isInteger(row.expectedActionCount) || row.expectedActionCount <= 0) errors.push(`INVALID_ACTION_COUNT:${id}`);
    if (!Number.isInteger(row.expectedCoveringSamples) || !Number.isInteger(row.expectedTerminalSamples)) errors.push(`INVALID_SAMPLE_COUNT:${id}`);
    if (JSON.stringify(row).includes("TBD")) errors.push(`UNFROZEN_TBD:${id}`);
    const stepIds = row.steps.map((step) => step.id);
    if (new Set(stepIds).size !== stepIds.length) errors.push(`DUPLICATE_STEP_ID:${id}`);
    const schemas = Array.isArray(row.schema) ? row.schema : [row.schema];
    const runs = Object.values(SCENARIO_RUN_REGISTRY).filter((run) => run.scenarioId === id);
    if (runs.length !== schemas.length || !schemas.every((schema) => runs.some((run) => run.schema === schema))) {
      errors.push(`SCENARIO_SCHEMA_RUN_COLLAPSE:${id}`);
    }
  }
  const burstOwner = SCENARIO_REGISTRY["burst-action-map"];
  if (!sameArray(burstOwner.owningSchemas, ["jyut6ping3"]) || !burstOwner.owningSchemaRationale) {
    errors.push("BURST_ACTION_MAP_OWNER_UNFROZEN");
  }
  for (const [scenarioId, pairs] of Object.entries(WEB06_PRESSURE_PAIR_REGISTRY)) {
    const row = SCENARIO_REGISTRY[scenarioId];
    if (!row?.overlapRequired || new Set(pairs.map((pair) => pair.subcase)).size !== pairs.length
      || pairs.some((pair) => !row.steps.some((step) => step.id === pair.earlierStepId)
        || !row.steps.some((step) => step.id === pair.laterStepId))) {
      errors.push(`PRESSURE_PAIR_REGISTRY_INVALID:${scenarioId}`);
    }
  }
  const exactCounts = {
    "existing-normal-guard": [94, 47, 47, 0, 46, 1],
    "rapid-jyutping": [286, 143, 141, 2, 138, 3],
    "rapid-long-jyutping": [316, 158, 156, 2, 153, 3],
    "rapid-luna": [358, 179, 177, 2, 174, 3],
    "burst-jyutping": [286, 143, 141, 2, 138, 3],
    "burst-luna": [358, 179, 177, 2, 174, 3],
    correction: [16, 8, 5, 3, 3, 1],
    "selection-paging": [14, 7, 4, 3, 2, 1],
    "selection-paging-jyutping": [62, 31, 27, 4, 25, 1],
    "burst-action-map": [16, 19, 3, 5, 7, 1],
    "fifo-pressure-barriers": [45, 23, 16, 7, 0, 1],
    "learned-row": [25, 13, 11, 1, 9, 2],
    "fair-peer-short": [6, 3, 2, 1, 1, 1],
    "peer-sustained": [120, 60, 59, 1, 58, 1],
    "extended-scheduler-barriers": [25, 40, 5, 10, 0, 1],
  };
  for (const [id, [events, actions, covering, terminal, gaps, windows]] of Object.entries(exactCounts)) {
    const row = SCENARIO_REGISTRY[id];
    if (row.expectedDomEventCount !== events || row.expectedActionCount !== actions
      || row.expectedCoveringSamples !== covering || row.expectedTerminalSamples !== terminal
      || row.expectedCadenceGapCount !== gaps || row.expectedInteractionWindowCount !== windows) {
      errors.push(`FROZEN_COUNT_MISMATCH:${id}`);
    }
  }
  for (const [identity, predicate] of Object.entries(WEB06_BEHAVIOR_PREDICATES)) {
    const separator = identity.indexOf(":");
    const scenarioId = identity.slice(0, separator);
    const stepId = identity.slice(separator + 1);
    if (separator <= 0 || !hasOwn(SCENARIO_REGISTRY, scenarioId)
      || !SCENARIO_REGISTRY[scenarioId].steps.some((step) => step.id === stepId)) {
      errors.push(`BEHAVIOR_PREDICATE_STEP_UNKNOWN:${identity}`);
    }
    if (!predicate.authorityClass || !predicate.fixture || !isSha256(predicate.fixtureSha256)
      || !predicate.fixtureCase || !predicate.expected || !predicate.fieldAuthority) {
      errors.push(`BEHAVIOR_PREDICATE_PROVENANCE_INCOMPLETE:${identity}`);
    }
    for (const field of Object.keys(predicate.expected ?? {})) {
      if (typeof predicate.fieldAuthority?.[field] !== "string" || !predicate.fieldAuthority[field]) {
        errors.push(`BEHAVIOR_PREDICATE_FIELD_AUTHORITY_MISSING:${identity}:${field}`);
      }
    }
  }
  if (PRIMARY_JYUTPING.length !== 47 || LONG_JYUTPING.length !== 52 || RAPID_LUNA.length !== 59) {
    errors.push("FROZEN_INPUT_LENGTH_MISMATCH");
  }
  return { ok: errors.length === 0, errors };
}

export function percentileNearestRank(values, p) {
  if (!Array.isArray(values) || values.length === 0) throw new Error("WEB06_EMPTY_DISTRIBUTION");
  if (!Number.isFinite(p) || p < 0 || p > 1) throw new Error("WEB06_INVALID_PERCENTILE");
  if (values.some((value) => !Number.isFinite(value))) throw new Error("WEB06_NONFINITE_DISTRIBUTION");
  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.ceil((sorted.length - 1) * p);
  return sorted[index];
}

export function distributionSummary(values) {
  return {
    count: values.length,
    median: percentileNearestRank(values, 0.5),
    p95: percentileNearestRank(values, 0.95),
    p99: percentileNearestRank(values, 0.99),
    max: Math.max(...values),
  };
}

export function evaluateThresholdDistribution(values, ceilings, label) {
  const summary = distributionSummary(values);
  const violations = [];
  for (const key of ["median", "p95", "p99", "max"]) {
    if (ceilings[key] !== undefined && summary[key] > ceilings[key]) {
      violations.push(`${label}:${key}:${summary[key]}>${ceilings[key]}`);
    }
  }
  return { pass: violations.length === 0, summary, violations };
}

export function classifyCadenceGap(actualGapMs, nominalGapMs) {
  if (!Number.isFinite(actualGapMs) || !Number.isFinite(nominalGapMs)) return "SETUP_INVALID_NONFINITE_CADENCE";
  const range = nominalGapMs === 40
    ? WEB06_THRESHOLDS.cadence.burst40
    : nominalGapMs === 120
      ? WEB06_THRESHOLDS.cadence.burst120
      : nominalGapMs === 100
        ? WEB06_THRESHOLDS.cadence.existing100
        : nominalGapMs === 60
          ? WEB06_THRESHOLDS.cadence.sustained60
          : null;
  if (!range) return "SETUP_INVALID_UNKNOWN_CADENCE";
  if (actualGapMs < range.minMs) return "TOO_SHORT";
  if (actualGapMs > range.maxMs) return "TOO_LONG";
  return "IN_RANGE";
}

/**
 * Cadence precedence is deliberately asymmetric: too-short cadence invalidates
 * latency/frame arithmetic but can never erase a behavior/order RED; too-long
 * cadence cannot erase any hard RED because it made the workload easier.
 */
export function classifyAttempt(attempt) {
  if (attempt.measurementCompleted === false && attempt.validRedObserved === true) {
    return "RED_INCOMPLETE_BEHAVIOR";
  }
  if (attempt.validRedObserved === true) {
    return attempt.behaviorRed || attempt.orderRed ? "RED_BEHAVIOR" : "RED";
  }
  if (attempt.setupInvalid) return "SETUP_INVALID";
  const behaviorRed = Boolean(attempt.behaviorRed || attempt.orderRed);
  const anyHardRed = Boolean(
    behaviorRed || attempt.latencyRed || attempt.terminalRed || attempt.frameRed || attempt.longTaskRed,
  );
  if (attempt.cadence === "TOO_SHORT") {
    return behaviorRed ? "RED_BEHAVIOR" : "NO_VERDICT_INVALID_CADENCE";
  }
  if (attempt.cadence === "TOO_LONG") {
    return anyHardRed ? "RED" : "NO_VERDICT_INVALID_CADENCE";
  }
  if (attempt.cadence !== "IN_RANGE" && attempt.cadence !== "NOT_APPLICABLE") {
    return "SETUP_INVALID";
  }
  return anyHardRed ? "RED" : "PASS";
}

export function evaluateAttemptSeries(attempts) {
  if (!Array.isArray(attempts) || attempts.length > WEB06_THRESHOLDS.attempts.maximum) {
    throw new Error("WEB06_ATTEMPT_CAP_EXCEEDED");
  }
  const ids = attempts.map((attempt) => attempt.attemptId);
  if (ids.some((id) => typeof id !== "string") || new Set(ids).size !== ids.length) {
    throw new Error("WEB06_ATTEMPT_ID_INVALID");
  }
  const retained = attempts.map((attempt) => {
    const verdict = classifyAttempt(attempt);
    const measurementCompleted = attempt.measurementCompleted !== false;
    const retainedLogicalRound = measurementCompleted
      && ["PASS", "RED", "RED_BEHAVIOR"].includes(verdict);
    const validForLatencyFrame = measurementCompleted && !attempt.setupInvalid
      && ["IN_RANGE", "NOT_APPLICABLE"].includes(attempt.cadence)
      && ["PASS", "RED", "RED_BEHAVIOR"].includes(verdict);
    return {
      ...attempt,
      verdict,
      retainedLogicalRound,
      retainedMeasured: retainedLogicalRound,
      validForLatencyFrame,
      retainedHardRed: verdict.startsWith("RED"),
    };
  });
  const measured = retained.filter((attempt) => attempt.retainedLogicalRound);
  const validLatencyFrame = retained.filter((attempt) => attempt.validForLatencyFrame);
  const hardReds = retained.filter((attempt) => attempt.retainedHardRed);
  const replaceable = retained.filter((attempt) =>
    attempt.verdict === "SETUP_INVALID" || attempt.verdict === "NO_VERDICT_INVALID_CADENCE");
  const complete = measured.length === WEB06_THRESHOLDS.attempts.requiredValid;
  const terminalIncompleteRed = retained.at(-1)?.verdict === "RED_INCOMPLETE_BEHAVIOR";
  if (measured.length > WEB06_THRESHOLDS.attempts.requiredValid) {
    throw new Error("WEB06_MEASURED_ROUND_RETRY_FORBIDDEN");
  }
  return {
    status: complete
      ? hardReds.length > 0 ? "COMPLETE_WITH_RED" : "COMPLETE_GREEN"
      : attempts.length === WEB06_THRESHOLDS.attempts.maximum || terminalIncompleteRed
        ? hardReds.length > 0
          ? "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS_WITH_PRESERVED_RED"
          : "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS"
        : "INCOMPLETE",
    retained,
    measuredCount: measured.length,
    validLatencyFrameCount: validLatencyFrame.length,
    preservedHardRedCount: hardReds.length,
    preservedHardRedAttemptIds: hardReds.map((attempt) => attempt.attemptId),
    replaceableCount: replaceable.length,
  };
}

function requireExchangeFields(exchange, names) {
  for (const name of names) {
    if (!exchange || typeof exchange !== "object" || Array.isArray(exchange)
      || !Number.isFinite(exchange[name])) {
      throw new Error(`SETUP_INVALID_CLOCK_CALIBRATION:${name}`);
    }
  }
}

export function computeDriverPageExchange(exchange) {
  requireExchangeFields(exchange, ["d0", "m1", "m2", "d3"]);
  const offset = ((exchange.m1 - exchange.d0) + (exchange.m2 - exchange.d3)) / 2;
  const netRtt = (exchange.d3 - exchange.d0) - (exchange.m2 - exchange.m1);
  if (!Number.isFinite(netRtt) || netRtt < 0) throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:driver-net-rtt");
  return {
    ...exchange,
    midpoint: (exchange.d0 + exchange.d3) / 2,
    offset,
    netRtt,
    uncertainty: netRtt / 2,
  };
}

export function computeMainWorkerExchange(exchange) {
  requireExchangeFields(exchange, ["m0", "w1", "w2", "m3"]);
  const offset = ((exchange.w1 - exchange.m0) + (exchange.w2 - exchange.m3)) / 2;
  const netRtt = (exchange.m3 - exchange.m0) - (exchange.w2 - exchange.w1);
  if (!Number.isFinite(netRtt) || netRtt < 0) throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:worker-net-rtt");
  return {
    ...exchange,
    midpoint: (exchange.m0 + exchange.m3) / 2,
    offset,
    netRtt,
    uncertainty: netRtt / 2,
  };
}

export function buildClockCalibration(preRaw, postRaw, kind) {
  const required = WEB06_THRESHOLDS.calibration.exchangesPerBoundary;
  if (!Array.isArray(preRaw) || !Array.isArray(postRaw)
    || preRaw.length !== required || postRaw.length !== required) {
    throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:exchange-count");
  }
  const compute = kind === "driver-page"
    ? computeDriverPageExchange
    : kind === "main-worker"
      ? computeMainWorkerExchange
      : null;
  if (!compute) throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:kind");
  const pre = preRaw.map(compute);
  const post = postRaw.map(compute);
  const select = (exchanges) => [...exchanges].sort((left, right) =>
    left.uncertainty - right.uncertainty || left.midpoint - right.midpoint)[0];
  const selectedPre = select(pre);
  const selectedPost = select(post);
  const maxUncertainty = WEB06_THRESHOLDS.calibration.uncertaintyMaxMs;
  if (selectedPre.uncertainty > maxUncertainty || selectedPost.uncertainty > maxUncertainty) {
    throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:uncertainty");
  }
  const drift = Math.abs(selectedPost.offset - selectedPre.offset);
  if (drift > WEB06_THRESHOLDS.calibration.offsetDriftMaxMs) {
    throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:offset-drift");
  }
  if (!(selectedPost.midpoint > selectedPre.midpoint)) {
    throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:boundary-order");
  }
  return { kind, pre, post, selectedPre, selectedPost, drift };
}

export function interpolateClockCalibration(calibration, at) {
  if (!Number.isFinite(at)) throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:interpolation-time");
  const { selectedPre: pre, selectedPost: post } = calibration;
  const fraction = (at - pre.midpoint) / (post.midpoint - pre.midpoint);
  if (fraction < 0 || fraction > 1) throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:outside-boundary");
  const offset = pre.offset + (post.offset - pre.offset) * fraction;
  const uncertainty = pre.uncertainty + (post.uncertainty - pre.uncertainty) * fraction;
  if (uncertainty < 0 || uncertainty > WEB06_THRESHOLDS.calibration.uncertaintyMaxMs) {
    throw new Error("SETUP_INVALID_CLOCK_CALIBRATION:interpolated-uncertainty");
  }
  return { fraction, offset, uncertainty };
}

export function correctDriverTimestamp(driverAt, calibration, pageReferenceAt) {
  const point = interpolateClockCalibration(calibration, pageReferenceAt);
  return { correctedAt: driverAt + point.offset, ...point };
}

export function correctWorkerTimestamp(workerAt, calibration, mainReferenceAt) {
  const point = interpolateClockCalibration(calibration, mainReferenceAt);
  return { correctedAt: workerAt - point.offset, ...point };
}

function observerModeLocalHardRed(mode) {
  if (mode?.hardRedBindingValid !== true) return false;
  const atOrAbove = (values, threshold) => Array.isArray(values)
    && values.some((value) => Number.isFinite(value) && value >= threshold);
  return atOrAbove(mode.sentinelCallbacksMs, WEB06_THRESHOLDS.observer.sentinelCallbackExclusiveMaxMs)
    || atOrAbove(
      mode.sentinelTotalPerEventMs,
      WEB06_THRESHOLDS.observer.sentinelTotalPerEventExclusiveMaxMs,
    )
    || atOrAbove(mode.collectorCallbacksMs, WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(mode.mainObserverCallbacksMs, WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(mode.workerCollectorCallbacksMs, WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)
    || atOrAbove(
      mode.instrumentationAddedLongTasksMs,
      WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs,
    );
}

function observerModeHardRedExpected(mode) {
  const parserRed = [mode?.commonVerdict, mode?.internalVerdict]
    .some((verdict) => ["RED", "RED_BEHAVIOR"].includes(verdict));
  return mode?.behaviorRedObserved === true
    || (mode?.hardRedBindingValid === true && (parserRed || observerModeLocalHardRed(mode)));
}

export function evaluateObserverOverhead(attempts) {
  if (!Array.isArray(attempts) || attempts.length > WEB06_THRESHOLDS.observer.maximumTripletAttempts) {
    throw new Error("WEB06_OBSERVER_ATTEMPT_CAP_EXCEEDED");
  }
  const modeNames = ["product", "minimal", "full"];
  const hardRedFor = (attempt, modeName) =>
    observerModeHardRedExpected(attempt?.[modeName]);
  const hardRedDeclarationViolations = attempts.flatMap((attempt) => modeNames
    .filter((modeName) => attempt?.[modeName]
      && attempt[modeName].hardRedObserved !== hardRedFor(attempt, modeName))
    .map((modeName) => `${attempt.attemptId}:${modeName}-hard-red-observation-mismatch`));
  const valid = attempts.filter((attempt) => attempt.valid === true);
  if (valid.length > WEB06_THRESHOLDS.observer.requiredTriplets) {
    throw new Error("WEB06_OBSERVER_VALID_TRIPLET_RETRY_FORBIDDEN");
  }
  if (valid.length < WEB06_THRESHOLDS.observer.requiredTriplets) {
    const preservedUnpairedReds = attempts.flatMap((attempt) => ["product", "minimal", "full"]
      .filter((modeName) => hardRedFor(attempt, modeName))
      .map((modeName) => `${attempt.attemptId}:${modeName}`));
    return {
      pass: false,
      status: attempts.length === WEB06_THRESHOLDS.observer.maximumTripletAttempts
        ? "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS"
        : "INCOMPLETE",
      violations: [
        ...hardRedDeclarationViolations,
        ...preservedUnpairedReds.map((identity) => `${identity}-unpaired-valid-red`),
      ].sort(),
      preservedUnpairedReds,
    };
  }
  const violations = [...hardRedDeclarationViolations];
  for (const attempt of attempts.filter((item) => item.valid !== true)) {
    for (const modeName of modeNames) {
      if (hardRedFor(attempt, modeName)) {
        violations.push(`${attempt.attemptId}:${modeName}-unpaired-valid-red`);
      }
    }
  }
  const durationFields = [
    ["sentinel-callback", "sentinelCallbacksMs"],
    ["sentinel-total", "sentinelTotalPerEventMs"],
    ["collector-callback", "collectorCallbacksMs"],
    ["underlying-long-task", "underlyingLongTasksMs"],
    ["instrumentation-added-long-task", "instrumentationAddedLongTasksMs"],
  ];
  for (const attempt of valid) {
    for (const modeName of modeNames) {
      const mode = attempt[modeName];
      if (!mode || typeof mode !== "object") {
        violations.push(`${attempt.attemptId}:${modeName}-mode-shape`);
        continue;
      }
      if (mode.measurementValid !== true) violations.push(`${attempt.attemptId}:${modeName}-measurement-invalid`);
      if (!Array.isArray(mode.samples) || mode.samples.length === 0
        || mode.samples.some((value) => !Number.isFinite(value) || value < 0)) {
        violations.push(`${attempt.attemptId}:${modeName}-samples-shape`);
      }
      for (const [label, field] of durationFields) {
        const values = mode[field];
        if (!Array.isArray(values) || (field === "sentinelCallbacksMs" && values.length === 0)
          || values.some((value) => !Number.isFinite(value) || value < 0)) {
          violations.push(`${attempt.attemptId}:${modeName}-${label}-shape`);
        }
      }
      if (!Number.isSafeInteger(mode.commonEventCount) || mode.commonEventCount <= 0
        || !Array.isArray(mode.sentinelTotalPerEventMs)
        || mode.sentinelTotalPerEventMs.length !== mode.commonEventCount) {
        violations.push(`${attempt.attemptId}:${modeName}-sentinel-total-count`);
      }
      if (!Number.isSafeInteger(mode.interactionWindowCount) || mode.interactionWindowCount <= 0
        || !Array.isArray(mode.sentinelTotalPerWindowMs)
        || mode.sentinelTotalPerWindowMs.length !== mode.interactionWindowCount
        || mode.sentinelTotalPerWindowMs.some((value) => !Number.isFinite(value) || value < 0)) {
        violations.push(`${attempt.attemptId}:${modeName}-sentinel-window-total-count`);
      }
      if (modeName === "product" && mode.instrumentationAddedLongTasksMs?.length !== 0) {
        violations.push(`${attempt.attemptId}:product-instrumentation-added-long-task-attribution`);
      }
      if (mode.callbackLedgerOverflowCount !== 0
        || !Number.isSafeInteger(mode.callbackLedgerCount)
        || mode.callbackLedgerCount !== mode.sentinelAccountedCallbackCount) {
        violations.push(`${attempt.attemptId}:${modeName}-sentinel-callback-conservation`);
      }
    }
  }
  const pooled = (mode) => valid.flatMap((attempt) => Array.isArray(attempt[mode]?.samples) ? attempt[mode].samples : []);
  const compare = (leftMode, rightMode) => {
    const left = distributionSummary(pooled(leftMode));
    const right = distributionSummary(pooled(rightMode));
    return {
      pair: `${leftMode}-vs-${rightMode}`,
      medianDelta: Math.abs(left.median - right.median),
      p95Delta: Math.abs(left.p95 - right.p95),
      maxDelta: Math.abs(left.max - right.max),
    };
  };
  const distributionsValid = modeNames.every((mode) => pooled(mode).length > 0
    && pooled(mode).every((value) => Number.isFinite(value) && value >= 0));
  const comparisons = distributionsValid
    ? [
        compare("product", "minimal"),
        compare("minimal", "full"),
      ]
    : [];
  for (const comparison of comparisons) {
    if (comparison.medianDelta > WEB06_THRESHOLDS.observer.absolutePooledMedianDeltaMs) violations.push(`${comparison.pair}:median`);
    if (comparison.p95Delta > WEB06_THRESHOLDS.observer.absolutePooledP95DeltaMs) violations.push(`${comparison.pair}:p95`);
    if (comparison.maxDelta > WEB06_THRESHOLDS.observer.absolutePooledMaxDeltaMs) violations.push(`${comparison.pair}:max`);
  }
  const slots = valid.map((attempt) => attempt.counterbalanceSlot);
  if (new Set(slots).size !== 5
    || slots.some((slot) => !hasOwn(WEB06_OBSERVER_COUNTERBALANCE, String(slot)))) {
    violations.push("counterbalance-slot-set");
  }
  const contexts = valid.map((attempt) => attempt.freshContextId);
  if (contexts.some((context) => typeof context !== "string" || !context) || new Set(contexts).size !== contexts.length) {
    violations.push("fresh-context-identity");
  }
  const allModeContexts = valid.flatMap((attempt) => attempt.modeContextIds ?? []);
  if (allModeContexts.length !== valid.length * 3
    || allModeContexts.some((context) => typeof context !== "string" || !context)
    || new Set(allModeContexts).size !== allModeContexts.length) {
    violations.push("fresh-mode-context-identities");
  }
  for (const attempt of valid) {
    if (!hasOwn(WEB06_OBSERVER_COUNTERBALANCE, String(attempt.counterbalanceSlot))
      || !sameArray(attempt.modeOrder, WEB06_OBSERVER_COUNTERBALANCE[attempt.counterbalanceSlot])) {
      violations.push(`${attempt.attemptId}:counterbalance-order`);
    }
    if (attempt.modeFixedBeforePageLoad !== true) violations.push(`${attempt.attemptId}:mode-not-fixed-before-load`);
    if (!Array.isArray(attempt.modeContextIds) || attempt.modeContextIds.length !== 3
      || new Set(attempt.modeContextIds).size !== 3) {
      violations.push(`${attempt.attemptId}:mode-context-identities`);
    }
    if (!(Array.isArray(attempt.product?.samples) && Array.isArray(attempt.minimal?.samples)
      && Array.isArray(attempt.full?.samples)
      && attempt.product.samples.length === attempt.minimal.samples.length
      && attempt.minimal.samples.length === attempt.full.samples.length)) {
      violations.push(`${attempt.attemptId}:common-sample-count`);
    }
    const commonDigests = [attempt.product?.commonEquivalenceDigest, attempt.minimal?.commonEquivalenceDigest, attempt.full?.commonEquivalenceDigest];
    if (!commonDigests.every(isSha256) || new Set(commonDigests).size !== 1) {
      violations.push(`${attempt.attemptId}:common-equivalence-digest`);
    }
    const internalDigests = [attempt.minimal?.internalEquivalenceDigest, attempt.full?.internalEquivalenceDigest];
    if (!internalDigests.every(isSha256) || internalDigests[0] !== internalDigests[1]) {
      violations.push(`${attempt.attemptId}:internal-equivalence-digest`);
    }
    const commonVerdicts = [attempt.product?.commonVerdict, attempt.minimal?.commonVerdict, attempt.full?.commonVerdict];
    if (!commonVerdicts.every((verdict) => ["PASS", "RED", "RED_BEHAVIOR"].includes(verdict))
      || new Set(commonVerdicts).size !== 1) {
      violations.push(`${attempt.attemptId}:common-verdict-disagreement`);
    }
    const internalVerdicts = [attempt.minimal?.internalVerdict, attempt.full?.internalVerdict];
    if (!internalVerdicts.every((verdict) => ["PASS", "RED", "RED_BEHAVIOR"].includes(verdict))
      || internalVerdicts[0] !== internalVerdicts[1]) {
      violations.push(`${attempt.attemptId}:internal-verdict-disagreement`);
    }
    const environments = [attempt.product?.environmentManifestSha256,
      attempt.minimal?.environmentManifestSha256, attempt.full?.environmentManifestSha256];
    if (!environments.every(isSha256) || new Set(environments).size !== 1) {
      violations.push(`${attempt.attemptId}:environment-drift`);
    }
    const environmentIds = [attempt.product?.environmentId,
      attempt.minimal?.environmentId, attempt.full?.environmentId];
    if (!environmentIds.every(isSha256) || new Set(environmentIds).size !== 1) {
      violations.push(`${attempt.attemptId}:environment-id-drift`);
    }
    for (const [modeName, mode] of modeNames.map((name) => [name, attempt[name]])) {
      if (!mode || typeof mode !== "object") continue;
      if (Array.isArray(mode.sentinelCallbacksMs)
        && mode.sentinelCallbacksMs.some((value) => value >= WEB06_THRESHOLDS.observer.sentinelCallbackExclusiveMaxMs)) violations.push(`${attempt.attemptId}:${modeName}-sentinel-callback`);
      if (Array.isArray(mode.sentinelTotalPerEventMs)
        && mode.sentinelTotalPerEventMs.some((value) => value >= WEB06_THRESHOLDS.observer.sentinelTotalPerEventExclusiveMaxMs)) violations.push(`${attempt.attemptId}:${modeName}-sentinel-total`);
      if (Array.isArray(mode.collectorCallbacksMs)
        && mode.collectorCallbacksMs.some((value) => value >= WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)) violations.push(`${attempt.attemptId}:${modeName}-collector-callback`);
      if (Array.isArray(mode.instrumentationAddedLongTasksMs)
        && mode.instrumentationAddedLongTasksMs.some((value) => value >= WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs)) violations.push(`${attempt.attemptId}:${modeName}-instrumentation-added-long-task`);
    }
  }
  violations.sort();
  return { pass: violations.length === 0, status: violations.length ? "RED" : "PASS", comparisons, violations };
}

function sameArray(left, right) {
  return Array.isArray(left) && Array.isArray(right)
    && left.length === right.length && left.every((value, index) => value === right[index]);
}

export function isSha256(value) {
  return typeof value === "string" && SHA256_RE.test(value);
}

const frozenContract = validateFrozenContract();
if (!frozenContract.ok) {
  throw new Error(`WEB06_INVALID_FROZEN_CONTRACT:${frozenContract.errors.join(",")}`);
}
