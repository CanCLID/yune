/**
 * WEB-06 frozen Phase-0 measurement contract.
 *
 * This module is intentionally independent of Playwright and the application.
 * Browser collectors emit raw receipts; this module owns the immutable
 * scenario/action registry, exact arithmetic, and fail-closed verdict rules.
 */

export const WEB06_METRIC_CONTRACT_VERSION = "web06-metric-v1";
export const WEB06_SCENARIO_REGISTRY_VERSION = "web06-scenarios-v1";

const SHA256_RE = /^[0-9a-f]{64}$/;

function deepFreeze(value) {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) deepFreeze(child);
    Object.freeze(value);
  }
  return value;
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
  setOption: { classification: "stateful-barrier", supersedable: false },
  selectSchema: { classification: "stateful-barrier", supersedable: false },
  getUserdbSnapshot: { classification: "read-only", supersedable: false },
  importUserdb: { classification: "stateful-barrier", supersedable: false },
  processKey: { classification: "native-key", supersedable: "printable-only" },
  stageAi: { classification: "adapter-only", supersedable: false, defaultOff: true },
  selectCandidate: { classification: "stateful-barrier", supersedable: false },
  deleteCandidate: { classification: "stateful-barrier", supersedable: false },
  flipPage: { classification: "stateful-barrier", supersedable: false },
  customize: { classification: "stateful-barrier", supersedable: false },
  customizeValue: { classification: "stateful-barrier", supersedable: false },
  deploy: { classification: "stateful-barrier", supersedable: false },
  deployCacheSnapshot: { classification: "read-only", supersedable: false },
  invalidateDeployCache: { classification: "stateful-barrier", supersedable: false },
  injectedAssetsManifest: { classification: "read-only", supersedable: false },
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
  const contract = ACTION_REGISTRY[kind];
  if (!contract) throw new Error(`WEB06_UNKNOWN_ACTION:${kind}`);
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
  });
}

const PRIMARY_JYUTPING = "ngodeigungsijigaahaidoumaaigangeihaaijansougeoi";
const LONG_JYUTPING = "taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng";
const RAPID_LUNA = "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong";
export const EMPTY_USERDB_FIXTURE_SHA256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

const CORRECTION_STEPS = [
  ...printableSteps("correction-pre", "nri", "sustained60", "pre-delete"),
  specialKeyStep("correction-backspace-1", "Backspace", "BackSpace", { subcase: "delete-1" }),
  specialKeyStep("correction-backspace-2", "Backspace", "BackSpace", { subcase: "delete-2" }),
  ...printableSteps("correction-resume", "ri", "sustained60", "resumed-input"),
  specialKeyStep("correction-commit", " ", "space", { subcase: "commit" }),
];

const SELECTION_PAGING_STEPS = [
  ...printableSteps("selection-ni", "ni", "sustained60", "luna-digit-select"),
  digitSelectStep("selection-ni-digit-2", "2", { subcase: "luna-digit-select" }),
  ...printableSteps("paging-zhongguo", "zhongguo", "sustained60", "luna-page-roundtrip"),
  specialKeyStep("paging-page-down", "PageDown", "Page_Down", { subcase: "luna-page-roundtrip" }),
  specialKeyStep("paging-page-up", "PageUp", "Page_Up", { subcase: "luna-page-roundtrip" }),
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
  digitSelectStep("learned-commit", "1", { subcase: "learn-and-persist", includeUserdbRefresh: true }),
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
  dictionaryExclude: [],
  isCangjie5: true,
});

