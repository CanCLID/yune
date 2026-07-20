import { expect, type Page } from "@playwright/test";

import {
  comparatorEndpointContractVersion,
  comparatorPeerPageSize,
  comparatorSelectorManifest,
  type ComparatorApp,
  type ComparatorCandidateTuple,
  type ComparatorDomTuple,
  type ComparatorEventBoundary,
  type ComparatorPageSizeSetup,
  type ComparatorSelectorManifest,
  type ComparatorStableObservation,
} from "./comparator-endpoint";

const endpointTimeoutMs = 30_000;
const yunePageSizeTransitions = [
  { target: 7, key: "ArrowRight" as const },
  { target: comparatorPeerPageSize, key: "ArrowLeft" as const },
];

export async function ensureYuneComparatorPageSize(
  page: Page,
): Promise<Omit<ComparatorPageSizeSetup, "engineProof" | "measurementPage">> {
  const control = page.getByLabel(/No\. of Candidates Per Page|Candidates Per Page/).last();
  await expect(control).toHaveValue(String(comparatorPeerPageSize));
  const initial = await readYuneComparatorPageSizeState(page);
  const actions: ComparatorPageSizeSetup["actions"] = [];
  for (const [index, transition] of yunePageSizeTransitions.entries()) {
    const action = await applyYunePageSizeTransition(page, control, index + 1, transition);
    const { target } = transition;
    const engineProbe = await probeYuneComparatorPageSize(page, target);
    actions.push({
      ...action,
      engineProbe,
    });
  }
  const final = await readYuneComparatorPageSizeState(page);
  return {
    contractVersion: "web06-page-size-setup-v1",
    requiredRows: comparatorPeerPageSize,
    initial: {
      uiValue: initial.uiValue,
      localStorageValue: initial.localStorageValue,
      persistedConfigValue: initial.persistedConfigValue,
      deployStatus: initial.deployStatus,
      persistenceDiagnosticCount: initial.persistenceDiagnosticCount,
    },
    actions,
    final: {
      uiValue: final.uiValue,
      localStorageValue: final.localStorageValue,
      persistedConfigValue: final.persistedConfigValue,
      deployStatus: final.deployStatus,
      loadingComplete: final.loadingComplete,
    },
  };
}

export async function ensureYuneComparatorMeasurementPageSize(
  page: Page,
): Promise<ComparatorPageSizeSetup["measurementPage"]> {
  const control = page.getByLabel(/No\. of Candidates Per Page|Candidates Per Page/).last();
  await expect(control).toHaveValue(String(comparatorPeerPageSize));
  const initial = await readYuneComparatorPageSizeState(page);
  const actions: ComparatorPageSizeSetup["measurementPage"]["actions"] = [];
  for (const [index, transition] of yunePageSizeTransitions.entries()) {
    actions.push(await applyYunePageSizeTransition(page, control, index + 1, transition));
  }
  const final = await readYuneComparatorPageSizeState(page);
  return {
    initial: {
      uiValue: initial.uiValue,
      localStorageValue: initial.localStorageValue,
      persistedConfigValue: initial.persistedConfigValue,
      deployStatus: initial.deployStatus,
      persistenceDiagnosticCount: initial.persistenceDiagnosticCount,
    },
    actions,
    final: {
      uiValue: final.uiValue,
      localStorageValue: final.localStorageValue,
      persistedConfigValue: final.persistedConfigValue,
      deployStatus: final.deployStatus,
      loadingComplete: final.loadingComplete,
    },
  };
}

