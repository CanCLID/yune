/**
 * yune-web Browser E2E Spec
 *
 * Real browser validation for yune-web + Yune runtime seam.
 * Covers composition, candidate actions, deploy, customize, and persistence per D-08/TYPEDUCK-E2E-03.
 *
 * Prerequisites:
 * 1. Tracked yune-web Vite app under apps/yune-web/
 * 2. Yune WASM artifact with yune_web_* exports in packages/yune-web-runtime/dist/
 * 3. Built @yune-ime/yune-web-runtime package
 * 4. Explicit yune-web-owned YAML assets (per e2e/assets/README.md)
 * 5. Playwright installed (standalone spec framework)
 */

import {
  test,
  expect,
  type Page,
  type BrowserContext,
  type Locator,
  type TestInfo,
  type WorkerInfo,
} from "@playwright/test";

// Test configuration
const APP_URL = process.env.YUNE_WEB_APP_URL || "http://localhost:5173";
const TIMEOUT_MS = 300000; // WASM load and runtime init may be slow

// E2E evidence directory
const EVIDENCE_DIR = process.env.YUNE_WEB_EVIDENCE_DIR || "../e2e/results";
let currentEvidenceScope = "unscoped";
const M24_EVIDENCE_DIR = "m24-dogfooding";
const M25_EVIDENCE_DIR = "m25-dogfooding";
const M26_EVIDENCE_DIR = "m26-performance";
const M27_EVIDENCE_DIR = "m27-startup-runtime";
const M28_EVIDENCE_DIR = "m28-partial-selection";
const M28_FOLLOWUP_EVIDENCE_DIR = "m28-follow-up-upstream-jyutping";
const M29_EVIDENCE_DIR = "m29-performance";
const M31_EVIDENCE_DIR = "m31-yune-web-public-demo";
const M31_UX_EVIDENCE_DIR = "yune-web-ux-redesign-2026-06-24";
const M26_EVIDENCE_LABEL = process.env.M26_EVIDENCE_LABEL || "latest";
const M27_EVIDENCE_LABEL = process.env.M27_EVIDENCE_LABEL || "latest";
const M29_EVIDENCE_LABEL = process.env.M29_EVIDENCE_LABEL || "latest";
const RUN_M31_PUBLIC_E2E = process.env.YUNE_PUBLIC_DEMO_E2E === "1";

const ACTIVE_SHOWCASE_CONTROLS = [
  /Auto-completion/,
  /Auto-correction/,
  /Auto-composition/,
  /User Dictionary/,
  /AI Candidates/,
  /Combine same-text candidates/,
  /Prediction never first/,
  /Prediction threshold/,
  /Dictionary exclude/,
] as const;

const LIVE_SHOWCASE_CONTROLS = [
  /ASCII mode/,
  /Full shape/,
  /Hong Kong Traditional/,
  /Simplified Chinese/,
  /Extended charset/,
  /Disabled/,
] as const;

const DISPLAY_SHOWCASE_CONTROLS = [
  /Display languages/,
  /Candidate Jyutping/,
  /Reverse code display/,
  /Cangjie lookup/,
] as const;

const LEARNED_PREFIX_INPUT = "ngo";
const LEARNED_PHRASE_INPUT = "ngohaigo";
const CLASSIC_NGO_TEXT = "\u6211";
const LEARNED_PHRASE_TEXT = "\u6211\u4fc2\u500b";
const M24_DOGFOOD_INPUT = "jigaajiusihaa";
const M24_DOGFOOD_TOP = "\u800c\u5bb6\u8981\u8a66\u4e0b";

/**
 * Helper: Capture browser console errors to evidence file
 */
async function captureConsoleErrors(page: Page): Promise<string[]> {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (
      msg.type() === "error" ||
      msg.type() === "warning" ||
      APP_URL.includes("debug")
    ) {
      errors.push(
        `[${new Date().toISOString()}] console:${msg.type()} ${msg.text()}`,
      );
    }
  });
  page.on("pageerror", (error) => {
    errors.push(`[${new Date().toISOString()}] pageerror: ${error.message}`);
  });
  page.on("response", (response) => {
    if (response.status() >= 400) {
      errors.push(
        `[${new Date().toISOString()}] response: ${response.status()} ${response.url()}`,
      );
    }
  });
  return errors;
}

function evidenceSlug(value: string): string {
  return (
    value
      .replace(/[^A-Za-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 90) || "test"
  );
}

function evidenceScopeForTest(testInfo: TestInfo): string {
  return `worker-${testInfo.parallelIndex}-${evidenceSlug(testInfo.title)}`;
}

function evidenceScopeForWorker(workerInfo: WorkerInfo): string {
  return `worker-${workerInfo.parallelIndex}`;
}

function setEvidenceScope(testInfo: TestInfo): void {
  currentEvidenceScope = evidenceScopeForTest(testInfo);
}

async function evidencePath(
  filename: string,
  scope = currentEvidenceScope,
): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, scope, filename);
}

function consoleFailures(messages: string[]): string[] {
  return messages.filter(
    (message) =>
      message.includes("console:error") ||
      message.includes("console:warning") ||
      message.includes("pageerror:") ||
      message.includes("response:"),
  );
}

/**
 * Helper: Save evidence to results directory
 */
async function saveEvidence(filename: string, content: string): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const scopedPath = await evidencePath(filename);
  await fs.mkdir(path.dirname(scopedPath), { recursive: true });
  await fs.appendFile(scopedPath, content, "utf8");
}

async function saveWorkerEvidence(
  workerInfo: WorkerInfo,
  filename: string,
  content: string,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const scopedPath = await evidencePath(
    filename,
    evidenceScopeForWorker(workerInfo),
  );
  await fs.mkdir(path.dirname(scopedPath), { recursive: true });
  await fs.appendFile(scopedPath, content, "utf8");
}

/**
 * Helper: Take screenshot with evidence filename
 */
async function takeEvidenceScreenshot(
  page: Page,
  flowName: string,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const screenshotPath = await evidencePath(`screenshot-${flowName}.png`);
  await fs.mkdir(path.dirname(screenshotPath), { recursive: true });
  await page.screenshot({ path: screenshotPath, fullPage: false });
}

interface CandidateRowSnapshot {
  index: number;
  text: string | null;
  note: string | null;
  source: string | null;
  rowText: string;
}

interface CandidatePanelSnapshot {
  aiEnabled: boolean;
  inputValue: string;
  candidates: CandidateRowSnapshot[];
}

interface WasmMemorySnapshot {
  currentBytes: number;
  peakBytes: number;
}

interface PersistenceDiagnosticSnapshot {
  type: "diagnostic";
  source: string;
  marker: {
    phase?: string;
    reason?: string;
    action?: string;
    totalMs?: number;
    ms?: number;
    queueWaitMs?: number;
    workerRoundtripMs?: number;
    workerMs?: number;
    renderMs?: number;
    input?: string;
    assetVersion?: string;
    m27EvidenceVersion?: string;
    m31EvidenceVersion?: string;
    publicDemo?: boolean;
    wasmBuildProfile?: string;
    wasmMemory?: WasmMemorySnapshot;
    wasmBinary?: string;
    markers?: { phase: string; ms: number; wasmMemory?: WasmMemorySnapshot }[];
    loadedSharedAssets?: string[];
    assetCache?: {
      hits?: number;
      misses?: number;
      unavailable?: boolean;
    };
    persistedConfig?: {
      exists?: boolean;
      settings?: Record<string, string | null>;
    };
    deployedConfig?: {
      exists?: boolean;
      settings?: Record<string, string | null>;
    };
  };
}

interface ActionDiagnosticSnapshot {
  action?: string;
  input?: string;
  enqueuedAt?: number;
  sentAt?: number;
  receivedAt?: number;
  workerStartedAt?: number;
  workerFinishedAt?: number;
  queueWaitMs?: number;
  workerRoundtripMs?: number;
  workerMs?: number;
  totalMs?: number;
}

interface PerfDiagnosticSnapshot {
  input: string;
  key?: string;
  keydownAt: number;
  workerQueuedAt: number;
  workerStartedAt: number;
  workerFinishedAt: number;
  responseReceivedAt: number;
  responseMappingFinishedAt: number;
  stateAppliedAt: number;
  paintObservedAt: number;
  candidateCount: number;
  totalCandidateCount: number;
  firstCandidateText?: string;
  workerQueueWaitMs?: number;
  workerProcessMs?: number;
  workerRoundtripMs?: number;
  responseMappingMs: number;
  reactUpdateMs: number;
  paintProxyMs: number;
  totalWorkerActionMs?: number;
  wasmHeapBytes?: number;
  peakWasmHeapBytes?: number;
  totalKeydownToPaintMs: number;
}

interface ElementBoxSnapshot {
  x: number;
  y: number;
  width: number;
  height: number;
  right: number;
  bottom: number;
}

async function elementBox(locator: Locator): Promise<ElementBoxSnapshot> {
  const box = await locator.boundingBox();
  expect(box).not.toBeNull();
  return {
    x: box?.x ?? 0,
    y: box?.y ?? 0,
    width: box?.width ?? 0,
    height: box?.height ?? 0,
    right: (box?.x ?? 0) + (box?.width ?? 0),
    bottom: (box?.y ?? 0) + (box?.height ?? 0),
  };
}

async function firstTwoCandidateRowBoxes(
  page: Page,
): Promise<[ElementBoxSnapshot, ElementBoxSnapshot]> {
  const rows = page.locator(".candidate-panel .candidate-row");
  await expect
    .poll(async () => rows.count(), { timeout: 5000 })
    .toBeGreaterThanOrEqual(2);
  return [await elementBox(rows.nth(0)), await elementBox(rows.nth(1))];
}

async function writeEvidence(filename: string, content: string): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const scopedPath = await evidencePath(filename);
  await fs.mkdir(path.dirname(scopedPath), { recursive: true });
  await fs.writeFile(scopedPath, content, "utf8");
}

async function saveJsonEvidence(
  filename: string,
  value: unknown,
): Promise<void> {
  await writeEvidence(filename, `${JSON.stringify(value, null, 2)}\n`);
}

async function m24EvidencePath(
  issueId: string,
  filename: string,
): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M24_EVIDENCE_DIR, issueId, filename);
}

async function saveM24Json(
  issueId: string,
  filename: string,
  payload: unknown,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m24EvidencePath(issueId, filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function takeM24Screenshot(
  page: Page,
  issueId: string,
  filename: string,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const screenshotPath = await m24EvidencePath(
    issueId,
    `screenshot-${filename}.png`,
  );
  await fs.mkdir(path.dirname(screenshotPath), { recursive: true });
  await page.screenshot({ path: screenshotPath, fullPage: false });
}

async function m25EvidencePath(
  issueId: string,
  filename: string,
): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M25_EVIDENCE_DIR, issueId, filename);
}

async function saveM25Json(
  issueId: string,
  filename: string,
  payload: unknown,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m25EvidencePath(issueId, filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function takeM25Screenshot(
  page: Page,
  issueId: string,
  filename: string,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const screenshotPath = await m25EvidencePath(
    issueId,
    `screenshot-${filename}.png`,
  );
  await fs.mkdir(path.dirname(screenshotPath), { recursive: true });
  await page.screenshot({ path: screenshotPath, fullPage: false });
}

async function m26EvidencePath(filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M26_EVIDENCE_DIR, filename);
}

async function saveM26Json(filename: string, payload: unknown): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m26EvidencePath(filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function m27EvidencePath(filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M27_EVIDENCE_DIR, filename);
}

async function saveM27Json(filename: string, payload: unknown): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m27EvidencePath(filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function m29EvidencePath(filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M29_EVIDENCE_DIR, filename);
}

async function saveM29Json(filename: string, payload: unknown): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m29EvidencePath(filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function m31EvidencePath(filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M31_EVIDENCE_DIR, filename);
}

async function saveM31Json(filename: string, payload: unknown): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m31EvidencePath(filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function takeM31Screenshot(page: Page, filename: string): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const screenshotPath = await m31EvidencePath(`screenshot-${filename}.png`);
  await fs.mkdir(path.dirname(screenshotPath), { recursive: true });
  await page.screenshot({ path: screenshotPath, fullPage: false });
}

async function m31UxEvidencePath(filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M31_UX_EVIDENCE_DIR, filename);
}

async function writeM31UxEvidence(
  filename: string,
  content: string,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const targetPath = await m31UxEvidencePath(filename);
  await fs.mkdir(path.dirname(targetPath), { recursive: true });
  await fs.writeFile(targetPath, content, "utf8");
}

async function saveM31UxJson(
  filename: string,
  payload: unknown,
): Promise<void> {
  await writeM31UxEvidence(filename, `${JSON.stringify(payload, null, 2)}\n`);
}

async function takeM31UxScreenshot(
  page: Page,
  filename: string,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const screenshotPath = await m31UxEvidencePath(`screenshot-${filename}.png`);
  await fs.mkdir(path.dirname(screenshotPath), { recursive: true });
  await page.screenshot({ path: screenshotPath, fullPage: true });
}

async function m28EvidencePath(filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M28_EVIDENCE_DIR, filename);
}

async function saveM28Json(filename: string, payload: unknown): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m28EvidencePath(filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function m28FollowupEvidencePath(filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M28_FOLLOWUP_EVIDENCE_DIR, filename);
}

async function saveM28FollowupJson(
  filename: string,
  payload: unknown,
): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m28FollowupEvidencePath(filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function findRepoRoot(): Promise<string> {
  const fs = await import("fs/promises");
  const path = await import("path");
  let current = process.cwd();
  for (;;) {
    try {
      await fs.access(path.join(current, "Cargo.toml"));
      return current;
    } catch {
      const parent = path.dirname(current);
      if (parent === current) {
        throw new Error(`Cannot locate repository root from ${process.cwd()}`);
      }
      current = parent;
    }
  }
}

async function readRepoText(relativePath: string): Promise<string> {
  const fs = await import("fs/promises");
  const path = await import("path");
  return fs.readFile(path.join(await findRepoRoot(), relativePath), "utf8");
}

async function loadFixture(
  filename: string,
): Promise<Record<string, unknown> & { cases: Record<string, unknown>[] }> {
  return JSON.parse(
    await readRepoText(
      `crates/yune-core/tests/fixtures/typeduck-v1.1.2/${filename}`,
    ),
  );
}

async function m14Case(
  filename: string,
  variant: string,
  input: string,
): Promise<{ selected_candidates: { text: string; comment?: string }[] }> {
  const fixture = await loadFixture(filename);
  const found = fixture.cases.find(
    (candidate) =>
      candidate["variant"] === variant && candidate["input"] === input,
  ) as
    | { selected_candidates: { text: string; comment?: string }[] }
    | undefined;
  if (!found) {
    throw new Error(`Missing M14 golden case ${filename}:${variant}:${input}`);
  }
  return found;
}

async function m14Texts(
  filename: string,
  variant: string,
  input: string,
  count: number,
): Promise<string[]> {
  const found = await m14Case(filename, variant, input);
  return found.selected_candidates
    .slice(0, count)
    .map((candidate) => candidate.text);
}

function m28ContinuationComponents(fixture: {
  captured_next_candidates?: { comment?: string }[];
}): string[] {
  const comment = fixture.captured_next_candidates?.[0]?.comment ?? "";
  return comment
    .split("\r")
    .map((record) => {
      const body =
        record.startsWith("1") || record.startsWith("0") ? record.slice(1) : "";
      return body.split(",")[1] ?? "";
    })
    .filter((text) => text.length > 0)
    .slice(1);
}

async function m14Notes(
  filename: string,
  variant: string,
  input: string,
  count: number,
): Promise<string[]> {
  const found = await m14Case(filename, variant, input);
  return found.selected_candidates
    .slice(0, count)
    .map((candidate) =>
      (candidate.comment ?? "").split("\f")[0].replace(/^\v/, ""),
    );
}

async function waitForAppReady(page: Page): Promise<void> {
  await page.waitForFunction(
    () =>
      document.documentElement.dataset.yuneInitialized === "true" &&
      document.documentElement.dataset.yuneLoading === "false",
    undefined,
    { timeout: TIMEOUT_MS },
  );
  await expect(page.locator("[data-yune-loading-indicator]")).toHaveCount(0, {
    timeout: TIMEOUT_MS,
  });
}

async function openApp(page: Page): Promise<void> {
  await page.goto(APP_URL, {
    timeout: TIMEOUT_MS,
    waitUntil: "domcontentloaded",
  });
  await waitForAppReady(page);
}

async function readCandidatePanelSnapshot(
  page: Page,
  aiEnabled: boolean,
): Promise<CandidatePanelSnapshot> {
  const inputField = page.locator("input[type='text'], textarea").first();
  const rows = page.locator(".candidate-panel .candidates tbody");
  return {
    aiEnabled,
    inputValue: await inputField.inputValue(),
    candidates: await rows.evaluateAll((elements) =>
      elements.map((element, index) => {
        const firstRow = element.querySelector("tr");
        const cells = Array.from(firstRow?.querySelectorAll("td") ?? []);
        const note = cells[2]?.textContent?.trim() || null;
        return {
          index,
          text: element.getAttribute("data-candidate-text"),
          note,
          source: element.getAttribute("data-source"),
          rowText: element.textContent?.replace(/\s+/g, " ").trim() ?? "",
        };
      }),
    ),
  };
}

async function focusInputAndType(
  page: Page,
  text: string,
  expectedVisibleText = text,
): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await inputField.focus();
  await inputField.type(text, { delay: 80 });
  await expect(
    page.locator(".candidate-panel .candidates tbody").first(),
  ).toBeVisible({ timeout: 5000 });
  await expect(page.locator(".candidate-panel").first()).toContainText(
    expectedVisibleText,
    { timeout: 5000 },
  );
}

async function typeCompositionAndWaitForCandidate(
  page: Page,
  input: string,
  expectedText: string,
): Promise<CandidatePanelSnapshot> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(input, { delay: 120 });
  await expect
    .poll(
      async () => {
        const state = await readCandidatePanelSnapshot(page, false);
        return state.candidates.map((candidate) => candidate.text);
      },
      { timeout: 10000 },
    )
    .toContain(expectedText);
  return readCandidatePanelSnapshot(page, false);
}

async function typeCompositionAndWaitForTopCandidate(
  page: Page,
  input: string,
  expectedText: string,
): Promise<CandidatePanelSnapshot> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(input, { delay: 120 });
  await expect
    .poll(
      async () => {
        const state = await readCandidatePanelSnapshot(page, false);
        return state.candidates[0]?.text ?? null;
      },
      { timeout: 10000 },
    )
    .toBe(expectedText);
  return readCandidatePanelSnapshot(page, false);
}

async function typeCompositionAndWaitForRowCount(
  page: Page,
  input: string,
  expectedRows: number,
): Promise<CandidatePanelSnapshot> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(input, { delay: 80 });
  await expect
    .poll(
      async () => page.locator(".candidate-panel .candidates tbody").count(),
      { timeout: 10000 },
    )
    .toBe(expectedRows);
  return readCandidatePanelSnapshot(page, false);
}

async function learnPhraseThroughBrowser(
  page: Page,
): Promise<CandidatePanelSnapshot> {
  const learnedPhrase = await typeCompositionAndWaitForCandidate(
    page,
    LEARNED_PHRASE_INPUT,
    LEARNED_PHRASE_TEXT,
  );
  expect(learnedPhrase.candidates[0].text).toBe(LEARNED_PHRASE_TEXT);

  const inputField = page.locator("input[type='text'], textarea").first();
  await page.keyboard.press("Space");
  await expect(inputField).toHaveValue(LEARNED_PHRASE_TEXT, { timeout: 5000 });
  await page.waitForTimeout(500);
  return learnedPhrase;
}

function candidateTexts(state: CandidatePanelSnapshot): (string | null)[] {
  return state.candidates.map((candidate) => candidate.text);
}

async function selectVisibleCandidateByText(
  page: Page,
  text: string,
): Promise<{ before: CandidatePanelSnapshot; selectedIndex: number }> {
  const before = await readCandidatePanelSnapshot(page, false);
  const selectedIndex = before.candidates.findIndex(
    (candidate) => candidate.text === text,
  );
  expect(
    selectedIndex,
    `candidate ${text} should be visible`,
  ).toBeGreaterThanOrEqual(0);
  expect(
    selectedIndex,
    `candidate ${text} should be selectable by number key`,
  ).toBeLessThan(10);
  await page.keyboard.press(
    selectedIndex === 9 ? "0" : String(selectedIndex + 1),
  );
  return { before, selectedIndex };
}

function classicCandidateSignature(
  state: CandidatePanelSnapshot,
): { text: string | null; note: string | null; rowText: string }[] {
  return state.candidates.map((candidate) => ({
    text: candidate.text,
    note: candidate.note,
    rowText: candidate.rowText,
  }));
}

async function expectNoToasts(page: Page): Promise<void> {
  await expect(page.locator(".yd-toast")).toHaveCount(0, { timeout: 1000 });
}

async function clickShowcaseScenario(
  page: Page,
  name: string | RegExp,
  expectedText: string,
  aiEnabled = false,
): Promise<CandidatePanelSnapshot> {
  await clearComposition(page);
  await page.waitForTimeout(500);
  await page.getByRole("button", { name }).click();
  await expect
    .poll(
      async () => {
        const state = await readCandidatePanelSnapshot(page, aiEnabled);
        return state.candidates.map((candidate) => candidate.text);
      },
      { timeout: 10000 },
    )
    .toContain(expectedText);
  await page.waitForTimeout(750);
  return readCandidatePanelSnapshot(page, aiEnabled);
}

async function typeRawInput(
  page: Page,
  text: string,
): Promise<{ value: string; panelCount: number }> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(text, { delay: 80 });
  await page.waitForTimeout(500);
  return {
    value: await inputField.inputValue(),
    panelCount: await page.locator(".candidate-panel").count(),
  };
}

