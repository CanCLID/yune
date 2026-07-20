import { test, expect, chromium, type Page } from "@playwright/test";

import { createHash } from "node:crypto";
import { statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import { mkdir, readFile, rm, stat } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  summarizeComparatorRatios,
  writeComparatorEvidence,
  type ComparatorResource,
  type ComparatorSample,
  type ComparatorWorkerMemory,
} from "./startup-benchmark/comparator-metrics";
import {
  comparatorDomTupleDigest,
  comparatorPeerLogicalInputIds,
  comparatorPeerCadenceMs,
  comparatorPeerPageSize,
  comparatorPinnedMyRimeCommit,
  comparatorSelectorManifest,
  parseComparatorIdentityManifest,
  validateCandidateObservation,
  validateEndpointEvidence,
  type ComparatorEndpointEvidence,
  type ComparatorIdentityManifest,
  type ComparatorDomTuple,
  type ComparatorPageSizeSetup,
  type ComparatorStableObservation,
} from "./startup-benchmark/comparator-endpoint";
import {
  comparatorEventCount,
  comparatorEventsSince,
  ensureYuneComparatorMeasurementPageSize,
  ensureYuneComparatorPageSize,
  exactYuneDiagnostic,
  installComparatorEndpointObserver,
  waitForStableCandidateEndpoint,
  waitForStableCommitEndpoint,
} from "./startup-benchmark/comparator-browser-endpoint";
import { appSchemaId, type StartupSchema } from "./startup-benchmark/scenarios";
import type { WasmMemorySnapshot } from "./startup-benchmark/metrics";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(__dirname, "..");
const resultRoot = path.join(
  __dirname,
  "results",
  process.env.YUNE_WEB_COMPARATOR_RESULT_ROOT ?? "yune-web-vs-my-rime-baseline",
);
const phaseName = process.env.YUNE_WEB_COMPARATOR_PHASE ?? "baseline";
const phaseDir = path.join(resultRoot, phaseName);
const trackedDist = path.join(appRoot, "dist");
const publicDist = path.join(appRoot, "public-demo", "dist");
const includeMyRime = process.env.YUNE_WEB_COMPARATOR_INCLUDE_MY_RIME === "1";
const myRimeUrl = process.env.YUNE_WEB_COMPARATOR_MY_RIME_URL ?? "https://my-rime.vercel.app/";
const myRimeBuild = process.env.YUNE_WEB_COMPARATOR_MY_RIME_BUILD ?? "unverified-live";
const myRimeScenarioId = process.env.YUNE_WEB_COMPARATOR_MY_RIME_SCENARIO_ID
  ?? (myRimeBuild === "unverified-live" ? "my-rime-live-unverified" : "my-rime-pinned");
const identityManifestPath = process.env.YUNE_WEB_COMPARATOR_IDENTITY_MANIFEST;
const sampleCount = Math.max(1, Math.floor(Number(process.env.YUNE_WEB_COMPARATOR_SAMPLES ?? "5")));
const readyTimeoutMs = 120_000;

interface ComparatorScenario {
  id: string;
  app: "yune-web" | "my-rime";
  build: string;
  schema: "luna_pinyin" | "jyutping";
  runtimeSchema?: StartupSchema;
  input: string;
  publicDemo?: boolean;
}

const yuneScenarios: ComparatorScenario[] = [
  {
    id: "yune-tracked",
    app: "yune-web",
    build: "tracked-dist",
    schema: "luna_pinyin",
    runtimeSchema: "luna_pinyin",
    input: "ni",
  },
  {
    id: "yune-tracked",
    app: "yune-web",
    build: "tracked-dist",
    schema: "jyutping",
    runtimeSchema: "jyut6ping3_mobile",
    input: "nei",
  },
  {
    id: "yune-public-demo",
    app: "yune-web",
    build: "public-demo-dist",
    schema: "luna_pinyin",
    runtimeSchema: "luna_pinyin",
    input: "ni",
    publicDemo: true,
  },
  {
    id: "yune-public-demo",
    app: "yune-web",
    build: "public-demo-dist",
    schema: "jyutping",
    runtimeSchema: "jyut6ping3_mobile",
    input: "nei",
    publicDemo: true,
  },
];

const myRimeScenarios: ComparatorScenario[] = [
  {
    id: myRimeScenarioId,
    app: "my-rime",
    build: myRimeBuild,
    schema: "luna_pinyin",
    input: "ni",
  },
  {
    id: myRimeScenarioId,
    app: "my-rime",
    build: myRimeBuild,
    schema: "jyutping",
    input: "nei",
  },
];

test.describe("YUNE WEB COMPARATOR benchmark", () => {
  test.skip(process.env.YUNE_WEB_COMPARATOR_BASELINE !== "1", "Set YUNE_WEB_COMPARATOR_BASELINE=1 to run this opt-in benchmark.");
  test.setTimeout(60 * 60 * 1000);

  test("YUNE WEB COMPARATOR browser baseline", async () => {
    await assertDistExists(trackedDist, "tracked apps/yune-web dist");
    await assertDistExists(publicDist, "public-demo dist");
    const identityManifest = await loadComparatorIdentityManifest();
    const trackedServer = await startStaticServer(trackedDist);
    const publicServer = await startStaticServer(publicDist);
    const samples: ComparatorSample[] = [];
    try {
      for (const scenario of yuneScenarios) {
        for (let index = 0; index < sampleCount; index += 1) {
          samples.push(await runYuneScenarioSample(
            scenario,
            index,
            scenario.publicDemo ? publicServer.url : trackedServer.url,
            scenario.publicDemo ? publicDist : trackedDist,
            identityForScenario(identityManifest?.manifest, scenario),
            identityManifest?.sha256,
          ));
        }
      }
      if (includeMyRime) {
        for (const scenario of myRimeScenarios) {
          for (let index = 0; index < sampleCount; index += 1) {
            samples.push(await runMyRimeScenarioSample(
              scenario,
              index,
              identityForScenario(identityManifest?.manifest, scenario),
              identityManifest?.sha256,
            ));
          }
        }
      }
    } finally {
      await trackedServer.close();
      await publicServer.close();
    }
    await writeComparatorEvidence(phaseDir, samples);
    expect(samples.length).toBeGreaterThan(0);
  });
});

