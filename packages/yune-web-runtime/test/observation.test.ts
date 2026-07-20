import { describe, expect, it } from "vitest";

import type {
  Web06RuntimeObservation,
  Web06RuntimeObservationFailure,
  Web06RuntimeObservationMode,
  Web06RuntimeResponseJsonCopy,
  Web06RuntimeSpan,
} from "../src/observation.js";
import { YuneWebResponseError } from "../src/response.js";
import { YuneWebRuntime } from "../src/runtime.js";
import { FakeYuneWebModule } from "./fake-module.js";

const modes = ["off", "minimal", "full"] as const satisfies readonly Web06RuntimeObservationMode[];

function responsePayload(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    handled: true,
    commits: ["你"],
    context: null,
    status: null,
    ...overrides,
  };
}

interface ObservationCapture {
  observation: Web06RuntimeObservation;
  spans: Web06RuntimeSpan[];
  jsonCopies: Web06RuntimeResponseJsonCopy[];
  failures: Web06RuntimeObservationFailure[];
  clockCalls(): number;
}

function captureObservation(
  mode: Web06RuntimeObservationMode,
  overrides: Partial<Web06RuntimeObservation> = {},
): ObservationCapture {
  const spans: Web06RuntimeSpan[] = [];
  const jsonCopies: Web06RuntimeResponseJsonCopy[] = [];
  const failures: Web06RuntimeObservationFailure[] = [];
  let now = 0;
  let clockCallCount = 0;
  const observation: Web06RuntimeObservation = {
    mode,
    now() {
      clockCallCount += 1;
      now += 0.25;
      return now;
    },
    onSpan(span) {
      spans.push(span);
    },
    onResponseJsonCopy(copy) {
      jsonCopies.push(copy);
    },
    onFailure(failure) {
      failures.push(failure);
    },
    ...overrides,
  };
  return {
    observation,
    spans,
    jsonCopies,
    failures,
    clockCalls: () => clockCallCount,
  };
}

function initializedRuntime(
  fake: FakeYuneWebModule,
  observation: Web06RuntimeObservation,
): YuneWebRuntime {
  return YuneWebRuntime.init(fake, {
    sharedDataDir: "/rime/shared",
    userDataDir: "/rime/user",
    schemaId: "yune_web_luna",
    web06Observation: observation,
  });
}

function callSymbols(fake: FakeYuneWebModule): string[] {
  return fake.callTrace().map(({ symbol }) => symbol);
}

function assertPointerFreeObservationShape(capture: ObservationCapture): void {
  for (const span of capture.spans) {
    expect(Object.keys(span).sort()).toEqual([
      "finishedAt",
      "operation",
      "outcome",
      "stage",
      "startedAt",
    ]);
  }
  for (const copy of capture.jsonCopies) {
    expect(Object.keys(copy).sort()).toEqual(["json", "operation"]);
  }
  for (const failure of capture.failures) {
    expect(Object.keys(failure).sort()).toEqual([
      "error",
      "hook",
      "operation",
      "stage",
    ]);
  }
}

function runAllOperations(mode: Web06RuntimeObservationMode) {
  const fake = new FakeYuneWebModule();
  const payloads = ["key", "select", "delete", "page", "ai"].map((commit) =>
    responsePayload({ commits: [commit] }),
  );
  const pointers = payloads.map((payload) => fake.response(payload));
  [
    fake.processKeyResult,
    fake.selectCandidateResult,
    fake.deleteCandidateResult,
    fake.flipPageResult,
    fake.stageAiResult,
  ] = pointers;
  fake.deployResult = 2;
  fake.customizeResult = 0;
  fake.setOptionResult = 1;
  fake.setAiEnabledResult = 1;
  const capture = captureObservation(mode);
  const runtime = initializedRuntime(fake, capture.observation);

  const results = [
    runtime.processKey(65, 4),
    runtime.selectCandidate(2),
    runtime.deleteCandidate(1),
    runtime.flipPage(true),
    runtime.deploy(),
    runtime.customize("schema", "key", "value"),
    runtime.setOption("ascii_mode", true),
    runtime.setAiEnabled(true),
    runtime.stageAi(),
  ];
  runtime.cleanup();

  return { capture, fake, payloads, pointers, results };
}