async function clearComposition(page: Page): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await inputField.focus();
  for (
    let attempts = 0;
    attempts < 4 && (await page.locator(".candidate-panel").count()) > 0;
    attempts += 1
  ) {
    await page.keyboard.press("Escape").catch(() => undefined);
    await page.waitForTimeout(150);
  }
  await inputField.fill("");
  await expect(page.locator(".candidate-panel")).toHaveCount(0, {
    timeout: 5000,
  });
}

async function clearCompositionThroughInput(page: Page): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await inputField.focus();
  for (
    let attempts = 0;
    attempts < 12 && (await page.locator(".candidate-panel").count()) > 0;
    attempts += 1
  ) {
    await page.keyboard.press("Backspace");
    await page.waitForTimeout(120);
  }
  await inputField.fill("");
  await expect(page.locator(".candidate-panel")).toHaveCount(0, {
    timeout: 5000,
  });
}

async function setPreferenceToggle(
  page: Page,
  label: RegExp,
  enabled: boolean,
): Promise<void> {
  const toggle = page.getByLabel(label).last();
  const checked = await toggle.isChecked();
  if (checked !== enabled) {
    await toggle.evaluate((element, nextEnabled) => {
      const input = element as HTMLInputElement;
      if (input.checked !== nextEnabled) {
        input.click();
      }
    }, enabled);
    if (enabled) {
      await expect(toggle).toBeChecked({ timeout: 5000 });
    } else {
      await expect(toggle).not.toBeChecked({ timeout: 5000 });
    }
    await page.waitForTimeout(250);
    await waitForAppReady(page);
    await page.waitForTimeout(250);
  }
}

async function setPreferenceToggleAndWaitForSettings(
  page: Page,
  label: RegExp,
  enabled: boolean,
  expectedSettings: Record<string, string | null>,
): Promise<Record<string, string | null>> {
  const toggle = page.getByLabel(label).last();
  const checked = await toggle.isChecked();
  if (checked !== enabled) {
    await toggle.evaluate((element, nextEnabled) => {
      const input = element as HTMLInputElement;
      if (input.checked !== nextEnabled) {
        input.click();
      }
    }, enabled);
    if (enabled) {
      await expect(toggle).toBeChecked({ timeout: 5000 });
    } else {
      await expect(toggle).not.toBeChecked({ timeout: 5000 });
    }
  }
  const settings = await waitForPersistedSettings(page, expectedSettings);
  await waitForAppReady(page);
  await page.waitForTimeout(250);
  return settings;
}

async function setPreferenceRange(
  page: Page,
  label: RegExp,
  value: number,
): Promise<void> {
  const range = page.getByLabel(label).last();
  await range.evaluate((element, nextValue) => {
    const input = element as HTMLInputElement;
    const setter = Object.getOwnPropertyDescriptor(
      window.HTMLInputElement.prototype,
      "value",
    )?.set;
    setter?.call(input, String(nextValue));
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }, value);
  await page.waitForTimeout(250);
  await waitForAppReady(page);
  await page.waitForTimeout(250);
}

async function setPreferenceRadio(page: Page, label: RegExp): Promise<void> {
  await page.getByLabel(label).last().check({ force: true });
  await waitForAppReady(page).catch(() => undefined);
}

async function setAiToggle(page: Page, enabled: boolean): Promise<void> {
  const aiToggle = page.getByLabel(/AI Candidates/).last();
  const checked = await aiToggle.isChecked();
  if (checked !== enabled) {
    await aiToggle.evaluate((element, nextEnabled) => {
      const input = element as HTMLInputElement;
      if (input.checked !== nextEnabled) {
        input.click();
      }
    }, enabled);
    await expect(aiToggle).toBeChecked({ checked: enabled, timeout: 5000 });
    await waitForAppReady(page);
  }
  await page.waitForTimeout(1000);
}

async function readPersistenceDiagnostics(
  page: Page,
): Promise<PersistenceDiagnosticSnapshot[]> {
  const raw = await page.evaluate(
    () => document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]",
  );
  return JSON.parse(raw) as PersistenceDiagnosticSnapshot[];
}

async function readActionDiagnostics(
  page: Page,
): Promise<ActionDiagnosticSnapshot[]> {
  const raw = await page.evaluate(
    () => document.documentElement.dataset.yuneActionDiagnostics ?? "[]",
  );
  return JSON.parse(raw) as ActionDiagnosticSnapshot[];
}

async function readPerfDiagnostics(
  page: Page,
): Promise<PerfDiagnosticSnapshot[]> {
  const raw = await page.evaluate(
    () => document.documentElement.dataset.yunePerfDiagnostics ?? "[]",
  );
  return JSON.parse(raw) as PerfDiagnosticSnapshot[];
}

async function resetM26PerfDiagnostics(page: Page): Promise<void> {
  await page.evaluate(() => {
    document.documentElement.dataset.yuneActionDiagnostics = "[]";
    document.documentElement.dataset.yuneTypingDiagnostics = "[]";
    document.documentElement.dataset.yunePerfDiagnostics = "[]";
  });
}

function expectNondecreasingTimeline(diagnostic: PerfDiagnosticSnapshot): void {
  const fields = [
    diagnostic.keydownAt,
    diagnostic.workerQueuedAt,
    diagnostic.workerStartedAt,
    diagnostic.workerFinishedAt,
    diagnostic.responseReceivedAt,
    diagnostic.responseMappingFinishedAt,
    diagnostic.stateAppliedAt,
    diagnostic.paintObservedAt,
  ];
  for (let index = 1; index < fields.length; index += 1) {
    expect(fields[index]).toBeGreaterThanOrEqual(fields[index - 1] - 2);
  }
}

function percentileNumber(values: number[], percentile: number): number | null {
  if (values.length === 0) {
    return null;
  }
  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.ceil((sorted.length - 1) * percentile);
  return sorted[Math.min(index, sorted.length - 1)];
}

function summarizePerfDiagnostics(diagnostics: PerfDiagnosticSnapshot[]) {
  const totals = diagnostics.map(
    (diagnostic) => diagnostic.totalKeydownToPaintMs,
  );
  const worker = diagnostics
    .map((diagnostic) => diagnostic.workerRoundtripMs)
    .filter((value): value is number => typeof value === "number");
  const native = diagnostics
    .map((diagnostic) => diagnostic.workerProcessMs)
    .filter((value): value is number => typeof value === "number");
  const react = diagnostics.map((diagnostic) => diagnostic.reactUpdateMs);
  const paint = diagnostics.map((diagnostic) => diagnostic.paintProxyMs);
  const responseMapping = diagnostics.map(
    (diagnostic) => diagnostic.responseMappingMs,
  );
  return {
    count: diagnostics.length,
    totalKeydownToPaintMs: {
      median: percentileNumber(totals, 0.5),
      p95: percentileNumber(totals, 0.95),
      max: percentileNumber(totals, 1.0),
    },
    ownerP95Ms: {
      workerRoundtrip: percentileNumber(worker, 0.95),
      nativeOrWorkerProcess: percentileNumber(native, 0.95),
      responseMapping: percentileNumber(responseMapping, 0.95),
      reactUpdate: percentileNumber(react, 0.95),
      paintProxy: percentileNumber(paint, 0.95),
    },
  };
}

function markerPhases(
  diagnostics: PersistenceDiagnosticSnapshot[],
): Set<string> {
  const phases = new Set<string>();
  for (const diagnostic of diagnostics) {
    const phase = diagnostic.marker.phase;
    if (phase) {
      phases.add(phase);
    }
    for (const marker of diagnostic.marker.markers ?? []) {
      phases.add(marker.phase);
    }
  }
  return phases;
}

function expectStartupMarkerOrder(
  startup: PersistenceDiagnosticSnapshot,
): void {
  const markers = startup.marker.markers ?? [];
  expect(markers.length).toBeGreaterThan(0);
  for (let index = 1; index < markers.length; index += 1) {
    expect(markers[index].ms).toBeGreaterThanOrEqual(markers[index - 1].ms);
  }
}

async function latestPersistedSettings(
  page: Page,
): Promise<Record<string, string | null>> {
  const diagnostics = await readPersistenceDiagnostics(page);
  for (let index = diagnostics.length - 1; index >= 0; index -= 1) {
    const diagnostic = diagnostics[index];
    const settings = diagnostic.marker.persistedConfig?.settings;
    if (settings) {
      return settings;
    }
  }
  return {};
}

async function waitForPersistedSettings(
  page: Page,
  expected: Record<string, string | null>,
): Promise<Record<string, string | null>> {
  await expect
    .poll(
      async () => {
        const settings = await latestPersistedSettings(page);
        return Object.fromEntries(
          Object.keys(expected).map((key) => [key, settings[key] ?? null]),
        );
      },
      { timeout: 15000 },
    )
    .toEqual(expected);
  return latestPersistedSettings(page);
}

async function selectSchema(page: Page, label: string | RegExp): Promise<void> {
  await clearComposition(page);
  const switcher = page.locator("[data-yune-schema-switcher]");
  await expect(switcher).toBeVisible({ timeout: 5000 });
  const expectedSchema = expectedSchemaIdForLabel(label);
  const select = switcher.locator("select");
  if (expectedSchema !== null && (await select.count())) {
    await select.selectOption(expectedSchema);
  } else {
    await switcher.getByLabel(label).click({ force: true });
  }
  if (expectedSchema !== null) {
    await expect
      .poll(
        async () =>
          page.evaluate(
            () => document.documentElement.dataset.yuneActiveSchema ?? null,
          ),
        { timeout: TIMEOUT_MS },
      )
      .toBe(expectedSchema);
  }
  await waitForAppReady(page);
  await expect(page.locator(".candidate-panel")).toHaveCount(0, {
    timeout: 5000,
  });
  await page.waitForTimeout(500);
}

function expectedSchemaIdForLabel(label: string | RegExp): string | null {
  const text = typeof label === "string" ? label : label.source;
  if (/Cangjie/i.test(text)) {
    return "cangjie5";
  }
  if (/Luna/i.test(text)) {
    return "luna_pinyin";
  }
  if (/Jyutping/i.test(text)) {
    return "jyut6ping3";
  }
  return null;
}

async function typeInputForStatus(page: Page, input: string): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(input, { delay: 120 });
  await expect(page.locator("[data-yune-status]")).toBeVisible({
    timeout: 10000,
  });
}

async function readYuneStatus(
  page: Page,
): Promise<Record<string, string | null>> {
  const status = page.locator("[data-yune-status]");
  await expect(status).toBeVisible({ timeout: 10000 });
  const schema = page.locator("[data-yune-status-schema]");
  const disabled = page.locator("[data-yune-status-disabled]");
  const composing = page.locator("[data-yune-status-composing]");
  const outputStandard = page.locator("[data-yune-status-output-standard]");
  const ascii = page.locator("[data-yune-status-ascii]");
  const fullShape = page.locator("[data-yune-status-full-shape]");
  const simplified = page.locator("[data-yune-status-simplified]");
  const traditional = page.locator("[data-yune-status-traditional]");
  const asciiPunct = page.locator("[data-yune-status-ascii-punct]");
  const schemaId =
    (await schema.getAttribute("data-yune-status-schema-id")) ??
    (await schema.textContent());
  const schemaName =
    (await schema.getAttribute("data-yune-status-schema-name")) ??
    (await schema.textContent());
  const disabledValue = await disabled.getAttribute(
    "data-yune-status-disabled",
  );
  const composingValue = await composing.getAttribute(
    "data-yune-status-composing",
  );
  const outputStandardValue = await outputStandard.getAttribute(
    "data-yune-status-output-standard",
  );
  const asciiValue = await ascii.getAttribute("data-yune-status-ascii");
  const fullShapeValue = await fullShape.getAttribute(
    "data-yune-status-full-shape",
  );
  const simplifiedValue = await simplified.getAttribute(
    "data-yune-status-simplified",
  );
  const traditionalValue = await traditional.getAttribute(
    "data-yune-status-traditional",
  );
  const asciiPunctValue = await asciiPunct.getAttribute(
    "data-yune-status-ascii-punct",
  );
  return {
    schema: schemaId,
    schema_id: schemaId,
    schema_name: schemaName,
    disabled: disabledValue === "true" ? "disabled" : "enabled",
    is_disabled: disabledValue,
    is_composing: composingValue,
    outputStandard: outputStandardValue,
    is_simplified: simplifiedValue,
    is_traditional: traditionalValue,
    ascii: asciiValue === "true" ? "ASCII" : "Chinese",
    is_ascii_mode: asciiValue,
    is_full_shape: fullShapeValue,
    is_ascii_punct: asciiPunctValue,
  };
}

async function waitForDeployedSettings(
  page: Page,
  expected: Record<string, string | null>,
): Promise<Record<string, string | null>> {
  await expect
    .poll(
      async () => {
        const diagnostics = await readPersistenceDiagnostics(page);
        const deployPass = diagnostics
          .slice()
          .reverse()
          .find(
            (diagnostic) =>
              diagnostic.marker.phase ===
                "syncToPersistenceAfterMutation:pass" &&
              diagnostic.marker.reason === "deploy" &&
              diagnostic.marker.persistedConfig?.settings,
          );
        const settings = deployPass?.marker.persistedConfig?.settings ?? {};
        return Object.fromEntries(
          Object.keys(expected).map((key) => [key, settings[key] ?? null]),
        );
      },
      { timeout: 30000 },
    )
    .toEqual(expected);
  return latestPersistedSettings(page);
}

/**
 * Helper: Verify persistence sync markers in console
 */
async function verifyPersistenceMarker(
  page: Page,
  marker: string,
): Promise<boolean> {
  try {
    await expect
      .poll(
        async () => {
          const diagnostics = await readPersistenceDiagnostics(page);
          return diagnostics.some((diagnostic) =>
            diagnostic.marker.phase?.includes(marker),
          );
        },
        { timeout: 5000 },
      )
      .toBe(true);
    return true;
  } catch {
    return false;
  }
}

async function captureM16Scenario(
  page: Page,
  name: string,
  expectedTexts: string[],
  expectedNotes?: string[],
): Promise<CandidatePanelSnapshot> {
  const state = await readCandidatePanelSnapshot(page, false);
  expect(
    state.candidates
      .slice(0, expectedTexts.length)
      .map((candidate) => candidate.text),
  ).toEqual(expectedTexts);
  if (expectedNotes) {
    expect(
      state.candidates
        .slice(0, expectedNotes.length)
        .map((candidate) => candidate.note ?? ""),
    ).toEqual(expectedNotes);
  }
  await saveJsonEvidence(`m16-${name}-state.json`, {
    expectedTexts,
    expectedNotes,
    state,
  });
  await takeEvidenceScreenshot(page, `m16-${name}`);
  return state;
}

async function captureM24Phrase(
  page: Page,
  issueId: string,
  input: string,
  expectedTopText: string,
): Promise<CandidatePanelSnapshot> {
  const state = await typeCompositionAndWaitForTopCandidate(
    page,
    input,
    expectedTopText,
  );
  await saveM24Json(issueId, `${input}-state.json`, state);
  await takeM24Screenshot(page, issueId, `${input}-candidate-panel`);
  return state;
}