test.describe("WEB06 comparator endpoint and alignment contract", () => {
  test.describe.configure({ timeout: 60_000 });

  test("uses the reviewed My RIME editable selector", () => {
    expect(comparatorSelectorManifest["my-rime"].editable).toBe("#container textarea");
  });

  test("does not resolve complete ni on the earlier n and 那 render", () => {
    const earlierN = candidateObservation("yune-web", "n", "那");
    const failures = validateCandidateObservation(earlierN, "ni");
    expect(failures).toContain("candidate-composition-is-not-complete-input");
    expect(failures).toContain("yune-final-input-diagnostic-is-not-coherent-with-dom");
  });

  test("external browser observer rejects the old body heuristic and waits past n", async ({ page }) => {
    await page.route("http://comparator.test/**", async route => {
      await route.fulfill({
        contentType: "text/html",
        body: [
          "<style>textarea,.n-popover,.n-menu,.n-menu-item,button{display:block;width:200px;height:24px}</style>",
          "<aside data-unrelated-copy>ni</aside>",
          "<div id='container'><textarea></textarea></div>",
          "<div class='n-popover'>",
          "<span class='preedit'>n</span>",
          "<div class='n-menu'>",
          "<div class='n-menu-item'><div class='n-menu-item-content--selected'>1 那</div></div>",
          "<div class='n-menu-item'>2 倪</div><div class='n-menu-item'>3 尼</div>",
          "<div class='n-menu-item'>4 泥</div><div class='n-menu-item'>5 擬</div>",
          "<div class='n-menu-item'>6 妳</div>",
          "</div>",
          "<button disabled>previous</button><button>next</button>",
          "</div>",
        ].join(""),
      });
    });
    await page.goto("http://comparator.test/?schemaId=luna_pinyin");
    await installComparatorEndpointObserver(page, "my-rime");
    const oldBodyHeuristicWouldStop = await page.evaluate(() => {
      const body = document.body.innerText;
      return body.includes("ni") && /(?:^|\n)\s*1\s+\S+/.test(body);
    });
    expect(oldBodyHeuristicWouldStop).toBe(true);
    const input = page.locator("#container textarea");
    await input.focus();
    const beforeEvent = await comparatorEventCount(page);
    await page.keyboard.type("ni", { delay: comparatorPeerCadenceMs });
    await page.evaluate(() => {
      setTimeout(() => {
        const preedit = document.querySelector(".preedit");
        const candidate = document.querySelector(".n-menu-item-content--selected");
        if (preedit) {
          preedit.textContent = "ni";
        }
        if (candidate) {
          candidate.textContent = "1 你";
        }
      }, 25);
    });
    const endpoint = await waitForStableCandidateEndpoint(
      page,
      "my-rime",
      "ni",
      beforeEvent,
    );
    const inputEvents = await comparatorEventsSince(page, beforeEvent);
    expect(endpoint.secondRaf.composition).toBe("ni");
    expect(endpoint.secondRaf.candidates[0]?.text).toBe("你");
    expect(endpoint.initial.revision).toBeGreaterThan(endpoint.event.revisionBeforeEvent);
    expect(inputEvents.map(event => event.key).join("")).toBe("ni");
    expect((inputEvents[1]?.timeStamp ?? 0) - (inputEvents[0]?.timeStamp ?? 0))
      .toBeGreaterThanOrEqual(48);
  });

  test("Yune diagnostic cross-check does not define the common external stop", async ({ page }) => {
    await page.route("http://yune-comparator.test/**", async route => {
      const row = (index: number, text: string, highlighted = false) =>
        "<div class='candidate-row" + (highlighted ? " highlighted" : "") + "'>"
        + "<span class='candidate-index'>" + index + "</span>"
        + "<span class='candidate-text'>" + text + "</span>"
        + "<span class='candidate-note'></span></div>";
      await route.fulfill({
        contentType: "text/html",
        body: [
          "<style>textarea,.candidate-panel,.candidates,.candidate-row,button,[data-yune-status]{display:block;width:200px;height:24px}</style>",
          "<textarea class='yd-input-area'></textarea>",
          "<div class='candidate-panel'><div class='candidate-preedit'>n</div>",
          "<div class='candidates'>",
          row(1, "那", true), row(2, "倪"), row(3, "尼"),
          row(4, "泥"), row(5, "擬"), row(6, "妳"),
          "</div><button class='page-nav' disabled>previous</button><button class='page-nav'>next</button></div>",
          "<section data-yune-status><span data-yune-status-schema data-yune-status-schema-id='luna_pinyin'></span>",
          "<span data-yune-status-composing='true'></span></section>",
        ].join(""),
      });
    });
    await page.goto("http://yune-comparator.test/?schemaId=luna_pinyin");
    await installComparatorEndpointObserver(page, "yune-web");
    const input = page.locator("textarea.yd-input-area");
    await input.focus();
    const beforeEvent = await comparatorEventCount(page);
    await page.keyboard.type("ni", { delay: comparatorPeerCadenceMs });
    await page.evaluate(() => {
      setTimeout(() => {
        const preedit = document.querySelector(".candidate-preedit");
        const candidate = document.querySelector(".candidate-row.highlighted .candidate-text");
        if (preedit) {
          preedit.textContent = "ni";
        }
        if (candidate) {
          candidate.textContent = "你";
        }
      }, 25);
      setTimeout(() => {
        (window as Window & { diagnosticInstalledAt?: number }).diagnosticInstalledAt = performance.now();
        document.documentElement.dataset.yunePerfDiagnostics = JSON.stringify([{
          input: "ni",
          renderedInput: "ni",
          renderRevision: 1,
          candidateCount: 6,
          totalCandidateCount: 6,
          firstCandidateText: "你",
        }]);
      }, 200);
    });
    const endpoint = await waitForStableCandidateEndpoint(
      page,
      "yune-web",
      "ni",
      beforeEvent,
      0,
    );
    const diagnosticInstalledAt = await page.evaluate(
      () => (window as Window & { diagnosticInstalledAt?: number }).diagnosticInstalledAt,
    );
    const currentEndpointRevision = await page.evaluate(
      () => (window as Window & { __web06ComparatorObserver?: { revision: number } })
        .__web06ComparatorObserver?.revision,
    );
    expect(endpoint.secondRaf.composition).toBe("ni");
    expect(endpoint.yuneDiagnostic?.firstCandidateText).toBe("你");
    expect(endpoint.secondRaf.observedAt).toBeLessThan(diagnosticInstalledAt ?? 0);
    expect(currentEndpointRevision).toBe(endpoint.secondRaf.revision);
  });

  test("requires a byte-identical tuple through both post-event animation frames", () => {
    const unstable = candidateObservation("my-rime", "ni", "你");
    unstable.firstRaf = { ...unstable.firstRaf, digest: "changed-between-frames" };
    expect(validateCandidateObservation(unstable, "ni"))
      .toContain("dom-digest-changed-during-double-raf");
  });

  test("freezes n KeyN then i KeyI and a post-endpoint Space barrier", () => {
    const sample = comparatorContractSample(
      "my-rime",
      "my-rime-peer",
      "peer-dist",
      comparatorIdentity(),
      50,
      35,
    );
    expect(validateEndpointEvidence(sample.endpoint, "ni", "你", "my-rime")).toEqual([]);
    if (!sample.endpoint) {
      throw new Error("test fixture endpoint missing");
    }
    sample.endpoint.commit.event = {
      ...sample.endpoint.commit.event,
      key: "1",
      code: "Digit1",
      timeStamp: sample.endpoint.candidate.secondRaf.observedAt - 1,
    };
    const failures = validateEndpointEvidence(sample.endpoint, "ni", "你", "my-rime");
    expect(failures).toContain("commit-event-is-not-space-key-code");
    expect(failures).toContain("commit-event-precedes-coherent-candidate-endpoint");
  });

  test("requires clean source provenance and real compiled table prism and reverse hashes", () => {
    const invalid = structuredClone(comparatorIdentity()) as unknown as {
      peer: {
        artifactSourceCommit: string;
        artifactSourceTree?: string;
        repositoryCommit: string;
        sourceTreeState: string;
        compiledHashes: Record<string, string>;
      };
    };
    delete invalid.peer.artifactSourceTree;
    invalid.peer.repositoryCommit = "6".repeat(40);
    invalid.peer.sourceTreeState = "dirty";
    invalid.peer.compiledHashes.reverse = "none";
    expect(() => parseComparatorIdentityManifest(JSON.stringify(invalid)))
      .toThrow(
        /peer-artifact-source-tree.*peer-repository-commit-is-not-artifact-source-commit.*peer-source-tree-state.*peer-compiled-reverse/,
      );
  });

  test("omits ratios when the essay identity negative control is incomplete", () => {
    const identity = comparatorIdentity();
    identity.logicalInputs = identity.logicalInputs.filter(input => input.id !== "essay");
    const rows = summarizeComparatorRatios([
      comparatorContractSample("yune-web", "yune-tracked", "tracked-dist", identity, 40, 30),
      comparatorContractSample("my-rime", "my-rime-peer", "peer-dist", identity, 50, 35),
    ]);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.packageAlignment).toBe("DATA_CONFOUNDED");
    expect(rows[0]?.ratioStatus).toBe("OMITTED");
    expect(rows[0]?.reasons.some(reason => reason.includes("essay"))).toBe(true);
    expect(rows[0]).not.toHaveProperty("p95InputToCandidateRatio");
    expect(rows[0]).not.toHaveProperty("p95CommitRatio");
  });

  test("publishes ratios only for complete aligned endpoint and data identities", () => {
    const identity = comparatorIdentity();
    const yuneSamples = Array.from({ length: 5 }, (_, sampleIndex) => comparatorContractSample(
      "yune-web",
      "yune-tracked",
      "tracked-dist",
      identity,
      40,
      30,
      sampleIndex,
    ));
    const peerSamples = Array.from({ length: 5 }, (_, sampleIndex) => comparatorContractSample(
      "my-rime",
      "my-rime-peer",
      "peer-dist",
      identity,
      50,
      35,
      sampleIndex,
    ));
    const rows = summarizeComparatorRatios([
      ...yuneSamples,
      ...peerSamples,
    ]);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.packageAlignment).toBe("PROVED");
    expect(rows[0]?.endpointAlignment).toBe("PROVED");
    expect(rows[0]?.ratioStatus).toBe("PUBLISHED");
    expect(rows[0]?.p95InputToCandidateRatio).toBe(0.8);
    expect(rows[0]?.p95CommitRatio).toBeCloseTo(30 / 35);
  });

  test("omits an otherwise complete ratio with fewer than five fresh-profile rounds", () => {
    const identity = comparatorIdentity();
    const rows = summarizeComparatorRatios([
      comparatorContractSample("yune-web", "yune-tracked", "tracked-dist", identity, 40, 30),
      comparatorContractSample("my-rime", "my-rime-peer", "peer-dist", identity, 50, 35),
    ]);
    expect(rows[0]?.ratioStatus).toBe("OMITTED");
    expect(rows[0]?.reasons).toContain(
      "yune-binding-rounds-are-not-exactly-five-contiguous-fresh-profile-samples",
    );
    expect(rows[0]?.reasons).toContain(
      "peer-binding-rounds-are-not-exactly-five-contiguous-fresh-profile-samples",
    );
  });

  test("refuses a ratio without the recorded Yune six-row UI deploy proof", () => {
    const identity = comparatorIdentity();
    const yuneSamples = Array.from({ length: 5 }, (_, sampleIndex) => comparatorContractSample(
      "yune-web", "yune-tracked", "tracked-dist", identity, 40, 30, sampleIndex,
    ));
    yuneSamples[2]!.pageSizeSetup = undefined;
    const peerSamples = Array.from({ length: 5 }, (_, sampleIndex) => comparatorContractSample(
      "my-rime", "my-rime-peer", "peer-dist", identity, 50, 35, sampleIndex,
    ));
    const rows = summarizeComparatorRatios([...yuneSamples, ...peerSamples]);
    expect(rows[0]?.ratioStatus).toBe("OMITTED");
    expect(rows[0]?.reasons).toContain(
      "endpoint:endpoint-yune-six-row-setup-provenance-is-missing-or-invalid",
    );
  });

  test("refuses a ratio without the pinned peer source and observed frozen selector contract", () => {
    const identity = comparatorIdentity();
    identity.peer.upstreamPinnedCommit = "2".repeat(40);
    const peer = comparatorContractSample(
      "my-rime",
      "my-rime-peer",
      "peer-dist",
      identity,
      50,
      35,
    );
    for (const tuple of [
      peer.endpoint?.candidate.initial,
      peer.endpoint?.candidate.firstRaf,
      peer.endpoint?.candidate.secondRaf,
    ]) {
      if (tuple) {
        tuple.selectorManifestId = "unreviewed-peer-selector";
        tuple.digest = comparatorDomTupleDigest(tuple);
      }
    }
    const rows = summarizeComparatorRatios([
      comparatorContractSample("yune-web", "yune-tracked", "tracked-dist", identity, 40, 30),
      peer,
    ]);
    expect(rows[0]?.ratioStatus).toBe("OMITTED");
    expect(rows[0]?.reasons).toContain("endpoint:endpoint-peer-source-is-not-the-pinned-my-rime-commit");
    expect(rows[0]?.reasons).toContain("endpoint:endpoint-frozen-selector-manifest-not-proved");
  });
});