describe("WEB-06 runtime observation modes", () => {
  it("preserves every runtime result and native call order in off, minimal, and full modes", () => {
    const runs = Object.fromEntries(modes.map((mode) => [mode, runAllOperations(mode)])) as Record<
      Web06RuntimeObservationMode,
      ReturnType<typeof runAllOperations>
    >;
    const baseline = runs.off;

    for (const mode of modes) {
      const run = runs[mode];
      expect(run.results).toEqual(baseline.results);
      expect(run.fake.callTrace()).toEqual(baseline.fake.callTrace());
      expect(run.fake.freedResponses()).toEqual(run.pointers);
      expect(run.capture.failures).toEqual([]);
      assertPointerFreeObservationShape(run.capture);
    }

    expect(runs.off.capture.spans).toEqual([]);
    expect(runs.off.capture.jsonCopies).toEqual([]);
    expect(runs.off.capture.clockCalls()).toBe(0);
    expect(runs.minimal.capture.jsonCopies).toEqual([]);
    expect(runs.minimal.capture.spans.map(({ operation, stage }) => [operation, stage])).toEqual([
      ["init", "abi-call"],
      ["process-key", "abi-call"],
      ["select-candidate", "abi-call"],
      ["delete-candidate", "abi-call"],
      ["flip-page", "abi-call"],
      ["deploy", "abi-call"],
      ["customize", "abi-call"],
      ["set-option", "abi-call"],
      ["set-ai-enabled", "abi-call"],
      ["stage-ai", "abi-call"],
      ["cleanup", "abi-call"],
    ]);
    expect(runs.full.capture.jsonCopies.map(({ json }) => json)).toEqual(
      runs.full.payloads.map((payload) => JSON.stringify(payload)),
    );
  });

  it("records full response stages in strict non-overlapping order without retaining pointers", () => {
    const fake = new FakeYuneWebModule();
    const exactJson = ` {"handled":true,"commits":["exact"],"context":null,"status":null}\n`;
    const responsePtr = fake.responseText(exactJson, false);
    fake.processKeyResult = responsePtr;
    const capture = captureObservation("full");
    const runtime = initializedRuntime(fake, capture.observation);

    expect(runtime.processKey(65)).toEqual(responsePayload({ handled: false, commits: ["exact"] }));

    const processSpans = capture.spans.filter(({ operation }) => operation === "process-key");
    expect(processSpans.map(({ stage }) => stage)).toEqual([
      "abi-call",
      "response-json-accessor",
      "response-byte-extraction",
      "response-json-parse",
      "response-shape-decode",
      "response-handled-accessor",
      "response-free",
    ]);
    for (const [index, span] of processSpans.entries()) {
      expect(span.finishedAt).toBeGreaterThanOrEqual(span.startedAt);
      if (index > 0) {
        expect(span.startedAt).toBeGreaterThanOrEqual(processSpans[index - 1].finishedAt);
      }
    }
    expect(capture.jsonCopies).toEqual([{ operation: "process-key", json: exactJson }]);
    expect(callSymbols(fake)).toEqual([
      "yune_web_init",
      "yune_web_process_key",
      "yune_web_response_json",
      "UTF8ToString",
      "yune_web_response_handled",
      "yune_web_free_response",
    ]);
    expect(fake.freedResponses()).toEqual([responsePtr]);
    assertPointerFreeObservationShape(capture);
  });

  it("freezes the selected mode at init and never toggles it mid-runtime", () => {
    const fake = new FakeYuneWebModule();
    fake.processKeyResult = fake.response(responsePayload());
    const capture = captureObservation("minimal");
    const runtime = initializedRuntime(fake, capture.observation);

    capture.observation.mode = "full";
    runtime.processKey(65);

    expect(capture.spans.map(({ stage }) => stage)).toEqual(["abi-call", "abi-call"]);
    expect(capture.jsonCopies).toEqual([]);
  });
});