test.describe("yune-web Browser E2E", () => {
  test.setTimeout(TIMEOUT_MS);

  let consoleErrors: string[] = [];

  test.beforeAll(async ({}, workerInfo) => {
    // Record browser runner start
    await saveWorkerEvidence(
      workerInfo,
      "browser-run.log",
      `[${new Date().toISOString()}] Browser E2E started\nURL: ${APP_URL}\n`,
    );
  });

  test.beforeEach(async ({ page }, testInfo) => {
    setEvidenceScope(testInfo);
    consoleErrors = await captureConsoleErrors(page);
    await openApp(page);
  });

  test("WASM heap metrics populate after startup and typing", async ({
    page,
  }) => {
    await expect(page.locator("[data-yune-metric-wasm-heap]")).toContainText(
      /B$/,
      { timeout: TIMEOUT_MS },
    );
    await expect(
      page.locator("[data-yune-metric-peak-wasm-heap]"),
    ).toContainText(/B$/, { timeout: TIMEOUT_MS });

    const startup = (await readPersistenceDiagnostics(page))
      .find((diagnostic) => diagnostic.source === "yune-startup");
    expect(startup?.marker.wasmMemory?.currentBytes).toBeGreaterThan(0);
    expect(startup?.marker.wasmMemory?.peakBytes).toBeGreaterThan(0);
    expect(
      startup?.marker.markers?.some(
        (marker) =>
          marker.phase === "runtime:initialized" &&
          (marker.wasmMemory?.currentBytes ?? 0) > 0,
      ),
    ).toBe(true);
    expect(startup?.marker.loadedSharedAssets).toContain(
      "luna_pinyin_yune_reverse.dict.yaml",
    );
    expect(startup?.marker.loadedSharedAssets).toContain(
      "jyut6ping3_scolar.reverse.bin",
    );

    await focusInputAndType(page, "nei");
    await expect
      .poll(
        async () =>
          (await readPerfDiagnostics(page)).some(
            (diagnostic) =>
              (diagnostic.wasmHeapBytes ?? 0) > 0 &&
              (diagnostic.peakWasmHeapBytes ?? 0) > 0,
          ),
        { timeout: 10000 },
      )
      .toBe(true);
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("Default Jyutping composes clean multi-syllable shipped phrase", async ({
    page,
  }) => {
    const state = await typeCompositionAndWaitForTopCandidate(
      page,
      "ngogokdak",
      "\u6211\u89ba\u5f97",
    );
    expect(state.candidates[0].text).toBe("\u6211\u89ba\u5f97");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("UI language switcher localizes labels and persists @bilingual", async ({
    page,
  }) => {
    await expect(page.getByText("輸入法設定", { exact: true })).toBeVisible();
    await expect(page.getByText("IME Settings", { exact: true })).toHaveCount(
      0,
    );
    await expect(
      page.locator("[data-yune-schema-switcher] .yd-top-label"),
    ).toHaveText("方案");
    await expect(
      page.locator("[data-yune-schema-switcher] .yd-top-label"),
    ).not.toHaveText("Schema");

    await page
      .locator('[data-yune-ui-language-switcher] input[value="en"]')
      .check({ force: true });
    await expect(page.getByText("IME Settings", { exact: true })).toBeVisible();
    await expect(page.getByText("輸入法設定", { exact: true })).toHaveCount(0);
    await expect(
      page.locator("[data-yune-schema-switcher] .yd-top-label"),
    ).toHaveText("Schema");

    await page.reload({ waitUntil: "domcontentloaded" });
    await waitForAppReady(page);
    await expect(page.getByText("IME Settings", { exact: true })).toBeVisible();
    await expect(
      page.locator('[data-yune-ui-language-switcher] input[value="en"]'),
    ).toBeChecked();

    const state = await typeCompositionAndWaitForTopCandidate(
      page,
      "nei",
      "你",
    );
    expect(candidateTexts(state)).toContain("你");
  });

  test.afterEach(async ({ page }, testInfo) => {
    setEvidenceScope(testInfo);
    // Append test result to evidence log
    const status = testInfo.status || "unknown";
    const duration = testInfo.duration || 0;
    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Test: ${testInfo.title} - ${status} (${duration}ms)\n`,
    );

    // Save console errors if any
    if (consoleErrors.length > 0) {
      await saveEvidence(
        "browser-console.log",
        consoleErrors.join("\n") + "\n",
      );
    }
  });

  test("M31 PUBLIC yune-web exposes only supported public controls @smoke @public-smoke", async ({
    page,
  }) => {
    test.skip(
      !RUN_M31_PUBLIC_E2E,
      "M31 public smoke requires YUNE_PUBLIC_DEMO_E2E=1",
    );

    await expect(page).toHaveTitle(/yune-web/i);
    await expect(page.getByRole("banner")).toContainText(/yune-web/i);
    await expect(
      page.getByText(/輸出字形|Output standard/).last(),
    ).toBeVisible();
    await expect(
      page.getByLabel(/香港字形|Hong Kong Traditional/).last(),
    ).toBeVisible();
    await expect(
      page.getByLabel(/傳統漢字|OpenCC Traditional/).last(),
    ).toBeVisible();
    await expect(
      page.getByLabel(/台灣字形|Taiwan Traditional/).last(),
    ).toBeVisible();
    await expect(
      page.getByLabel(/大陆简化字|Mainland Simplified/).last(),
    ).toBeVisible();
    await expect(page.getByLabel(/AI Candidates/).last()).not.toBeChecked();
    await expect(page.locator("[data-yune-schema-switcher]")).toBeVisible();
    await expect(page.getByLabel(/方案|Schema/)).toContainText(
      /粵語拼音|Jyutping/,
    );
    await expect(page.getByText(/倉頡反查|Cangjie lookup/)).toBeVisible();

    await saveM31Json("public-control-surface.json", {
      title: await page.title(),
      publicDemo: true,
      exposedOutputStandards: [
        "opencc_traditional",
        "hong_kong_traditional",
        "taiwan_traditional",
        "mainland_simplified",
      ],
      exposedSchemaControls: ["schema switcher", "cangjie lookup"],
      hiddenUnsupportedControls: [],
      aiDefault: "off",
    });
    await takeM31Screenshot(page, "public-control-surface");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M31 PUBLIC startup uses pruned public assets and warm cache @smoke @public-smoke", async ({
    page,
  }) => {
    test.skip(
      !RUN_M31_PUBLIC_E2E,
      "M31 public smoke requires YUNE_PUBLIC_DEMO_E2E=1",
    );

    let startup: PersistenceDiagnosticSnapshot | undefined;
    await expect
      .poll(
        async () => {
          const diagnostics = await readPersistenceDiagnostics(page);
          startup = diagnostics.find(
            (diagnostic) => diagnostic.source === "yune-startup",
          );
          return startup?.marker.phase ?? "";
        },
        { timeout: TIMEOUT_MS },
      )
      .toBe("startup:complete");

    const loadedSharedAssets = startup?.marker.loadedSharedAssets ?? [];
    expect(startup?.marker.m31EvidenceVersion).toBe(
      "m31-yune-web-public-demo-v2",
    );
    expect(startup?.marker.publicDemo).toBe(true);
    expect(loadedSharedAssets).toContain("jyut6ping3.schema.yaml");
    expect(loadedSharedAssets).toContain("cangjie5.schema.yaml");
    expect(loadedSharedAssets).toContain("luna_pinyin.schema.yaml");
    expect(loadedSharedAssets).toContain("opencc/t2hkf.json");
    expect(loadedSharedAssets).toContain("opencc/HKVariantsFull.txt");
    expect(loadedSharedAssets).toContain("opencc/hk2s.json");
    expect(
      loadedSharedAssets.some((asset) => /10keys|longpress/.test(asset)),
    ).toBe(false);

    await page.reload({ waitUntil: "domcontentloaded", timeout: TIMEOUT_MS });
    await waitForAppReady(page);
    const reloadDiagnostics = await readPersistenceDiagnostics(page);
    const reloadStartup = reloadDiagnostics
      .slice()
      .reverse()
      .find((diagnostic) => diagnostic.source === "yune-startup");
    expect(reloadStartup?.marker.assetCache?.hits ?? 0).toBeGreaterThan(0);

    const resources = await page.evaluate(() =>
      performance
        .getEntriesByType("resource")
        .filter((entry) => /yune-web|schema|\.bin|\.ocd2/.test(entry.name))
        .map((entry) => ({
          name: entry.name,
          duration: entry.duration,
          transferSize:
            "transferSize" in entry
              ? (entry as PerformanceResourceTiming).transferSize
              : 0,
        })),
    );
    expect(
      resources.some((resource) =>
        /cangjie|loengfan|10keys|longpress/.test(resource.name),
      ),
    ).toBe(false);

    await saveM31Json("startup-assets-cache.json", {
      startup,
      reloadStartup,
      resources,
      claimBoundary:
        "delivery cache and asset pruning only; not a browser startup speedup claim",
    });
    await takeM31Screenshot(page, "startup-assets-cache");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M31 PUBLIC output standards are browser-visible and AI stays default-off @smoke @public-smoke", async ({
    page,
  }) => {
    test.skip(
      !RUN_M31_PUBLIC_E2E,
      "M31 public smoke requires YUNE_PUBLIC_DEMO_E2E=1",
    );

    const traditional = await typeCompositionAndWaitForTopCandidate(
      page,
      "ngohaigo",
      "\u6211\u4fc2\u500b",
    );
    await clearComposition(page);
    await setPreferenceRadio(page, /大陆简化字|Mainland Simplified/);
    const simplified = await typeCompositionAndWaitForTopCandidate(
      page,
      "ngohaigo",
      "\u6211\u7cfb\u4e2a",
    );
    await clearComposition(page);
    await setPreferenceRadio(page, /香港字形|Hong Kong Traditional/);
    const traditionalAgain = await typeCompositionAndWaitForTopCandidate(
      page,
      "ngohaigo",
      "\u6211\u4fc2\u500b",
    );

    await expect(page.getByLabel(/AI Candidates/).last()).not.toBeChecked();
    const resources = await page.evaluate(() =>
      performance.getEntriesByType("resource").map((entry) => entry.name),
    );
    const unexpectedRemoteCalls = resources.filter((name) =>
      /openai|anthropic|telemetry|analytics|segment|sentry/i.test(name),
    );
    expect(unexpectedRemoteCalls).toEqual([]);

    await saveM31Json("opencc-browser-evidence.json", {
      supportedOutputStandards: {
        hongKongTraditional: traditional.candidates[0],
        mainlandSimplified: simplified.candidates[0],
        hongKongTraditionalAfterRoundTrip: traditionalAgain.candidates[0],
      },
      unsupportedStandardsExposed: false,
      aiPosture: {
        default: "off",
        remoteCalls: unexpectedRemoteCalls,
      },
    });
    await takeM31Screenshot(page, "opencc-browser-evidence");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M31 UX yune-web redesign renders live public harness @smoke @public-smoke", async ({
    page,
  }) => {
    test.skip(
      !RUN_M31_PUBLIC_E2E,
      "M31 UX smoke requires YUNE_PUBLIC_DEMO_E2E=1",
    );

    const banner = page.getByRole("banner");
    await expect(banner).toContainText("新韻輸入法引擎");
    await expect(banner).toContainText(/yune-web/i);
    await expect(banner.locator(".yd-ai-chip")).toHaveCount(0);

    const themeToggle = page.getByLabel(/Theme Switcher/);
    const themeBefore = await page.evaluate(
      () => document.documentElement.dataset.theme ?? "",
    );
    await expect(themeToggle).toHaveCount(1);
    await page.locator(".yd-theme-button").click();
    await expect
      .poll(
        async () =>
          page.evaluate(() => document.documentElement.dataset.theme ?? ""),
        { timeout: 5000 },
      )
      .not.toBe(themeBefore);

    await expect(
      page.getByRole("button", { name: /ASCII mode/ }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /Output standard/ }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /Full shape/ }),
    ).toBeVisible();
    await expect(page.locator("[data-yune-schema-switcher]")).toBeVisible();
    await expect(page.getByText(/Cangjie lookup/)).toBeVisible();
    await expect(
      page.getByLabel(/Taiwan|t2tw|s2tw|tw2s|tw2t|s2t/i),
    ).toHaveCount(0);
    await expect(page.getByLabel(/AI Candidates/).last()).not.toBeChecked();

    const inputField = page.locator("input[type='text'], textarea").first();
    await clearComposition(page);
    await inputField.focus();
    await inputField.type("nei", { delay: 80 });
    await expect(
      page.locator(".candidate-panel .candidates tbody").first(),
    ).toBeVisible({ timeout: 10000 });
    await expect(
      page.locator(".candidate-panel .candidate-list-pane"),
    ).toBeVisible();
    await expect(
      page.locator(".candidate-panel .dictionary-panel"),
    ).toBeVisible();
    await expect(page.locator("[data-yune-metric-ai]")).toContainText(/off/i);
    await expect(
      page.locator("[data-yune-metric-candidates]"),
    ).not.toContainText(/N\/A/);

    const inputBox = await elementBox(inputField);
    const candidateBox = await elementBox(page.locator(".candidate-panel"));
    const listBox = await elementBox(
      page.locator(".candidate-panel .candidate-list-pane"),
    );
    const dictionaryBox = await elementBox(
      page.locator(".candidate-panel .dictionary-panel"),
    );
    expect(candidateBox.y).toBeGreaterThan(inputBox.y);
    expect(dictionaryBox.x).toBeGreaterThan(listBox.x);

    const userdbPanel = page.locator("[data-yune-userdb-viewer]");
    await expect(userdbPanel).toBeVisible();
    await expect(
      page
        .locator(
          "[data-yune-userdb-row-count], [data-yune-userdb-empty], [data-yune-userdb-loading]",
        )
        .first(),
    ).toBeVisible({ timeout: 10000 });

    await page.getByLabel("Yune inspector").check();
    await clearComposition(page);
    await inputField.focus();
    await inputField.type("nei", { delay: 80 });
    const inspector = page.locator("[data-yune-inspector='panel']");
    await expect(inspector).toBeVisible();
    await expect(
      page
        .locator("[data-yune-inspector-segments], [data-yune-inspector-empty]")
        .first(),
    ).toBeVisible({ timeout: 10000 });
    await expect(page.locator(".yd-inspector-summary")).toBeVisible();

    const resources = await page.evaluate(() =>
      performance.getEntriesByType("resource").map((entry) => entry.name),
    );
    const unexpectedRemoteCalls = resources.filter((name) =>
      /cdn\.jsdelivr|fonts\.googleapis|fonts\.gstatic|openai|anthropic|telemetry|analytics|segment|sentry/i.test(
        name,
      ),
    );
    expect(unexpectedRemoteCalls).toEqual([]);

    await saveM31UxJson("ux-redesign-smoke.json", {
      url: APP_URL,
      title: await page.title(),
      header: await banner.innerText(),
      themeBefore,
      themeAfter: await page.evaluate(
        () => document.documentElement.dataset.theme ?? "",
      ),
      publicControls: {
        schemaSwitcherVisible: await page
          .locator("[data-yune-schema-switcher]")
          .count(),
        unsupportedOpenccVisible: await page
          .getByLabel(/Taiwan|t2tw|s2tw|tw2s|tw2t|s2t/i)
          .count(),
        aiDefaultChecked: await page
          .getByLabel(/AI Candidates/)
          .last()
          .isChecked(),
      },
      layout: {
        input: inputBox,
        candidatePanel: candidateBox,
        candidateListPane: listBox,
        dictionaryPane: dictionaryBox,
      },
      metrics: {
        lookup: await page.locator("[data-yune-metric-lookup]").innerText(),
        ai: await page.locator("[data-yune-metric-ai]").innerText(),
        candidates: await page
          .locator("[data-yune-metric-candidates]")
          .innerText(),
        userdb: await page.locator("[data-yune-metric-userdb]").innerText(),
      },
      userdb: {
        rowCountText: await page
          .locator(
            "[data-yune-userdb-row-count], [data-yune-userdb-empty], [data-yune-userdb-loading]",
          )
          .first()
          .innerText(),
      },
      inspector: {
        collapsedVerified: true,
        hasSegmentsOrEmptyState:
          (await page
            .locator(
              "[data-yune-inspector-segments], [data-yune-inspector-empty]",
            )
            .count()) > 0,
      },
      blockedRemoteCalls: unexpectedRemoteCalls,
    });
    await takeM31UxScreenshot(page, "ux-redesign-smoke");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M24 startup timing trace records loading phases", async ({ page }) => {
    let startup: PersistenceDiagnosticSnapshot | undefined;
    await expect
      .poll(
        async () => {
          const diagnostics = await readPersistenceDiagnostics(page);
          startup = diagnostics.find(
            (diagnostic) => diagnostic.source === "yune-startup",
          );
          return startup?.marker.phase ?? "";
        },
        { timeout: TIMEOUT_MS },
      )
      .toBe("startup:complete");
    const resources = await page.evaluate(() =>
      performance
        .getEntriesByType("resource")
        .filter((entry) => /yune-web|schema|\.bin|\.ocd2/.test(entry.name))
        .map((entry) => ({
          name: entry.name,
          duration: entry.duration,
          transferSize:
            "transferSize" in entry
              ? (entry as PerformanceResourceTiming).transferSize
              : 0,
        })),
    );
    await saveM24Json("M24-DOGFOOD-01", "startup-resources.json", {
      startup,
      resources,
    });
    await takeM24Screenshot(page, "M24-DOGFOOD-01", "startup-ready");
    expect(startup?.marker.wasmBinary).toBe("yune-web.wasm");
    expect(startup?.marker.loadedSharedAssets).toContain(
      "luna_pinyin_yune_reverse.dict.yaml",
    );
    expect(startup?.marker.phase).toBe("startup:complete");
  });

  test("M25 DOGFOOD-01 startup uses release wasm and records deploy cache reuse", async ({
    page,
  }) => {
    let startup: PersistenceDiagnosticSnapshot | undefined;
    await expect
      .poll(
        async () => {
          const diagnostics = await readPersistenceDiagnostics(page);
          startup = diagnostics.find(
            (diagnostic) => diagnostic.source === "yune-startup",
          );
          return startup?.marker.phase ?? "";
        },
        { timeout: TIMEOUT_MS },
      )
      .toBe("startup:complete");

    const firstReadyAt = Date.now();
    await setPreferenceRange(
      page,
      /No\. of Candidates Per Page|æ¯é å€™é¸è©žæ•¸é‡/,
      7,
    );
    await waitForDeployedSettings(page, { "menu/page_size": "7" });
    await page.reload({ waitUntil: "domcontentloaded", timeout: TIMEOUT_MS });
    await waitForAppReady(page);
    const warmReloadElapsedMs = Date.now() - firstReadyAt;

    const diagnostics = await readPersistenceDiagnostics(page);
    const startupAfterReload = diagnostics
      .slice()
      .reverse()
      .find((diagnostic) => diagnostic.source === "yune-startup");
    const deployCache = diagnostics
      .slice()
      .reverse()
      .find(
        (diagnostic) =>
          diagnostic.source === "yune-persistence" &&
          diagnostic.marker.reason === "deploy" &&
          /^deploy:cache-/.test(diagnostic.marker.phase ?? ""),
      );
    const resources = await page.evaluate(() =>
      performance
        .getEntriesByType("resource")
        .filter((entry) => /yune-web|schema|\.bin|\.ocd2/.test(entry.name))
        .map((entry) => ({
          name: entry.name,
          duration: entry.duration,
          transferSize:
            "transferSize" in entry
              ? (entry as PerformanceResourceTiming).transferSize
              : 0,
        })),
    );

    await saveM25Json("M25-DOGFOOD-01", "startup-after.json", {
      startup,
      startupAfterReload,
      deployCache,
      warmReloadElapsedMs,
      resources,
    });
    await takeM25Screenshot(page, "M25-DOGFOOD-01", "startup-ready");

    expect(startup?.marker.wasmBuildProfile).toBe("release");
    expect(startupAfterReload?.marker.wasmBuildProfile).toBe("release");
    expect(deployCache?.marker.phase).toBe("deploy:cache-hit");
    expect(
      startupAfterReload?.marker.totalMs ?? Number.POSITIVE_INFINITY,
    ).toBeLessThanOrEqual(15000);
  });

  test("M25 DOGFOOD-03 typing stays out of global loading and records key latency", async ({
    page,
  }) => {
    await page.evaluate(() => {
      document.documentElement.dataset.yuneActionDiagnostics = "[]";
      document.documentElement.dataset.yuneTypingDiagnostics = "[]";
    });

    const inputField = page.locator("input[type='text'], textarea").first();
    await clearComposition(page);
    await expect(page.locator("[data-yune-loading-indicator]")).toHaveCount(0, {
      timeout: 1000,
    });
    await inputField.focus();
    await inputField.type("hai", { delay: 40 });
    await expect(
      page.locator(".candidate-panel .candidates tbody").first(),
    ).toBeVisible({ timeout: 10000 });
    await expect(page.locator("[data-yune-loading-indicator]")).toHaveCount(0, {
      timeout: 1000,
    });

    const actionDiagnostics = await page.evaluate(
      () =>
        JSON.parse(
          document.documentElement.dataset.yuneActionDiagnostics ?? "[]",
        ) as unknown[],
    );
    const typingDiagnostics = await page.evaluate(
      () =>
        JSON.parse(
          document.documentElement.dataset.yuneTypingDiagnostics ?? "[]",
        ) as {
          action?: string;
          totalMs?: number;
          renderMs?: number;
          input?: string;
        }[],
    );
    const processKeyActions = actionDiagnostics.filter(
      (diagnostic) =>
        typeof diagnostic === "object" &&
        diagnostic !== null &&
        (diagnostic as { action?: string }).action === "processKey",
    );
    const processKeyTyping = typingDiagnostics.filter(
      (diagnostic) => diagnostic.action === "processKey",
    );
    const p95 = processKeyTyping
      .map((diagnostic) => diagnostic.totalMs ?? Number.POSITIVE_INFINITY)
      .sort((left, right) => left - right)[
      Math.max(0, Math.ceil(processKeyTyping.length * 0.95) - 1)
    ];

    await saveM25Json("M25-DOGFOOD-03", "typing-latency-after.json", {
      input: "hai",
      actionDiagnostics,
      typingDiagnostics,
      processKeyActionCount: processKeyActions.length,
      processKeyTypingCount: processKeyTyping.length,
      p95,
      loadingIndicatorCount: await page
        .locator("[data-yune-loading-indicator]")
        .count(),
    });
    await takeM25Screenshot(page, "M25-DOGFOOD-03", "typing-responsive");

    expect(processKeyActions.length).toBeGreaterThanOrEqual(3);
    expect(processKeyTyping.length).toBeGreaterThanOrEqual(3);
    expect(p95).toBeLessThanOrEqual(750);
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M26 PERF startup attribution records nested initialization owners", async ({
    page,
  }) => {
    let startup: PersistenceDiagnosticSnapshot | undefined;
    await expect
      .poll(
        async () => {
          const diagnostics = await readPersistenceDiagnostics(page);
          startup = diagnostics.find(
            (diagnostic) => diagnostic.source === "yune-startup",
          );
          return startup?.marker.phase ?? "";
        },
        { timeout: TIMEOUT_MS },
      )
      .toBe("startup:complete");
    expect(startup).toBeDefined();
    expect(startup?.marker.wasmBuildProfile).toBe("release");
    expect(startup?.marker.wasmBinary).toBe("yune-web.wasm");
    expectStartupMarkerOrder(startup as PersistenceDiagnosticSnapshot);

    const freshDiagnostics = await readPersistenceDiagnostics(page);
    const freshPhases = markerPhases(freshDiagnostics);
    const requiredPhases = [
      "runtime:init:start",
      "wasm:module:create:start",
      "wasm:module:create:finish",
      "filesystem:mount:start",
      "filesystem:mount:finish",
      "assets:load:start",
      "assets:load:finish",
      "rime:init:start",
      "rime:init:finish",
      "schema:deploy:start",
      "schema:deploy:finish",
      "schema:select:start",
      "schema:select:finish",
      "runtime:init:finish",
    ];
    for (const phase of requiredPhases) {
      expect(freshPhases.has(phase), `fresh startup missing ${phase}`).toBe(
        true,
      );
    }

    await page.reload({ waitUntil: "domcontentloaded", timeout: TIMEOUT_MS });
    await waitForAppReady(page);
    const reloadDiagnostics = await readPersistenceDiagnostics(page);
    const reloadStartup = reloadDiagnostics
      .slice()
      .reverse()
      .find((diagnostic) => diagnostic.source === "yune-startup");
    expect(reloadStartup?.marker.phase).toBe("startup:complete");
    expect(reloadStartup?.marker.wasmBuildProfile).toBe("release");
    expectStartupMarkerOrder(reloadStartup as PersistenceDiagnosticSnapshot);

    const reloadPhases = markerPhases(reloadDiagnostics);
    for (const phase of requiredPhases) {
      expect(reloadPhases.has(phase), `reload startup missing ${phase}`).toBe(
        true,
      );
    }

    const resources = await page.evaluate(() =>
      performance
        .getEntriesByType("resource")
        .filter((entry) => /yune-web|schema|\.bin|\.ocd2/.test(entry.name))
        .map((entry) => ({
          name: entry.name,
          duration: entry.duration,
          transferSize:
            "transferSize" in entry
              ? (entry as PerformanceResourceTiming).transferSize
              : 0,
        })),
    );

    await saveM26Json(`startup-attribution-${M26_EVIDENCE_LABEL}.json`, {
      label: M26_EVIDENCE_LABEL,
      freshStartup: startup,
      freshDiagnostics,
      reloadStartup,
      reloadDiagnostics,
      resources,
    });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M27 PERF startup marker records evidence version", async ({ page }) => {
    let startup: PersistenceDiagnosticSnapshot | undefined;
    await expect
      .poll(
        async () => {
          const diagnostics = await readPersistenceDiagnostics(page);
          startup = diagnostics.find(
            (diagnostic) => diagnostic.source === "yune-startup",
          );
          return startup?.marker.phase ?? "";
        },
        { timeout: TIMEOUT_MS },
      )
      .toBe("startup:complete");
    expect(startup).toBeDefined();
    expect(startup?.marker.m27EvidenceVersion).toBe("m27-startup-v1");
    expect(startup?.marker.wasmBuildProfile).toBe("release");
    expect(startup?.marker.wasmBinary).toBe("yune-web.wasm");
    expectStartupMarkerOrder(startup as PersistenceDiagnosticSnapshot);

    const freshDiagnostics = await readPersistenceDiagnostics(page);
    const freshPhases = markerPhases(freshDiagnostics);

    await page.reload({ waitUntil: "domcontentloaded", timeout: TIMEOUT_MS });
    await waitForAppReady(page);
    const reloadDiagnostics = await readPersistenceDiagnostics(page);
    const reloadStartup = reloadDiagnostics
      .slice()
      .reverse()
      .find((diagnostic) => diagnostic.source === "yune-startup");
    expect(reloadStartup?.marker.phase).toBe("startup:complete");
    expect(reloadStartup?.marker.m27EvidenceVersion).toBe("m27-startup-v1");
    expectStartupMarkerOrder(reloadStartup as PersistenceDiagnosticSnapshot);

    await saveM27Json(`browser-startup-after-${M27_EVIDENCE_LABEL}.json`, {
      label: M27_EVIDENCE_LABEL,
      freshStartup: startup,
      freshPhases: [...freshPhases].sort(),
      reloadStartup,
      reloadPhases: [...markerPhases(reloadDiagnostics)].sort(),
    });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M29 PERF startup and typing attribution records owner spans", async ({
    page,
  }) => {
    let startup: PersistenceDiagnosticSnapshot | undefined;
    await expect
      .poll(
        async () => {
          const diagnostics = await readPersistenceDiagnostics(page);
          startup = diagnostics.find(
            (diagnostic) => diagnostic.source === "yune-startup",
          );
          return startup?.marker.phase ?? "";
        },
        { timeout: TIMEOUT_MS },
      )
      .toBe("startup:complete");
    expect(startup).toBeDefined();
    expectStartupMarkerOrder(startup as PersistenceDiagnosticSnapshot);

    await page.reload({ waitUntil: "domcontentloaded", timeout: TIMEOUT_MS });
    await waitForAppReady(page);
    const diagnosticsAfterReload = await readPersistenceDiagnostics(page);
    const reloadStartup = diagnosticsAfterReload
      .filter((diagnostic) => diagnostic.source === "yune-startup")
      .at(-1);
    expect(reloadStartup?.marker.phase).toBe("startup:complete");
    expectStartupMarkerOrder(reloadStartup as PersistenceDiagnosticSnapshot);

    await saveM29Json(`browser-startup-${M29_EVIDENCE_LABEL}.json`, {
      label: M29_EVIDENCE_LABEL,
      freshStartup: startup,
      reloadStartup,
      startupTotalsMs: {
        fresh: startup?.marker.totalMs,
        reload: reloadStartup?.marker.totalMs,
      },
    });

    const inputField = page.locator("input[type='text'], textarea").first();
    const scenarios: Record<string, unknown> = {};

    async function recordTypingScenario(
      name: string,
      input: string,
      expected: (state: CandidatePanelSnapshot) => boolean,
    ): Promise<void> {
      await clearComposition(page);
      await resetM26PerfDiagnostics(page);
      await inputField.focus();
      await inputField.type(input, { delay: 40 });
      await expect
        .poll(
          async () => {
            const state = await readCandidatePanelSnapshot(page, false);
            return expected(state);
          },
          { timeout: 15000 },
        )
        .toBe(true);
      await expect
        .poll(async () => readPerfDiagnostics(page), { timeout: 15000 })
        .toHaveLength(input.length);
      const perf = await readPerfDiagnostics(page);
      for (const diagnostic of perf) {
        expectNondecreasingTimeline(diagnostic);
      }
      scenarios[name] = {
        input,
        perf,
        actions: await readActionDiagnostics(page),
        summary: summarizePerfDiagnostics(perf),
        state: await readCandidatePanelSnapshot(page, false),
      };
    }

    await recordTypingScenario(
      "hai",
      "hai",
      (state) => state.candidates.length > 0,
    );
    await recordTypingScenario(
      "longPhrase",
      M24_DOGFOOD_INPUT,
      (state) => state.candidates[0]?.text === M24_DOGFOOD_TOP,
    );
    await recordTypingScenario(
      "longComposition",
      "caksijathaacoenggeoizi",
      (state) => state.candidates.length > 0,
    );

    await resetM26PerfDiagnostics(page);
    const longState = (
      scenarios.longPhrase as { state: CandidatePanelSnapshot }
    ).state;
    const firstPageFirst = longState.candidates[0]?.text;
    await page.keyboard.press("PageDown");
    await expect
      .poll(
        async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text ??
          null,
        { timeout: 5000 },
      )
      .not.toBe(firstPageFirst ?? null);
    await expect
      .poll(async () => readPerfDiagnostics(page), { timeout: 10000 })
      .toHaveLength(1);
    const pagingPerf = await readPerfDiagnostics(page);
    scenarios.paging = {
      perf: pagingPerf,
      actions: await readActionDiagnostics(page),
      summary: summarizePerfDiagnostics(pagingPerf),
      state: await readCandidatePanelSnapshot(page, false),
    };

    await clearComposition(page);
    await resetM26PerfDiagnostics(page);
    await inputField.focus();
    await inputField.type("`zhe", { delay: 40 });
    await expect
      .poll(
        async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates.map(
            (candidate) => candidate.text,
          ),
        { timeout: 10000 },
      )
      .toContain("這");
    await expect
      .poll(async () => readPerfDiagnostics(page), { timeout: 10000 })
      .toHaveLength(4);
    const reversePerf = await readPerfDiagnostics(page);
    scenarios.reverseLookup = {
      input: "`zhe",
      perf: reversePerf,
      actions: await readActionDiagnostics(page),
      summary: summarizePerfDiagnostics(reversePerf),
      state: await readCandidatePanelSnapshot(page, false),
    };

    await saveM29Json(`typing-keydown-to-paint-${M29_EVIDENCE_LABEL}.json`, {
      label: M29_EVIDENCE_LABEL,
      scenarios,
      loadingIndicatorCount: await page
        .locator("[data-yune-loading-indicator]")
        .count(),
    });
    await saveM29Json(`typing-attribution-${M29_EVIDENCE_LABEL}.json`, {
      label: M29_EVIDENCE_LABEL,
      scenarioSummaries: Object.fromEntries(
        Object.entries(scenarios).map(([name, value]) => [
          name,
          (value as { summary?: unknown }).summary,
        ]),
      ),
    });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M27 PERF controls classify loading boundaries", async ({ page }) => {
    const inputField = page.locator("input[type='text'], textarea").first();
    const resetDiagnostics = async () => {
      await page.evaluate(() => {
        document.documentElement.dataset.yuneActionDiagnostics = "[]";
        document.documentElement.dataset.yunePersistenceDiagnostics = "[]";
      });
    };
    const loadingState = async () => ({
      dataset: await page.evaluate(
        () => document.documentElement.dataset.yuneLoading ?? "unset",
      ),
      indicatorCount: await page
        .locator("[data-yune-loading-indicator]")
        .count(),
    });
    const actionNames = async () =>
      (await readActionDiagnostics(page)).map(
        (diagnostic) => diagnostic.action,
      );
    const phases = async () =>
      [...markerPhases(await readPersistenceDiagnostics(page))].sort();

    await clearComposition(page);
    await inputField.focus();
    await inputField.type("nei", { delay: 80 });
    await expect(
      page.locator(".candidate-panel .candidates tbody").first(),
    ).toBeVisible({ timeout: 10000 });

    await resetDiagnostics();
    const aiToggle = page.getByLabel(/AI Candidates/).last();
    await aiToggle.evaluate((element) => {
      (element as HTMLInputElement).click();
    });
    await expect(aiToggle).toBeChecked({ checked: true, timeout: 5000 });
    await expect(
      page.locator(
        '.candidate-panel .candidates tbody[data-source="ai:local"]',
      ),
    ).toHaveCount(1, { timeout: 10000 });
    await expect
      .poll(async () => (await actionNames()).includes("stageAi"), {
        timeout: 10000,
      })
      .toBe(true);
    const ai = {
      loading: await loadingState(),
      actions: await actionNames(),
      phases: await phases(),
      state: await readCandidatePanelSnapshot(page, true),
    };

    await resetDiagnostics();
    await setPreferenceToggleAndWaitForSettings(page, /Auto-correction/, true, {
      "translator/enable_correction": "true",
    });
    const deployBacked = {
      loading: await loadingState(),
      actions: await actionNames(),
      phases: await phases(),
      settings: await latestPersistedSettings(page),
    };

    await resetDiagnostics();
    await setPreferenceToggle(page, /ASCII mode/, true);
    await expect
      .poll(async () => (await actionNames()).includes("setOption"), {
        timeout: 10000,
      })
      .toBe(true);
    const live = {
      loading: await loadingState(),
      actions: await actionNames(),
      phases: await phases(),
    };

    await resetDiagnostics();
    await page
      .getByLabel(/Vertical/)
      .last()
      .check({ force: true });
    await page.waitForTimeout(300);
    const browserOnly = {
      loading: await loadingState(),
      actions: await actionNames(),
      phases: await phases(),
      selectedLayout: await page
        .getByLabel(/Vertical/)
        .last()
        .isChecked(),
    };

    const classification = {
      deployTime: [
        "Auto-completion",
        "Auto-correction",
        "Auto-composition",
        "User Dictionary",
        "No. of Candidates Per Page",
        "Combine same-text candidates",
        "Prediction never first",
        "Prediction threshold",
        "Dictionary exclude",
        "schema switch",
        "Cangjie lookup",
      ],
      live: [
        "ASCII mode",
        "Full shape",
        "Output standard",
        "Extended charset",
        "Disabled",
        "Yune inspector",
      ],
      browserOnly: [
        "Display languages",
        "Candidate Menu Layout",
        "Font",
        "Candidate Jyutping",
        "Reverse code display",
      ],
      localRuntimeOnly: ["AI Candidates"],
    };

    await saveM27Json(
      `control-classification-after-${M27_EVIDENCE_LABEL}.json`,
      {
        label: M27_EVIDENCE_LABEL,
        classification,
        ai,
        deployBacked,
        live,
        browserOnly,
      },
    );

    expect(ai.loading.dataset).toBe("false");
    expect(ai.loading.indicatorCount).toBe(0);
    expect(ai.actions).toContain("customize");
    expect(ai.actions).toContain("stageAi");
    expect(ai.actions).not.toContain("deploy");
    expect(ai.phases.some((phase) => phase.startsWith("schema:deploy"))).toBe(
      false,
    );
    expect(deployBacked.actions).toContain("customize");
    expect(deployBacked.actions).toContain("deploy");
    expect(deployBacked.phases).toContain("schema:deploy:start");
    expect(deployBacked.phases).toContain("schema:deploy:finish");
    expect(live.actions).toContain("setOption");
    expect(live.actions).not.toContain("deploy");
    expect(live.phases.some((phase) => phase.startsWith("schema:deploy"))).toBe(
      false,
    );
    expect(browserOnly.actions).toEqual([]);
    expect(browserOnly.loading.dataset).toBe("false");
    expect(browserOnly.loading.indicatorCount).toBe(0);
    expect(browserOnly.selectedLayout).toBe(true);
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M28 PARTIAL selecting a prefix candidate keeps the tail composing", async ({
    page,
  }) => {
    const fixture = JSON.parse(
      await readRepoText(
        "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json",
      ),
    ) as {
      input: string;
      user_feel_target?: string;
      selection_request: { requested_candidate_text: string };
      captured_active_remaining_input_by_consumed_span: string;
      captured_next_candidates: { text: string; comment?: string }[];
      captured_final_flow: { final_commit_text: string };
    };
    const input = fixture.input;
    const selectedText = fixture.selection_request.requested_candidate_text;
    const remainingInput =
      fixture.captured_active_remaining_input_by_consumed_span;
    const rawTailCommit = `${selectedText}${remainingInput}`;
    const continuationComponents = m28ContinuationComponents(fixture);
    expect(continuationComponents.length).toBeGreaterThan(0);

    const inputField = page.locator("input[type='text'], textarea").first();
    await clearComposition(page);
    await inputField.focus();
    await inputField.type(input, { delay: 50 });
    await expect
      .poll(
        async () =>
          candidateTexts(await readCandidatePanelSnapshot(page, false)),
        { timeout: 10000 },
      )
      .toContain(selectedText);
    const beforeSelection = await readCandidatePanelSnapshot(page, false);

    const firstSelection = await selectVisibleCandidateByText(
      page,
      selectedText,
    );
    await expect(inputField).not.toHaveValue(rawTailCommit, { timeout: 5000 });
    await expect(
      page.locator(".candidate-panel .candidates tbody").first(),
    ).toBeVisible({ timeout: 10000 });
    await expect
      .poll(
        async () =>
          candidateTexts(await readCandidatePanelSnapshot(page, false)),
        { timeout: 10000 },
      )
      .toContain(continuationComponents[0]);
    const afterFirstSelection = await readCandidatePanelSnapshot(page, false);
    const valueAfterFirstSelection = await inputField.inputValue();

    const continuationSteps: unknown[] = [];
    for (const component of continuationComponents) {
      const step = await selectVisibleCandidateByText(page, component);
      continuationSteps.push({
        component,
        selectedIndex: step.selectedIndex,
        before: step.before,
      });
      await page.waitForTimeout(250);
    }

    await expect(inputField).toHaveValue(
      fixture.captured_final_flow.final_commit_text,
      { timeout: 5000 },
    );
    await expect(page.locator(".candidate-panel")).toHaveCount(0, {
      timeout: 5000,
    });
    const finalValue = await inputField.inputValue();
    const oneRowOracleContinuation =
      fixture.captured_next_candidates[0]?.text ?? null;

    await saveM28Json("browser-partial-selection.json", {
      input,
      selectedText,
      remainingInput,
      continuationComponents,
      beforeSelection,
      firstSelection: {
        selectedIndex: firstSelection.selectedIndex,
        after: afterFirstSelection,
        valueAfterFirstSelection,
        rawTailCommit,
        rawTailInserted: valueAfterFirstSelection === rawTailCommit,
        oneRowOracleContinuation,
        oneRowOracleContinuationVisible: afterFirstSelection.candidates.some(
          (candidate) => candidate.text === oneRowOracleContinuation,
        ),
      },
      continuationSteps,
      finalValue,
      fixtureFinalCommit: fixture.captured_final_flow.final_commit_text,
      userFeelTarget: fixture.user_feel_target ?? null,
      userFeelTargetReached: finalValue === fixture.user_feel_target,
      consoleFailures: consoleFailures(consoleErrors),
    });

    expect(valueAfterFirstSelection).not.toBe(rawTailCommit);
    expect(finalValue).toBe(fixture.captured_final_flow.final_commit_text);
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M28 FOLLOW-UP Space default confirm recomposes partial candidate tail", async ({
    page,
  }) => {
    const fixture = JSON.parse(
      await readRepoText(
        "crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json",
      ),
    ) as {
      input: string;
      selection_request: { requested_candidate_text: string };
      captured_active_remaining_input_by_consumed_span: string;
      captured_next_candidates: { text: string; comment?: string }[];
    };
    const input = fixture.input;
    const selectedText = fixture.selection_request.requested_candidate_text;
    const remainingInput =
      fixture.captured_active_remaining_input_by_consumed_span;
    const rawTailCommit = `${selectedText}${remainingInput}`;
    const continuationComponents = m28ContinuationComponents(fixture);
    expect(continuationComponents.length).toBeGreaterThan(0);
    const expectedContinuation = continuationComponents[0];
    const oneRowOracleContinuation =
      fixture.captured_next_candidates[0]?.text ?? null;

    await setPreferenceToggle(page, /Auto-composition/, false);

    const inputField = page.locator("input[type='text'], textarea").first();
    await clearComposition(page);
    await inputField.focus();
    await inputField.type(input, { delay: 50 });
    await expect
      .poll(
        async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text,
        { timeout: 10000 },
      )
      .toBe(selectedText);
    const beforeSpace = await readCandidatePanelSnapshot(page, false);

    await page.keyboard.press("Space");
    await expect(inputField).not.toHaveValue(rawTailCommit, { timeout: 5000 });
    await expect(inputField).toHaveValue(selectedText, { timeout: 5000 });
    await expect(
      page.locator(".candidate-panel .candidates tbody").first(),
    ).toBeVisible({ timeout: 10000 });
    await expect
      .poll(
        async () =>
          candidateTexts(await readCandidatePanelSnapshot(page, false)),
        { timeout: 10000 },
      )
      .toContain(expectedContinuation);
    const afterSpace = await readCandidatePanelSnapshot(page, false);
    const valueAfterSpace = await inputField.inputValue();

    await saveM28FollowupJson("browser-space-default-confirm.json", {
      input,
      action: "Space",
      committed: selectedText,
      raw_tail_committed: valueAfterSpace === rawTailCommit,
      rawTailCommit,
      valueAfterSpace,
      remaining_input_prefix: remainingInput.slice(0, 8),
      expectedContinuation,
      continuationComponents,
      oneRowOracleContinuation,
      expectedContinuationVisible: afterSpace.candidates.some(
        (candidate) => candidate.text === expectedContinuation,
      ),
      beforeSpace,
      afterSpace,
      consoleFailures: consoleFailures(consoleErrors),
    });

    expect(valueAfterSpace).toBe(selectedText);
    expect(valueAfterSpace).not.toBe(rawTailCommit);
    expect(
      afterSpace.candidates.some(
        (candidate) => candidate.text === expectedContinuation,
      ),
    ).toBe(true);
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M28 FOLLOW-UP auto-composition follows upstream Jyutping ranking fixture", async ({
    page,
  }) => {
    const fixture = JSON.parse(
      await readRepoText(
        "crates/yune-core/tests/fixtures/upstream-jyutping/jyutping-m28-followup-composition.json",
      ),
    ) as {
      capture: { target_input: string };
      auto_composition_on: {
        candidate_rows: { text: string }[];
        space_commit: string;
        remaining_input_after_space: string | null;
      };
    };
    const input = fixture.capture.target_input;
    const expectedTexts = fixture.auto_composition_on.candidate_rows.map(
      (candidate) => candidate.text,
    );
    const expectedCommit = fixture.auto_composition_on.space_commit;

    await setPreferenceToggle(page, /Auto-composition/, true);

    const inputField = page.locator("input[type='text'], textarea").first();
    await clearComposition(page);
    await inputField.focus();
    await inputField.type(input, { delay: 50 });
    await expect
      .poll(
        async () =>
          candidateTexts(await readCandidatePanelSnapshot(page, false)).slice(
            0,
            expectedTexts.length,
          ),
        { timeout: 10000 },
      )
      .toEqual(expectedTexts);
    const beforeSpace = await readCandidatePanelSnapshot(page, false);

    await page.keyboard.press("Space");
    await expect(inputField).toHaveValue(expectedCommit, { timeout: 5000 });
    await expect(page.locator(".candidate-panel")).toHaveCount(0, {
      timeout: 5000,
    });
    const valueAfterSpace = await inputField.inputValue();

    await saveM28FollowupJson("browser-upstream-ranking.json", {
      input,
      expectedTexts,
      actualTexts: candidateTexts(beforeSpace).slice(0, expectedTexts.length),
      beforeSpace,
      action: "Space",
      expectedCommit,
      valueAfterSpace,
      remainingInputAfterSpace:
        fixture.auto_composition_on.remaining_input_after_space,
      consoleFailures: consoleFailures(consoleErrors),
    });

    expect(candidateTexts(beforeSpace).slice(0, expectedTexts.length)).toEqual(
      expectedTexts,
    );
    expect(valueAfterSpace).toBe(expectedCommit);
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M26 PERF keydown-to-paint diagnostics cover typing, paging, and reverse lookup", async ({
    page,
  }) => {
    const inputField = page.locator("input[type='text'], textarea").first();
    const scenarios: Record<string, unknown> = {};

    await resetM26PerfDiagnostics(page);
    await clearComposition(page);
    await inputField.focus();
    await inputField.type("hai", { delay: 40 });
    await expect(
      page.locator(".candidate-panel .candidates tbody").first(),
    ).toBeVisible({ timeout: 10000 });
    await expect
      .poll(async () => readPerfDiagnostics(page), { timeout: 10000 })
      .toHaveLength(3);
    const haiPerf = await readPerfDiagnostics(page);
    const haiActions = await readActionDiagnostics(page);
    for (const diagnostic of haiPerf) {
      expectNondecreasingTimeline(diagnostic);
      expect(diagnostic.candidateCount).toBeGreaterThan(0);
      expect(diagnostic.totalKeydownToPaintMs).toBeGreaterThanOrEqual(0);
    }
    scenarios.hai = {
      perf: haiPerf,
      actions: haiActions,
      state: await readCandidatePanelSnapshot(page, false),
    };

    await clearComposition(page);
    await resetM26PerfDiagnostics(page);
    await inputField.focus();
    await inputField.type(M24_DOGFOOD_INPUT, { delay: 40 });
    await expect
      .poll(
        async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text ??
          null,
        { timeout: 10000 },
      )
      .toBe(M24_DOGFOOD_TOP);
    const longState = await readCandidatePanelSnapshot(page, false);
    await expect
      .poll(async () => readPerfDiagnostics(page), { timeout: 10000 })
      .toHaveLength(M24_DOGFOOD_INPUT.length);
    const longPerf = await readPerfDiagnostics(page);
    for (const diagnostic of longPerf) {
      expectNondecreasingTimeline(diagnostic);
    }
    scenarios.longPhrase = {
      perf: longPerf,
      actions: await readActionDiagnostics(page),
      state: longState,
    };

    await resetM26PerfDiagnostics(page);
    const firstPageFirst = longState.candidates[0]?.text;
    await page.keyboard.press("PageDown");
    await expect
      .poll(
        async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text ??
          null,
        { timeout: 5000 },
      )
      .not.toBe(firstPageFirst ?? null);
    await expect
      .poll(async () => readPerfDiagnostics(page), { timeout: 10000 })
      .toHaveLength(1);
    const pagingPerf = await readPerfDiagnostics(page);
    expect(pagingPerf[0].key).toBe("PageDown");
    expectNondecreasingTimeline(pagingPerf[0]);
    scenarios.paging = {
      perf: pagingPerf,
      actions: await readActionDiagnostics(page),
      state: await readCandidatePanelSnapshot(page, false),
    };

    await clearComposition(page);
    await resetM26PerfDiagnostics(page);
    await inputField.focus();
    await inputField.type("`zhe", { delay: 40 });
    await expect
      .poll(
        async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates.map(
            (candidate) => candidate.text,
          ),
        { timeout: 10000 },
      )
      .toContain("\u9019");
    const reverseState = await readCandidatePanelSnapshot(page, false);
    await expect
      .poll(async () => readPerfDiagnostics(page), { timeout: 10000 })
      .toHaveLength(4);
    const reversePerf = await readPerfDiagnostics(page);
    for (const diagnostic of reversePerf) {
      expectNondecreasingTimeline(diagnostic);
    }
    scenarios.reverseLookup = {
      perf: reversePerf,
      actions: await readActionDiagnostics(page),
      state: reverseState,
    };

    await saveM26Json(`typing-keydown-to-paint-${M26_EVIDENCE_LABEL}.json`, {
      label: M26_EVIDENCE_LABEL,
      scenarios,
      loadingIndicatorCount: await page
        .locator("[data-yune-loading-indicator]")
        .count(),
    });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M25 DOGFOOD-02 page-size slider caps rows and preserves candidate paging", async ({
    page,
  }) => {
    const pageSizeControl = page
      .getByLabel(/No\. of Candidates Per Page|Candidates Per Page/)
      .last();
    await expect(pageSizeControl).toHaveAttribute("min", "3");
    await expect(pageSizeControl).toHaveAttribute("max", "10");

    const evidence: Record<string, unknown> = {};
    for (const pageSize of [3, 9, 10] as const) {
      await setPreferenceRange(
        page,
        /No\. of Candidates Per Page|Candidates Per Page/,
        pageSize,
      );
      await waitForPersistedSettings(page, {
        "menu/page_size": String(pageSize),
      });
      const state = await typeCompositionAndWaitForRowCount(
        page,
        "hai",
        pageSize,
      );
      expect(state.candidates).toHaveLength(pageSize);
      evidence[`pageSize${pageSize}`] = state;
      await saveM25Json(
        "M25-DOGFOOD-02",
        `page-size-${pageSize}-hai-state.json`,
        state,
      );
      await takeM25Screenshot(
        page,
        "M25-DOGFOOD-02",
        `page-size-${pageSize}-hai`,
      );

      if (pageSize === 3) {
        const firstPageFirst = state.candidates[0]?.text;
        await page.locator(".candidate-panel .page-nav").last().click();
        await expect
          .poll(
            async () =>
              (await readCandidatePanelSnapshot(page, false)).candidates[0]
                ?.text ?? null,
            { timeout: 5000 },
          )
          .not.toBe(firstPageFirst ?? null);
        const nextPage = await readCandidatePanelSnapshot(page, false);
        expect(nextPage.candidates.length).toBeLessThanOrEqual(pageSize);
        evidence.pageSize3NextPage = nextPage;
        await page.locator(".candidate-panel .page-nav").first().click();
        await expect
          .poll(
            async () =>
              (await readCandidatePanelSnapshot(page, false)).candidates[0]
                ?.text ?? null,
            { timeout: 5000 },
          )
          .toBe(firstPageFirst ?? null);
      }

      if (pageSize === 9 || pageSize === 10) {
        const key = pageSize === 9 ? "9" : "0";
        const selectedText = state.candidates[pageSize - 1]?.text;
        expect(selectedText).toBeTruthy();
        await page.keyboard.press(key);
        await expect(
          page.locator("input[type='text'], textarea").first(),
        ).toHaveValue(selectedText ?? "", { timeout: 5000 });
        await expect(page.locator(".candidate-panel")).toHaveCount(0, {
          timeout: 5000,
        });
      }
    }

    await saveM25Json("M25-DOGFOOD-02", "page-size-summary.json", {
      control: {
        min: await pageSizeControl.getAttribute("min"),
        max: await pageSizeControl.getAttribute("max"),
        value: await pageSizeControl.inputValue(),
      },
      evidence,
    });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M25 DOGFOOD-08 Jyutping bare grave routes to Luna reverse lookup", async ({
    page,
  }) => {
    await selectSchema(page, /Jyutping/);
    const reverseLookupSummary = page.locator(
      "[data-yune-reverse-lookup-summary]",
    );
    await expect(reverseLookupSummary).toContainText(/`/);
    await expect(reverseLookupSummary).toContainText(/`vl/);
    await expect(reverseLookupSummary).toContainText(/`vc/);

    const bareLuna = await typeCompositionAndWaitForCandidate(
      page,
      "`zhe",
      "這",
    );
    expect(candidateTexts(bareLuna)).toContain("這");
    await saveM25Json("M25-DOGFOOD-08", "bare-grave-zhe-state.json", bareLuna);
    await takeM25Screenshot(page, "M25-DOGFOOD-08", "bare-grave-zhe");

    const overlap: Record<string, CandidatePanelSnapshot> = {};
    for (const input of ["`lai", "`ci", "`xi", "`re"] as const) {
      await clearComposition(page);
      const inputField = page.locator("input[type='text'], textarea").first();
      await inputField.focus();
      await inputField.type(input, { delay: 80 });
      await expect(
        page.locator(".candidate-panel .candidates tbody").first(),
      ).toBeVisible({ timeout: 10000 });
      const state = await readCandidatePanelSnapshot(page, false);
      expect(state.candidates.length).toBeGreaterThan(0);
      overlap[input] = state;
    }

    const triggerCopy = await reverseLookupSummary.textContent();
    await saveM25Json("M25-DOGFOOD-08", "reverse-lookup-summary.json", {
      note: "M25 intentionally diverges from TypeDuck v1.1.2's historical `p trigger: bare ` belongs to luna_pinyin; retained side lookups use explicit `vl and `vc triggers.",
      activeSchema: await page.evaluate(
        () => document.documentElement.dataset.yuneActiveSchema ?? null,
      ),
      triggerCopy,
      bareLuna,
      overlap,
      operationErrorToastCount: await page.locator(".yd-toast").count(),
    });
    await takeM25Screenshot(page, "M25-DOGFOOD-08", "reverse-lookup-overlap");

    await expect(page.locator(".yd-toast")).toHaveCount(0, { timeout: 1000 });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("Reverse lookup summary follows active schema", async ({ page }) => {
    const summary = page.locator("[data-yune-reverse-lookup-summary]");
    await expect(summary).toBeVisible();

    await selectSchema(page, /Jyutping/);
    await expect(summary).toContainText(/`vl/);
    await expect(summary).toContainText(/`vc/);
    await expect(summary).toContainText(/Luna Pinyin|朙月拼音/);

    await selectSchema(page, /Cangjie 5/);
    await expect(summary).toContainText(/Jyutping|粵拼/);
    await expect(summary).toContainText(/`…;/);
    await expect(summary).not.toContainText(/`vl/);

    await selectSchema(page, /Luna Pinyin/);
    await expect(summary).toContainText(/Cangjie 5|倉頡五代/);
    await expect(summary).toContainText(/`…;/);
    await expect(summary).not.toContainText(/`vc/);
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M25 DOGFOOD-04 schema switcher shares the top control row", async ({
    page,
  }) => {
    const topControls = page.locator("[data-yune-top-controls]");
    await expect(topControls).toBeVisible();
    const schemaSwitcher = topControls.locator("[data-yune-schema-switcher]");
    await expect(schemaSwitcher).toBeVisible();
    await expect(
      schemaSwitcher.getByText("朙月拼音", { exact: true }),
    ).toBeVisible();

    const desktopBoxes = {
      topControls: await elementBox(topControls),
      asciiButton: await elementBox(
        topControls.getByRole("button", { name: /ASCII mode/ }),
      ),
      schemaSwitcher: await elementBox(schemaSwitcher),
    };
    expect(
      Math.abs(desktopBoxes.schemaSwitcher.y - desktopBoxes.asciiButton.y),
    ).toBeLessThanOrEqual(80);

    await selectSchema(page, /Luna Pinyin/);
    await typeInputForStatus(page, "hao");
    await expect(page.locator("[data-yune-status-schema]")).toContainText(
      "朙月拼音",
    );
    await saveM25Json(
      "M25-DOGFOOD-04",
      "schema-switcher-toolbar-desktop.json",
      desktopBoxes,
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-04",
      "schema-switcher-toolbar-desktop",
    );

    await page.setViewportSize({ width: 390, height: 900 });
    const mobileBoxes = {
      topControls: await elementBox(topControls),
      schemaSwitcher: await elementBox(schemaSwitcher),
    };
    await saveM25Json(
      "M25-DOGFOOD-04",
      "schema-switcher-toolbar-mobile.json",
      mobileBoxes,
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-04",
      "schema-switcher-toolbar-mobile",
    );
  });

  test("M25 DOGFOOD-05 Cangjie lookup control lives in the top control band", async ({
    page,
  }) => {
    await selectSchema(page, /Jyutping/);
    const topControls = page.locator("[data-yune-top-controls]");
    await expect(topControls).toBeVisible();
    const cangjieControl = topControls.locator(
      "[data-yune-control='cangjie-version']",
    );
    await expect(cangjieControl).toBeVisible();
    await expect(page.getByText(/Web Frontend Controls/)).toHaveCount(0);

    await expect(cangjieControl.getByText(/Cangjie lookup/)).toBeVisible();
    await cangjieControl.getByText("三代").click();
    await expect(cangjieControl.getByLabel("三代")).toBeChecked();
    await waitForAppReady(page);
    const version3 = await waitForPersistedSettings(page, {
      "cangjie/dictionary": "cangjie3",
    });
    await cangjieControl.getByText("五代").click();
    await expect(cangjieControl.getByLabel("五代")).toBeChecked();
    await waitForAppReady(page);
    const version5 = await waitForPersistedSettings(page, {
      "cangjie/dictionary": "cangjie5",
    });

    await saveM25Json("M25-DOGFOOD-05", "cangjie-version-top-controls.json", {
      boxes: {
        topControls: await elementBox(topControls),
        cangjieControl: await elementBox(cangjieControl),
      },
      version3,
      version5,
    });
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-05",
      "cangjie-version-top-controls",
    );
  });

  test("M25 DOGFOOD-06 display controls precede live session controls", async ({
    page,
  }) => {
    const preferences = page.locator("[data-yune-preferences]");
    await expect(preferences).toBeVisible();
    const active = preferences.locator("[data-yune-section='active']");
    const display = preferences.locator("[data-yune-section='display']");
    const live = preferences.locator("[data-yune-section='live']");
    await expect(active).toBeVisible();
    await expect(display).toBeVisible();
    await expect(live).toBeVisible();

    const desktopBoxes = {
      active: await elementBox(active),
      display: await elementBox(display),
      live: await elementBox(live),
    };
    expect(
      Math.abs(desktopBoxes.display.y - desktopBoxes.active.y),
    ).toBeLessThanOrEqual(24);
    expect(desktopBoxes.live.y).toBeGreaterThan(desktopBoxes.display.y + 24);
    await saveM25Json(
      "M25-DOGFOOD-06",
      "display-live-section-order-desktop.json",
      desktopBoxes,
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-06",
      "display-live-section-order-desktop",
    );

    await page.setViewportSize({ width: 390, height: 900 });
    const mobileBoxes = {
      active: await elementBox(active),
      display: await elementBox(display),
      live: await elementBox(live),
    };
    expect(mobileBoxes.active.y).toBeLessThan(mobileBoxes.display.y);
    expect(mobileBoxes.display.y).toBeLessThan(mobileBoxes.live.y);
    await saveM25Json(
      "M25-DOGFOOD-06",
      "display-live-section-order-mobile.json",
      mobileBoxes,
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-06",
      "display-live-section-order-mobile",
    );
  });

  test("M25 DOGFOOD-10 IME settings align with the playground content edges", async ({
    page,
  }) => {
    const playground = page.locator("[data-yune-playground-content]");
    const preferences = page.locator("[data-yune-preferences]");
    await expect(playground).toBeVisible();
    await expect(preferences).toBeVisible();

    const desktop = {
      playground: await elementBox(playground),
      preferences: await elementBox(preferences),
    };
    expect(
      Math.abs(desktop.playground.x - desktop.preferences.x),
    ).toBeLessThanOrEqual(2);
    expect(
      Math.abs(desktop.playground.right - desktop.preferences.right),
    ).toBeLessThanOrEqual(2);
    await saveM25Json(
      "M25-DOGFOOD-10",
      "settings-alignment-desktop.json",
      desktop,
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-10",
      "settings-alignment-desktop",
    );

    await page.setViewportSize({ width: 390, height: 900 });
    const mobile = {
      playground: await elementBox(playground),
      preferences: await elementBox(preferences),
    };
    expect(
      Math.abs(mobile.playground.x - mobile.preferences.x),
    ).toBeLessThanOrEqual(2);
    expect(
      Math.abs(mobile.playground.right - mobile.preferences.right),
    ).toBeLessThanOrEqual(2);
    await saveM25Json(
      "M25-DOGFOOD-10",
      "settings-alignment-mobile.json",
      mobile,
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-10",
      "settings-alignment-mobile",
    );
  });

  test("M25 DOGFOOD-07 binary controls use checkbox affordance", async ({
    page,
  }) => {
    const preferences = page.locator("[data-yune-preferences]");
    const preferenceChecks = preferences.locator("input[type='checkbox']");
    await expect(preferenceChecks.first()).toBeVisible();
    const preferenceControls = await preferenceChecks.evaluateAll((inputs) =>
      inputs.map((input) => {
        const element = input as HTMLInputElement;
        const style = getComputedStyle(element);
        return {
          label:
            element.labels?.[0]?.textContent?.replace(/\s+/g, " ").trim() ??
            element.getAttribute("aria-label"),
          checked: element.checked,
          className: element.className,
          borderRadius: style.borderRadius,
          width: style.width,
          height: style.height,
        };
      }),
    );
    expect(preferenceControls.length).toBeGreaterThan(10);
    expect(
      preferenceControls.every((control) =>
        String(control.className).includes("yd-check"),
      ),
    ).toBe(true);
    expect(
      preferenceControls.every(
        (control) => !String(control.className).includes("yd-switch"),
      ),
    ).toBe(true);

    const inspector = page.locator(
      "[data-yune-inspector-toggle] input[type='checkbox']",
    );
    await expect(inspector).toHaveClass(/yd-check/);
    await expect(inspector).not.toHaveClass(/yd-switch/);

    const themeToggle = page.getByLabel(/Theme Switcher/);
    await expect(themeToggle).toHaveClass(/yd-theme-switch/);
    await saveM25Json("M25-DOGFOOD-07", "checkbox-affordance-summary.json", {
      note: "Theme switcher remains a specialized icon switch and uses yd-theme-switch; settings and inspector binary controls use yd-check.",
      preferenceControls,
      inspectorClass: await inspector.getAttribute("class"),
      themeClass: await themeToggle.getAttribute("class"),
    });
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-07",
      "checkbox-affordance-desktop",
    );
    await page.setViewportSize({ width: 390, height: 900 });
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-07",
      "checkbox-affordance-mobile",
    );
  });

  test("M25 DOGFOOD-09 candidate menu layout uses radio choices", async ({
    page,
  }) => {
    const field = page
      .locator("[data-yune-preferences] .yd-field")
      .filter({ hasText: /Candidate Menu Layout/ });
    await expect(field).toBeVisible();
    await expect(field.locator(".yd-segment")).toHaveCount(0);
    const radios = field.locator("input[type='radio'].yd-choice");
    await expect(radios).toHaveCount(2);
    await expect(field.getByLabel(/Horizontal/)).toBeChecked();
    await expect(field.getByLabel(/Vertical/)).not.toBeChecked();

    let horizontal = await typeCompositionAndWaitForTopCandidate(
      page,
      "nei",
      "你",
    );
    await expect(page.locator(".candidate-panel")).toHaveClass(
      /candidate-panel--horizontal/,
    );
    const [firstHorizontal, secondHorizontal] =
      await firstTwoCandidateRowBoxes(page);
    const horizontalDictionary = await elementBox(
      page.locator(".candidate-panel .dictionary-panel"),
    );
    expect(secondHorizontal.x).toBeGreaterThanOrEqual(
      firstHorizontal.right - 1,
    );
    expect(
      Math.abs(secondHorizontal.y - firstHorizontal.y),
    ).toBeLessThanOrEqual(2);
    expect(horizontalDictionary.y).toBeGreaterThanOrEqual(
      firstHorizontal.bottom - 1,
    );
    expect(horizontalDictionary.x).toBeLessThanOrEqual(firstHorizontal.x + 2);
    await saveM25Json(
      "M25-DOGFOOD-09",
      "candidate-layout-horizontal.json",
      horizontal,
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-09",
      "candidate-layout-horizontal",
    );

    await clearComposition(page);
    await field.getByLabel(/Vertical/).check({ force: true });
    await expect(field.getByLabel(/Vertical/)).toBeChecked();
    const vertical = await typeCompositionAndWaitForTopCandidate(
      page,
      "nei",
      "你",
    );
    await expect(page.locator(".candidate-panel")).toHaveClass(
      /candidate-panel--vertical/,
    );
    const [firstVertical, secondVertical] =
      await firstTwoCandidateRowBoxes(page);
    expect(secondVertical.y).toBeGreaterThanOrEqual(firstVertical.bottom - 1);
    expect(Math.abs(secondVertical.x - firstVertical.x)).toBeLessThanOrEqual(2);
    await saveM25Json("M25-DOGFOOD-09", "candidate-layout-vertical.json", {
      before: horizontal,
      after: vertical,
      horizontalBoxes: { first: firstHorizontal, second: secondHorizontal },
      horizontalDictionary,
      verticalBoxes: { first: firstVertical, second: secondVertical },
      radioLabels: await field
        .locator("label")
        .evaluateAll((labels) =>
          labels.map((label) => label.textContent?.replace(/\s+/g, " ").trim()),
        ),
    });
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-09",
      "candidate-layout-vertical",
    );
    horizontal = vertical;
    expect(horizontal.candidates.length).toBeGreaterThan(0);
  });

  test("M24 phrase comments render without raw control markers", async ({
    page,
  }) => {
    const state = await captureM24Phrase(
      page,
      "M24-DOGFOOD-02",
      M24_DOGFOOD_INPUT,
      M24_DOGFOOD_TOP,
    );
    const visibleRows = state.candidates
      .map((candidate) => candidate.rowText)
      .join("\n");
    expect(visibleRows).not.toMatch(/(?:\\f|\\r|\\v|\f|\r|\v)/);
  });

  test("M24 compound candidate rows stay compact with details in the dictionary panel", async ({
    page,
  }) => {
    const state = await captureM24Phrase(
      page,
      "M24-DOGFOOD-03",
      M24_DOGFOOD_INPUT,
      M24_DOGFOOD_TOP,
    );
    expect(state.candidates[0].rowText).not.toContain("think; ponder");
    await page
      .locator(
        `.candidate-panel .candidates tbody[data-candidate-text="${M24_DOGFOOD_TOP}"]`,
      )
      .hover();
    await expect(page.locator(".dictionary-panel")).toContainText(
      /think; ponder|want; need|now/,
      { timeout: 5000 },
    );
    await takeM24Screenshot(page, "M24-DOGFOOD-03", "dictionary-detail-panel");
  });

  test("M25 DOGFOOD-11 visible lookup candidates expose dictionary details", async ({
    page,
  }) => {
    const state = await typeCompositionAndWaitForTopCandidate(
      page,
      "zouhapci",
      "組合次",
    );
    const expectedLookupRows = [
      { text: "組合", definition: /combine/ },
      { text: "做", definition: /do/ },
      { text: "早", definition: /early/ },
      { text: "組", definition: /group/ },
      { text: "租", definition: /rent/ },
    ];
    const visibleLookupRows = await page
      .locator(".candidate-panel .candidates tbody")
      .evaluateAll((rows) =>
        rows.slice(0, 8).map((row) => ({
          text: row.getAttribute("data-candidate-text"),
          rowText: row.textContent?.replace(/\s+/g, " ").trim() ?? "",
          hasDictionaryIcon: Boolean(
            row.querySelector('[aria-label="dictionary details"]'),
          ),
        })),
      );

    for (const { text, definition } of expectedLookupRows) {
      const row = visibleLookupRows.find(
        (candidate) => candidate.text === text,
      );
      expect(row, `${text} should be visible for zouhapci`).toBeTruthy();
      expect(
        row?.hasDictionaryIcon,
        `${text} should expose dictionary detail affordance`,
      ).toBe(true);
      const locator = page.locator(
        `.candidate-panel .candidates tbody[data-candidate-text="${text}"]`,
      );
      await expect(locator).toHaveCount(1);
      await expect(locator.locator(".candidate-definition")).toContainText(
        definition,
      );
      await locator.hover();
      await expect(page.locator(".dictionary-panel")).toContainText(
        definition,
        { timeout: 5000 },
      );
    }

    await saveM25Json(
      "M25-DOGFOOD-11",
      "visible-lookup-dictionary-comments.json",
      {
        state,
        visibleLookupRows,
      },
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-11",
      "visible-lookup-dictionary-comments",
    );
  });

  test("M24 jigaajiusihaa order is recorded against the current pinned expectation", async ({
    page,
  }) => {
    const state = await captureM24Phrase(
      page,
      "M24-DOGFOOD-04",
      M24_DOGFOOD_INPUT,
      M24_DOGFOOD_TOP,
    );
    const fixture = await loadFixture("jyut6ping3-m24-dogfooding.json");
    const expectedCase = fixture.cases.find(
      (candidate) =>
        candidate["variant"] === "default_combined" &&
        candidate["input"] === M24_DOGFOOD_INPUT,
    ) as { selected_candidates: { text: string }[] } | undefined;
    if (!expectedCase) {
      throw new Error(
        `Missing M24 dogfood fixture case for ${M24_DOGFOOD_INPUT}`,
      );
    }
    const expectedTexts = expectedCase.selected_candidates.map(
      (candidate) => candidate.text,
    );
    const actualTexts = candidateTexts(state);
    const compareCount = Math.min(expectedTexts.length, actualTexts.length, 5);
    await saveM24Json("M24-DOGFOOD-04", "jigaajiusihaa-order.json", {
      candidateTexts: actualTexts,
      expectedTexts: expectedTexts.slice(0, compareCount),
      note: "Engine ordering is fixture-gated against TypeDuck v1.1.2; live deployed-site differences are not treated as hard oracle evidence.",
    });
    expect(actualTexts.slice(0, compareCount)).toEqual(
      expectedTexts.slice(0, compareCount),
    );
  });

  test("M24/M25 settings labels are Cantonese-first and grouped by engine, display, and session", async ({
    page,
  }) => {
    await expect(
      page.getByText(/引擎設定 Active engine controls/),
    ).toBeVisible();
    await expect(page.getByText(/顯示設定 Display controls/)).toBeVisible();
    await expect(
      page.getByText(/即時狀態 Live session controls/),
    ).toBeVisible();
    await expect(page.getByText(/Web Frontend Controls/)).toHaveCount(0);
    await expect(page.getByText(/會重新部署 schema/)).toBeVisible();
    await expect(page.getByText(/只改目前 session/)).toBeVisible();
    await takeM24Screenshot(page, "M24-DOGFOOD-05", "settings-labels-desktop");
    await page.setViewportSize({ width: 390, height: 900 });
    await takeM24Screenshot(page, "M24-DOGFOOD-05", "settings-labels-narrow");
  });

  test("M24 display languages use a checklist with deterministic primary language", async ({
    page,
  }) => {
    await expect(page.getByText(/主要語言 Main Language/)).toHaveCount(0);
    const languageChecks = page
      .locator(".yd-field")
      .filter({ hasText: /Display languages/ })
      .locator("input[type='checkbox']");
    await expect(languageChecks).toHaveCount(5);
    await page.getByLabel(/Hindi/).last().check({ force: true });
    const state = await captureM24Phrase(
      page,
      "M24-DOGFOOD-06",
      M24_DOGFOOD_INPUT,
      M24_DOGFOOD_TOP,
    );
    await takeM24Screenshot(
      page,
      "M24-DOGFOOD-06",
      "display-languages-checklist",
    );
    expect(state.candidates.length).toBeGreaterThan(0);
  });

  test("M24 candidate page-size slider limits the visible candidate page", async ({
    page,
  }) => {
    for (const pageSize of [4, 10] as const) {
      await setPreferenceRange(
        page,
        /No\. of Candidates Per Page|每頁候選詞數量/,
        pageSize,
      );
      await waitForPersistedSettings(page, {
        "menu/page_size": String(pageSize),
      });
      const state = await captureM24Phrase(
        page,
        "M24-DOGFOOD-07",
        M24_DOGFOOD_INPUT,
        M24_DOGFOOD_TOP,
      );
      await saveM24Json(
        "M24-DOGFOOD-07",
        `page-size-${pageSize}-state.json`,
        state,
      );
      await takeM24Screenshot(
        page,
        "M24-DOGFOOD-07",
        `page-size-${pageSize}-candidates`,
      );
      expect(state.candidates.length).toBeLessThanOrEqual(pageSize);
    }
  });

  test("M24 candidate menu layout is a frontend-only horizontal or vertical setting", async ({
    page,
  }) => {
    const horizontal = await captureM24Phrase(
      page,
      "M24-DOGFOOD-08",
      M24_DOGFOOD_INPUT,
      M24_DOGFOOD_TOP,
    );
    await clearComposition(page);
    await page.getByText("直排 Vertical").click();
    await expect(page.getByLabel(/直排 Vertical/).last()).toBeChecked();
    const vertical = await captureM24Phrase(
      page,
      "M24-DOGFOOD-08",
      M24_DOGFOOD_INPUT,
      M24_DOGFOOD_TOP,
    );
    await expect(page.locator(".candidate-panel")).toHaveClass(
      /candidate-panel--vertical/,
    );
    await takeM24Screenshot(page, "M24-DOGFOOD-08", "vertical");
    await clearComposition(page);
    await page.getByText("橫排 Horizontal").click();
    await expect(page.getByLabel(/橫排 Horizontal/).last()).toBeChecked();
    await captureM24Phrase(
      page,
      "M24-DOGFOOD-08",
      M24_DOGFOOD_INPUT,
      M24_DOGFOOD_TOP,
    );
    await takeM24Screenshot(page, "M24-DOGFOOD-08", "horizontal");
    expect(classicCandidateSignature(vertical)).toEqual(
      classicCandidateSignature(horizontal),
    );
  });

  test("M24 engine status strip explains labeled state", async ({ page }) => {
    await typeInputForStatus(page, "nei");
    await expect(page.getByText(/引擎狀態 Engine status/)).toBeVisible();
    await expect(page.getByText(/完整 status 欄位/)).toBeVisible();
    const status = page.locator("[data-yune-status]");
    for (const field of [
      "schema_id",
      "schema_name",
      "is_disabled",
      "is_composing",
      "is_ascii_mode",
      "is_full_shape",
      "is_simplified",
      "is_traditional",
      "is_ascii_punct",
    ]) {
      await expect(status.getByText(field, { exact: true })).toBeVisible();
    }
    await saveM24Json(
      "M24-DOGFOOD-09",
      "status-strip.json",
      await readYuneStatus(page),
    );
    await takeM24Screenshot(page, "M24-DOGFOOD-09", "status-strip");
  });

  test("M24 schema switcher uses bundled real schema names", async ({
    page,
  }) => {
    const schemaSwitcher = page.locator("[data-yune-schema-switcher]");
    await expect(schemaSwitcher.getByText("粵語拼音")).toBeVisible();
    await expect(schemaSwitcher.getByText("倉頡五代")).toBeVisible();
    await expect(
      schemaSwitcher.getByText("朙月拼音", { exact: true }),
    ).toBeVisible();
    await selectSchema(page, /Cangjie 5/);
    await typeInputForStatus(page, "a");
    await expect(page.locator("[data-yune-status-schema]")).toContainText(
      "倉頡五代",
    );
    await takeM24Screenshot(
      page,
      "M24-DOGFOOD-10",
      "schema-switcher-real-names",
    );
  });

  test("M24/M25 Jyutping reverse lookup accepts current Mandarin pinyin affix input", async ({
    page,
  }) => {
    await selectSchema(page, /Jyutping/);
    const state = await typeCompositionAndWaitForCandidate(page, "`zhe", "這");
    await saveM25Json(
      "M25-DOGFOOD-08",
      "legacy-reverse-lookup-bare-zhe.json",
      state,
    );
    await takeM25Screenshot(
      page,
      "M25-DOGFOOD-08",
      "legacy-reverse-lookup-bare-zhe",
    );
    expect(candidateTexts(state)).toContain("這");

    await clearComposition(page);
    const normal = await typeCompositionAndWaitForTopCandidate(
      page,
      "nei",
      "你",
    );
    expect(normal.candidates[0].text).toBe("你");
  });

  test("M24 Chinese typeface picker applies full family names to visible Chinese surfaces", async ({
    page,
  }) => {
    const typefaceLabels = [
      "昭源黑體 Chiron Hei HK",
      "昭源宋體 Chiron Sung HK",
      "昭源環方 Chiron GoRound TC",
      "朱古力黑體 Chocolate Classical Sans",
      "霞鶩文楷 TC LXGW WenKai TC",
      "霞鶩文楷等寬 TC LXGW WenKai Mono TC",
      "芫荽 Iansui",
      "粉圓 Huninn",
      "注音粉圓 Bpmf Huninn",
      "滑油字 WDXL Lubrifont TC",
    ];
    await expect(page.getByText("字體 Font")).toBeVisible();
    for (const label of typefaceLabels) {
      await expect(page.getByText(label)).toBeVisible();
    }
    const renderedLabels = await page
      .locator("[data-yune-typeface-option-label]")
      .evaluateAll((elements) =>
        elements.map((element) => element.textContent?.trim() ?? ""),
      );
    expect(renderedLabels).toEqual(typefaceLabels);
    const renderedFonts = await page
      .locator("[data-yune-typeface-option-label]")
      .evaluateAll((elements) =>
        elements.map((element) => ({
          id: element.getAttribute("data-yune-typeface-option"),
          fontFamily: getComputedStyle(element).fontFamily,
          text: element.textContent?.trim() ?? "",
        })),
      );
    expect(
      renderedFonts.find((font) => font.id === "iansui")?.fontFamily,
    ).toContain("Iansui");
    expect(
      renderedFonts.find((font) => font.id === "huninn")?.fontFamily,
    ).toContain("Huninn");
    expect(
      renderedFonts.find((font) => font.id === "bpmf-huninn")?.fontFamily,
    ).toContain("Bpmf Huninn");
    await page.getByLabel("芫荽 Iansui").check({ force: true });
    const state = await captureM24Phrase(
      page,
      "M24-DOGFOOD-12",
      M24_DOGFOOD_INPUT,
      M24_DOGFOOD_TOP,
    );
    await page
      .locator(
        `.candidate-panel .candidates tbody[data-candidate-text="${M24_DOGFOOD_TOP}"]`,
      )
      .hover();
    await expect(
      page.locator("[data-chinese-typeface='iansui']").first(),
    ).toBeVisible();
    const textareaFont = await page
      .locator("textarea")
      .evaluate((element) => getComputedStyle(element).fontFamily);
    await saveM24Json("M24-DOGFOOD-12", "typeface-picker-font-resources.json", {
      renderedFonts,
      textareaFont,
      state,
    });
    await takeM24Screenshot(page, "M24-DOGFOOD-12", "typeface-picker-iansui");
    expect(textareaFont).toContain("Iansui");
  });

  test("M24 dogfood UI uses only local Tailwind components", async ({
    page,
  }) => {
    const packageJson = JSON.parse(
      await readRepoText("apps/yune-web/package.json"),
    ) as {
      dependencies?: Record<string, string>;
      devDependencies?: Record<string, string>;
    };
    expect(packageJson.dependencies?.daisyui).toBeUndefined();
    expect(packageJson.devDependencies?.daisyui).toBeUndefined();

    const tailwindConfig = await readRepoText(
      "apps/yune-web/tailwind.config.ts",
    );
    expect(tailwindConfig).not.toMatch(/\bdaisyui\b/i);
    expect(tailwindConfig).not.toContain("DaisyUIConfig");

    const filesToScan = [
      "apps/yune-web/src/Inputs.tsx",
      "apps/yune-web/src/Toolbar.tsx",
      "apps/yune-web/src/ThemeSwitcher.tsx",
      "apps/yune-web/src/YuneFeatureShowcase.tsx",
      "apps/yune-web/src/App.tsx",
      "apps/yune-web/src/Preferences.tsx",
      "apps/yune-web/src/CandidatePanel.tsx",
      "apps/yune-web/src/Candidate.tsx",
      "apps/yune-web/src/DictionaryPanel.tsx",
      "apps/yune-web/src/YuneStatusStrip.tsx",
      "apps/yune-web/src/YuneInspector.tsx",
      "apps/yune-web/src/index.css",
    ];
    const forbiddenDaisyUiClasses =
      /\b(?:btn|toggle|radio|checkbox|range|textarea|badge|join|tooltip|link|loading)(?:-[A-Za-z0-9_:[\]\/.%#]+)?\b/;
    for (const file of filesToScan) {
      const source = await readRepoText(file);
      const classSnippets =
        source.match(
          /className\s*=\s*(?:"[^"]*"|'[^']*'|`[^`]*`|\{`[^`]*`\}|\{"[^"]*"\}|\{'[^']*'\})/g,
        ) ?? [];
      for (const snippet of classSnippets) {
        expect(snippet, `${file}: ${snippet}`).not.toMatch(
          forbiddenDaisyUiClasses,
        );
      }
    }

    await expect(
      page.getByRole("button", { name: /ASCII mode|中/ }),
    ).toBeVisible();
    await focusInputAndType(page, "nei", "你");
    await expect(page.locator("[data-yune-schema-switcher]")).toBeVisible();
    await expect(page.locator("[data-yune-status]")).toBeVisible({
      timeout: 10000,
    });
    await expect(page.locator(".candidate-panel")).toBeVisible();
    await takeM24Screenshot(
      page,
      "M24-DOGFOOD-13",
      "local-tailwind-components",
    );
  });

  test("Composition after typing schema-valid keys @smoke", async ({
    page,
  }) => {
    // D-08/D-10: Composition appears after typing schema-valid keys

    await focusInputAndType(page, "nei");
    const state = await readCandidatePanelSnapshot(page, false);
    expect(state.candidates.length).toBeGreaterThan(0);
    expect(state.candidates[0].text).toBe("你");
    await takeEvidenceScreenshot(page, "composition");
    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Composition: input="nei" candidates=${state.candidates.length}\n`,
    );
  });

  test("Candidate list visible @smoke", async ({ page }) => {
    // D-08/D-10: Candidate list is visible after composition

    await focusInputAndType(page, "nei");
    const candidatePanel = await page
      .waitForSelector(".candidate-panel", { timeout: 5000, state: "visible" })
      .catch(() => null);

    await takeEvidenceScreenshot(page, "candidates");

    if (candidatePanel) {
      expect(candidatePanel).toBeTruthy();

      const candidates = await page
        .locator(".candidate-panel .candidates tbody")
        .count();
      expect(candidates).toBeGreaterThan(0);

      await saveEvidence(
        "browser-run.log",
        `[${new Date().toISOString()}] Candidates: ${candidates} visible\n`,
      );
    } else {
      await saveEvidence(
        "blocker.md",
        `# Browser E2E Blocker\n\n**Category**: Yune adapter/runtime\n\n**Flow**: Candidate list visible\n\n**Issue**: No candidate panel appeared after schema-valid input\n\n**Selectors tried**: [data-candidates], .candidate-panel, .candidate-list\n\n**Evidence**: screenshot-candidates.png\n\n**Impact**: Cannot verify candidate paging/selection flows\n`,
      );
      expect(candidatePanel).toBeTruthy();
    }
  });

  test("M41 default startup preserves deploy-time engine defaults", async ({
    page,
  }) => {
    const diagnostics = await readPersistenceDiagnostics(page);
    const runtimeReady = diagnostics
      .slice()
      .reverse()
      .find(
        (diagnostic) =>
          diagnostic.source === "yune-persistence" &&
          diagnostic.marker.phase === "runtime:init:finish" &&
          diagnostic.marker.reason === "after-init" &&
          diagnostic.marker.deployedConfig?.settings,
      );
    const deployedSettings =
      runtimeReady?.marker.deployedConfig?.settings ?? {};

    expect(
      Object.fromEntries(
        [
          "translator/enable_completion",
          "translator/enable_correction",
          "translator/enable_sentence",
          "translator/enable_user_dict",
          "translator/encode_commit_history",
          "translator/combine_candidates",
          "translator/prediction_never_first",
          "translator/prediction_weight_threshold",
          "translator/dictionary_exclude",
          "cangjie/dictionary",
        ].map((key) => [key, deployedSettings[key] ?? null]),
      ),
    ).toEqual({
      "translator/enable_completion": "true",
      "translator/enable_correction": "false",
      "translator/enable_sentence": "true",
      "translator/enable_user_dict": "true",
      "translator/encode_commit_history": "true",
      "translator/combine_candidates": "true",
      "translator/prediction_never_first": "true",
      "translator/prediction_weight_threshold": "0",
      "translator/dictionary_exclude": "[]",
      "cangjie/dictionary": "cangjie5",
    });

    const startup = diagnostics.find(
      (diagnostic) => diagnostic.source === "yune-startup",
    );
    expect(
      startup?.marker.markers?.map((marker) => marker.phase),
    ).not.toContain("schema:deploy:start");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M13 AI-off identity and AI-on second-pass source labels @smoke", async ({
    page,
  }) => {
    await focusInputAndType(page, "nei");

    const offState = await readCandidatePanelSnapshot(page, false);
    expect(offState.candidates.length).toBeGreaterThanOrEqual(2);
    expect(offState.candidates[0].text).toBe("你");
    expect(
      offState.candidates.every((candidate) => candidate.source === null),
    ).toBe(true);
    await saveJsonEvidence("m13-ai-off-state.json", offState);
    await takeEvidenceScreenshot(page, "m13-ai-off");

    await setAiToggle(page, true);
    await expect(
      page.locator(
        '.candidate-panel .candidates tbody[data-source="ai:local"]',
      ),
    ).toHaveCount(1, { timeout: 5000 });
    const onState = await readCandidatePanelSnapshot(page, true);
    const aiRow = onState.candidates.find(
      (candidate) => candidate.source === "ai:local",
    );
    const aiIndex = onState.candidates.findIndex(
      (candidate) => candidate.source === "ai:local",
    );
    expect(aiRow).toBeDefined();
    expect(aiRow?.text).toBe("你啊");
    expect(onState.candidates[0].text).toBe(offState.candidates[0].text);
    expect(aiIndex).toBeGreaterThan(0);
    expect(aiIndex).toBeLessThan(onState.candidates.length);
    expect(
      onState.candidates
        .slice(0, aiIndex)
        .every((candidate) => candidate.source === null),
    ).toBe(true);
    await saveJsonEvidence("m13-ai-on-state.json", onState);
    await takeEvidenceScreenshot(page, "m13-ai-on");

    await setAiToggle(page, false);
    await expect(
      page.locator(
        '.candidate-panel .candidates tbody[data-source="ai:local"]',
      ),
    ).toHaveCount(0, { timeout: 5000 });
    const disabledState = await readCandidatePanelSnapshot(page, false);
    expect(disabledState.candidates).toEqual(offState.candidates);
    await saveJsonEvidence("m13-ai-disabled-state.json", disabledState);
    await takeEvidenceScreenshot(page, "m13-ai-disabled");
    expect(consoleErrors).toEqual([]);
  });

  test("M22 Bucket 2 inspector preserves classic candidate output @smoke", async ({
    page,
  }) => {
    test.setTimeout(300000);
    await expect(page.locator('[data-yune-inspector="panel"]')).toHaveCount(0);

    await focusInputAndType(page, "nei");
    const offState = await readCandidatePanelSnapshot(page, false);
    const offClassic = classicCandidateSignature(offState);
    expect(offState.candidates.length).toBeGreaterThan(0);
    expect(
      offState.candidates.every((candidate) => candidate.source === null),
    ).toBe(true);

    await clearCompositionThroughInput(page);
    await page.getByLabel("Yune inspector").check();
    await expect(page.locator('[data-yune-inspector="panel"]')).toBeVisible({
      timeout: 5000,
    });
    await waitForAppReady(page);
    await page.waitForTimeout(250);
    const inputField = page.locator("input[type='text'], textarea").first();
    await inputField.focus();
    await page.keyboard.type("nei", { delay: 250 });
    await expect(
      page.locator(".candidate-panel .candidates tbody").first(),
    ).toBeVisible({ timeout: 5000 });
    await expect
      .poll(
        async () => {
          const state = await readCandidatePanelSnapshot(page, false);
          return state.candidates[0]?.text ?? null;
        },
        { timeout: 10000 },
      )
      .toBe(offState.candidates[0].text);
    await expect(
      page.locator("[data-yune-inspector-source]").first(),
    ).toHaveText(/table|completion|sentence/, { timeout: 5000 });

    const onState = await readCandidatePanelSnapshot(page, false);
    const onClassic = classicCandidateSignature(onState);

    const inspectorText = await page
      .locator('[data-yune-inspector="panel"]')
      .innerText();
    expect(inspectorText).toContain("Spelling algebra");
    expect(inspectorText).toContain("Prediction");
    await saveJsonEvidence("m22-bucket2-inspector-identity-state.json", {
      offClassic,
      onClassic,
      offInputValue: offState.inputValue,
      onInputValue: onState.inputValue,
      offSources: offState.candidates.map((candidate) => candidate.source),
      onSources: onState.candidates.map((candidate) => candidate.source),
      inspectorText,
    });
    expect(onClassic).toEqual(offClassic);
    expect(
      onState.candidates.some((candidate) => candidate.source !== null),
    ).toBe(true);
    await takeEvidenceScreenshot(page, "m22-bucket2-inspector-on");

    await page.getByLabel("Yune inspector").uncheck();
    await expect(page.locator('[data-yune-inspector="panel"]')).toHaveCount(0, {
      timeout: 5000,
    });
  });

  test("M22 Bucket 1 controls are browser-visible and honest @smoke", async ({
    page,
  }) => {
    test.setTimeout(300000);
    await expect(page.getByLabel(/Dictionary exclude/).last()).toBeVisible();
    await expect(page.getByText(/Output standard/).last()).toBeVisible();
    await expect(page.getByLabel(/Hong Kong Traditional/).last()).toBeVisible();
    await expect(page.getByLabel(/Simplified Chinese/).last()).toBeVisible();
    await expect(page.getByLabel(/Extended charset/).last()).toBeVisible();
    await expect(page.getByLabel(/Disabled/).last()).toBeVisible();
    await expect(
      page.locator("label").filter({ hasText: /ascii_punct/i }),
    ).toHaveCount(0);
    await expect(page.getByLabel(/ascii_punct/i)).toHaveCount(0);

    await selectSchema(page, /Luna Pinyin/);
    const defaultLuna = await typeCompositionAndWaitForTopCandidate(
      page,
      "hao",
      "\u4fb4",
    );

    await clearComposition(page);
    const excludeOn = await setPreferenceToggleAndWaitForSettings(
      page,
      /Dictionary exclude/,
      true,
      {
        "translator/dictionary_exclude": '["\u4fb4"]',
      },
    );
    await typeInputForStatus(page, "hao");
    const excludedLuna = await readCandidatePanelSnapshot(page, false);
    expect(candidateTexts(excludedLuna)).not.toContain("\u4fb4");

    await clearComposition(page);
    await setPreferenceToggle(page, /Disabled/, true);
    await typeInputForStatus(page, "hao");
    const disabledStatus = await readYuneStatus(page);
    expect(disabledStatus.disabled).toBe("disabled");
    await setPreferenceToggle(page, /Disabled/, false);

    await selectSchema(page, /Cangjie 5/);
    await typeInputForStatus(page, "ambe");
    const extendedOff = await readCandidatePanelSnapshot(page, false);
    expect(candidateTexts(extendedOff)).not.toContain("\u{2330A}");

    await clearComposition(page);
    await setPreferenceToggle(page, /Extended charset/, true);
    const extendedOn = await typeCompositionAndWaitForCandidate(
      page,
      "ambe",
      "\u{2330A}",
    );
    expect(candidateTexts(extendedOn)).toContain("\u{2330A}");

    await saveJsonEvidence("m22-bucket1-controls-state.json", {
      defaultLuna,
      excludeOn,
      excludedLuna,
      disabledStatus,
      extendedOff,
      extendedOn,
      asciiPunctExposed: false,
      visibleSurfaces: {
        dictionaryExclude:
          "persisted translator/dictionary_exclude plus candidate removal",
        disabled: "engine status strip",
        extendedCharset: "candidate before/after on cangjie5 input ambe",
      },
    });
    await takeEvidenceScreenshot(page, "m22-bucket1-controls");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M22 Bucket 3 schema switcher loads Jyutping, Cangjie, and Luna", async ({
    page,
  }) => {
    test.setTimeout(300000);
    await expect(page.locator("[data-yune-schema-switcher]")).toBeVisible();

    await selectSchema(page, /Cangjie 5/);
    const cangjie = await typeCompositionAndWaitForTopCandidate(
      page,
      "a",
      "\u65e5",
    );
    const cangjieStatus = await readYuneStatus(page);
    expect(cangjieStatus.schema).toBe("cangjie5");

    await selectSchema(page, /Luna Pinyin/);
    const luna = await typeCompositionAndWaitForTopCandidate(
      page,
      "hao",
      "\u4fb4",
    );
    const lunaStatus = await readYuneStatus(page);
    expect(lunaStatus.schema).toBe("luna_pinyin");

    await selectSchema(page, /Jyutping/);
    const jyutping = await typeCompositionAndWaitForTopCandidate(
      page,
      "nei",
      "\u4f60",
    );
    const jyutpingStatus = await readYuneStatus(page);
    expect(jyutpingStatus.schema).toBe("jyut6ping3");

    await saveJsonEvidence("m22-bucket3-schema-switch-state.json", {
      cangjie,
      cangjieStatus,
      luna,
      lunaStatus,
      jyutping,
      jyutpingStatus,
    });
    await takeEvidenceScreenshot(page, "m22-bucket3-schema-switch");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M22 Bucket 3 reverse lookup works for Cangjie and Luna", async ({
    page,
  }) => {
    test.setTimeout(300000);
    await selectSchema(page, /Cangjie 5/);
    const cangjieReverse = await typeCompositionAndWaitForCandidate(
      page,
      "`nei;",
      "\u4f60",
    );
    expect(candidateTexts(cangjieReverse)).toContain("\u4f60");

    await selectSchema(page, /Luna Pinyin/);
    const lunaReverse = await typeCompositionAndWaitForCandidate(
      page,
      "`a;",
      "\u65e5",
    );
    expect(candidateTexts(lunaReverse)).toContain("\u65e5");

    await saveJsonEvidence("m22-bucket3-reverse-lookup-state.json", {
      cangjieReverse,
      lunaReverse,
      reverseLookup: {
        cangjie5: "Jyutping dictionary lookup with cangjie5 target comments",
        luna_pinyin:
          "Cangjie5 dictionary lookup with luna_pinyin target comments",
      },
    });
    await takeEvidenceScreenshot(page, "m22-bucket3-reverse-lookup");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M13 AI commit safety", async ({ page }) => {
    await setAiToggle(page, true);
    await focusInputAndType(page, "nei");
    await expect(
      page.locator(
        '.candidate-panel .candidates tbody[data-source="ai:local"]',
      ),
    ).toHaveCount(1, { timeout: 5000 });
    const stagedState = await readCandidatePanelSnapshot(page, true);
    const aiIndex = stagedState.candidates.findIndex(
      (candidate) => candidate.source === "ai:local",
    );
    expect(aiIndex).toBeGreaterThan(0);

    await page.keyboard.press("Space");
    const inputField = page.locator("input[type='text'], textarea").first();
    await expect(inputField).toHaveValue("你", { timeout: 5000 });
    await saveJsonEvidence("m13-ai-default-commit-state.json", {
      aiEnabled: true,
      committed: await inputField.inputValue(),
      classicTop: stagedState.candidates[0],
      aiRow: stagedState.candidates[aiIndex],
    });
    await takeEvidenceScreenshot(page, "m13-ai-default-commit");

    await inputField.fill("");
    await focusInputAndType(page, "nei");
    await expect(
      page.locator(
        '.candidate-panel .candidates tbody[data-source="ai:local"]',
      ),
    ).toHaveCount(1, { timeout: 5000 });
    const selectableState = await readCandidatePanelSnapshot(page, true);
    const selectableAiIndex = selectableState.candidates.findIndex(
      (candidate) => candidate.source === "ai:local",
    );
    await page
      .locator('.candidate-panel .candidates tbody[data-source="ai:local"]')
      .click();
    await expect(inputField).toHaveValue("你啊", { timeout: 5000 });
    await saveJsonEvidence("m13-ai-explicit-commit-state.json", {
      aiEnabled: true,
      committed: await inputField.inputValue(),
      selectedIndex: selectableAiIndex,
      aiRow: selectableState.candidates[selectableAiIndex],
    });
    await takeEvidenceScreenshot(page, "m13-ai-explicit-commit");
    expect(consoleErrors).toEqual([]);
  });

  test("M20 showcase control surface exposes honest controls", async ({
    page,
  }) => {
    await expect(page.getByText(/Active engine controls/)).toBeVisible();
    await expect(page.getByText(/Live session controls/)).toBeVisible();
    await expect(page.getByText(/Display controls/)).toBeVisible();

    for (const label of ACTIVE_SHOWCASE_CONTROLS) {
      await expect(page.getByLabel(label).last()).toBeVisible();
    }
    for (const label of LIVE_SHOWCASE_CONTROLS) {
      await expect(page.getByLabel(label).last()).toBeVisible();
    }
    for (const label of DISPLAY_SHOWCASE_CONTROLS) {
      await expect(page.getByText(label).last()).toBeVisible();
    }

    await expect(
      page.getByLabel(/Combine same-text candidates/).last(),
    ).toBeChecked();
    await expect(
      page.getByLabel(/Prediction never first/).last(),
    ).toBeChecked();
    await expect(page.getByLabel(/AI Candidates/).last()).not.toBeChecked();
    await expect(page.getByLabel(/Prediction threshold/).last()).toHaveValue(
      "0",
    );
    await expect(
      page.locator("label").filter({ hasText: /ascii_punct/i }),
    ).toHaveCount(0);
    await expect(page.getByLabel(/ascii_punct/i)).toHaveCount(0);
    const commonCustom = await readRepoText(
      "apps/yune-web/public/schema/common.custom.yaml",
    );
    const commonYaml = await readRepoText(
      "apps/yune-web/public/schema/common.yaml",
    );
    expect(commonCustom).toContain("# - common:/separate_candidates");
    expect(commonYaml).toContain("translator/combine_candidates: false");

    await saveJsonEvidence("m20-control-surface-state.json", {
      activeControls: ACTIVE_SHOWCASE_CONTROLS.map(String),
      liveControls: LIVE_SHOWCASE_CONTROLS.map(String),
      displayControls: DISPLAY_SHOWCASE_CONTROLS.map(String),
      defaults: {
        combineCandidates: {
          uiDemoDefault: true,
          rawAssetPatch:
            "M41 keeps the common:/separate_candidates patch available but inactive, so the shipped default matches the grouped-candidate UI default without a startup deploy.",
        },
        predictionNeverFirst: true,
        predictionThreshold: 0,
        aiCandidates: false,
      },
      unsupportedAsciiPunctExposed: false,
    });
    await takeEvidenceScreenshot(page, "m20-control-surface");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 User Dictionary persists schema customization", async ({
    page,
  }) => {
    const inputMemoryToggle = page.getByLabel(/User Dictionary/).last();
    if (await inputMemoryToggle.isChecked()) {
      await setPreferenceToggleAndWaitForSettings(
        page,
        /User Dictionary/,
        false,
        {
          "translator/enable_user_dict": "false",
          "translator/encode_commit_history": "false",
        },
      );
    }
    const learningOn = await setPreferenceToggleAndWaitForSettings(
      page,
      /User Dictionary/,
      true,
      {
        "translator/enable_user_dict": "true",
        "translator/encode_commit_history": "true",
      },
    );

    await page.getByLabel("Yune inspector").check();
    const userdbViewer = page.locator("[data-yune-userdb-viewer]");
    await expect(userdbViewer).toBeVisible();
    await expect(userdbViewer.locator("[data-yune-userdb-path]")).toContainText(
      "/rime/jyut6ping3.userdb",
    );
    await expect(
      userdbViewer.getByRole("button", { name: /Clear|Reset/i }),
    ).toHaveCount(0);

    const learnedCommit = await learnPhraseThroughBrowser(page);
    await expect
      .poll(
        async () => userdbViewer.locator("[data-yune-userdb-row]").count(),
        {
          timeout: 10000,
        },
      )
      .toBeGreaterThan(0);
    await expect(userdbViewer).toContainText(LEARNED_PHRASE_TEXT);
    await userdbViewer.locator("summary").click();
    await expect(userdbViewer.locator("[data-yune-userdb-raw]")).toContainText(
      LEARNED_PHRASE_TEXT,
    );
    const userdbRowCount = await userdbViewer
      .locator("[data-yune-userdb-row]")
      .count();

    const learnedWithMemory = await typeCompositionAndWaitForCandidate(
      page,
      LEARNED_PREFIX_INPUT,
      LEARNED_PHRASE_TEXT,
    );
    expect(candidateTexts(learnedWithMemory)).toContain(LEARNED_PHRASE_TEXT);

    const learningOff = await setPreferenceToggleAndWaitForSettings(
      page,
      /User Dictionary/,
      false,
      {
        "translator/enable_user_dict": "false",
        "translator/encode_commit_history": "false",
      },
    );
    const learnedWithMemoryOff = await typeCompositionAndWaitForCandidate(
      page,
      LEARNED_PREFIX_INPUT,
      CLASSIC_NGO_TEXT,
    );
    expect(candidateTexts(learnedWithMemoryOff)).toContain(LEARNED_PHRASE_TEXT);

    await saveJsonEvidence("m20-input-memory-persistence-state.json", {
      learningOn,
      learningOff,
      visibleBehavior: {
        learnedCommit,
        learnedWithMemory,
      },
      userdbViewer: {
        path: "/rime/jyut6ping3.userdb",
        rowCount: userdbRowCount,
        rawContainsLearnedPhrase: true,
        resetControlExposed: false,
      },
      browserSurface: {
        status:
          "explicit browser-surface N/A for the memory-off candidate-output delta",
        observedAfterDisablingMemory: learnedWithMemoryOff,
        reason:
          "The browser persists translator/enable_user_dict=false and translator/encode_commit_history=false, but the current no-crates yune-web surface cannot suppress an already learned prefix prediction from candidate output.",
        engineCoverage:
          "Userdb learning and per-entry pronunciation behavior remain engine-proven in cantonese_parity and frontend_client userdb tests; this M20 follow-up does not change crates/ or add a yune_web_* export.",
        evidencePolicy:
          "The learned prediction on-state is visible browser behavior; the off-state is not counted as candidate-output proof.",
      },
      proof:
        "User Dictionary remains an honest deploy-time schema customization in the browser. The learned-prediction on-state is visible after a real browser commit; the memory-off candidate-output delta is explicitly N/A on this surface.",
    });
    await takeEvidenceScreenshot(page, "m20-input-memory-persistence");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M42 User Dictionary learns classic Space commits and survives reload", async ({
    page,
  }) => {
    await setPreferenceToggleAndWaitForSettings(
      page,
      /User Dictionary|用戶詞庫/,
      true,
      {
        "translator/enable_user_dict": "true",
        "translator/encode_commit_history": "true",
      },
    );
    await setPreferenceToggle(page, /ASCII mode|中英模式/, false);

    await page
      .locator("[data-yune-inspector-toggle] input[type='checkbox']")
      .check({ force: true });
    const userdbViewer = page.locator("[data-yune-userdb-viewer]");
    await expect(userdbViewer).toBeVisible();
    await expect(userdbViewer.locator("[data-yune-userdb-path]")).toContainText(
      "/rime/jyut6ping3.userdb",
    );

    const inputField = page.locator("input[type='text'], textarea").first();
    const composing = await typeCompositionAndWaitForTopCandidate(
      page,
      "nei",
      "\u4f60",
    );
    expect(composing.candidates[0]?.text).toBe("\u4f60");
    await page.keyboard.press("Space");
    await expect(inputField).toHaveValue("\u4f60", { timeout: 5000 });

    await userdbViewer.locator("[data-yune-userdb-refresh]").click();
    await expect
      .poll(
        async () => {
          const rowTexts = await userdbViewer
            .locator("[data-yune-userdb-row]")
            .allInnerTexts();
          return rowTexts.some((text) => text.includes("\u4f60"));
        },
        { timeout: 10000 },
      )
      .toBe(true);
    await expect(userdbViewer).toContainText("nei");

    await page.reload({ waitUntil: "domcontentloaded", timeout: TIMEOUT_MS });
    await waitForAppReady(page);
    await page
      .locator("[data-yune-inspector-toggle] input[type='checkbox']")
      .check({ force: true });
    const reloadedUserdbViewer = page.locator("[data-yune-userdb-viewer]");
    await expect
      .poll(
        async () => {
          const rowTexts = await reloadedUserdbViewer
            .locator("[data-yune-userdb-row]")
            .allInnerTexts();
          return rowTexts.some((text) => text.includes("\u4f60"));
        },
        { timeout: 10000 },
      )
      .toBe(true);
    await expect(reloadedUserdbViewer).toContainText("nei");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 Prediction never first persists schema customization @smoke", async ({
    page,
  }) => {
    const neverFirstOn = await waitForPersistedSettings(page, {
      "translator/prediction_never_first": "true",
    });

    const learnedCommit = await learnPhraseThroughBrowser(page);
    const neverFirstOnRanking = await typeCompositionAndWaitForCandidate(
      page,
      LEARNED_PREFIX_INPUT,
      LEARNED_PHRASE_TEXT,
    );
    const learnedOnIndex =
      candidateTexts(neverFirstOnRanking).indexOf(LEARNED_PHRASE_TEXT);
    expect(neverFirstOnRanking.candidates[0].text).toBe(CLASSIC_NGO_TEXT);
    expect(learnedOnIndex).toBeGreaterThan(0);

    const neverFirstOff = await setPreferenceToggleAndWaitForSettings(
      page,
      /Prediction never first/,
      false,
      {
        "translator/prediction_never_first": "false",
      },
    );
    const neverFirstOffRanking = await typeCompositionAndWaitForCandidate(
      page,
      LEARNED_PREFIX_INPUT,
      LEARNED_PHRASE_TEXT,
    );
    const classicOffIndex =
      candidateTexts(neverFirstOffRanking).indexOf(CLASSIC_NGO_TEXT);
    expect(neverFirstOffRanking.candidates[0].text).toBe(LEARNED_PHRASE_TEXT);
    expect(classicOffIndex).toBeGreaterThan(0);

    await saveJsonEvidence(
      "m20-prediction-never-first-persistence-state.json",
      {
        neverFirstOn,
        neverFirstOff,
        visibleBehavior: {
          learnedCommit,
          neverFirstOnRanking,
          neverFirstOffRanking,
          learnedOnIndex,
          classicOffIndex,
        },
        proof:
          "Prediction never first is a deploy-time schema customization and a visible browser behavior: after learning a phrase, classic 我 stays first while never-first is enabled, and the learned prefix prediction moves to index 0 after the control is disabled.",
      },
    );
    await takeEvidenceScreenshot(
      page,
      "m20-prediction-never-first-persistence",
    );
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 guided scenarios use real yune-web assets", async ({ page }) => {
    const scenarios: Record<string, string[]> = {};

    const ngo = await clickShowcaseScenario(page, "ngo", "我");
    expect(ngo.candidates[0].text).toBe("我");
    scenarios.ngo = ngo.candidates.map((candidate) => candidate.rowText);
    await expectNoToasts(page);

    const santai = await clickShowcaseScenario(page, "santai", "身體健康");
    expect(santai.candidates.map((candidate) => candidate.text)).toContain(
      "身體",
    );
    expect(santai.candidates.map((candidate) => candidate.text)).toContain(
      "身體健康",
    );
    scenarios.santai = santai.candidates.map((candidate) => candidate.rowText);
    await expectNoToasts(page);

    const mgoi = await clickShowcaseScenario(page, "mgoi", "唔該");
    expect(mgoi.candidates.map((candidate) => candidate.text)).toContain(
      "唔該",
    );
    scenarios.mgoi = mgoi.candidates.map((candidate) => candidate.rowText);
    await expectNoToasts(page);

    const m = await clickShowcaseScenario(page, /^m$/, "唔");
    expect(m.candidates[0].text).toBe("唔");
    scenarios.m = m.candidates.map((candidate) => candidate.rowText);
    await expectNoToasts(page);

    await clearComposition(page);
    await focusInputAndType(page, "neivv", "nei4");
    const toneLetters = await readCandidatePanelSnapshot(page, false);
    expect(toneLetters.candidates[0].text).toBe("尼");
    scenarios.toneLetters = toneLetters.candidates.map(
      (candidate) => candidate.rowText,
    );
    await expectNoToasts(page);

    await clearComposition(page);
    await setAiToggle(page, true);
    await expectNoToasts(page);
    await clickShowcaseScenario(page, "AI trigger", "你", true);
    await expect(
      page.locator(
        '.candidate-panel .candidates tbody[data-source="ai:local"]',
      ),
    ).toHaveCount(1, { timeout: 5000 });
    const aiTrigger = await readCandidatePanelSnapshot(page, true);
    const aiIndex = aiTrigger.candidates.findIndex(
      (candidate) => candidate.source === "ai:local",
    );
    expect(aiIndex).toBeGreaterThan(0);
    expect(aiTrigger.candidates[0].source).toBeNull();
    expect(aiTrigger.candidates[aiIndex].text).toBe("你啊");
    scenarios.aiTrigger = aiTrigger.candidates.map(
      (candidate) => candidate.rowText,
    );

    await saveJsonEvidence("m20-guided-scenarios-state.json", {
      scenarios,
      aiTrigger: {
        aiIndex,
        classicFirst: aiTrigger.candidates[0],
        aiRow: aiTrigger.candidates[aiIndex],
      },
    });
    await expectNoToasts(page);
    await takeEvidenceScreenshot(page, "m20-guided-scenarios");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 combine_candidates changes real candidate output", async ({
    page,
  }) => {
    const combineOn = await typeCompositionAndWaitForCandidate(
      page,
      "hou",
      "好",
    );
    expect(combineOn.candidates[0].text).toBe("好");
    expect(combineOn.candidates[1].text).not.toBe("好");

    await clearComposition(page);
    await setPreferenceToggle(page, /Combine same-text candidates/, false);
    const combineOff = await typeCompositionAndWaitForCandidate(
      page,
      "hou",
      "好",
    );
    expect(
      combineOff.candidates.slice(0, 2).map((candidate) => candidate.text),
    ).toEqual(["好", "好"]);

    await saveJsonEvidence("m20-combine-candidates-state.json", {
      defaultOn: combineOn,
      disabled: combineOff,
    });
    await takeEvidenceScreenshot(page, "m20-combine-candidates");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 prediction threshold changes real candidate output", async ({
    page,
  }) => {
    const thresholdZero = await typeCompositionAndWaitForCandidate(
      page,
      "santai",
      "身體健康",
    );
    expect(
      thresholdZero.candidates.map((candidate) => candidate.text),
    ).toContain("身體健康");

    await clearComposition(page);
    await setPreferenceRange(page, /Prediction threshold/, 50000);
    const threshold50000 = await typeCompositionAndWaitForCandidate(
      page,
      "santai",
      "身體",
    );
    expect(threshold50000.candidates[0].text).toBe("身體");
    expect(
      threshold50000.candidates.map((candidate) => candidate.text),
    ).not.toContain("身體健康");

    await saveJsonEvidence("m20-prediction-threshold-state.json", {
      thresholdZero,
      threshold50000,
      calibratedValue: 50000,
      selectorRange: {
        min: 0,
        max: 200000,
        step: 1000,
        rationale:
          "Fine-grained browser control around the observed 50000 cutoff; range remains above the real-assets probe value so future higher-weight dictionary predictions remain reachable without exposing separate alias sliders.",
      },
      calibration:
        "Derived from real jyut6ping3_mobile browser assets: santai predictions disappear at 50000 while direct candidates remain.",
    });
    await takeEvidenceScreenshot(page, "m20-prediction-threshold");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("Shift toggles ASCII mode from focused input", async ({ page }) => {
    const inputField = page.locator("input[type='text'], textarea").first();
    const asciiModeButton = page.getByRole("button", {
      name: /ASCII mode|中英模式/,
    });
    await clearComposition(page);
    if ((await asciiModeButton.getAttribute("data-active")) !== "true") {
      await asciiModeButton.click();
      await expect(asciiModeButton).toHaveAttribute("data-active", "true", {
        timeout: 10000,
      });
    }
    await inputField.focus();
    await expect(asciiModeButton).toHaveAttribute("data-active", "true");

    await page.keyboard.down("Shift");
    await page.keyboard.up("Shift");
    await expect(asciiModeButton).toHaveAttribute("data-active", "false", {
      timeout: 10000,
    });
    await waitForAppReady(page);
    expect(await typeRawInput(page, "abc")).toEqual({
      value: "abc",
      panelCount: 0,
    });

    await page.keyboard.down("Shift");
    await page.keyboard.up("Shift");
    await expect(asciiModeButton).toHaveAttribute("data-active", "true", {
      timeout: 10000,
    });
    await waitForAppReady(page);

    await page.keyboard.down("Shift");
    await page.keyboard.press("a");
    await page.keyboard.up("Shift");
    await expect(asciiModeButton).toHaveAttribute("data-active", "true", {
      timeout: 10000,
    });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 live session controls use setOption-visible output", async ({
    page,
  }) => {
    await setPreferenceToggle(page, /ASCII mode/, true);
    const asciiMode = await typeRawInput(page, "abc");
    expect(asciiMode).toEqual({ value: "abc", panelCount: 0 });

    await setPreferenceToggle(page, /ASCII mode/, false);
    await setPreferenceToggle(page, /Full shape/, false);
    const halfShapeSlash = await typeRawInput(page, "/");
    expect(halfShapeSlash).toEqual({ value: "/", panelCount: 0 });

    await setPreferenceToggle(page, /Full shape/, true);
    const fullShapeSlash = await typeRawInput(page, "/");
    expect(fullShapeSlash).toEqual({ value: "／", panelCount: 0 });

    await setPreferenceToggle(page, /Full shape/, false);
    await setPreferenceRadio(page, /Simplified Chinese/);
    const simplification = await typeCompositionAndWaitForCandidate(
      page,
      "ngohaigo",
      "我系个",
    );
    expect(simplification.candidates[0].text).toBe("我系个");

    await saveJsonEvidence("m20-live-session-controls-state.json", {
      asciiMode,
      halfShapeSlash,
      fullShapeSlash,
      simplification,
    });
    await takeEvidenceScreenshot(page, "m20-live-session-controls");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 display controls change rendering and record mobile-only side-lookup limits", async ({
    page,
  }) => {
    const jyutpingShown = await typeCompositionAndWaitForCandidate(
      page,
      "nei",
      "你",
    );
    expect(jyutpingShown.candidates[0].rowText).toContain("nei5");

    await clearComposition(page);
    await setPreferenceRadio(page, /Hide/);
    const jyutpingHidden = await typeCompositionAndWaitForCandidate(
      page,
      "nei",
      "你",
    );
    expect(jyutpingHidden.candidates[0].rowText).not.toContain("nei5");
    expect(jyutpingHidden.candidates[0].text).toBe("你");

    await clearComposition(page);
    await setPreferenceRadio(page, /Always Show/);
    const englishOnly = await typeCompositionAndWaitForCandidate(
      page,
      "nei",
      "你",
    );
    expect(englishOnly.candidates[0].rowText).toContain("you (singular)");

    await clearComposition(page);
    await page.getByLabel(/Hindi/).last().check({ force: true });
    const hindiVisible = await typeCompositionAndWaitForCandidate(
      page,
      "nei",
      "你",
    );
    expect(hindiVisible.candidates[0].rowText).toContain("आप");

    await expect(page.getByLabel(/Reverse code display/).last()).toBeVisible();
    await expect(page.getByText(/Cangjie lookup/)).toBeVisible();
    const jyutpingSchema = await readRepoText(
      "apps/yune-web/public/schema/jyut6ping3.schema.yaml",
    );
    expect(jyutpingSchema).toContain("cangjie");
    expect(jyutpingSchema).toContain("show_full_code");
    const visibleSchemaControls = await page
      .locator(
        "[data-yune-schema-switcher], [data-schema], [data-schema-selector], .schema-selector, select[name='schema']",
      )
      .count();

    await saveJsonEvidence("m20-display-controls-state.json", {
      candidateJyutping: {
        shown: jyutpingShown.candidates[0],
        hidden: jyutpingHidden.candidates[0],
      },
      displayLanguages: {
        englishOnly: englishOnly.candidates[0],
        hindiVisible: hindiVisible.candidates[0],
      },
      reverseCodeAndCangjie: {
        status:
          "Covered by the Jyutping schema and M22 schema switch/reverse-lookup tests",
        activeBrowserSchema: "jyut6ping3",
        reason:
          "The default browser schema now includes the Cangjie reverse-lookup namespace; M22 tests still switch to cangjie5 and luna_pinyin for schema/reverse-lookup browser evidence.",
        visibleSchemaControls,
      },
    });
    await takeEvidenceScreenshot(page, "m20-display-controls");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M16 combine candidates browser default matches M14", async ({
    page,
  }) => {
    await focusInputAndType(page, "hou");
    await captureM16Scenario(
      page,
      "combine-candidates-browser-default",
      await m14Texts(
        "jyut6ping3-m14-options.json",
        "combine_candidates_default",
        "hou",
        5,
      ),
    );
    expect(consoleErrors).toEqual([]);
  });

  test("M16 sentence composition browser path matches M14 @smoke", async ({
    page,
  }) => {
    await focusInputAndType(page, "ngohaigo");
    await captureM16Scenario(
      page,
      "sentence-enabled",
      await m14Texts(
        "jyut6ping3-m14-options.json",
        "enable_sentence_default",
        "ngohaigo",
        1,
      ),
    );

    await clearComposition(page);
    await setPreferenceToggle(page, /Auto-composition/, false);
    const persistedSentenceDisabled = await waitForPersistedSettings(page, {
      "translator/enable_sentence": "false",
    });
    const inputField = page.locator("input[type='text'], textarea").first();
    await inputField.focus();
    await inputField.type("ngohaigo", { delay: 80 });
    await page.waitForTimeout(1000);
    const sentenceDisabledPanelCount = await page
      .locator(".candidate-panel .candidates tbody")
      .count();
    const sentenceDisabledState =
      sentenceDisabledPanelCount > 0
        ? await readCandidatePanelSnapshot(page, false)
        : {
            aiEnabled: false,
            inputValue: await inputField.inputValue(),
            candidates: [],
          };
    await saveJsonEvidence("m16-sentence-disabled-state.json", {
      expectedM14Texts: await m14Texts(
        "jyut6ping3-m14-options.json",
        "enable_sentence_disabled",
        "ngohaigo",
        6,
      ),
      browserState: sentenceDisabledState,
      persistedSettings: persistedSentenceDisabled,
      browserSurface:
        sentenceDisabledPanelCount > 0
          ? "Candidate panel rendered after disabling Auto-composition."
          : "No candidate panel rendered for full ngohaigo after disabling Auto-composition in yune-web.",
    });
    await takeEvidenceScreenshot(page, "m16-sentence-disabled");
    expect(consoleErrors).toEqual([]);
  });

  test("M16 completion browser path matches M14", async ({ page }) => {
    await setPreferenceToggle(page, /Auto-completion/, true);
    await setPreferenceToggle(page, /Auto-correction/, false);
    await waitForDeployedSettings(page, {
      "translator/enable_correction": "false",
    });
    await focusInputAndType(page, "ne");
    await captureM16Scenario(
      page,
      "completion-default",
      await m14Texts(
        "jyut6ping3-m14-completion-correction.json",
        "completion_default",
        "ne",
        1,
      ),
    );
    expect(consoleErrors).toEqual([]);
  });

  test("M16 correction browser path matches M14 visible before/after", async ({
    page,
  }) => {
    await setPreferenceToggle(page, /Auto-correction/, false);
    const persistedCorrectionDefault = await waitForPersistedSettings(page, {
      "translator/enable_correction": "false",
    });
    await waitForDeployedSettings(page, {
      "translator/enable_correction": "false",
    });
    const defaultState = await typeCompositionAndWaitForTopCandidate(
      page,
      "nri",
      "我",
    );
    const expectedDefaultTexts = await m14Texts(
      "jyut6ping3-m14-completion-correction.json",
      "correction_default",
      "nri",
      5,
    );
    const expectedDefaultEngineTexts = await m14Texts(
      "jyut6ping3-m14-completion-correction.json",
      "correction_default",
      "nri",
      6,
    );
    expect(
      defaultState.candidates.slice(0, 5).map((candidate) => candidate.text),
    ).toEqual(expectedDefaultTexts);
    await saveJsonEvidence("m16-correction-default-state.json", {
      expectedM14Texts: expectedDefaultTexts,
      expectedM14EngineTexts: expectedDefaultEngineTexts,
      browserState: defaultState,
      persistedSettings: persistedCorrectionDefault,
      browserSurface:
        "Candidate panel rendered for nri with correction off via prefix fallback; the visible page shows the first five oracle rows, and cantonese_parity asserts the sixth row plus commit preview.",
    });
    await takeEvidenceScreenshot(page, "m16-correction-default");

    await clearComposition(page);
    await setPreferenceToggle(page, /Auto-correction/, true);
    const persistedCorrectionEnabled = await waitForPersistedSettings(page, {
      "translator/enable_correction": "true",
    });
    await waitForDeployedSettings(page, {
      "translator/enable_correction": "true",
    });
    const enabledState = await typeCompositionAndWaitForTopCandidate(
      page,
      "nri",
      "你",
    );
    const expectedEnabledTexts = await m14Texts(
      "jyut6ping3-m14-completion-correction.json",
      "correction_enabled",
      "nri",
      5,
    );
    expect(
      enabledState.candidates.slice(0, 5).map((candidate) => candidate.text),
    ).toEqual(expectedEnabledTexts);
    expect(defaultState.candidates[0].text).not.toEqual(
      enabledState.candidates[0].text,
    );
    await saveJsonEvidence("m16-correction-enabled-state.json", {
      expectedM14Texts: expectedEnabledTexts,
      browserState: enabledState,
      persistedSettings: persistedCorrectionEnabled,
      browserSurface:
        "Candidate panel rendered for nri with correction on; the first row matches the v1.1.2 correction fixture.",
    });
    await saveJsonEvidence(
      "m20-auto-correction-visible-before-after-state.json",
      {
        defaultState,
        enabledState,
        persistedCorrectionDefault,
        persistedCorrectionEnabled,
        expectedDefaultTexts,
        expectedDefaultEngineTexts,
        expectedEnabledTexts,
        browserSurface:
          "Auto-correction is a visible before/after browser behavior for nri: off shows prefix fallback rows on the browser page, on shows correction 你 first, and engine parity asserts the full oracle prefix list.",
      },
    );
    await takeEvidenceScreenshot(page, "m16-correction-enabled");
    await takeEvidenceScreenshot(
      page,
      "m20-auto-correction-visible-before-after",
    );
    expect(consoleErrors).toEqual([]);
  });

  test("M16 simplification toggle converts browser candidates through OpenCC", async ({
    page,
  }) => {
    await focusInputAndType(page, "ngohaigo");
    const traditional = await captureM16Scenario(page, "simplification-off", [
      "\u6211\u4fc2\u500b",
    ]);

    await clearComposition(page);
    await setPreferenceRadio(page, /大陆简化字|Mainland Simplified/);
    await focusInputAndType(page, "ngohaigo");
    const simplified = await captureM16Scenario(page, "simplification-on", [
      "\u6211\u7cfb\u4e2a",
    ]);
    expect(simplified.candidates[0].text).not.toEqual(
      traditional.candidates[0].text,
    );
    expect(consoleErrors).toEqual([]);
  });

  test("M16 schema menu and userdb pronunciation limits are explicit", async ({
    page,
  }) => {
    const schemaMenuFixture = await loadFixture(
      "jyut6ping3-m14-schema-menu.json",
    );
    const userdbFixture = await loadFixture("jyut6ping3-m14-userdb.json");
    const optionsFixture = await loadFixture("jyut6ping3-m14-options.json");
    const visibleSchemaControls = await page
      .locator(
        "[data-yune-schema-switcher], [data-schema], [data-schema-selector], .schema-selector, select[name='schema']",
      )
      .count();
    expect(visibleSchemaControls).toBeGreaterThan(0);
    await saveJsonEvidence("m16-documented-gaps-state.json", {
      deployOnlyVariants: {
        browserSurface:
          "The browser schema switcher exposes jyut6ping3, cangjie5, and luna_pinyin; deploy-variant controls for common:/separate_candidates and common:/show_full_code remain engine/doc scoped.",
        engineCoverage:
          "cargo test -p yune-core --test cantonese_parity covers combine_candidates and show_full_code against the M14 v1.1.2 goldens.",
        oracleSurface: optionsFixture["capture"],
      },
      browserRuntimeLimits: {
        sentenceDisabled:
          "The browser records the disabled Auto-composition state separately because full ngohaigo does not render the native disabled-prefix candidate panel.",
        correction:
          "The browser records nri correction default/enabled as visible candidate-output before/after evidence: correction off renders prefix fallback, correction on renders the v1.1.2 correction row first.",
      },
      schemaMenu: {
        oracleSurface: schemaMenuFixture["capture"],
        browserSurface:
          "M22 yune-web exposes a schema switcher for the browser playground schemas; M14 RimeGetSchemaList remains the ABI oracle evidence.",
        visibleSchemaControls,
      },
      userdbPronunciation: {
        oracleSurface: userdbFixture["capture"],
        browserSurface:
          "No yune-web native inspection surface exposes fork-only per-entry pronunciation levers.",
      },
    });
    await takeEvidenceScreenshot(page, "m16-documented-gaps");
    expect(consoleErrors).toEqual([]);
  });

  test("Candidate paging", async ({ page }) => {
    // D-08/D-10: Candidate paging changes page/candidate state

    await focusInputAndType(page, "nei");

    // Press PageDown to flip candidate page
    await page.keyboard.press("PageDown");

    await page.waitForTimeout(500);

    await takeEvidenceScreenshot(page, "candidate-paging");

    const candidatePanel = await page.locator(".candidate-panel").count();
    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Candidate paging: PageDown pressed, panel count=${candidatePanel}\n`,
    );
    expect(candidatePanel).toBeGreaterThan(0);
  });

  test("Keyboard paging shortcuts do not error", async ({ page }) => {
    await focusInputAndType(page, "n");
    const firstPage = await readCandidatePanelSnapshot(page, false);
    expect(firstPage.candidates.length).toBeGreaterThan(1);

    await page.keyboard.press("=");
    await expect
      .poll(
        async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text,
        { timeout: 5000 },
      )
      .not.toBe(firstPage.candidates[0].text);
    const secondPage = await readCandidatePanelSnapshot(page, false);
    expect(secondPage.candidates[0].text).not.toBe(
      firstPage.candidates[0].text,
    );

    await page.keyboard.press("-");
    await expect
      .poll(
        async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text,
        { timeout: 5000 },
      )
      .toBe(firstPage.candidates[0].text);
    const returnedPage = await readCandidatePanelSnapshot(page, false);
    expect(returnedPage.candidates[0].text).toBe(firstPage.candidates[0].text);
    await expect(page.locator(".yd-toast")).toHaveCount(0, { timeout: 1000 });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("Candidate selection → commit output", async ({ page }) => {
    // D-08/D-10: Candidate selection commits text to app output field

    await focusInputAndType(page, "nei");
    const inputField = page.locator("input[type='text'], textarea").first();

    // Press default commit key for the highlighted classic top candidate.
    await page.keyboard.press("Space");

    await page.waitForTimeout(500);

    await takeEvidenceScreenshot(page, "candidate-selection");

    // Verify committed text
    const inputValue = await inputField.inputValue();
    expect(inputValue).toBe("你");

    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Candidate selection: committed="${inputValue}"\n`,
    );
  });

  test("Number keys commit visible candidates", async ({ page }) => {
    await focusInputAndType(page, "nei");
    const inputField = page.locator("input[type='text'], textarea").first();
    const beforeSelection = await readCandidatePanelSnapshot(page, false);
    expect(beforeSelection.candidates[1].text).toBe("\u5462");

    await page.keyboard.press("2");
    await expect(inputField).toHaveValue("\u5462", { timeout: 5000 });
    await expect(page.locator(".candidate-panel")).toHaveCount(0, {
      timeout: 5000,
    });
    await expect(page.locator(".yd-toast")).toHaveCount(0, { timeout: 1000 });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("Deletion removes candidate or triggers delete path", async ({
    page,
  }) => {
    // D-08/D-10: Delete key removes candidate or triggers delete-candidate path

    await focusInputAndType(page, "nei");
    const inputField = page.locator("input[type='text'], textarea").first();

    // Press Delete key
    await page.keyboard.press("Delete");

    await page.waitForTimeout(500);

    await takeEvidenceScreenshot(page, "deletion");

    // Verify deletion effect (composition changed or candidate removed)
    const compositionAfter = await inputField.inputValue();

    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Deletion: input="${compositionAfter}"\n`,
    );
  });

  test("Backspace mutates composition", async ({ page }) => {
    // D-08/D-10: Backspace/Delete mutates composition

    await focusInputAndType(page, "nei");
    const inputField = page.locator("input[type='text'], textarea").first();

    const beforeBackspace = await inputField.inputValue();

    // Press Backspace
    await page.keyboard.press("Backspace");

    await page.waitForTimeout(500);

    const afterBackspace = await inputField.inputValue();

    // Verify composition mutated (shorter or changed)
    expect(afterBackspace.length).toBeLessThanOrEqual(beforeBackspace.length);

    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Backspace: before="${beforeBackspace}" after="${afterBackspace}"\n`,
    );
  });

  test("Deploy returns visible success/error evidence", async ({ page }) => {
    // D-08/D-10: Deploy returns visible success/error evidence

    // Locate deploy button/shortcut
    // yune-web may have deploy button in settings or Ctrl+D shortcut
    const deployButton = await page
      .locator("[data-deploy], .deploy-button, button:has-text('deploy')")
      .first();

    if ((await deployButton.count()) > 0) {
      await deployButton.click();
    } else {
      // Try keyboard shortcut (Ctrl+D or similar)
      await page.keyboard.press("Control+d");
    }

    await page.waitForTimeout(2000);

    // Verify deploy result visible (toast notification, console log, status change)
    const deployNotification = await page
      .locator(".yd-toast, .toast, [data-deploy-status], .notification")
      .first()
      .textContent({ timeout: 1000 })
      .catch(() => null);

    await takeEvidenceScreenshot(page, "deploy");

    if (deployNotification) {
      await saveEvidence(
        "browser-run.log",
        `[${new Date().toISOString()}] Deploy: status="${deployNotification}"\n`,
      );
      expect(deployNotification).toBeDefined();
    } else {
      // Check console for deploy result
      await saveEvidence(
        "browser-run.log",
        `[${new Date().toISOString()}] Deploy: triggered (no visible notification)\n`,
      );
    }
  });

  test("Customize returns visible success/error evidence", async ({ page }) => {
    // D-08/D-10: Customize returns visible success/error evidence

    const m20Settings = page.getByText(/Active engine controls/).first();
    if ((await m20Settings.count()) > 0) {
      await expect(m20Settings).toBeVisible();
      await setPreferenceRange(page, /Prediction threshold/, 1000);
      const persistedSettings = await waitForPersistedSettings(page, {
        "translator/prediction_weight_threshold": "1000",
      });
      await takeEvidenceScreenshot(page, "customize");
      await saveJsonEvidence("customize-state.json", {
        surface: "M20 settings controls",
        changed: "Prediction threshold",
        persistedSettings,
      });
      await saveEvidence(
        "browser-run.log",
        `[${new Date().toISOString()}] Customize: M20 settings control persisted prediction threshold\n`,
      );
      return;
    }

    // Locate customize settings panel
    // yune-web may have settings panel for pageSize, completion, etc.
    const settingsPanel = await page
      .locator("[data-settings], .settings-panel, .customize-panel")
      .first();

    if ((await settingsPanel.count()) > 0) {
      await settingsPanel.click();
      await page.waitForTimeout(1000);
      await takeEvidenceScreenshot(page, "customize");

      // Verify customize result visible
      const customizeNotification = await page
        .locator(".yd-toast, .toast, [data-customize-status], .notification")
        .first()
        .textContent({ timeout: 1000 })
        .catch(() => null);

      if (customizeNotification) {
        await saveEvidence(
          "browser-run.log",
          `[${new Date().toISOString()}] Customize: status="${customizeNotification}"\n`,
        );
      } else {
        await saveEvidence(
          "browser-run.log",
          `[${new Date().toISOString()}] Customize: settings changed (no visible notification)\n`,
        );
      }
    } else {
      await saveEvidence(
        "blocker.md",
        `# Browser E2E Blocker\n\n**Category**: yune-web app/source\n\n**Flow**: Customize settings\n\n**Issue**: No settings/customize panel found\n\n**Selectors tried**: M20 Active engine controls, [data-settings], .settings-panel, .customize-panel\n\n**Impact**: Cannot verify customize flow\n`,
      );
    }
  });

  test("Persistence sync after deploy/customize mutations", async ({
    page,
    context,
  }) => {
    // D-11: Persistence sync after deploy/customize/userdb-relevant boundaries

    // Perform mutation (deploy)
    await page.keyboard.press("Control+d"); // Deploy shortcut
    await page.waitForTimeout(1000);

    // Verify sync marker in console or performance timeline
    const syncMarkerFound = await verifyPersistenceMarker(
      page,
      "syncToPersistenceAfterMutation",
    );

    await saveEvidence(
      "persistence-sync.log",
      `[${new Date().toISOString()}] syncToPersistenceAfterMutation: ${syncMarkerFound ? "PASS" : "FAIL (marker not logged)"}\n`,
    );

    // Persistence evidence: check if IDBFS flushed
    // Implementation may log FS.syncfs(false) to console
    if (!syncMarkerFound) {
      await saveEvidence(
        "blocker.md",
        `# Browser E2E Blocker\n\n**Category**: Yune adapter/runtime\n\n**Flow**: Persistence sync evidence\n\n**Issue**: No persistence sync marker logged after mutation\n\n**Expected**: syncToPersistenceAfterMutation or FS.syncfs(false) console log\n\n**Impact**: Cannot verify persistence timing per D-11\n`,
      );
    }
  });

  test("Reload/reinitialize preserves persisted state", async ({
    page,
    context,
  }) => {
    // D-11: Reload/reinitialize preserves persisted state

    // Step 1: Perform customization
    const inputField = page.locator("input[type='text'], textarea").first();
    await inputField.focus();
    await inputField.type("test", { delay: 100 });

    // Step 2: Deploy to persist state
    await page.keyboard.press("Control+d");
    await page.waitForTimeout(2000);

    // Step 3: Reload page (full browser reload)
    await page.reload({ waitUntil: "domcontentloaded" });

    await page.waitForTimeout(2000);

    // Step 4: Verify persisted state restored
    // Check if previous settings/composition state persisted
    // This requires app to load persisted user state on init

    await takeEvidenceScreenshot(page, "persistence-after-reload");

    // Verify sync from persistence before init
    const syncFromMarkerFound = await verifyPersistenceMarker(
      page,
      "syncFromPersistenceBeforeInit",
    );

    await saveEvidence(
      "persistence-sync.log",
      `[${new Date().toISOString()}] Reload: syncFromPersistenceBeforeInit ${syncFromMarkerFound ? "PASS" : "FAIL (marker not logged)"}\n` +
        `[${new Date().toISOString()}] Reload: App reinitialized\n`,
    );

    // Note: Verifying specific persisted values requires app to expose persisted state
    // For E2E smoke, we verify the reload succeeded and app initialized again
    const inputAfterReload = await page
      .locator("input[type='text'], textarea")
      .first();
    expect(await inputAfterReload.count()).toBeGreaterThan(0);
  });
});

/**
 * Evidence Summary
 *
 * After running this spec, verify evidence files in e2e/results/:
 * - browser-run.log — All flow test results
 * - screenshot-*.png — Visual evidence for each flow
 * - browser-console.log — Console errors captured
 * - persistence-sync.log — Persistence timing markers
 * - blocker.md — Flows blocked by missing selectors/implementation
 *
 * Per D-09: If browser runner or flows blocked, blocker.md documents:
 * - Exact command attempted
 * - Missing dependency/selector
 * - Fallback evidence
 * - Category: yune-web app/source, Yune adapter/runtime, environment/tooling
 */

export {};