function comparatorContractSample(
  app: "yune-web" | "my-rime",
  scenarioId: string,
  build: string,
  identity: ComparatorIdentityManifest,
  inputToCandidateMs: number,
  commitMs: number,
  sampleIndex = 0,
): ComparatorSample {
  const candidate = candidateObservation(app, "ni", "你");
  return {
    scenarioId,
    app,
    build,
    schema: "luna_pinyin",
    schemaInput: "ni",
    sampleIndex,
    url: "http://127.0.0.1/comparator",
    readyToInputMs: 1,
    cadenceMs: comparatorPeerCadenceMs,
    inputToCandidateMs,
    commitMs,
    firstCandidateText: "你",
    committedValue: "你",
    endpoint: {
      inputEvents: [
        {
          ordinal: 1,
          type: "keydown",
          key: "n",
          code: "KeyN",
          timeStamp: candidate.event.timeStamp - comparatorPeerCadenceMs,
          revisionBeforeEvent: 0,
        },
        candidate.event,
      ],
      candidate,
      commit: commitObservation(app, "你"),
    },
    ...(app === "yune-web" ? { pageSizeSetup: pageSizeSetupProof() } : {}),
    identity,
    identityManifestSha256: "d".repeat(64),
    resources: [],
    workerUrls: [],
    consoleErrors: [],
  };
}