async function applyYunePageSizeTransition(
  page: Page,
  control: ReturnType<Page["getByLabel"]>,
  ordinal: number,
  transition: (typeof yunePageSizeTransitions)[number],
): Promise<ComparatorPageSizeSetup["measurementPage"]["actions"][number]> {
  const fromUiValue = await control.inputValue();
  const diagnosticStart = await page.evaluate(() => (
    JSON.parse(document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]") as unknown[]
  ).length);
  await control.focus();
  await page.keyboard.press(transition.key);
  await expect(control).toHaveValue(String(transition.target));
  await page.waitForFunction(({ expected, start }) => {
    const diagnostics = JSON.parse(
      document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]",
    ) as Array<{
      marker?: {
        phase?: string;
        reason?: string;
        persistedConfig?: { settings?: Record<string, string | null> };
      };
    }>;
    return diagnostics.slice(start).some(diagnostic =>
      diagnostic.marker?.phase === "syncToPersistenceAfterMutation:pass"
      && diagnostic.marker.reason === "deploy"
      && diagnostic.marker.persistedConfig?.settings?.["menu/page_size"] === String(expected)
    )
      && document.documentElement.dataset.yuneLoading !== "true"
      && document.querySelector("[data-yune-deploy-status-view]")?.getAttribute("data-status") === "success";
  }, { expected: transition.target, start: diagnosticStart }, { timeout: 30_000 });
  const state = await readYuneComparatorPageSizeState(
    page,
    diagnosticStart,
    String(transition.target),
  );
  return {
    ordinal,
    fromUiValue,
    targetUiValue: String(transition.target),
    interaction: {
      kind: "keyboard",
      key: transition.key,
      control: "preferences-page-size-range",
    },
    deployStatus: state.deployStatus,
    loadingComplete: state.loadingComplete,
    localStorageValue: state.localStorageValue,
    persistedConfigValue: state.persistedConfigValue,
    persistenceDiagnosticIndex: state.persistenceDiagnosticIndex,
  };
}

async function probeYuneComparatorPageSize(
  page: Page,
  expectedRows: number,
): Promise<ComparatorPageSizeSetup["actions"][number]["engineProbe"]> {
  const editable = page.locator("textarea.yd-input-area");
  await expect(editable).toHaveCount(1);
  await expect(editable).toBeVisible();
  await editable.click();
  await page.keyboard.type("ni", { delay: 60 });
  await expect.poll(async () => page.evaluate(expected => {
    const panel = document.querySelector(".candidate-panel");
    const rows = panel ? [...panel.querySelectorAll(".candidates .candidate-row")] : [];
    const candidates = rows.map(row =>
      (row.querySelector(".candidate-text")?.textContent ?? "").replace(/\s+/g, " ").trim()
    );
    const buttons = panel ? [...panel.querySelectorAll(".page-nav")] : [];
    const previous = buttons[0] as HTMLButtonElement | undefined;
    const next = buttons[1] as HTMLButtonElement | undefined;
    const composition = (panel?.querySelector(".candidate-preedit")?.textContent ?? "")
      .replace(/\s+/g, "");
    const composing = document.querySelector("[data-yune-status-composing]")
      ?.getAttribute("data-yune-status-composing") === "true";
    return rows.length === expected
      && candidates.every(Boolean)
      && composition === "ni"
      && buttons.length === 2
      && previous?.disabled === true
      && next?.disabled === false
      && composing;
  }, expectedRows), { timeout: 30_000 }).toBe(true);
  const snapshot = await page.evaluate(() => {
    const panel = document.querySelector(".candidate-panel");
    const rows = panel ? [...panel.querySelectorAll(".candidates .candidate-row")] : [];
    const buttons = panel ? [...panel.querySelectorAll(".page-nav")] : [];
    const previous = buttons[0] as HTMLButtonElement | undefined;
    const next = buttons[1] as HTMLButtonElement | undefined;
    return {
      candidates: rows.map(row =>
        (row.querySelector(".candidate-text")?.textContent ?? "").replace(/\s+/g, " ").trim()
      ),
      pageIndex: previous?.disabled === true ? 0 : null,
      buttonCount: buttons.length,
      previousDisabled: previous?.disabled ?? null,
      nextDisabled: next?.disabled ?? null,
    };
  });
  await page.keyboard.press("Escape");
  await expect.poll(async () => page.evaluate(() => ({
    panelCount: document.querySelectorAll(".candidate-panel").length,
    composing: document.querySelector("[data-yune-status-composing]")
      ?.getAttribute("data-yune-status-composing") === "true",
  })), { timeout: 30_000 }).toEqual({ panelCount: 0, composing: false });
  return {
    input: "ni",
    candidateRows: snapshot.candidates.length,
    candidates: snapshot.candidates,
    pageIndex: snapshot.pageIndex,
    buttonCount: snapshot.buttonCount,
    previousDisabled: snapshot.previousDisabled,
    nextDisabled: snapshot.nextDisabled,
    resetKey: "Escape",
    resetEmpty: true,
  };
}