type FailureScenario = {
  name: string;
  exactErrorIdentity?: boolean;
  prepare(fake: FakeYuneWebModule): { responsePtr: number; expectedError: unknown };
};

const failureScenarios: FailureScenario[] = [
  {
    name: "null response",
    prepare(fake) {
      fake.processKeyResult = 0;
      return {
        responsePtr: 0,
        expectedError: new YuneWebResponseError("YuneWeb adapter returned null response"),
      };
    },
  },
  {
    name: "null response JSON",
    prepare(fake) {
      const responsePtr = fake.responseWithJsonPointer(0);
      fake.processKeyResult = responsePtr;
      return {
        responsePtr,
        expectedError: new YuneWebResponseError("YuneWeb adapter returned null response JSON"),
      };
    },
  },
  {
    name: "response JSON accessor failure",
    exactErrorIdentity: true,
    prepare(fake) {
      const expectedError = new Error("response JSON accessor failed");
      const responsePtr = fake.response(responsePayload());
      fake.processKeyResult = responsePtr;
      fake.fail("yune_web_response_json", expectedError);
      return { responsePtr, expectedError };
    },
  },
  {
    name: "response extraction failure",
    prepare(fake) {
      const responsePtr = fake.responseWithJsonPointer(999_999);
      fake.processKeyResult = responsePtr;
      return {
        responsePtr,
        expectedError: new Error("Unexpected missing fake string pointer: 999999"),
      };
    },
  },
  {
    name: "malformed JSON",
    prepare(fake) {
      const responsePtr = fake.responseText("{not json");
      fake.processKeyResult = responsePtr;
      return {
        responsePtr,
        expectedError: new YuneWebResponseError("YuneWeb adapter returned malformed response JSON"),
      };
    },
  },
  {
    name: "shape decode failure",
    prepare(fake) {
      const responsePtr = fake.response(responsePayload({ handled: "yes" }));
      fake.processKeyResult = responsePtr;
      return {
        responsePtr,
        expectedError: new YuneWebResponseError("YuneWeb response handled field must be boolean"),
      };
    },
  },
  {
    name: "handled accessor failure",
    exactErrorIdentity: true,
    prepare(fake) {
      const expectedError = new Error("handled accessor failed");
      const responsePtr = fake.response(responsePayload());
      fake.processKeyResult = responsePtr;
      fake.fail("yune_web_response_handled", expectedError);
      return { responsePtr, expectedError };
    },
  },
  {
    name: "free failure",
    exactErrorIdentity: true,
    prepare(fake) {
      const expectedError = new Error("free failed");
      const responsePtr = fake.response(responsePayload());
      fake.processKeyResult = responsePtr;
      fake.fail("yune_web_free_response", expectedError);
      return { responsePtr, expectedError };
    },
  },
  {
    name: "ABI failure",
    exactErrorIdentity: true,
    prepare(fake) {
      const expectedError = new Error("ABI failed");
      fake.fail("yune_web_process_key", expectedError);
      return { responsePtr: 0, expectedError };
    },
  },
];

function errorSignature(error: unknown): { constructor: unknown; name?: string; message?: string } {
  if (!(error instanceof Error)) {
    return { constructor: undefined };
  }
  return {
    constructor: error.constructor,
    name: error.name,
    message: error.message,
  };
}

function runFailure(mode: Web06RuntimeObservationMode, scenario: FailureScenario) {
  const fake = new FakeYuneWebModule();
  const { responsePtr, expectedError } = scenario.prepare(fake);
  const capture = captureObservation(mode);
  const runtime = initializedRuntime(fake, capture.observation);
  let thrown: unknown;
  try {
    runtime.processKey(65);
  } catch (error) {
    thrown = error;
  }
  return { capture, expectedError, fake, responsePtr, thrown };
}