function pageSizeSetupProof(): ComparatorPageSizeSetup {
  return {
    contractVersion: "web06-page-size-setup-v1",
    requiredRows: 6,
    initial: {
      uiValue: "6",
      localStorageValue: "6",
      persistedConfigValue: "6",
      deployStatus: "idle",
      persistenceDiagnosticCount: 4,
    },
    actions: [
      {
        ordinal: 1,
        fromUiValue: "6",
        targetUiValue: "7",
        interaction: {
          kind: "keyboard",
          key: "ArrowRight",
          control: "preferences-page-size-range",
        },
        deployStatus: "success",
        loadingComplete: true,
        localStorageValue: "7",
        persistedConfigValue: "7",
        persistenceDiagnosticIndex: 5,
        engineProbe: {
          input: "ni",
          candidateRows: 7,
          candidates: ["你", "擬", "尼", "泥", "呢", "妳", "妮"],
          pageIndex: 0,
          buttonCount: 2,
          previousDisabled: true,
          nextDisabled: false,
          resetKey: "Escape",
          resetEmpty: true,
        },
      },
      {
        ordinal: 2,
        fromUiValue: "7",
        targetUiValue: "6",
        interaction: {
          kind: "keyboard",
          key: "ArrowLeft",
          control: "preferences-page-size-range",
        },
        deployStatus: "success",
        loadingComplete: true,
        localStorageValue: "6",
        persistedConfigValue: "6",
        persistenceDiagnosticIndex: 6,
        engineProbe: {
          input: "ni",
          candidateRows: 6,
          candidates: ["你", "擬", "尼", "泥", "呢", "妳"],
          pageIndex: 0,
          buttonCount: 2,
          previousDisabled: true,
          nextDisabled: false,
          resetKey: "Escape",
          resetEmpty: true,
        },
      },
    ],
    final: {
      uiValue: "6",
      localStorageValue: "6",
      persistedConfigValue: "6",
      deployStatus: "success",
      loadingComplete: true,
    },
    measurementPage: {
      initial: {
        uiValue: "6",
        localStorageValue: "6",
        persistedConfigValue: "6",
        deployStatus: "idle",
        persistenceDiagnosticCount: 4,
      },
      actions: [
        {
          ordinal: 1,
          fromUiValue: "6",
          targetUiValue: "7",
          interaction: {
            kind: "keyboard",
            key: "ArrowRight",
            control: "preferences-page-size-range",
          },
          deployStatus: "success",
          loadingComplete: true,
          localStorageValue: "7",
          persistedConfigValue: "7",
          persistenceDiagnosticIndex: 5,
        },
        {
          ordinal: 2,
          fromUiValue: "7",
          targetUiValue: "6",
          interaction: {
            kind: "keyboard",
            key: "ArrowLeft",
            control: "preferences-page-size-range",
          },
          deployStatus: "success",
          loadingComplete: true,
          localStorageValue: "6",
          persistedConfigValue: "6",
          persistenceDiagnosticIndex: 6,
        },
      ],
      final: {
        uiValue: "6",
        localStorageValue: "6",
        persistedConfigValue: "6",
        deployStatus: "success",
        loadingComplete: true,
      },
    },
    engineProof: {
      candidateRows: 6,
      pageIndex: 0,
      buttonCount: 2,
      previousDisabled: true,
      nextDisabled: false,
    },
  };
}

function candidateObservation(
  app: "yune-web" | "my-rime",
  composition: string,
  candidateText: string,
): ComparatorStableObservation {
  const tuple = comparatorTuple(app, {
    revision: 2,
    observedAt: 80,
    composition,
    candidates: [
      { label: "1", text: candidateText, comment: "" },
      { label: "2", text: "倪", comment: "" },
      { label: "3", text: "尼", comment: "" },
      { label: "4", text: "泥", comment: "" },
      { label: "5", text: "擬", comment: "" },
      { label: "6", text: "妳", comment: "" },
    ],
    candidateSurfaceCount: 1,
    page: { index: 0, buttonCount: 2, previousDisabled: true, nextDisabled: false },
    highlightedIndex: 0,
    caret: {
      selectorCount: 1,
      value: "",
      selectionStart: 0,
      selectionEnd: 0,
      selectionDirection: "none",
      active: true,
      visible: true,
      disabled: false,
    },
    status: {
      schemaId: "luna_pinyin",
      composing: true,
      surfaceVisible: true,
      digest: "candidate-status",
    },
  });
  return {
    event: {
      ordinal: 2,
      type: "keydown",
      key: composition.at(-1) ?? "",
      code: "KeyI",
      timeStamp: 70,
      revisionBeforeEvent: 1,
    },
    initial: tuple,
    firstRaf: { ...tuple, observedAt: 90 },
    secondRaf: { ...tuple, observedAt: 100 },
    ...(app === "yune-web" ? {
      yuneDiagnostic: {
        index: 1,
        input: composition,
        renderedInput: composition,
        renderRevision: 2,
        candidateCount: 6,
        totalCandidateCount: 6,
        firstCandidateText: candidateText,
      },
    } : {}),
  };
}

function commitObservation(
  app: "yune-web" | "my-rime",
  committedValue: string,
): ComparatorStableObservation {
  const tuple = comparatorTuple(app, {
    revision: 4,
    observedAt: 120,
    composition: "",
    candidates: [],
    candidateSurfaceCount: 0,
    page: { index: null, buttonCount: 0, previousDisabled: null, nextDisabled: null },
    highlightedIndex: -1,
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
      composing: false,
      surfaceVisible: false,
      digest: "commit-status",
    },
  });
  return {
    event: {
      ordinal: 3,
      type: "keydown",
      key: " ",
      code: "Space",
      timeStamp: 110,
      revisionBeforeEvent: 3,
    },
    initial: tuple,
    firstRaf: { ...tuple, observedAt: 130 },
    secondRaf: { ...tuple, observedAt: 140 },
  };
}

function comparatorTuple(
  app: "yune-web" | "my-rime",
  tuple: Omit<ComparatorDomTuple, "contractVersion" | "selectorManifestId" | "digest">,
): ComparatorDomTuple {
  const result: ComparatorDomTuple = {
    contractVersion: "web06-comparator-endpoint-v1",
    selectorManifestId: comparatorSelectorManifest[app].id,
    ...tuple,
    digest: "",
  };
  result.digest = comparatorDomTupleDigest(result);
  return result;
}