export async function readYuneComparatorPageSizeState(
  page: Page,
  diagnosticStart = 0,
  expectedPageSize?: string,
): Promise<{
  uiValue: string;
  localStorageValue: string | null;
  persistedConfigValue: string | null;
  deployStatus: string | null;
  loadingComplete: boolean;
  persistenceDiagnosticCount: number;
  persistenceDiagnosticIndex: number;
}> {
  const control = page.getByLabel(/No\. of Candidates Per Page|Candidates Per Page/).last();
  const pageState = await page.evaluate(({ start, expected }) => {
    const diagnostics = JSON.parse(
      document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]",
    ) as Array<{
      marker?: {
        phase?: string;
        reason?: string;
        persistedConfig?: { settings?: Record<string, string | null> };
      };
    }>;
    const indexed = diagnostics.map((diagnostic, index) => ({ diagnostic, index }));
    const match = indexed.slice(start).reverse().find(({ diagnostic }) => {
      const pageSize = diagnostic.marker?.persistedConfig?.settings?.["menu/page_size"];
      return pageSize !== undefined
        && (expected === undefined || (
          diagnostic.marker?.phase === "syncToPersistenceAfterMutation:pass"
          && diagnostic.marker.reason === "deploy"
          && pageSize === expected
        ));
    });
    return {
      localStorageValue: localStorage.getItem("pageSize"),
      persistedConfigValue: match?.diagnostic.marker?.persistedConfig?.settings?.["menu/page_size"] ?? null,
      deployStatus: document.querySelector("[data-yune-deploy-status-view]")?.getAttribute("data-status") ?? null,
      loadingComplete: document.documentElement.dataset.yuneLoading !== "true",
      persistenceDiagnosticCount: diagnostics.length,
      persistenceDiagnosticIndex: match?.index ?? -1,
    };
  }, { start: diagnosticStart, expected: expectedPageSize });
  return {
    uiValue: await control.inputValue(),
    ...pageState,
  };
}

interface YuneDiagnostic {
  input?: string;
  renderedInput?: string;
  renderRevision?: number;
  candidateCount?: number;
  totalCandidateCount?: number;
  firstCandidateText?: string;
  totalKeydownToPaintMs?: number;
  workerProcessMs?: number;
  workerRoundtripMs?: number;
  wasmHeapBytes?: number;
  peakWasmHeapBytes?: number;
}

interface BrowserComparatorObserver {
  app: ComparatorApp;
  selectors: ComparatorSelectorManifest;
  revision: number;
  events: ComparatorEventBoundary[];
  mutationObserver: MutationObserver;
  flush(): void;
  read(): ComparatorDomTuple;
  stop(): void;
}

type ComparatorWindow = Window & {
  __web06ComparatorObserver?: BrowserComparatorObserver;
};