function liveOptionDescriptors({ asciiMode = false, extendedCharset = false } = {}) {
  return SHIFT_TAP_LIVE_OPTION_ACTIONS.map(([option, defaultValue]) => ({
    kind: "setOption",
    args: [
      option,
      option === "ascii_mode" ? asciiMode
        : option === "extended_charset" ? extendedCharset
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
    const liveOptions = liveOptionDescriptors();
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
  directActionStep("extended-error-target", "customizeValue", ["default", "web06/injected_error", "true"], {
    subcase: "error-barrier",
    domEventType: "submit",
    control: "[data-yune-control-customize-value-form]",
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
    steps: repeatPhrase("rapid-jyutping", PRIMARY_JYUTPING, "sustained60"),
  }),
  "rapid-long-jyutping": scenario("rapid-long-jyutping", {
    schema: "jyut6ping3",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "sustained60",
    steps: repeatPhrase("rapid-long-jyutping", LONG_JYUTPING, "sustained60"),
  }),
  "rapid-luna": scenario("rapid-luna", {
    schema: "luna_pinyin",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "sustained60",
    steps: repeatPhrase("rapid-luna", RAPID_LUNA, "sustained60"),
  }),
  "burst-jyutping": scenario("burst-jyutping", {
    schema: "jyut6ping3",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "burst-cycle",
    steps: burstPhrase("burst-jyutping", PRIMARY_JYUTPING),
  }),
  "burst-luna": scenario("burst-luna", {
    schema: "luna_pinyin",
    authority: "latency-and-page-shape-only",
    binding: true,
    cadence: "burst-cycle",
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
    schema: ["luna_pinyin"],
    authority: "oracle-exact",
    authorityFixture: [
      "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json",
      "crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-lattice.json",
    ],
    authorityCases: [
      {
        fixtureCase: "luna-pinyin-actions.json::select_ni_second",
        flow: "ni -> digit 2",
        capturedCommitText: "擬",
      },
      {
        fixtureCase: "luna-pinyin-lattice.json::sentence_lattice_zhongguo",
        flow: "zhongguo -> PageDown -> PageUp",
      },
    ],
    binding: true,
    cadence: "sustained60",
    steps: SELECTION_PAGING_STEPS,
  }),
  "burst-action-map": scenario("burst-action-map", {
    schema: "jyut6ping3",
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
    authorityFixture: "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m14-userdb.json",
    binding: true,
    cadence: "sustained60",
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

export const EVENT_ACTION_RULES = deepFreeze([
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
  { id: "candidate-delete-long-press", event: "pointerdown", condition: "800ms hold without move/cancel", classification: "mapped-action(s) after long-press", actions: ["deleteCandidate"] },
  { id: "page-button", event: "click", classification: "mapped-action(s)", actions: ["flipPage"] },
  { id: "live-option-control", event: "change", classification: "mapped-action(s)", actions: ["setOption"] },
  { id: "schema-control", event: "change", classification: "mapped-action(s)", actions: ["selectSchema"] },
  { id: "userdb-refresh-background", event: "background", classification: "mapped-action(s)", actions: ["getUserdbSnapshot"] },
  { id: "userdb-import-control", event: "change", classification: "mapped-action(s)", actions: ["importUserdb"] },
  { id: "ai-second-pass-background", event: "background", classification: "mapped-action(s)", actions: ["stageAi"] },
  { id: "customize-control", event: "change", classification: "mapped-action(s)", actions: ["customize"] },
  { id: "customize-value-control", event: "submit", classification: "mapped-action(s)", actions: ["customizeValue"] },
  { id: "deploy-control", event: "click", classification: "mapped-action(s)", actions: ["deploy"] },
  { id: "deploy-cache-read-background", event: "background", classification: "mapped-action(s)", actions: ["deployCacheSnapshot"] },
  { id: "deploy-cache-invalidate-control", event: "click", classification: "mapped-action(s)", actions: ["invalidateDeployCache"] },
  { id: "injected-assets-read-background", event: "background", classification: "mapped-action(s)", actions: ["injectedAssetsManifest"] },
]);

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
  const row = SCENARIO_REGISTRY[scenarioId];
  if (!row) throw new Error(`WEB06_UNKNOWN_SCENARIO:${scenarioId}`);
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
  }
  const exactCounts = {
    "existing-normal-guard": [47, 0],
    "rapid-jyutping": [141, 2],
    "rapid-long-jyutping": [156, 2],
    "rapid-luna": [177, 2],
    "burst-jyutping": [141, 2],
    "burst-luna": [177, 2],
    correction: [5, 3],
    "selection-paging": [10, 3],
    "burst-action-map": [3, 5],
    "fifo-pressure-barriers": [16, 7],
    "learned-row": [11, 1],
    "fair-peer-short": [2, 1],
    "peer-sustained": [59, 1],
    "extended-scheduler-barriers": [5, 10],
  };
  for (const [id, [covering, terminal]] of Object.entries(exactCounts)) {
    const row = SCENARIO_REGISTRY[id];
    if (row.expectedCoveringSamples !== covering || row.expectedTerminalSamples !== terminal) {
      errors.push(`FROZEN_COUNT_MISMATCH:${id}:${row.expectedCoveringSamples}+${row.expectedTerminalSamples}`);
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
  const retained = attempts.map((attempt) => ({ ...attempt, verdict: classifyAttempt(attempt) }));
  const measured = retained.filter((attempt) => attempt.verdict === "PASS" || attempt.verdict.startsWith("RED"));
  const replaceable = retained.filter((attempt) => attempt.verdict === "SETUP_INVALID" || attempt.verdict === "NO_VERDICT_INVALID_CADENCE");
  const complete = measured.length === WEB06_THRESHOLDS.attempts.requiredValid;
  if (measured.length > WEB06_THRESHOLDS.attempts.requiredValid) {
    throw new Error("WEB06_MEASURED_ROUND_RETRY_FORBIDDEN");
  }
  return {
    status: complete
      ? measured.some((attempt) => attempt.verdict.startsWith("RED")) ? "COMPLETE_WITH_RED" : "COMPLETE_GREEN"
      : attempts.length === WEB06_THRESHOLDS.attempts.maximum
        ? "SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS"
        : "INCOMPLETE",
    retained,
    measuredCount: measured.length,
    replaceableCount: replaceable.length,
  };
}

function requireExchangeFields(exchange, names) {
  for (const name of names) {
    if (!Number.isFinite(exchange[name])) throw new Error(`SETUP_INVALID_CLOCK_CALIBRATION:${name}`);
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
  if (preRaw?.length !== required || postRaw?.length !== required) {
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

export function evaluateObserverOverhead(attempts) {
  if (!Array.isArray(attempts) || attempts.length > WEB06_THRESHOLDS.observer.maximumTripletAttempts) {
    throw new Error("WEB06_OBSERVER_ATTEMPT_CAP_EXCEEDED");
  }
  const valid = attempts.filter((attempt) => attempt.valid === true);
  if (valid.length > WEB06_THRESHOLDS.observer.requiredTriplets) {
    throw new Error("WEB06_OBSERVER_VALID_TRIPLET_RETRY_FORBIDDEN");
  }
  if (valid.length < WEB06_THRESHOLDS.observer.requiredTriplets) {
    return {
      pass: false,
      status: attempts.length === WEB06_THRESHOLDS.observer.maximumTripletAttempts
        ? "SETUP_NO_GO_INSUFFICIENT_VALID_TRIPLETS"
        : "INCOMPLETE",
      violations: [],
    };
  }
  const pooled = (mode) => valid.flatMap((attempt) => attempt[mode].samples);
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
  const comparisons = [compare("product", "minimal"), compare("minimal", "full")];
  const violations = [];
  for (const comparison of comparisons) {
    if (comparison.medianDelta > WEB06_THRESHOLDS.observer.absolutePooledMedianDeltaMs) violations.push(`${comparison.pair}:median`);
    if (comparison.p95Delta > WEB06_THRESHOLDS.observer.absolutePooledP95DeltaMs) violations.push(`${comparison.pair}:p95`);
    if (comparison.maxDelta > WEB06_THRESHOLDS.observer.absolutePooledMaxDeltaMs) violations.push(`${comparison.pair}:max`);
  }
  const slots = valid.map((attempt) => attempt.counterbalanceSlot);
  if (new Set(slots).size !== 5 || slots.some((slot) => !WEB06_OBSERVER_COUNTERBALANCE[slot])) {
    violations.push("counterbalance-slot-set");
  }
  const contexts = valid.map((attempt) => attempt.freshContextId);
  if (contexts.some((context) => typeof context !== "string" || !context) || new Set(contexts).size !== contexts.length) {
    violations.push("fresh-context-identity");
  }
  for (const attempt of valid) {
    if (!sameArray(attempt.modeOrder, WEB06_OBSERVER_COUNTERBALANCE[attempt.counterbalanceSlot])) {
      violations.push(`${attempt.attemptId}:counterbalance-order`);
    }
    if (attempt.modeFixedBeforePageLoad !== true) violations.push(`${attempt.attemptId}:mode-not-fixed-before-load`);
    if (!(attempt.product.samples.length === attempt.minimal.samples.length
      && attempt.minimal.samples.length === attempt.full.samples.length)) {
      violations.push(`${attempt.attemptId}:common-sample-count`);
    }
    if (!attempt.product.commonCountDigest
      || new Set([attempt.product.commonCountDigest, attempt.minimal.commonCountDigest, attempt.full.commonCountDigest]).size !== 1) {
      violations.push(`${attempt.attemptId}:common-count-digest`);
    }
    if (!attempt.minimal.internalCountDigest || attempt.minimal.internalCountDigest !== attempt.full.internalCountDigest) {
      violations.push(`${attempt.attemptId}:internal-count-digest`);
    }
    if (new Set([attempt.product.commonVerdict, attempt.minimal.commonVerdict, attempt.full.commonVerdict]).size !== 1) {
      violations.push(`${attempt.attemptId}:common-verdict-disagreement`);
    }
    if (attempt.minimal.internalVerdict !== attempt.full.internalVerdict) violations.push(`${attempt.attemptId}:internal-verdict-disagreement`);
    for (const mode of [attempt.product, attempt.minimal, attempt.full]) {
      if (mode.sentinelCallbacksMs?.some((value) => value >= WEB06_THRESHOLDS.observer.sentinelCallbackExclusiveMaxMs)) violations.push(`${attempt.attemptId}:sentinel-callback`);
      if (mode.sentinelTotalPerEventMs?.some((value) => value >= WEB06_THRESHOLDS.observer.sentinelTotalPerEventExclusiveMaxMs)) violations.push(`${attempt.attemptId}:sentinel-total`);
      if (mode.collectorCallbacksMs?.some((value) => value >= WEB06_THRESHOLDS.observer.collectorCallbackExclusiveMaxMs)) violations.push(`${attempt.attemptId}:collector-callback`);
      if (mode.instrumentationLongTasksMs?.some((value) => value >= WEB06_THRESHOLDS.observer.rejectInstrumentationLongTaskAtOrAboveMs)) violations.push(`${attempt.attemptId}:instrumentation-long-task`);
    }
  }
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