function comparatorIdentity(): ComparatorIdentityManifest {
  const yunePackageHash = "a".repeat(64);
  const peerPackageHash = "b".repeat(64);
  const logicalHash = "c".repeat(64);
  const reproducibleSide = (side: "yune" | "peer") => {
    const packageHash = side === "yune" ? yunePackageHash : peerPackageHash;
    const artifactSourceCommit = side === "yune" ? "1".repeat(40) : "2".repeat(40);
    return {
      repositoryCommit: artifactSourceCommit,
      upstreamPinnedCommit: side === "yune" ? artifactSourceCommit : comparatorPinnedMyRimeCommit,
      artifactSourceCommit,
      artifactSourceTree: "5".repeat(40),
      sourceTreeState: "clean" as const,
      artifactSha256: packageHash,
      generatedManifestSha256: packageHash,
      completeArtifactManifestSha256: packageHash,
      buildCommand: "sealed-" + side + "-build-v1",
      packageManager: {
        name: (side === "yune" ? "npm" : "pnpm") as "npm" | "pnpm",
        version: "10.0.0",
        lockSha256: packageHash,
        integrityManifestSha256: packageHash,
      },
      toolchain: {
        nodeVersion: "v22.0.0",
        emscriptenVersion: "4.0.23",
        emscriptenCommit: "3".repeat(40),
        compilerVersion: "clang 21",
      },
      resolvedRecipes: [{
        id: "luna-package",
        repository: "https://example.test/luna",
        commit: "4".repeat(40),
        logicalBytesSha256: logicalHash,
      }],
      compiledHashes: {
        table: packageHash,
        prism: packageHash,
        reverse: packageHash,
        "data-model": "none",
        runtime: packageHash,
      },
    };
  };
  return {
    version: "web06-peer-data-v1",
    yune: reproducibleSide("yune"),
    peer: reproducibleSide("peer"),
    logicalInputs: comparatorPeerLogicalInputIds.map(id => id === "grammar-model"
      ? { id, yuneSha256: "none", peerSha256: "none", explicitNone: true }
      : { id, yuneSha256: logicalHash, peerSha256: logicalHash }),
    effectiveConfiguration: { yuneSha256: logicalHash, peerSha256: logicalHash },
    freshEmptyUserdb: true,
    sameEndpointObserver: true,
  };
}

async function runYuneScenarioSample(
  scenario: ComparatorScenario,
  sampleIndex: number,
  baseUrl: string,
  distRoot: string,
  identity: ComparatorIdentityManifest | undefined,
  identityManifestSha256: string | undefined,
): Promise<ComparatorSample> {
  const userDataDir = await freshUserDataDir(scenario, sampleIndex);
  const context = await chromium.launchPersistentContext(userDataDir, {
    headless: true,
    viewport: { width: 1365, height: 900 },
    locale: "zh-HK",
  });
  try {
    await context.addInitScript(({ schema }) => {
      localStorage.setItem("activeSchema", schema);
      localStorage.setItem("uiLanguage", "en");
      localStorage.setItem("enableAI", "false");
      localStorage.setItem("pageSize", "6");
    }, { schema: appSchemaId(scenario.runtimeSchema ?? "luna_pinyin") });
    const url = `${baseUrl}/?benchmark=yune-web-comparator&schema=${encodeURIComponent(scenario.runtimeSchema ?? "luna_pinyin")}&scenario=${encodeURIComponent(scenario.id)}&sample=${sampleIndex}`;
    const setupPage = await context.newPage();
    const setupConsoleErrors = captureConsoleErrors(setupPage);
    await loadAndWaitYuneReady(
      setupPage,
      url + "&phase=page-size-setup",
      scenario.runtimeSchema ?? "luna_pinyin",
    );
    const pageSizeSetupBeforeReload = await ensureYuneComparatorPageSize(setupPage);
    await setupPage.close();

    const page = await context.newPage();
    const measurementConsoleErrors = captureConsoleErrors(page);
    const startedAt = Date.now();
    await loadAndWaitYuneReady(
      page,
      url + "&phase=measurement",
      scenario.runtimeSchema ?? "luna_pinyin",
    );
    const readyAt = Date.now();
    const measurementPageSize = await ensureYuneComparatorMeasurementPageSize(page);
    if (measurementPageSize.final.uiValue !== String(comparatorPeerPageSize)
        || measurementPageSize.final.localStorageValue !== String(comparatorPeerPageSize)
        || measurementPageSize.final.persistedConfigValue !== String(comparatorPeerPageSize)
        || !measurementPageSize.final.loadingComplete) {
      throw new Error("Yune comparator measurement page did not reload at the frozen six-row setting");
    }
    const readyStartup = await yuneStartupMarker(page);
    const input = page.locator("textarea.yd-input-area");
    await input.fill("");
    await installComparatorEndpointObserver(page, "yune-web");
    const beforePerfCount = await yunePerfCount(page);
    const beforeInputEventCount = await comparatorEventCount(page);
    await input.click();
    await page.keyboard.type(scenario.input, { delay: comparatorPeerCadenceMs });
    const candidateEndpoint = await waitForStableCandidateEndpoint(
      page,
      "yune-web",
      scenario.input,
      beforeInputEventCount,
      beforePerfCount,
    );
    const candidateDiagnosticIndex = candidateEndpoint.yuneDiagnostic?.index;
    const inputEvents = await comparatorEventsSince(page, beforeInputEventCount);
    if (candidateDiagnosticIndex === undefined) {
      throw new Error("Yune comparator candidate endpoint did not retain its exact final-input diagnostic");
    }
    const candidatePerf = await exactYuneDiagnostic(page, candidateDiagnosticIndex);
    const firstCandidateText = candidateEndpoint.secondRaf.candidates[0]?.text;
    const selectedCandidateText = candidateEndpoint.secondRaf.candidates[
      candidateEndpoint.secondRaf.highlightedIndex
    ]?.text;
    if (!selectedCandidateText) {
      throw new Error("Yune comparator candidate endpoint did not expose a highlighted candidate");
    }
    const beforeCommitEventCount = await comparatorEventCount(page);
    await page.keyboard.press("Space");
    const commitEndpoint = await waitForStableCommitEndpoint(
      page,
      "yune-web",
      selectedCandidateText,
      beforeCommitEventCount,
    );
    const committedValue = commitEndpoint.secondRaf.caret.value;
    const endpoint = { inputEvents, candidate: candidateEndpoint, commit: commitEndpoint };
    assertMeasuredEndpoint(endpoint, scenario, committedValue);
    const commitPerf = await latestYunePerf(page);
    let resources = [
      ...await collectPageResources(page),
      ...await collectWorkerResources(page),
    ];
    resources = appendYuneSyntheticResources(resources, readyStartup, distRoot, url);
    return {
      scenarioId: scenario.id,
      app: scenario.app,
      build: scenario.build,
      schema: scenario.schema,
      schemaInput: scenario.input,
      sampleIndex,
      url,
      readyToInputMs: readyAt - startedAt,
      cadenceMs: comparatorPeerCadenceMs,
      inputToCandidateMs: candidateEndpoint.secondRaf.observedAt - candidateEndpoint.event.timeStamp,
      commitMs: commitEndpoint.secondRaf.observedAt - commitEndpoint.event.timeStamp,
      firstCandidateText,
      committedValue,
      endpoint,
      pageSizeSetup: {
        ...pageSizeSetupBeforeReload,
        measurementPage: measurementPageSize,
        engineProof: {
          candidateRows: candidateEndpoint.secondRaf.candidates.length,
          pageIndex: candidateEndpoint.secondRaf.page.index,
          buttonCount: candidateEndpoint.secondRaf.page.buttonCount,
          previousDisabled: candidateEndpoint.secondRaf.page.previousDisabled,
          nextDisabled: candidateEndpoint.secondRaf.page.nextDisabled,
        },
      },
      identity,
      identityManifestSha256,
      wasmMemory: {
        ready: readyStartup?.wasmMemory,
        candidate: yuneWasmFromPerf(candidatePerf),
        commit: yuneWasmFromPerf(commitPerf),
      },
      yunePerf: {
        internalKeydownToPaintMs: candidatePerf?.totalKeydownToPaintMs,
        workerProcessMs: candidatePerf?.workerProcessMs,
        workerRoundtripMs: candidatePerf?.workerRoundtripMs,
        firstCandidateText: candidatePerf?.firstCandidateText,
      },
      browserMemory: await collectBrowserMemory(page),
      resources,
      storageEstimate: await storageEstimate(page),
      workerUrls: page.workers().map(worker => worker.url()),
      consoleErrors: [...setupConsoleErrors, ...measurementConsoleErrors],
    };
  } finally {
    await context.close();
    await rm(userDataDir, { recursive: true, force: true });
  }
}