export async function installComparatorEndpointObserver(
  page: Page,
  app: ComparatorApp,
): Promise<void> {
  await page.evaluate(({ targetApp, selectors, contractVersion }) => {
    const scope = window as ComparatorWindow;
    scope.__web06ComparatorObserver?.stop();

    function isVisible(element: Element | null): element is HTMLElement {
      if (!(element instanceof HTMLElement)) {
        return false;
      }
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.display !== "none"
        && style.visibility !== "hidden"
        && Number(style.opacity || "1") !== 0
        && rect.width > 0
        && rect.height > 0;
    }

    function normalizedText(value: string | null | undefined): string {
      return (value ?? "").replace(/\s+/g, " ").trim();
    }

    function normalizedComposition(value: string | null | undefined): string {
      return (value ?? "").replace(/\s+/g, "");
    }

    function disabled(button: Element | undefined): boolean | null {
      if (!button) {
        return null;
      }
      if (button instanceof HTMLButtonElement) {
        return button.disabled || button.getAttribute("aria-disabled") === "true";
      }
      return button.getAttribute("aria-disabled") === "true";
    }

    function myRimeCandidate(row: Element): ComparatorCandidateTuple {
      const raw = normalizedText(row.textContent);
      const matched = /^(\S+)\s+(\S+)(?:\s+(.*))?$/.exec(raw);
      return {
        label: matched?.[1] ?? "",
        text: matched?.[2] ?? raw,
        comment: matched?.[3] ?? "",
      };
    }

    function yuneCandidate(row: Element): ComparatorCandidateTuple {
      return {
        label: normalizedText(row.querySelector(selectors.candidateLabel)?.textContent),
        text: normalizedText(row.querySelector(selectors.candidateText)?.textContent),
        comment: normalizedText(row.querySelector(selectors.candidateComment)?.textContent),
      };
    }

    function myRimeComposition(root: Element | null): string {
      if (!root) {
        return "";
      }
      const clone = root.cloneNode(true);
      if (!(clone instanceof Element)) {
        return "";
      }
      clone.querySelectorAll(".n-menu, button, svg, style, script").forEach(element => element.remove());
      return normalizedComposition(clone.textContent);
    }

    const endpointSelectors = [
      selectors.editable,
      selectors.candidateRoot,
      selectors.candidateSurface,
      selectors.status,
    ].filter(Boolean).join(",");

    function endpointNode(node: Node | null, includeDescendants: boolean): boolean {
      const element = node instanceof Element ? node : node?.parentElement;
      if (!element || endpointSelectors === "") {
        return false;
      }
      return element.matches(endpointSelectors)
        || element.closest(endpointSelectors) !== null
        || (includeDescendants && element.querySelector(endpointSelectors) !== null);
    }

    function endpointMutation(records: MutationRecord[]): boolean {
      return records.some(record =>
        endpointNode(record.target, false)
        || [...record.addedNodes].some(node => endpointNode(node, true))
        || [...record.removedNodes].some(node => endpointNode(node, true))
      );
    }

    const state: BrowserComparatorObserver = {
      app: targetApp,
      selectors,
      revision: 0,
      events: [],
      mutationObserver: new MutationObserver(records => {
        if (endpointMutation(records)) {
          state.revision += 1;
        }
      }),
      flush(): void {
        if (endpointMutation(state.mutationObserver.takeRecords())) {
          state.revision += 1;
        }
      },
      read(): ComparatorDomTuple {
        const editableMatches = [...document.querySelectorAll(selectors.editable)];
        const editable = editableMatches[0];
        const visibleSurfaces = [...document.querySelectorAll(selectors.candidateSurface)]
          .filter(isVisible);
        const surface = visibleSurfaces[0] ?? null;
        const root = surface?.closest(selectors.candidateRoot) ?? null;
        const rows = surface
          ? [...surface.querySelectorAll(selectors.candidateRows)].filter(isVisible)
          : [];
        const candidates = rows.map(row =>
          targetApp === "yune-web" ? yuneCandidate(row) : myRimeCandidate(row)
        );
        const highlightedIndex = rows.findIndex(row =>
          row.matches(selectors.highlighted)
          || row.querySelector(selectors.highlighted) !== null
        );
        const pageButtons = root
          ? [...root.querySelectorAll(selectors.pageButtons)].filter(isVisible)
          : [];
        const previousPageDisabled = disabled(pageButtons[0]);
        const composition = targetApp === "yune-web"
          ? normalizedComposition(root?.querySelector(selectors.composition)?.textContent)
          : myRimeComposition(root);
        const statusElement = targetApp === "yune-web"
          ? document.querySelector(selectors.status)
          : root;
        const schemaId = targetApp === "yune-web"
          ? statusElement?.querySelector("[data-yune-status-schema]")
            ?.getAttribute("data-yune-status-schema-id") ?? ""
          : new URL(location.href).searchParams.get("schemaId") ?? "";
        const composing = targetApp === "yune-web"
          ? statusElement?.querySelector("[data-yune-status-composing]")
            ?.getAttribute("data-yune-status-composing") === "true"
          : visibleSurfaces.length === 1;
        const textarea = editable instanceof HTMLTextAreaElement || editable instanceof HTMLInputElement
          ? editable
          : undefined;
        const statusDigest = JSON.stringify({
          schemaId,
          composing,
          visibleCandidateSurfaces: visibleSurfaces.length,
          statusText: normalizedText(statusElement?.textContent),
          activeTag: document.activeElement?.tagName ?? "",
          activeClass: document.activeElement?.getAttribute("class") ?? "",
          editableMatches: editableMatches.length,
        });
        const payload = {
          contractVersion,
          selectorManifestId: selectors.id,
          composition,
          candidates,
          candidateSurfaceCount: visibleSurfaces.length,
          page: {
            index: previousPageDisabled === true ? 0 : null,
            buttonCount: pageButtons.length,
            previousDisabled: previousPageDisabled,
            nextDisabled: disabled(pageButtons[1]),
          },
          highlightedIndex,
          caret: {
            selectorCount: editableMatches.length,
            value: textarea?.value ?? normalizedText(editable?.textContent),
            selectionStart: textarea?.selectionStart ?? null,
            selectionEnd: textarea?.selectionEnd ?? null,
            selectionDirection: textarea?.selectionDirection ?? null,
            active: document.activeElement === editable,
            visible: isVisible(editable),
            disabled: textarea?.disabled ?? editable?.getAttribute("aria-disabled") === "true",
          },
          status: {
            schemaId,
            composing,
            surfaceVisible: visibleSurfaces.length === 1,
            digest: statusDigest,
          },
        };
        return {
          ...payload,
          revision: state.revision,
          observedAt: performance.now(),
          digest: JSON.stringify(payload),
        };
      },
      stop(): void {
        state.mutationObserver.disconnect();
        document.removeEventListener("keydown", recordKeydown, true);
      },
    };

    function recordKeydown(event: KeyboardEvent): void {
      state.flush();
      state.events.push({
        ordinal: state.events.length + 1,
        type: "keydown",
        key: event.key,
        code: event.code,
        timeStamp: event.timeStamp,
        revisionBeforeEvent: state.revision,
      });
    }

    state.mutationObserver.observe(document.documentElement, {
      attributes: true,
      characterData: true,
      childList: true,
      subtree: true,
    });
    document.addEventListener("keydown", recordKeydown, true);
    scope.__web06ComparatorObserver = state;
  }, {
    targetApp: app,
    selectors: comparatorSelectorManifest[app],
    contractVersion: comparatorEndpointContractVersion,
  });
}