describe("WEB-06 response failure and pointer ownership equivalence", () => {
  for (const scenario of failureScenarios) {
    it(`preserves ${scenario.name} across off, minimal, and full modes`, () => {
      const runs = Object.fromEntries(modes.map((mode) => [mode, runFailure(mode, scenario)])) as Record<
        Web06RuntimeObservationMode,
        ReturnType<typeof runFailure>
      >;
      const baseline = runs.off;

      for (const mode of modes) {
        const run = runs[mode];
        expect(errorSignature(run.thrown)).toEqual(errorSignature(baseline.thrown));
        expect(errorSignature(run.thrown)).toEqual(errorSignature(run.expectedError));
        if (scenario.exactErrorIdentity === true) {
          expect(run.thrown).toBe(run.expectedError);
        }
        expect(run.fake.callTrace()).toEqual(baseline.fake.callTrace());
        expect(run.fake.freedResponses()).toEqual(
          run.responsePtr === 0 ? [] : [run.responsePtr],
        );
        expect(run.capture.failures).toEqual([]);
        assertPointerFreeObservationShape(run.capture);
      }
    });
  }

  it("marks thrown component spans while still freeing the owned response exactly once", () => {
    const malformed = runFailure("full", failureScenarios[4]);
    expect(
      malformed.capture.spans
        .filter(({ operation }) => operation === "process-key")
        .map(({ stage, outcome }) => [stage, outcome]),
    ).toEqual([
      ["abi-call", "returned"],
      ["response-json-accessor", "returned"],
      ["response-byte-extraction", "returned"],
      ["response-json-parse", "threw"],
      ["response-free", "returned"],
    ]);
    expect(malformed.fake.freedResponses()).toEqual([malformed.responsePtr]);
  });
});

describe("WEB-06 observer failure isolation", () => {
  it("reports throwing span and JSON-copy callbacks without altering the app result", () => {
    const fake = new FakeYuneWebModule();
    const responsePtr = fake.response(responsePayload());
    fake.processKeyResult = responsePtr;
    const failures: Web06RuntimeObservationFailure[] = [];
    const capture = captureObservation("full", {
      onSpan() {
        throw new Error("span collector failed");
      },
      onResponseJsonCopy() {
        throw new Error("JSON collector failed");
      },
      onFailure(failure) {
        failures.push(failure);
      },
    });
    const runtime = initializedRuntime(fake, capture.observation);

    expect(runtime.processKey(65)).toEqual(responsePayload());
    expect(fake.freedResponses()).toEqual([responsePtr]);
    expect(failures.some(({ hook }) => hook === "span")).toBe(true);
    expect(failures.some(({ hook }) => hook === "response-json-copy")).toBe(true);
  });

  it("never lets a throwing failure callback replace the native error", () => {
    const nativeError = new Error("handled failed");
    const fake = new FakeYuneWebModule();
    const responsePtr = fake.response(responsePayload());
    fake.processKeyResult = responsePtr;
    fake.fail("yune_web_response_handled", nativeError);
    const capture = captureObservation("full", {
      onSpan() {
        throw new Error("span collector failed");
      },
      onFailure() {
        throw new Error("failure collector failed");
      },
    });
    const runtime = initializedRuntime(fake, capture.observation);

    expect(() => runtime.processKey(65)).toThrow(nativeError);
    expect(fake.freedResponses()).toEqual([responsePtr]);
  });

  it("reports broken clocks out of band while preserving results and ownership", () => {
    const fake = new FakeYuneWebModule();
    const responsePtr = fake.response(responsePayload());
    fake.processKeyResult = responsePtr;
    const failures: Web06RuntimeObservationFailure[] = [];
    const capture = captureObservation("full", {
      now() {
        throw new Error("clock failed");
      },
      onFailure(failure) {
        failures.push(failure);
      },
    });
    const runtime = initializedRuntime(fake, capture.observation);

    expect(runtime.processKey(65)).toEqual(responsePayload());
    expect(fake.freedResponses()).toEqual([responsePtr]);
    expect(capture.spans).toEqual([]);
    expect(failures.length).toBeGreaterThan(0);
    expect(failures.every(({ hook }) => hook === "clock-start")).toBe(true);
  });
});