async function runMyRimeScenarioSample(
  scenario: ComparatorScenario,
  sampleIndex: number,
  identity: ComparatorIdentityManifest | undefined,
  identityManifestSha256: string | undefined,
): Promise<ComparatorSample> {
  const userDataDir = await freshUserDataDir(scenario, sampleIndex);
  const context = await chromium.launchPersistentContext(userDataDir, {
    headless: true,
    viewport: { width: 1365, height: 900 },
    locale: "zh-HK",
  });
  try {
    await context.addInitScript(() => {
      localStorage.setItem("pageSize", "6");
    });
    const page = await context.newPage();
    const consoleErrors = captureConsoleErrors(page);
    const url = myRimeScenarioUrl(scenario, sampleIndex);
    const startedAt = Date.now();
    await loadAndWaitMyRimeReady(page, url);
    await expect.poll(
      async () => (await myRimeWorkerMemory(page))?.heapBytes ?? 0,
      { timeout: readyTimeoutMs },
    ).toBeGreaterThan(0);
    const readyAt = Date.now();
    const readyMemory = await myRimeWorkerMemory(page);
    const input = await editableInput(page);
    await clearEditable(input);
    await installComparatorEndpointObserver(page, "my-rime");
    const beforeInputEventCount = await comparatorEventCount(page);
    await input.click();
    await page.keyboard.type(scenario.input, { delay: comparatorPeerCadenceMs });
    const candidateEndpoint = await waitForStableCandidateEndpoint(
      page,
      "my-rime",
      scenario.input,
      beforeInputEventCount,
    );
    const candidateMemory = await myRimeWorkerMemory(page);
    const inputEvents = await comparatorEventsSince(page, beforeInputEventCount);
    const firstCandidateText = candidateEndpoint.secondRaf.candidates[0]?.text;
    const selectedCandidateText = candidateEndpoint.secondRaf.candidates[
      candidateEndpoint.secondRaf.highlightedIndex
    ]?.text;
    if (!selectedCandidateText) {
      throw new Error("My RIME comparator candidate endpoint did not expose a highlighted candidate");
    }
    const beforeCommitEventCount = await comparatorEventCount(page);
    await page.keyboard.press("Space");
    const commitEndpoint = await waitForStableCommitEndpoint(
      page,
      "my-rime",
      selectedCandidateText,
      beforeCommitEventCount,
    );
    const committedValue = commitEndpoint.secondRaf.caret.value;
    const endpoint = { inputEvents, candidate: candidateEndpoint, commit: commitEndpoint };
    assertMeasuredEndpoint(endpoint, scenario, committedValue);
    const commitMemory = await myRimeWorkerMemory(page);
    return {
      scenarioId: scenario.id,
      app: scenario.app,
      build: scenario.build,
      schema: scenario.schema,
      schemaInput: scenario.input,
      sampleIndex,
      url,
      readyToInputMs: readyAt - startedAt,
      cadenceMs: comparatorPeerCadenceMs,
      inputToCandidateMs: candidateEndpoint.secondRaf.observedAt - candidateEndpoint.event.timeStamp,
      commitMs: commitEndpoint.secondRaf.observedAt - commitEndpoint.event.timeStamp,
      firstCandidateText,
      committedValue,
      endpoint,
      identity,
      identityManifestSha256,
      wasmMemory: {
        ready: workerMemorySnapshot(readyMemory),
        candidate: workerMemorySnapshot(candidateMemory),
        commit: workerMemorySnapshot(commitMemory),
        worker: commitMemory ?? candidateMemory ?? readyMemory,
      },
      browserMemory: await collectBrowserMemory(page),
      resources: [
        ...await collectPageResources(page),
        ...await collectWorkerResources(page),
      ],
      storageEstimate: await storageEstimate(page),
      workerUrls: page.workers().map(worker => worker.url()),
      consoleErrors,
    };
  } finally {
    await context.close();
    await rm(userDataDir, { recursive: true, force: true });
  }
}

async function freshUserDataDir(scenario: ComparatorScenario, sampleIndex: number): Promise<string> {
  const dir = path.join(
    os.tmpdir(),
    `yune-web-comparator-${process.pid}-${scenario.id}-${scenario.schema}-${sampleIndex}-${Date.now()}`,
  );
  await rm(dir, { recursive: true, force: true });
  await mkdir(dir, { recursive: true });
  return dir;
}

async function loadComparatorIdentityManifest(): Promise<{
  manifest: ComparatorIdentityManifest;
  sha256: string;
} | undefined> {
  if (!identityManifestPath) {
    return undefined;
  }
  const manifestFile = path.resolve(identityManifestPath);
  const text = await readFile(manifestFile, "utf8");
  return {
    manifest: parseComparatorIdentityManifest(text),
    sha256: createHash("sha256").update(text).digest("hex"),
  };
}

function identityForScenario(
  manifest: ComparatorIdentityManifest | undefined,
  scenario: ComparatorScenario,
): ComparatorIdentityManifest | undefined {
  if (!manifest || scenario.schema !== "luna_pinyin" || scenario.input !== "ni") {
    return undefined;
  }
  if (scenario.app === "yune-web") {
    const yuneBuild = process.env.YUNE_WEB_COMPARATOR_IDENTITY_YUNE_BUILD ?? "tracked-dist";
    return scenario.build === yuneBuild ? manifest : undefined;
  }
  return scenario.build === myRimeBuild ? manifest : undefined;
}

function assertMeasuredEndpoint(
  endpoint: ComparatorEndpointEvidence,
  scenario: ComparatorScenario,
  committedValue: string,
): void {
  const failures = validateEndpointEvidence(endpoint, scenario.input, committedValue, scenario.app);
  if (failures.length > 0) {
    throw new Error(
      "WEB06 comparator rejected the measured " + scenario.app
      + " selector/composition/page/commit endpoint: " + failures.join("; "),
    );
  }
}