export async function comparatorEventCount(page: Page): Promise<number> {
  return page.evaluate(() => {
    const state = (window as ComparatorWindow).__web06ComparatorObserver;
    if (!state) {
      throw new Error("WEB06 comparator endpoint observer is not installed");
    }
    return state.events.length;
  });
}

export async function comparatorEventsSince(
  page: Page,
  afterEventOrdinal: number,
): Promise<ComparatorEventBoundary[]> {
  return page.evaluate(afterOrdinal => {
    const state = (window as ComparatorWindow).__web06ComparatorObserver;
    if (!state) {
      throw new Error("WEB06 comparator endpoint observer is not installed");
    }
    state.flush();
    return state.events.filter(event => event.ordinal > afterOrdinal);
  }, afterEventOrdinal);
}

export async function waitForStableCandidateEndpoint(
  page: Page,
  app: ComparatorApp,
  expectedInput: string,
  afterEventOrdinal: number,
  yuneDiagnosticStartIndex = 0,
): Promise<ComparatorStableObservation> {
  return page.evaluate(async ({
    targetApp,
    expected,
    afterOrdinal,
    diagnosticStart,
    timeoutMs,
    pageSize,
  }) => {
    const state = (window as ComparatorWindow).__web06ComparatorObserver;
    if (!state || state.app !== targetApp) {
      throw new Error("WEB06 comparator endpoint observer is missing or belongs to the wrong app");
    }
    const deadline = performance.now() + timeoutMs;
    let lastTuple: ComparatorDomTuple | undefined;
    let lastEvent: ComparatorEventBoundary | undefined;
    let lastDiagnostics: YuneDiagnostic[] = [];
    const nextFrame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    while (performance.now() <= deadline) {
      state.flush();
      const events = state.events.filter(event => event.ordinal > afterOrdinal);
      const expectedKey = expected.at(-1);
      const event = events.at(-1);
      const tuple = state.read();
      lastTuple = tuple;
      lastEvent = event;
      const candidateReady = event !== undefined
        && event.key === expectedKey
        && tuple.revision > event.revisionBeforeEvent
        && tuple.observedAt >= event.timeStamp
        && tuple.composition === expected
        && tuple.candidateSurfaceCount === 1
        && tuple.candidates.length === pageSize
        && tuple.candidates.every(candidate => candidate.text !== "")
        && tuple.page.index === 0
        && tuple.page.buttonCount === 2
        && tuple.page.previousDisabled === true
        && tuple.page.nextDisabled === false
        && tuple.highlightedIndex >= 0
        && tuple.highlightedIndex < tuple.candidates.length
        && tuple.caret.selectorCount === 1
        && tuple.caret.active
        && tuple.caret.visible
        && !tuple.caret.disabled
        && tuple.caret.selectionStart !== null
        && tuple.caret.selectionEnd !== null
        && tuple.status.schemaId !== ""
        && tuple.status.composing
        && tuple.status.surfaceVisible;
      if (candidateReady && event) {
        const initial = tuple;
        await nextFrame();
        const firstRaf = state.read();
        await nextFrame();
        const secondRaf = state.read();
        const stable = initial.revision === firstRaf.revision
          && firstRaf.revision === secondRaf.revision
          && initial.digest === firstRaf.digest
          && firstRaf.digest === secondRaf.digest;
        if (stable) {
          if (targetApp !== "yune-web") {
            return {
              event,
              initial,
              firstRaf,
              secondRaf,
            };
          }
          while (performance.now() <= deadline) {
            const diagnostics = JSON.parse(
              document.documentElement.dataset.yunePerfDiagnostics ?? "[]",
            ) as YuneDiagnostic[];
            lastDiagnostics = diagnostics;
            const exact = diagnostics
              .slice(diagnosticStart)
              .map((diagnostic, offset) => ({ diagnostic, index: diagnosticStart + offset }))
              .filter(({ diagnostic }) =>
                diagnostic.input === expected
                && diagnostic.renderedInput === expected
                && diagnostic.firstCandidateText === secondRaf.candidates[0]?.text
                && diagnostic.candidateCount === secondRaf.candidates.length
                && Number.isInteger(diagnostic.renderRevision)
              )
              .at(-1);
            if (exact) {
              return {
                event,
                initial,
                firstRaf,
                secondRaf,
                yuneDiagnostic: {
                  index: exact.index,
                  input: exact.diagnostic.input ?? "",
                  renderedInput: exact.diagnostic.renderedInput ?? "",
                  renderRevision: exact.diagnostic.renderRevision ?? 0,
                  candidateCount: exact.diagnostic.candidateCount ?? 0,
                  totalCandidateCount: exact.diagnostic.totalCandidateCount ?? 0,
                  firstCandidateText: exact.diagnostic.firstCandidateText ?? "",
                },
              };
            }
            state.flush();
            if (state.read().digest !== secondRaf.digest) {
              break;
            }
            await nextFrame();
          }
        }
      }
      await nextFrame();
    }
    throw new Error("Timed out waiting for coherent WEB06 candidate endpoint: " + JSON.stringify({
      app: targetApp,
      expected,
      afterEventOrdinal: afterOrdinal,
      lastEvent,
      lastTuple,
      diagnosticsTail: lastDiagnostics.slice(-3),
    }));
  }, {
    targetApp: app,
    expected: expectedInput,
    afterOrdinal: afterEventOrdinal,
    diagnosticStart: yuneDiagnosticStartIndex,
    timeoutMs: endpointTimeoutMs,
    pageSize: comparatorPeerPageSize,
  });
}