async function loadAndWaitYuneReady(page: Page, url: string, schema: StartupSchema): Promise<void> {
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    ({ expectedSchema, appSchema }) => {
      const root = document.documentElement;
      const textarea = document.querySelector("textarea.yd-input-area") as HTMLTextAreaElement | null;
      const diagnostics = JSON.parse(root.dataset.yunePersistenceDiagnostics ?? "[]") as Array<{
        source?: string;
        marker?: { phase?: string };
      }>;
      const startupComplete = diagnostics.some(entry =>
        entry.source === "yune-startup" && entry.marker?.phase === "startup:complete"
      );
      const activeSchema = root.dataset.yuneActiveSchema;
      const expectedActive = expectedSchema === "jyut6ping3_mobile"
        ? activeSchema === "jyut6ping3" || activeSchema === "jyut6ping3_mobile"
        : activeSchema === appSchema;
      return root.dataset.yuneInitialized === "true"
        && root.dataset.yuneLoading !== "true"
        && startupComplete
        && expectedActive
        && textarea !== null
        && !textarea.disabled
        && document.querySelector("[data-yune-loading-indicator]") === null;
    },
    { expectedSchema: schema, appSchema: appSchemaId(schema) },
    { timeout: readyTimeoutMs },
  );
}

async function loadAndWaitMyRimeReady(page: Page, url: string): Promise<void> {
  await page.goto(url, { waitUntil: "domcontentloaded" });
  await page.waitForFunction(
    () => {
      const editable = document.querySelector("#container textarea");
      const copyLinkButton = [...document.querySelectorAll("button")]
        .find(button => button.getAttribute("title") === "Copy link for current IME") as HTMLButtonElement | undefined;
      return editable !== null
        && !(editable as HTMLInputElement | HTMLTextAreaElement).disabled
        && copyLinkButton !== undefined
        && !copyLinkButton.disabled;
    },
    undefined,
    { timeout: readyTimeoutMs },
  );
}

async function editableInput(page: Page) {
  const input = page.locator("#container textarea");
  await expect(input).toBeVisible({ timeout: readyTimeoutMs });
  return input;
}

async function clearEditable(locator: ReturnType<Page["locator"]>): Promise<void> {
  await locator.evaluate((element) => {
    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) {
      element.value = "";
      element.dispatchEvent(new Event("input", { bubbles: true }));
      return;
    }
    element.textContent = "";
    element.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function yunePerfCount(page: Page): Promise<number> {
  return await page.evaluate(() => (JSON.parse(document.documentElement.dataset.yunePerfDiagnostics ?? "[]") as unknown[]).length);
}

async function latestYunePerf(page: Page): Promise<{
  totalKeydownToPaintMs?: number;
  workerProcessMs?: number;
  workerRoundtripMs?: number;
  firstCandidateText?: string;
  wasmHeapBytes?: number;
  peakWasmHeapBytes?: number;
} | undefined> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yunePerfDiagnostics ?? "[]") as Array<{
      totalKeydownToPaintMs?: number;
      workerProcessMs?: number;
      workerRoundtripMs?: number;
      firstCandidateText?: string;
      wasmHeapBytes?: number;
      peakWasmHeapBytes?: number;
    }>;
    return diagnostics.at(-1);
  });
}

async function yuneStartupMarker(page: Page): Promise<{
  wasmMemory?: WasmMemorySnapshot;
  wasmGlue?: string;
  wasmBinary?: string;
  loadedExplicitAssets?: string[];
  loadedSharedAssets?: string[];
} | undefined> {
  return await page.evaluate(() => {
    const diagnostics = JSON.parse(document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]") as Array<{
      source?: string;
      marker?: {
        phase?: string;
        wasmMemory?: { currentBytes: number; peakBytes: number };
        wasmGlue?: string;
        wasmBinary?: string;
        loadedExplicitAssets?: string[];
        loadedSharedAssets?: string[];
      };
    }>;
    return diagnostics
      .slice()
      .reverse()
      .find(entry => entry.source === "yune-startup" && entry.marker?.phase === "startup:complete")
      ?.marker;
  });
}

function yuneWasmFromPerf(perf: { wasmHeapBytes?: number; peakWasmHeapBytes?: number } | undefined): WasmMemorySnapshot | undefined {
  if (perf?.wasmHeapBytes === undefined && perf?.peakWasmHeapBytes === undefined) {
    return undefined;
  }
  return {
    currentBytes: perf.wasmHeapBytes ?? perf.peakWasmHeapBytes ?? 0,
    peakBytes: perf.peakWasmHeapBytes ?? perf.wasmHeapBytes ?? 0,
  };
}

async function myRimeWorkerMemory(page: Page): Promise<ComparatorWorkerMemory | undefined> {
  const workers = page.workers();
  for (const worker of workers.slice().reverse()) {
    const value = await worker.evaluate(() => {
      const scope = globalThis as typeof globalThis & {
        Module?: { HEAPU8?: Uint8Array; wasmMemory?: WebAssembly.Memory };
        HEAPU8?: Uint8Array;
        wasmMemory?: WebAssembly.Memory;
      };
      const moduleHeapBytes = scope.Module?.HEAPU8?.byteLength ?? scope.Module?.HEAPU8?.buffer.byteLength;
      const globalHeapBytes = scope.HEAPU8?.byteLength ?? scope.HEAPU8?.buffer.byteLength;
      const wasmMemoryBytes = scope.Module?.wasmMemory?.buffer.byteLength ?? scope.wasmMemory?.buffer.byteLength;
      const heapBytes = moduleHeapBytes ?? globalHeapBytes ?? wasmMemoryBytes;
      return {
        heapBytes,
        moduleHeapBytes,
        globalHeapBytes,
        wasmMemoryBytes,
        exportedKeys: Object.keys(scope.Module ?? {}).slice(0, 20),
      };
    }).catch(() => undefined);
    if (value?.heapBytes) {
      return value;
    }
  }
  return undefined;
}

function workerMemorySnapshot(memory: ComparatorWorkerMemory | undefined): WasmMemorySnapshot | undefined {
  if (!memory?.heapBytes) {
    return undefined;
  }
  return {
    currentBytes: memory.heapBytes,
    peakBytes: memory.heapBytes,
  };
}

async function collectPageResources(page: Page): Promise<ComparatorResource[]> {
  return await page.evaluate(() =>
    performance.getEntriesByType("resource").map(entry => {
      const resource = entry as PerformanceResourceTiming;
      return {
        context: "page" as const,
        name: resource.name,
        initiatorType: resource.initiatorType,
        transferSize: resource.transferSize,
        encodedBodySize: resource.encodedBodySize,
        decodedBodySize: resource.decodedBodySize,
        duration: Math.round(resource.duration),
      };
    })
  );
}

async function collectWorkerResources(page: Page): Promise<ComparatorResource[]> {
  const resources: ComparatorResource[] = [];
  for (const worker of page.workers()) {
    const entries = await worker.evaluate(() =>
      performance.getEntriesByType("resource").map(entry => {
        const resource = entry as PerformanceResourceTiming;
        return {
          name: resource.name,
          initiatorType: resource.initiatorType,
          transferSize: resource.transferSize,
          encodedBodySize: resource.encodedBodySize,
          decodedBodySize: resource.decodedBodySize,
          duration: Math.round(resource.duration),
        };
      })
    ).catch(() => []);
    resources.push(...entries.map(entry => ({ ...entry, context: "worker" as const })));
  }
  return resources;
}

function appendYuneSyntheticResources(
  resources: ComparatorResource[],
  startup: {
    wasmGlue?: string;
    wasmBinary?: string;
    loadedExplicitAssets?: string[];
    loadedSharedAssets?: string[];
  } | undefined,
  distRoot: string,
  pageUrl: string,
): ComparatorResource[] {
  const existing = new Set(resources.map(resource => stripQuery(resource.name)));
  const names = new Set<string>();
  if (startup?.wasmGlue) {
    names.add(startup.wasmGlue);
  }
  if (startup?.wasmBinary) {
    names.add(startup.wasmBinary);
  }
  for (const asset of startup?.loadedExplicitAssets ?? []) {
    names.add(`schema/${asset}`);
  }
  for (const asset of startup?.loadedSharedAssets ?? []) {
    names.add(`schema/${asset}`);
  }
  return [
    ...resources,
    ...[...names].flatMap(name => {
      const url = new URL(name, pageUrl).toString();
      if (existing.has(stripQuery(url))) {
        return [];
      }
      const file = path.join(distRoot, ...name.split("/"));
      return [{
        context: "synthetic-worker" as const,
        name: url,
        initiatorType: "worker",
        transferSize: syntheticSize(file),
        encodedBodySize: syntheticSize(file),
        decodedBodySize: syntheticSize(file),
        duration: 0,
      }];
    }),
  ];
}

function syntheticSize(file: string): number {
  try {
    return Number(statSync(file).size);
  } catch {
    return 0;
  }
}

function stripQuery(name: string): string {
  try {
    const url = new URL(name);
    url.search = "";
    url.hash = "";
    return url.toString();
  } catch {
    return name.split("?")[0] ?? name;
  }
}

async function collectBrowserMemory(page: Page): Promise<Record<string, number>> {
  const values: Record<string, number> = {};
  const cdp = await page.context().newCDPSession(page);
  await cdp.send("Performance.enable");
  const metrics = await cdp.send("Performance.getMetrics");
  for (const metric of metrics.metrics) {
    if (["JSHeapUsedSize", "JSHeapTotalSize", "Nodes", "Documents", "LayoutCount", "RecalcStyleCount"].includes(metric.name)) {
      values[metric.name] = metric.value;
    }
  }
  const uaMemory = await page.evaluate(async () => {
    const performanceWithMemory = performance as Performance & {
      measureUserAgentSpecificMemory?: () => Promise<{ bytes: number }>;
      memory?: { usedJSHeapSize?: number; totalJSHeapSize?: number; jsHeapSizeLimit?: number };
    };
    try {
      if (performanceWithMemory.measureUserAgentSpecificMemory) {
        return { userAgentSpecificMemoryBytes: (await performanceWithMemory.measureUserAgentSpecificMemory()).bytes };
      }
    } catch {
      return {};
    }
    return {
      usedJSHeapSize: performanceWithMemory.memory?.usedJSHeapSize,
      totalJSHeapSize: performanceWithMemory.memory?.totalJSHeapSize,
      jsHeapSizeLimit: performanceWithMemory.memory?.jsHeapSizeLimit,
    };
  });
  Object.assign(values, Object.fromEntries(
    Object.entries(uaMemory).filter((entry): entry is [string, number] => typeof entry[1] === "number"),
  ));
  return values;
}

async function storageEstimate(page: Page): Promise<{ usage?: number; quota?: number } | undefined> {
  return await page.evaluate(async () => {
    if (!navigator.storage?.estimate) {
      return undefined;
    }
    return navigator.storage.estimate();
  });
}

function captureConsoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", msg => {
    if (msg.type() === "error" || msg.type() === "warning") {
      errors.push(`console:${msg.type()} ${msg.text()}`);
    }
  });
  page.on("pageerror", error => {
    errors.push(`pageerror: ${error.message}`);
  });
  page.on("response", response => {
    if (response.status() >= 400) {
      errors.push(`response:${response.status()} ${response.url()}`);
    }
  });
  return errors;
}

function myRimeScenarioUrl(scenario: ComparatorScenario, sampleIndex: number): string {
  const url = new URL(myRimeUrl);
  url.searchParams.set("schemaId", scenario.schema === "jyutping" ? "jyut6ping3" : "luna_pinyin");
  if (scenario.schema === "jyutping") {
    url.searchParams.set("variantName", "\u6e2f");
  }
  url.searchParams.set("codexBaseline", "1");
  url.searchParams.set("sample", String(sampleIndex));
  return url.toString();
}

async function startStaticServer(root: string): Promise<{ url: string; close: () => Promise<void> }> {
  const server = createServer(async (request, response) => {
    try {
      const requestUrl = new URL(request.url ?? "/", "http://127.0.0.1");
      const rawPath = decodeURIComponent(requestUrl.pathname === "/" ? "/index.html" : requestUrl.pathname);
      const relative = rawPath.replace(/^\/+/, "");
      const file = path.resolve(root, relative);
      if (!file.startsWith(path.resolve(root))) {
        response.writeHead(403);
        response.end("Forbidden");
        return;
      }
      const fileStat = await stat(file);
      if (!fileStat.isFile()) {
        response.writeHead(404);
        response.end("Not found");
        return;
      }
      response.setHeader("Content-Type", contentType(file));
      response.setHeader("Content-Length", fileStat.size);
      response.setHeader("Cache-Control", cacheControl(file));
      response.end(await readFile(file));
    } catch {
      response.writeHead(404);
      response.end("Not found");
    }
  });
  await new Promise<void>((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve());
  });
  const address = server.address();
  if (typeof address !== "object" || address === null) {
    throw new Error("Static server did not expose a TCP address");
  }
  return {
    url: `http://127.0.0.1:${address.port}`,
    close: () => closeServer(server),
  };
}

async function closeServer(server: Server): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.close(error => error ? reject(error) : resolve());
  });
}

function contentType(file: string): string {
  const ext = path.extname(file).toLowerCase();
  switch (ext) {
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

function cacheControl(file: string): string {
  if (/index\.html$/i.test(file)) {
    return "no-cache";
  }
  return "public, max-age=31536000, immutable";
}

async function assertDistExists(dir: string, label: string): Promise<void> {
  try {
    const file = path.join(dir, "index.html");
    const fileStat = await stat(file);
    if (fileStat.isFile()) {
      return;
    }
  } catch {
    // Report below.
  }
  throw new Error(`Missing ${label} at ${dir}. Run the yune-web production build commands first.`);
}