export async function waitForStableCommitEndpoint(
  page: Page,
  app: ComparatorApp,
  expectedCommittedValue: string,
  afterEventOrdinal: number,
): Promise<ComparatorStableObservation> {
  return page.evaluate(async ({
    targetApp,
    expected,
    afterOrdinal,
    timeoutMs,
  }) => {
    const state = (window as ComparatorWindow).__web06ComparatorObserver;
    if (!state || state.app !== targetApp) {
      throw new Error("WEB06 comparator endpoint observer is missing or belongs to the wrong app");
    }
    const deadline = performance.now() + timeoutMs;
    let lastTuple: ComparatorDomTuple | undefined;
    let lastEvent: ComparatorEventBoundary | undefined;
    const nextFrame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    while (performance.now() <= deadline) {
      state.flush();
      const event = state.events
        .filter(candidate => candidate.ordinal > afterOrdinal)
        .at(-1);
      const tuple = state.read();
      lastTuple = tuple;
      lastEvent = event;
      const commitReady = event !== undefined
        && event.key === " "
        && tuple.revision > event.revisionBeforeEvent
        && tuple.observedAt >= event.timeStamp
        && tuple.caret.selectorCount === 1
        && tuple.caret.active
        && tuple.caret.value === expected
        && tuple.caret.selectionStart === expected.length
        && tuple.caret.selectionEnd === expected.length
        && tuple.caret.visible
        && !tuple.caret.disabled
        && tuple.composition === ""
        && tuple.candidateSurfaceCount === 0
        && tuple.candidates.length === 0
        && tuple.status.schemaId !== ""
        && !tuple.status.composing
        && !tuple.status.surfaceVisible;
      if (commitReady && event) {
        const initial = tuple;
        await nextFrame();
        const firstRaf = state.read();
        await nextFrame();
        const secondRaf = state.read();
        if (initial.revision === firstRaf.revision
            && firstRaf.revision === secondRaf.revision
            && initial.digest === firstRaf.digest
            && firstRaf.digest === secondRaf.digest) {
          return { event, initial, firstRaf, secondRaf };
        }
      }
      await nextFrame();
    }
    throw new Error("Timed out waiting for coherent WEB06 commit endpoint: " + JSON.stringify({
      app: targetApp,
      expectedCommittedValue: expected,
      afterEventOrdinal: afterOrdinal,
      lastEvent,
      lastTuple,
    }));
  }, {
    targetApp: app,
    expected: expectedCommittedValue,
    afterOrdinal: afterEventOrdinal,
    timeoutMs: endpointTimeoutMs,
  });
}

export async function exactYuneDiagnostic(
  page: Page,
  index: number,
): Promise<YuneDiagnostic | undefined> {
  return page.evaluate(diagnosticIndex => {
    const diagnostics = JSON.parse(
      document.documentElement.dataset.yunePerfDiagnostics ?? "[]",
    ) as YuneDiagnostic[];
    return diagnostics[diagnosticIndex];
  }, index);
}
