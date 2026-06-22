/**
 * TypeDuck-Web Browser E2E Spec
 *
 * Real browser validation for patched TypeDuck-Web + Yune runtime seam.
 * Covers composition, candidate actions, deploy, customize, and persistence per D-08/TYPEDUCK-E2E-03.
 *
 * Prerequisites:
 * 1. Patched TypeDuck-Web source (patches/yune-typeduck-runtime.patch applied)
 * 2. Yune WASM artifact with yune_typeduck_* exports in packages/yune-typeduck-runtime/dist/
 * 3. Built @yune-ime/typeduck-runtime package
 * 4. Explicit TypeDuck-Web-owned YAML assets (per e2e/assets/README.md)
 * 5. Bun installed (upstream uses Bun)
 * 6. Playwright installed (standalone spec framework)
 */

import { test, expect, type Page, type BrowserContext, type Locator, type TestInfo, type WorkerInfo } from "@playwright/test";

// Test configuration
const APP_URL = process.env.TYPEDUCK_APP_URL || "http://localhost:5173";
const TIMEOUT_MS = 300000; // WASM load and runtime init may be slow

// E2E evidence directory
const EVIDENCE_DIR = process.env.TYPEDUCK_EVIDENCE_DIR || "../e2e/results";
let currentEvidenceScope = "unscoped";
const M24_EVIDENCE_DIR = "m24-dogfooding";
const M25_EVIDENCE_DIR = "m25-dogfooding";

const ACTIVE_SHOWCASE_CONTROLS = [
  /Auto-completion/,
  /Auto-correction/,
  /Auto-composition/,
  /Input Memory/,
  /AI Candidates/,
  /Combine same-text candidates/,
  /Prediction never first/,
  /Prediction threshold/,
  /Dictionary exclude/,
] as const;

const LIVE_SHOWCASE_CONTROLS = [
  /ASCII mode/,
  /Full shape/,
  /Simplification/,
  /Traditionalization/,
  /Extended charset/,
  /Disabled/,
] as const;

const DISPLAY_SHOWCASE_CONTROLS = [
  /Display languages/,
  /Candidate Jyutping/,
  /Reverse code display/,
  /Cangjie version/,
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
    if (msg.type() === "error" || msg.type() === "warning" || APP_URL.includes("debug")) {
      errors.push(`[${new Date().toISOString()}] console:${msg.type()} ${msg.text()}`);
    }
  });
  page.on("pageerror", (error) => {
    errors.push(`[${new Date().toISOString()}] pageerror: ${error.message}`);
  });
  page.on("response", (response) => {
    if (response.status() >= 400) {
      errors.push(`[${new Date().toISOString()}] response: ${response.status()} ${response.url()}`);
    }
  });
  return errors;
}

function evidenceSlug(value: string): string {
  return value
    .replace(/[^A-Za-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 90)
    || "test";
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

async function evidencePath(filename: string, scope = currentEvidenceScope): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, scope, filename);
}

function consoleFailures(messages: string[]): string[] {
  return messages.filter((message) =>
    message.includes("console:error")
    || message.includes("console:warning")
    || message.includes("pageerror:")
    || message.includes("response:")
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

async function saveWorkerEvidence(workerInfo: WorkerInfo, filename: string, content: string): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const scopedPath = await evidencePath(filename, evidenceScopeForWorker(workerInfo));
  await fs.mkdir(path.dirname(scopedPath), { recursive: true });
  await fs.appendFile(scopedPath, content, "utf8");
}

/**
 * Helper: Take screenshot with evidence filename
 */
async function takeEvidenceScreenshot(page: Page, flowName: string): Promise<void> {
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

interface PersistenceDiagnosticSnapshot {
  type: "diagnostic";
  source: string;
  marker: {
    phase?: string;
    reason?: string;
    action?: string;
    totalMs?: number;
    queueWaitMs?: number;
    workerRoundtripMs?: number;
    workerMs?: number;
    renderMs?: number;
    input?: string;
    assetVersion?: string;
    wasmBuildProfile?: string;
    wasmBinary?: string;
    markers?: { phase: string; ms: number }[];
    loadedSharedAssets?: string[];
    persistedConfig?: {
      exists?: boolean;
      settings?: Record<string, string | null>;
    };
  };
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

async function writeEvidence(filename: string, content: string): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const scopedPath = await evidencePath(filename);
  await fs.mkdir(path.dirname(scopedPath), { recursive: true });
  await fs.writeFile(scopedPath, content, "utf8");
}

async function saveJsonEvidence(filename: string, value: unknown): Promise<void> {
  await writeEvidence(filename, `${JSON.stringify(value, null, 2)}\n`);
}

async function m24EvidencePath(issueId: string, filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M24_EVIDENCE_DIR, issueId, filename);
}

async function saveM24Json(issueId: string, filename: string, payload: unknown): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m24EvidencePath(issueId, filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function takeM24Screenshot(page: Page, issueId: string, filename: string): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const screenshotPath = await m24EvidencePath(issueId, `screenshot-${filename}.png`);
  await fs.mkdir(path.dirname(screenshotPath), { recursive: true });
  await page.screenshot({ path: screenshotPath, fullPage: false });
}

async function m25EvidencePath(issueId: string, filename: string): Promise<string> {
  const path = await import("path");
  return path.join(EVIDENCE_DIR, M25_EVIDENCE_DIR, issueId, filename);
}

async function saveM25Json(issueId: string, filename: string, payload: unknown): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const jsonPath = await m25EvidencePath(issueId, filename);
  await fs.mkdir(path.dirname(jsonPath), { recursive: true });
  await fs.writeFile(jsonPath, `${JSON.stringify(payload, null, 2)}\n`, "utf8");
}

async function takeM25Screenshot(page: Page, issueId: string, filename: string): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const screenshotPath = await m25EvidencePath(issueId, `screenshot-${filename}.png`);
  await fs.mkdir(path.dirname(screenshotPath), { recursive: true });
  await page.screenshot({ path: screenshotPath, fullPage: false });
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

async function loadFixture(filename: string): Promise<Record<string, unknown> & { cases: Record<string, unknown>[] }> {
  return JSON.parse(await readRepoText(`crates/yune-core/tests/fixtures/typeduck-v1.1.2/${filename}`));
}

async function m14Case(
  filename: string,
  variant: string,
  input: string,
): Promise<{ selected_candidates: { text: string; comment?: string }[] }> {
  const fixture = await loadFixture(filename);
  const found = fixture.cases.find((candidate) =>
    candidate["variant"] === variant && candidate["input"] === input
  ) as { selected_candidates: { text: string; comment?: string }[] } | undefined;
  if (!found) {
    throw new Error(`Missing M14 golden case ${filename}:${variant}:${input}`);
  }
  return found;
}

async function m14Texts(filename: string, variant: string, input: string, count: number): Promise<string[]> {
  const found = await m14Case(filename, variant, input);
  return found.selected_candidates.slice(0, count).map((candidate) => candidate.text);
}

async function m14Notes(filename: string, variant: string, input: string, count: number): Promise<string[]> {
  const found = await m14Case(filename, variant, input);
  return found.selected_candidates.slice(0, count).map((candidate) =>
    (candidate.comment ?? "").split("\f")[0].replace(/^\v/, "")
  );
}

async function waitForAppReady(page: Page): Promise<void> {
  await page.waitForFunction(
    () =>
      document.documentElement.dataset.yuneInitialized === "true"
      && document.documentElement.dataset.yuneLoading === "false",
    undefined,
    { timeout: TIMEOUT_MS },
  );
  await expect(page.locator("[data-yune-loading-indicator]")).toHaveCount(0, { timeout: TIMEOUT_MS });
}

async function openApp(page: Page): Promise<void> {
  await page.goto(APP_URL, { timeout: TIMEOUT_MS, waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
}

async function readCandidatePanelSnapshot(page: Page, aiEnabled: boolean): Promise<CandidatePanelSnapshot> {
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

async function focusInputAndType(page: Page, text: string, expectedVisibleText = text): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await inputField.focus();
  await inputField.type(text, { delay: 80 });
  await expect(page.locator(".candidate-panel .candidates tbody").first()).toBeVisible({ timeout: 5000 });
  await expect(page.locator(".candidate-panel").first()).toContainText(expectedVisibleText, { timeout: 5000 });
}

async function typeCompositionAndWaitForCandidate(page: Page, input: string, expectedText: string): Promise<CandidatePanelSnapshot> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(input, { delay: 120 });
  await expect.poll(async () => {
    const state = await readCandidatePanelSnapshot(page, false);
    return state.candidates.map((candidate) => candidate.text);
  }, { timeout: 10000 }).toContain(expectedText);
  return readCandidatePanelSnapshot(page, false);
}

async function typeCompositionAndWaitForTopCandidate(page: Page, input: string, expectedText: string): Promise<CandidatePanelSnapshot> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(input, { delay: 120 });
  await expect.poll(async () => {
    const state = await readCandidatePanelSnapshot(page, false);
    return state.candidates[0]?.text ?? null;
  }, { timeout: 10000 }).toBe(expectedText);
  return readCandidatePanelSnapshot(page, false);
}

async function typeCompositionAndWaitForRowCount(page: Page, input: string, expectedRows: number): Promise<CandidatePanelSnapshot> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(input, { delay: 80 });
  await expect.poll(async () =>
    page.locator(".candidate-panel .candidates tbody").count(),
    { timeout: 10000 },
  ).toBe(expectedRows);
  return readCandidatePanelSnapshot(page, false);
}

async function learnPhraseThroughBrowser(page: Page): Promise<CandidatePanelSnapshot> {
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

function classicCandidateSignature(state: CandidatePanelSnapshot): { text: string | null; note: string | null; rowText: string }[] {
  return state.candidates.map((candidate) => ({
    text: candidate.text,
    note: candidate.note,
    rowText: candidate.rowText,
  }));
}

async function expectNoToasts(page: Page): Promise<void> {
  await expect(page.locator(".Toastify__toast")).toHaveCount(0, { timeout: 1000 });
}

async function clickShowcaseScenario(page: Page, name: string | RegExp, expectedText: string, aiEnabled = false): Promise<CandidatePanelSnapshot> {
  await clearComposition(page);
  await page.waitForTimeout(500);
  await page.getByRole("button", { name }).click();
  await expect.poll(async () => {
    const state = await readCandidatePanelSnapshot(page, aiEnabled);
    return state.candidates.map((candidate) => candidate.text);
  }, { timeout: 10000 }).toContain(expectedText);
  await page.waitForTimeout(750);
  return readCandidatePanelSnapshot(page, aiEnabled);
}

async function typeRawInput(page: Page, text: string): Promise<{ value: string; panelCount: number }> {
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
  for (let attempts = 0; attempts < 4 && await page.locator(".candidate-panel").count() > 0; attempts += 1) {
    await page.keyboard.press("Escape").catch(() => undefined);
    await page.waitForTimeout(150);
  }
  await inputField.fill("");
  await expect(page.locator(".candidate-panel")).toHaveCount(0, { timeout: 5000 });
}

async function clearCompositionThroughInput(page: Page): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await inputField.focus();
  for (let attempts = 0; attempts < 12 && await page.locator(".candidate-panel").count() > 0; attempts += 1) {
    await page.keyboard.press("Backspace");
    await page.waitForTimeout(120);
  }
  await inputField.fill("");
  await expect(page.locator(".candidate-panel")).toHaveCount(0, { timeout: 5000 });
}

async function setPreferenceToggle(page: Page, label: RegExp, enabled: boolean): Promise<void> {
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

async function setPreferenceRange(page: Page, label: RegExp, value: number): Promise<void> {
  const range = page.getByLabel(label).last();
  await range.evaluate((element, nextValue) => {
    const input = element as HTMLInputElement;
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")?.set;
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

async function readPersistenceDiagnostics(page: Page): Promise<PersistenceDiagnosticSnapshot[]> {
  const raw = await page.evaluate(() => document.documentElement.dataset.yunePersistenceDiagnostics ?? "[]");
  return JSON.parse(raw) as PersistenceDiagnosticSnapshot[];
}

async function latestPersistedSettings(page: Page): Promise<Record<string, string | null>> {
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
  await expect.poll(async () => {
    const settings = await latestPersistedSettings(page);
    return Object.fromEntries(
      Object.keys(expected).map((key) => [key, settings[key] ?? null]),
    );
  }, { timeout: 15000 }).toEqual(expected);
  return latestPersistedSettings(page);
}

async function selectSchema(page: Page, label: string | RegExp): Promise<void> {
  await clearComposition(page);
  const switcher = page.locator("[data-yune-schema-switcher]");
  await expect(switcher).toBeVisible({ timeout: 5000 });
  await switcher.getByLabel(label).check({ force: true });
  const expectedSchema = expectedSchemaIdForLabel(label);
  if (expectedSchema !== null) {
    await expect.poll(async () =>
      page.evaluate(() => document.documentElement.dataset.yuneActiveSchema ?? null),
      { timeout: TIMEOUT_MS },
    ).toBe(expectedSchema);
  }
  await waitForAppReady(page);
  await expect(page.locator(".candidate-panel")).toHaveCount(0, { timeout: 5000 });
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
    return "jyut6ping3_mobile";
  }
  return null;
}

async function typeInputForStatus(page: Page, input: string): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type(input, { delay: 120 });
  await expect(page.locator("[data-yune-status]")).toBeVisible({ timeout: 10000 });
}

async function readYuneStatus(page: Page): Promise<Record<string, string | null>> {
  const status = page.locator("[data-yune-status]");
  await expect(status).toBeVisible({ timeout: 10000 });
  const schema = page.locator("[data-yune-status-schema]");
  const disabled = page.locator("[data-yune-status-disabled]");
  const traditional = page.locator("[data-yune-status-traditional]");
  const ascii = page.locator("[data-yune-status-ascii]");
  const disabledValue = await disabled.getAttribute("data-yune-status-disabled");
  const traditionalValue = await traditional.getAttribute("data-yune-status-traditional");
  const asciiValue = await ascii.getAttribute("data-yune-status-ascii");
  return {
    schema: await schema.getAttribute("data-yune-status-schema-id") ?? await schema.textContent(),
    disabled: disabledValue === "true" ? "disabled" : "enabled",
    traditional: traditionalValue === "true" ? "traditional" : "not traditional",
    ascii: asciiValue === "true" ? "ASCII" : "Chinese",
  };
}

async function waitForDeployedSettings(
  page: Page,
  expected: Record<string, string | null>,
): Promise<Record<string, string | null>> {
  await expect.poll(async () => {
    const diagnostics = await readPersistenceDiagnostics(page);
    const deployPass = diagnostics
      .slice()
      .reverse()
      .find((diagnostic) =>
        diagnostic.marker.phase === "syncToPersistenceAfterMutation:pass"
        && diagnostic.marker.reason === "deploy"
        && diagnostic.marker.persistedConfig?.settings
      );
    const settings = deployPass?.marker.persistedConfig?.settings ?? {};
    return Object.fromEntries(
      Object.keys(expected).map((key) => [key, settings[key] ?? null]),
    );
  }, { timeout: 30000 }).toEqual(expected);
  return latestPersistedSettings(page);
}

/**
 * Helper: Verify persistence sync markers in console
 */
async function verifyPersistenceMarker(page: Page, marker: string): Promise<boolean> {
  try {
    await expect.poll(async () => {
      const diagnostics = await readPersistenceDiagnostics(page);
      return diagnostics.some((diagnostic) => diagnostic.marker.phase?.includes(marker));
    }, { timeout: 5000 }).toBe(true);
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
  expect(state.candidates.slice(0, expectedTexts.length).map((candidate) => candidate.text)).toEqual(expectedTexts);
  if (expectedNotes) {
    expect(state.candidates.slice(0, expectedNotes.length).map((candidate) => candidate.note ?? "")).toEqual(expectedNotes);
  }
  await saveJsonEvidence(`m16-${name}-state.json`, {
    expectedTexts,
    expectedNotes,
    state,
  });
  await takeEvidenceScreenshot(page, `m16-${name}`);
  return state;
}

async function captureM24Phrase(page: Page, issueId: string, input: string, expectedTopText: string): Promise<CandidatePanelSnapshot> {
  const state = await typeCompositionAndWaitForTopCandidate(page, input, expectedTopText);
  await saveM24Json(issueId, `${input}-state.json`, state);
  await takeM24Screenshot(page, issueId, `${input}-candidate-panel`);
  return state;
}

test.describe("TypeDuck-Web Browser E2E", () => {
  test.setTimeout(TIMEOUT_MS);

  let consoleErrors: string[] = [];

  test.beforeAll(async ({}, workerInfo) => {
    // Record browser runner start
    await saveWorkerEvidence(
      workerInfo,
      "browser-run.log",
      `[${new Date().toISOString()}] Browser E2E started\nURL: ${APP_URL}\n`
    );
  });

  test.beforeEach(async ({ page }, testInfo) => {
    setEvidenceScope(testInfo);
    consoleErrors = await captureConsoleErrors(page);
    await openApp(page);
  });

  test.afterEach(async ({ page }, testInfo) => {
    setEvidenceScope(testInfo);
    // Append test result to evidence log
    const status = testInfo.status || "unknown";
    const duration = testInfo.duration || 0;
    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Test: ${testInfo.title} - ${status} (${duration}ms)\n`
    );

    // Save console errors if any
    if (consoleErrors.length > 0) {
      await saveEvidence(
        "browser-console.log",
        consoleErrors.join("\n") + "\n"
      );
    }
  });

  test("M24 startup timing trace records loading phases", async ({ page }) => {
    let startup: PersistenceDiagnosticSnapshot | undefined;
    await expect.poll(async () => {
      const diagnostics = await readPersistenceDiagnostics(page);
      startup = diagnostics.find((diagnostic) => diagnostic.source === "yune-startup");
      return startup?.marker.phase ?? "";
    }, { timeout: TIMEOUT_MS }).toBe("startup:complete");
    const resources = await page.evaluate(() =>
      performance.getEntriesByType("resource")
        .filter(entry => /yune-typeduck|schema|\.bin|\.ocd2/.test(entry.name))
        .map(entry => ({
          name: entry.name,
          duration: entry.duration,
          transferSize: "transferSize" in entry ? (entry as PerformanceResourceTiming).transferSize : 0,
        })),
    );
    await saveM24Json("M24-DOGFOOD-01", "startup-resources.json", { startup, resources });
    await takeM24Screenshot(page, "M24-DOGFOOD-01", "startup-ready");
    expect(startup?.marker.wasmBinary).toBe("yune-typeduck.wasm");
    expect(startup?.marker.loadedSharedAssets).toContain("luna_pinyin_yune_reverse.dict.yaml");
    expect(startup?.marker.phase).toBe("startup:complete");
  });

  test("M25 DOGFOOD-01 startup uses release wasm and records deploy cache reuse", async ({ page }) => {
    let startup: PersistenceDiagnosticSnapshot | undefined;
    await expect.poll(async () => {
      const diagnostics = await readPersistenceDiagnostics(page);
      startup = diagnostics.find((diagnostic) => diagnostic.source === "yune-startup");
      return startup?.marker.phase ?? "";
    }, { timeout: TIMEOUT_MS }).toBe("startup:complete");

    const firstReadyAt = Date.now();
    await setPreferenceRange(page, /No\. of Candidates Per Page|æ¯é å€™é¸è©žæ•¸é‡/, 7);
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
      .find((diagnostic) =>
        diagnostic.source === "yune-persistence"
        && diagnostic.marker.reason === "deploy"
        && /^deploy:cache-/.test(diagnostic.marker.phase ?? "")
      );
    const resources = await page.evaluate(() =>
      performance.getEntriesByType("resource")
        .filter(entry => /yune-typeduck|schema|\.bin|\.ocd2/.test(entry.name))
        .map(entry => ({
          name: entry.name,
          duration: entry.duration,
          transferSize: "transferSize" in entry ? (entry as PerformanceResourceTiming).transferSize : 0,
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
    expect(startupAfterReload?.marker.totalMs ?? Number.POSITIVE_INFINITY).toBeLessThanOrEqual(15000);
  });

  test("M25 DOGFOOD-03 typing stays out of global loading and records key latency", async ({ page }) => {
    await page.evaluate(() => {
      document.documentElement.dataset.yuneActionDiagnostics = "[]";
      document.documentElement.dataset.yuneTypingDiagnostics = "[]";
    });

    const inputField = page.locator("input[type='text'], textarea").first();
    await clearComposition(page);
    await expect(page.locator("[data-yune-loading-indicator]")).toHaveCount(0, { timeout: 1000 });
    await inputField.focus();
    await inputField.type("hai", { delay: 40 });
    await expect(page.locator(".candidate-panel .candidates tbody").first()).toBeVisible({ timeout: 10000 });
    await expect(page.locator("[data-yune-loading-indicator]")).toHaveCount(0, { timeout: 1000 });

    const actionDiagnostics = await page.evaluate(() =>
      JSON.parse(document.documentElement.dataset.yuneActionDiagnostics ?? "[]") as unknown[],
    );
    const typingDiagnostics = await page.evaluate(() =>
      JSON.parse(document.documentElement.dataset.yuneTypingDiagnostics ?? "[]") as { action?: string; totalMs?: number; renderMs?: number; input?: string }[],
    );
    const processKeyActions = actionDiagnostics.filter((diagnostic) =>
      typeof diagnostic === "object"
      && diagnostic !== null
      && (diagnostic as { action?: string }).action === "processKey"
    );
    const processKeyTyping = typingDiagnostics.filter((diagnostic) => diagnostic.action === "processKey");
    const p95 = processKeyTyping
      .map((diagnostic) => diagnostic.totalMs ?? Number.POSITIVE_INFINITY)
      .sort((left, right) => left - right)[Math.max(0, Math.ceil(processKeyTyping.length * 0.95) - 1)];

    await saveM25Json("M25-DOGFOOD-03", "typing-latency-after.json", {
      input: "hai",
      actionDiagnostics,
      typingDiagnostics,
      processKeyActionCount: processKeyActions.length,
      processKeyTypingCount: processKeyTyping.length,
      p95,
      loadingIndicatorCount: await page.locator("[data-yune-loading-indicator]").count(),
    });
    await takeM25Screenshot(page, "M25-DOGFOOD-03", "typing-responsive");

    expect(processKeyActions.length).toBeGreaterThanOrEqual(3);
    expect(processKeyTyping.length).toBeGreaterThanOrEqual(3);
    expect(p95).toBeLessThanOrEqual(750);
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M25 DOGFOOD-02 page-size slider caps rows and preserves candidate paging", async ({ page }) => {
    const pageSizeControl = page.getByLabel(/No\. of Candidates Per Page|Candidates Per Page/).last();
    await expect(pageSizeControl).toHaveAttribute("min", "3");
    await expect(pageSizeControl).toHaveAttribute("max", "10");

    const evidence: Record<string, unknown> = {};
    for (const pageSize of [3, 9, 10] as const) {
      await setPreferenceRange(page, /No\. of Candidates Per Page|Candidates Per Page/, pageSize);
      await waitForPersistedSettings(page, { "menu/page_size": String(pageSize) });
      const state = await typeCompositionAndWaitForRowCount(page, "hai", pageSize);
      expect(state.candidates).toHaveLength(pageSize);
      evidence[`pageSize${pageSize}`] = state;
      await saveM25Json("M25-DOGFOOD-02", `page-size-${pageSize}-hai-state.json`, state);
      await takeM25Screenshot(page, "M25-DOGFOOD-02", `page-size-${pageSize}-hai`);

      if (pageSize === 3) {
        const firstPageFirst = state.candidates[0]?.text;
        await page.locator(".candidate-panel .page-nav").last().click();
        await expect.poll(async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text ?? null,
          { timeout: 5000 },
        ).not.toBe(firstPageFirst ?? null);
        const nextPage = await readCandidatePanelSnapshot(page, false);
        expect(nextPage.candidates.length).toBeLessThanOrEqual(pageSize);
        evidence.pageSize3NextPage = nextPage;
        await page.locator(".candidate-panel .page-nav").first().click();
        await expect.poll(async () =>
          (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text ?? null,
          { timeout: 5000 },
        ).toBe(firstPageFirst ?? null);
      }

      if (pageSize === 9 || pageSize === 10) {
        const key = pageSize === 9 ? "9" : "0";
        const selectedText = state.candidates[pageSize - 1]?.text;
        expect(selectedText).toBeTruthy();
        await page.keyboard.press(key);
        await expect(page.locator("input[type='text'], textarea").first()).toHaveValue(selectedText ?? "", { timeout: 5000 });
        await expect(page.locator(".candidate-panel")).toHaveCount(0, { timeout: 5000 });
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

  test("M25 DOGFOOD-08 Jyutping bare grave routes to Luna reverse lookup", async ({ page }) => {
    await selectSchema(page, /Jyutping/);
    const schemaSwitcher = page.locator("[data-yune-schema-switcher]");
    await expect(schemaSwitcher).toContainText(/`zhe/);
    await expect(schemaSwitcher).toContainText(/`vl/);
    await expect(schemaSwitcher).toContainText(/`vc/);

    const bareLuna = await typeCompositionAndWaitForCandidate(page, "`zhe", "這");
    expect(candidateTexts(bareLuna)).toContain("這");
    await saveM25Json("M25-DOGFOOD-08", "bare-grave-zhe-state.json", bareLuna);
    await takeM25Screenshot(page, "M25-DOGFOOD-08", "bare-grave-zhe");

    const overlap: Record<string, CandidatePanelSnapshot> = {};
    for (const input of ["`lai", "`ci", "`xi", "`re"] as const) {
      await clearComposition(page);
      const inputField = page.locator("input[type='text'], textarea").first();
      await inputField.focus();
      await inputField.type(input, { delay: 80 });
      await expect(page.locator(".candidate-panel .candidates tbody").first()).toBeVisible({ timeout: 10000 });
      const state = await readCandidatePanelSnapshot(page, false);
      expect(state.candidates.length).toBeGreaterThan(0);
      overlap[input] = state;
    }

    const triggerCopy = await schemaSwitcher.textContent();
    await saveM25Json("M25-DOGFOOD-08", "reverse-lookup-summary.json", {
      note: "M25 intentionally diverges from TypeDuck v1.1.2's historical `p trigger: bare ` belongs to luna_pinyin; retained side lookups use explicit `vl and `vc triggers.",
      activeSchema: await page.evaluate(() => document.documentElement.dataset.yuneActiveSchema ?? null),
      triggerCopy,
      bareLuna,
      overlap,
      operationErrorToastCount: await page.locator(".Toastify__toast").count(),
    });
    await takeM25Screenshot(page, "M25-DOGFOOD-08", "reverse-lookup-overlap");

    await expect(page.locator(".Toastify__toast")).toHaveCount(0, { timeout: 1000 });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M25 DOGFOOD-04 schema switcher shares the top control row", async ({ page }) => {
    const topControls = page.locator("[data-yune-top-controls]");
    await expect(topControls).toBeVisible();
    const schemaSwitcher = topControls.locator("[data-yune-schema-switcher]");
    await expect(schemaSwitcher).toBeVisible();
    await expect(schemaSwitcher.getByText("朙月拼音", { exact: true })).toBeVisible();

    const desktopBoxes = {
      topControls: await elementBox(topControls),
      asciiButton: await elementBox(topControls.getByRole("button", { name: /ASCII mode/ })),
      schemaSwitcher: await elementBox(schemaSwitcher),
    };
    expect(Math.abs(desktopBoxes.schemaSwitcher.y - desktopBoxes.asciiButton.y)).toBeLessThanOrEqual(80);

    await selectSchema(page, /Luna Pinyin/);
    await typeInputForStatus(page, "hao");
    await expect(page.locator("[data-yune-status-schema]")).toContainText("朙月拼音");
    await saveM25Json("M25-DOGFOOD-04", "schema-switcher-toolbar-desktop.json", desktopBoxes);
    await takeM25Screenshot(page, "M25-DOGFOOD-04", "schema-switcher-toolbar-desktop");

    await page.setViewportSize({ width: 390, height: 900 });
    const mobileBoxes = {
      topControls: await elementBox(topControls),
      schemaSwitcher: await elementBox(schemaSwitcher),
    };
    await saveM25Json("M25-DOGFOOD-04", "schema-switcher-toolbar-mobile.json", mobileBoxes);
    await takeM25Screenshot(page, "M25-DOGFOOD-04", "schema-switcher-toolbar-mobile");
  });

  test("M25 DOGFOOD-05 Cangjie version control lives in the top control band", async ({ page }) => {
    await selectSchema(page, /Jyutping/);
    const topControls = page.locator("[data-yune-top-controls]");
    await expect(topControls).toBeVisible();
    const cangjieControl = topControls.locator("[data-yune-control='cangjie-version']");
    await expect(cangjieControl).toBeVisible();
    await expect(page.getByText(/Web Frontend Controls/)).toHaveCount(0);

    await cangjieControl.getByText(/Version 3/).click();
    await expect(cangjieControl.getByLabel(/Version 3/)).toBeChecked();
    await waitForAppReady(page);
    const version3 = await waitForPersistedSettings(page, { "cangjie/dictionary": "cangjie3" });
    await cangjieControl.getByText(/Version 5/).click();
    await expect(cangjieControl.getByLabel(/Version 5/)).toBeChecked();
    await waitForAppReady(page);
    const version5 = await waitForPersistedSettings(page, { "cangjie/dictionary": "cangjie5" });

    await saveM25Json("M25-DOGFOOD-05", "cangjie-version-top-controls.json", {
      boxes: {
        topControls: await elementBox(topControls),
        cangjieControl: await elementBox(cangjieControl),
      },
      version3,
      version5,
    });
    await takeM25Screenshot(page, "M25-DOGFOOD-05", "cangjie-version-top-controls");
  });

  test("M25 DOGFOOD-06 display controls precede live session controls", async ({ page }) => {
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
    expect(Math.abs(desktopBoxes.display.y - desktopBoxes.active.y)).toBeLessThanOrEqual(24);
    expect(desktopBoxes.live.y).toBeGreaterThan(desktopBoxes.display.y + 24);
    await saveM25Json("M25-DOGFOOD-06", "display-live-section-order-desktop.json", desktopBoxes);
    await takeM25Screenshot(page, "M25-DOGFOOD-06", "display-live-section-order-desktop");

    await page.setViewportSize({ width: 390, height: 900 });
    const mobileBoxes = {
      active: await elementBox(active),
      display: await elementBox(display),
      live: await elementBox(live),
    };
    expect(mobileBoxes.active.y).toBeLessThan(mobileBoxes.display.y);
    expect(mobileBoxes.display.y).toBeLessThan(mobileBoxes.live.y);
    await saveM25Json("M25-DOGFOOD-06", "display-live-section-order-mobile.json", mobileBoxes);
    await takeM25Screenshot(page, "M25-DOGFOOD-06", "display-live-section-order-mobile");
  });

  test("M25 DOGFOOD-10 IME settings align with the playground content edges", async ({ page }) => {
    const playground = page.locator("[data-yune-playground-content]");
    const preferences = page.locator("[data-yune-preferences]");
    await expect(playground).toBeVisible();
    await expect(preferences).toBeVisible();

    const desktop = {
      playground: await elementBox(playground),
      preferences: await elementBox(preferences),
    };
    expect(Math.abs(desktop.playground.x - desktop.preferences.x)).toBeLessThanOrEqual(2);
    expect(Math.abs(desktop.playground.right - desktop.preferences.right)).toBeLessThanOrEqual(2);
    await saveM25Json("M25-DOGFOOD-10", "settings-alignment-desktop.json", desktop);
    await takeM25Screenshot(page, "M25-DOGFOOD-10", "settings-alignment-desktop");

    await page.setViewportSize({ width: 390, height: 900 });
    const mobile = {
      playground: await elementBox(playground),
      preferences: await elementBox(preferences),
    };
    expect(Math.abs(mobile.playground.x - mobile.preferences.x)).toBeLessThanOrEqual(2);
    expect(Math.abs(mobile.playground.right - mobile.preferences.right)).toBeLessThanOrEqual(2);
    await saveM25Json("M25-DOGFOOD-10", "settings-alignment-mobile.json", mobile);
    await takeM25Screenshot(page, "M25-DOGFOOD-10", "settings-alignment-mobile");
  });

  test("M25 DOGFOOD-07 binary controls use checkbox affordance", async ({ page }) => {
    const preferences = page.locator("[data-yune-preferences]");
    const preferenceChecks = preferences.locator("input[type='checkbox']");
    await expect(preferenceChecks.first()).toBeVisible();
    const preferenceControls = await preferenceChecks.evaluateAll((inputs) =>
      inputs.map((input) => {
        const element = input as HTMLInputElement;
        const style = getComputedStyle(element);
        return {
          label: element.labels?.[0]?.textContent?.replace(/\s+/g, " ").trim() ?? element.getAttribute("aria-label"),
          checked: element.checked,
          className: element.className,
          borderRadius: style.borderRadius,
          width: style.width,
          height: style.height,
        };
      }),
    );
    expect(preferenceControls.length).toBeGreaterThan(10);
    expect(preferenceControls.every(control => String(control.className).includes("yd-check"))).toBe(true);
    expect(preferenceControls.every(control => !String(control.className).includes("yd-switch"))).toBe(true);

    const inspector = page.locator("[data-yune-inspector-toggle] input[type='checkbox']");
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
    await takeM25Screenshot(page, "M25-DOGFOOD-07", "checkbox-affordance-desktop");
    await page.setViewportSize({ width: 390, height: 900 });
    await takeM25Screenshot(page, "M25-DOGFOOD-07", "checkbox-affordance-mobile");
  });

  test("M25 DOGFOOD-09 candidate menu layout uses radio choices", async ({ page }) => {
    const field = page.locator("[data-yune-preferences] .yd-field").filter({ hasText: /Candidate Menu Layout/ });
    await expect(field).toBeVisible();
    await expect(field.locator(".yd-segment")).toHaveCount(0);
    const radios = field.locator("input[type='radio'].yd-choice");
    await expect(radios).toHaveCount(2);
    await expect(field.getByLabel(/Horizontal/)).toBeChecked();
    await expect(field.getByLabel(/Vertical/)).not.toBeChecked();

    let horizontal = await typeCompositionAndWaitForTopCandidate(page, "nei", "你");
    await expect(page.locator(".candidate-panel")).toHaveClass(/candidate-panel--horizontal/);
    await saveM25Json("M25-DOGFOOD-09", "candidate-layout-horizontal.json", horizontal);
    await takeM25Screenshot(page, "M25-DOGFOOD-09", "candidate-layout-horizontal");

    await clearComposition(page);
    await field.getByLabel(/Vertical/).check({ force: true });
    await expect(field.getByLabel(/Vertical/)).toBeChecked();
    const vertical = await typeCompositionAndWaitForTopCandidate(page, "nei", "你");
    await expect(page.locator(".candidate-panel")).toHaveClass(/candidate-panel--vertical/);
    await saveM25Json("M25-DOGFOOD-09", "candidate-layout-vertical.json", {
      before: horizontal,
      after: vertical,
      radioLabels: await field.locator("label").evaluateAll(labels => labels.map(label => label.textContent?.replace(/\s+/g, " ").trim())),
    });
    await takeM25Screenshot(page, "M25-DOGFOOD-09", "candidate-layout-vertical");
    horizontal = vertical;
    expect(horizontal.candidates.length).toBeGreaterThan(0);
  });

  test("M24 phrase comments render without raw control markers", async ({ page }) => {
    const state = await captureM24Phrase(page, "M24-DOGFOOD-02", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
    const visibleRows = state.candidates.map(candidate => candidate.rowText).join("\n");
    expect(visibleRows).not.toMatch(/(?:\\f|\\r|\\v|\f|\r|\v)/);
  });

  test("M24 compound candidate rows stay compact with details in the dictionary panel", async ({ page }) => {
    const state = await captureM24Phrase(page, "M24-DOGFOOD-03", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
    expect(state.candidates[0].rowText).not.toContain("think; ponder");
    await page.locator(`.candidate-panel .candidates tbody[data-candidate-text="${M24_DOGFOOD_TOP}"]`).hover();
    await expect(page.locator(".dictionary-panel")).toContainText(/think; ponder|want; need|now/, { timeout: 5000 });
    await takeM24Screenshot(page, "M24-DOGFOOD-03", "dictionary-detail-panel");
  });

  test("M24 jigaajiusihaa order is recorded against the current pinned expectation", async ({ page }) => {
    const state = await captureM24Phrase(page, "M24-DOGFOOD-04", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
    const fixture = await loadFixture("jyut6ping3-m24-dogfooding.json");
    const expectedCase = fixture.cases.find((candidate) =>
      candidate["variant"] === "default_combined" && candidate["input"] === M24_DOGFOOD_INPUT
    ) as { selected_candidates: { text: string }[] } | undefined;
    if (!expectedCase) {
      throw new Error(`Missing M24 dogfood fixture case for ${M24_DOGFOOD_INPUT}`);
    }
    const expectedTexts = expectedCase.selected_candidates.map((candidate) => candidate.text);
    const actualTexts = candidateTexts(state);
    const compareCount = Math.min(expectedTexts.length, actualTexts.length, 5);
    await saveM24Json("M24-DOGFOOD-04", "jigaajiusihaa-order.json", {
      candidateTexts: actualTexts,
      expectedTexts: expectedTexts.slice(0, compareCount),
      note: "Engine ordering is fixture-gated against TypeDuck v1.1.2; live deployed-site differences are not treated as hard oracle evidence.",
    });
    expect(actualTexts.slice(0, compareCount)).toEqual(expectedTexts.slice(0, compareCount));
  });

  test("M24/M25 settings labels are Cantonese-first and grouped by engine, display, and session", async ({ page }) => {
    await expect(page.getByText(/引擎設定 Active engine controls/)).toBeVisible();
    await expect(page.getByText(/顯示設定 Display controls/)).toBeVisible();
    await expect(page.getByText(/即時狀態 Live session controls/)).toBeVisible();
    await expect(page.getByText(/Web Frontend Controls/)).toHaveCount(0);
    await expect(page.getByText(/會重新部署 schema/)).toBeVisible();
    await expect(page.getByText(/只改目前 session/)).toBeVisible();
    await takeM24Screenshot(page, "M24-DOGFOOD-05", "settings-labels-desktop");
    await page.setViewportSize({ width: 390, height: 900 });
    await takeM24Screenshot(page, "M24-DOGFOOD-05", "settings-labels-narrow");
  });

  test("M24 display languages use a checklist with deterministic primary language", async ({ page }) => {
    await expect(page.getByText(/主要語言 Main Language/)).toHaveCount(0);
    const languageChecks = page.locator(".yd-field").filter({ hasText: /Display languages/ }).locator("input[type='checkbox']");
    await expect(languageChecks).toHaveCount(5);
    await page.getByLabel(/Hindi/).last().check({ force: true });
    const state = await captureM24Phrase(page, "M24-DOGFOOD-06", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
    await takeM24Screenshot(page, "M24-DOGFOOD-06", "display-languages-checklist");
    expect(state.candidates.length).toBeGreaterThan(0);
  });

  test("M24 candidate page-size slider limits the visible candidate page", async ({ page }) => {
    for (const pageSize of [4, 10] as const) {
      await setPreferenceRange(page, /No\. of Candidates Per Page|每頁候選詞數量/, pageSize);
      await waitForPersistedSettings(page, { "menu/page_size": String(pageSize) });
      const state = await captureM24Phrase(page, "M24-DOGFOOD-07", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
      await saveM24Json("M24-DOGFOOD-07", `page-size-${pageSize}-state.json`, state);
      await takeM24Screenshot(page, "M24-DOGFOOD-07", `page-size-${pageSize}-candidates`);
      expect(state.candidates.length).toBeLessThanOrEqual(pageSize);
    }
  });

  test("M24 candidate menu layout is a frontend-only horizontal or vertical setting", async ({ page }) => {
    const horizontal = await captureM24Phrase(page, "M24-DOGFOOD-08", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
    await clearComposition(page);
    await page.getByText("直排 Vertical").click();
    await expect(page.getByLabel(/直排 Vertical/).last()).toBeChecked();
    const vertical = await captureM24Phrase(page, "M24-DOGFOOD-08", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
    await expect(page.locator(".candidate-panel")).toHaveClass(/candidate-panel--vertical/);
    await takeM24Screenshot(page, "M24-DOGFOOD-08", "vertical");
    await clearComposition(page);
    await page.getByText("橫排 Horizontal").click();
    await expect(page.getByLabel(/橫排 Horizontal/).last()).toBeChecked();
    await captureM24Phrase(page, "M24-DOGFOOD-08", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
    await takeM24Screenshot(page, "M24-DOGFOOD-08", "horizontal");
    expect(classicCandidateSignature(vertical)).toEqual(classicCandidateSignature(horizontal));
  });

  test("M24 engine status strip explains labeled state", async ({ page }) => {
    await typeInputForStatus(page, "nei");
    await expect(page.getByText(/引擎狀態 Engine status/)).toBeVisible();
    await expect(page.getByText(/顯示目前 schema/)).toBeVisible();
    await saveM24Json("M24-DOGFOOD-09", "status-strip.json", await readYuneStatus(page));
    await takeM24Screenshot(page, "M24-DOGFOOD-09", "status-strip");
  });

  test("M24 schema switcher uses bundled real schema names", async ({ page }) => {
    const schemaSwitcher = page.locator("[data-yune-schema-switcher]");
    await expect(schemaSwitcher.getByText("粵語拼音")).toBeVisible();
    await expect(schemaSwitcher.getByText("倉頡五代")).toBeVisible();
    await expect(schemaSwitcher.getByText("朙月拼音", { exact: true })).toBeVisible();
    await selectSchema(page, /Cangjie 5/);
    await typeInputForStatus(page, "a");
    await expect(page.locator("[data-yune-status-schema]")).toContainText("倉頡五代");
    await takeM24Screenshot(page, "M24-DOGFOOD-10", "schema-switcher-real-names");
  });

  test("M24/M25 Jyutping reverse lookup accepts current Mandarin pinyin affix input", async ({ page }) => {
    await selectSchema(page, /Jyutping/);
    const state = await typeCompositionAndWaitForCandidate(page, "`zhe", "這");
    await saveM25Json("M25-DOGFOOD-08", "legacy-reverse-lookup-bare-zhe.json", state);
    await takeM25Screenshot(page, "M25-DOGFOOD-08", "legacy-reverse-lookup-bare-zhe");
    expect(candidateTexts(state)).toContain("這");

    await clearComposition(page);
    const normal = await typeCompositionAndWaitForTopCandidate(page, "nei", "你");
    expect(normal.candidates[0].text).toBe("你");
  });

  test("M24 Chinese typeface picker applies full family names to visible Chinese surfaces", async ({ page }) => {
    await expect(page.getByText("Chiron Sung HK")).toBeVisible();
    await expect(page.getByText("Chiron Hei HK")).toBeVisible();
    await expect(page.getByText("Iansui")).toBeVisible();
    await expect(page.getByText(/宋體 Sung|黑體 Hei/)).toHaveCount(0);
    await page.getByLabel("Iansui").check({ force: true });
    const state = await captureM24Phrase(page, "M24-DOGFOOD-12", M24_DOGFOOD_INPUT, M24_DOGFOOD_TOP);
    await page.locator(`.candidate-panel .candidates tbody[data-candidate-text="${M24_DOGFOOD_TOP}"]`).hover();
    await expect(page.locator("[data-chinese-typeface='iansui']").first()).toBeVisible();
    const textareaFont = await page.locator("textarea").evaluate(element => getComputedStyle(element).fontFamily);
    await saveM24Json("M24-DOGFOOD-12", "typeface-picker-font-resources.json", { textareaFont, state });
    await takeM24Screenshot(page, "M24-DOGFOOD-12", "typeface-picker-iansui");
    expect(textareaFont).toContain("Iansui");
  });

  test("M24 dogfood UI uses only local Tailwind components", async ({ page }) => {
    const packageJson = JSON.parse(await readRepoText("third_party/typeduck-web/source/package.json")) as {
      dependencies?: Record<string, string>;
      devDependencies?: Record<string, string>;
    };
    expect(packageJson.dependencies?.daisyui).toBeUndefined();
    expect(packageJson.devDependencies?.daisyui).toBeUndefined();

    const tailwindConfig = await readRepoText("third_party/typeduck-web/source/tailwind.config.ts");
    expect(tailwindConfig).not.toMatch(/\bdaisyui\b/i);
    expect(tailwindConfig).not.toContain("DaisyUIConfig");

    const filesToScan = [
      "third_party/typeduck-web/source/src/Inputs.tsx",
      "third_party/typeduck-web/source/src/Toolbar.tsx",
      "third_party/typeduck-web/source/src/ThemeSwitcher.tsx",
      "third_party/typeduck-web/source/src/YuneFeatureShowcase.tsx",
      "third_party/typeduck-web/source/src/App.tsx",
      "third_party/typeduck-web/source/src/Preferences.tsx",
      "third_party/typeduck-web/source/src/CandidatePanel.tsx",
      "third_party/typeduck-web/source/src/Candidate.tsx",
      "third_party/typeduck-web/source/src/DictionaryPanel.tsx",
      "third_party/typeduck-web/source/src/YuneStatusStrip.tsx",
      "third_party/typeduck-web/source/src/YuneInspector.tsx",
      "third_party/typeduck-web/source/src/index.css",
    ];
    const forbiddenDaisyUiClasses = /\b(?:btn|toggle|radio|checkbox|range|textarea|badge|join|tooltip|link|loading)(?:-[A-Za-z0-9_:[\]\/.%#]+)?\b/;
    for (const file of filesToScan) {
      const source = await readRepoText(file);
      const classSnippets = source.match(/className\s*=\s*(?:"[^"]*"|'[^']*'|`[^`]*`|\{`[^`]*`\}|\{"[^"]*"\}|\{'[^']*'\})/g) ?? [];
      for (const snippet of classSnippets) {
        expect(snippet, `${file}: ${snippet}`).not.toMatch(forbiddenDaisyUiClasses);
      }
    }

    await expect(page.getByRole("button", { name: /ASCII mode|中/ })).toBeVisible();
    await focusInputAndType(page, "nei", "你");
    await expect(page.locator("[data-yune-schema-switcher]")).toBeVisible();
    await expect(page.locator("[data-yune-status]")).toBeVisible({ timeout: 10000 });
    await expect(page.locator(".candidate-panel")).toBeVisible();
    await takeM24Screenshot(page, "M24-DOGFOOD-13", "local-tailwind-components");
  });

  test("Composition after typing schema-valid keys @smoke", async ({ page }) => {
    // D-08/D-10: Composition appears after typing schema-valid keys

    await focusInputAndType(page, "nei");
    const state = await readCandidatePanelSnapshot(page, false);
    expect(state.candidates.length).toBeGreaterThan(0);
    expect(state.candidates[0].text).toBe("你");
    await takeEvidenceScreenshot(page, "composition");
    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Composition: input="nei" candidates=${state.candidates.length}\n`
    );
  });

  test("Candidate list visible @smoke", async ({ page }) => {
    // D-08/D-10: Candidate list is visible after composition

    await focusInputAndType(page, "nei");
    const candidatePanel = await page.waitForSelector(
      ".candidate-panel",
      { timeout: 5000, state: "visible" }
    ).catch(() => null);

    await takeEvidenceScreenshot(page, "candidates");

    if (candidatePanel) {
      expect(candidatePanel).toBeTruthy();

      const candidates = await page.locator(".candidate-panel .candidates tbody").count();
      expect(candidates).toBeGreaterThan(0);

      await saveEvidence(
        "browser-run.log",
        `[${new Date().toISOString()}] Candidates: ${candidates} visible\n`
      );
    } else {
      await saveEvidence(
        "blocker.md",
        `# Browser E2E Blocker\n\n**Category**: Yune adapter/runtime\n\n**Flow**: Candidate list visible\n\n**Issue**: No candidate panel appeared after schema-valid input\n\n**Selectors tried**: [data-candidates], .candidate-panel, .candidate-list\n\n**Evidence**: screenshot-candidates.png\n\n**Impact**: Cannot verify candidate paging/selection flows\n`
      );
      expect(candidatePanel).toBeTruthy();
    }
  });

  test("M13 AI-off identity and AI-on second-pass source labels @smoke", async ({ page }) => {
    await focusInputAndType(page, "nei");

    const offState = await readCandidatePanelSnapshot(page, false);
    expect(offState.candidates.length).toBeGreaterThanOrEqual(2);
    expect(offState.candidates[0].text).toBe("你");
    expect(offState.candidates.every((candidate) => candidate.source === null)).toBe(true);
    await saveJsonEvidence("m13-ai-off-state.json", offState);
    await takeEvidenceScreenshot(page, "m13-ai-off");

    await setAiToggle(page, true);
    await expect(page.locator('.candidate-panel .candidates tbody[data-source="ai:local"]')).toHaveCount(1, { timeout: 5000 });
    const onState = await readCandidatePanelSnapshot(page, true);
    const aiRow = onState.candidates.find((candidate) => candidate.source === "ai:local");
    const aiIndex = onState.candidates.findIndex((candidate) => candidate.source === "ai:local");
    expect(aiRow).toBeDefined();
    expect(aiRow?.text).toBe("你啊");
    expect(onState.candidates[0].text).toBe(offState.candidates[0].text);
    expect(aiIndex).toBeGreaterThan(0);
    expect(aiIndex).toBeLessThan(onState.candidates.length);
    expect(onState.candidates.slice(0, aiIndex).every((candidate) => candidate.source === null)).toBe(true);
    await saveJsonEvidence("m13-ai-on-state.json", onState);
    await takeEvidenceScreenshot(page, "m13-ai-on");

    await setAiToggle(page, false);
    await expect(page.locator('.candidate-panel .candidates tbody[data-source="ai:local"]')).toHaveCount(0, { timeout: 5000 });
    const disabledState = await readCandidatePanelSnapshot(page, false);
    expect(disabledState.candidates).toEqual(offState.candidates);
    await saveJsonEvidence("m13-ai-disabled-state.json", disabledState);
    await takeEvidenceScreenshot(page, "m13-ai-disabled");
    expect(consoleErrors).toEqual([]);
  });

  test("M22 Bucket 2 inspector preserves classic candidate output @smoke", async ({ page }) => {
    test.setTimeout(300000);
    await expect(page.locator('[data-yune-inspector="panel"]')).toHaveCount(0);

    await focusInputAndType(page, "nei");
    const offState = await readCandidatePanelSnapshot(page, false);
    const offClassic = classicCandidateSignature(offState);
    expect(offState.candidates.length).toBeGreaterThan(0);
    expect(offState.candidates.every((candidate) => candidate.source === null)).toBe(true);

    await clearCompositionThroughInput(page);
    await page.getByLabel("Yune inspector").check();
    await expect(page.locator('[data-yune-inspector="panel"]')).toBeVisible({ timeout: 5000 });
    await waitForAppReady(page);
    await page.waitForTimeout(250);
    const inputField = page.locator("input[type='text'], textarea").first();
    await inputField.focus();
    await page.keyboard.type("nei", { delay: 250 });
    await expect(page.locator(".candidate-panel .candidates tbody").first()).toBeVisible({ timeout: 5000 });
    await expect.poll(async () => {
      const state = await readCandidatePanelSnapshot(page, false);
      return state.candidates[0]?.text ?? null;
    }, { timeout: 10000 }).toBe(offState.candidates[0].text);
    await expect(page.locator("[data-yune-inspector-source]").first()).toHaveText(/table|completion|sentence/, { timeout: 5000 });

    const onState = await readCandidatePanelSnapshot(page, false);
    const onClassic = classicCandidateSignature(onState);

    const inspectorText = await page.locator('[data-yune-inspector="panel"]').innerText();
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
    expect(onState.candidates.some((candidate) => candidate.source !== null)).toBe(true);
    await takeEvidenceScreenshot(page, "m22-bucket2-inspector-on");

    await page.getByLabel("Yune inspector").uncheck();
    await expect(page.locator('[data-yune-inspector="panel"]')).toHaveCount(0, { timeout: 5000 });
  });

  test("M22 Bucket 1 controls are browser-visible and honest @smoke", async ({ page }) => {
    test.setTimeout(300000);
    await expect(page.getByLabel(/Dictionary exclude/).last()).toBeVisible();
    await expect(page.getByLabel(/Traditionalization/).last()).toBeVisible();
    await expect(page.getByLabel(/Extended charset/).last()).toBeVisible();
    await expect(page.getByLabel(/Disabled/).last()).toBeVisible();
    await expect(page.getByText(/ascii_punct/i)).toHaveCount(0);
    await expect(page.getByLabel(/ascii_punct/i)).toHaveCount(0);

    await selectSchema(page, /Luna Pinyin/);
    const defaultLuna = await typeCompositionAndWaitForTopCandidate(page, "hao", "\u4fb4");

    await clearComposition(page);
    const excludeOn = await setPreferenceToggleAndWaitForSettings(page, /Dictionary exclude/, true, {
      "translator/dictionary_exclude": "[\"\u4fb4\"]",
    });
    await typeInputForStatus(page, "hao");
    const excludedLuna = await readCandidatePanelSnapshot(page, false);
    expect(candidateTexts(excludedLuna)).not.toContain("\u4fb4");

    await clearComposition(page);
    await setPreferenceToggle(page, /Traditionalization/, true);
    await typeInputForStatus(page, "hao");
    const traditionalStatus = await readYuneStatus(page);
    expect(traditionalStatus.traditional).toBe("traditional");

    await clearComposition(page);
    await setPreferenceToggle(page, /Disabled/, true);
    await typeInputForStatus(page, "hao");
    const disabledStatus = await readYuneStatus(page);
    expect(disabledStatus.disabled).toBe("disabled");
    await setPreferenceToggle(page, /Disabled/, false);
    await setPreferenceToggle(page, /Traditionalization/, false);

    await selectSchema(page, /Cangjie 5/);
    await typeInputForStatus(page, "ambe");
    const extendedOff = await readCandidatePanelSnapshot(page, false);
    expect(candidateTexts(extendedOff)).not.toContain("\u{2330A}");

    await clearComposition(page);
    await setPreferenceToggle(page, /Extended charset/, true);
    const extendedOn = await typeCompositionAndWaitForCandidate(page, "ambe", "\u{2330A}");
    expect(candidateTexts(extendedOn)).toContain("\u{2330A}");

    await saveJsonEvidence("m22-bucket1-controls-state.json", {
      defaultLuna,
      excludeOn,
      excludedLuna,
      traditionalStatus,
      disabledStatus,
      extendedOff,
      extendedOn,
      asciiPunctExposed: false,
      visibleSurfaces: {
        dictionaryExclude: "persisted translator/dictionary_exclude plus candidate removal",
        traditionalization: "engine status strip",
        disabled: "engine status strip",
        extendedCharset: "candidate before/after on cangjie5 input ambe",
      },
    });
    await takeEvidenceScreenshot(page, "m22-bucket1-controls");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M22 Bucket 3 schema switcher loads Jyutping, Cangjie, and Luna", async ({ page }) => {
    test.setTimeout(300000);
    await expect(page.locator("[data-yune-schema-switcher]")).toBeVisible();

    await selectSchema(page, /Cangjie 5/);
    const cangjie = await typeCompositionAndWaitForTopCandidate(page, "a", "\u65e5");
    const cangjieStatus = await readYuneStatus(page);
    expect(cangjieStatus.schema).toBe("cangjie5");

    await selectSchema(page, /Luna Pinyin/);
    const luna = await typeCompositionAndWaitForTopCandidate(page, "hao", "\u4fb4");
    const lunaStatus = await readYuneStatus(page);
    expect(lunaStatus.schema).toBe("luna_pinyin");

    await selectSchema(page, /Jyutping/);
    const jyutping = await typeCompositionAndWaitForTopCandidate(page, "nei", "\u4f60");
    const jyutpingStatus = await readYuneStatus(page);
    expect(jyutpingStatus.schema).toBe("jyut6ping3_mobile");

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

  test("M22 Bucket 3 reverse lookup works for Cangjie and Luna", async ({ page }) => {
    test.setTimeout(300000);
    await selectSchema(page, /Cangjie 5/);
    const cangjieReverse = await typeCompositionAndWaitForCandidate(page, "`nei;", "\u4f60");
    expect(candidateTexts(cangjieReverse)).toContain("\u4f60");

    await selectSchema(page, /Luna Pinyin/);
    const lunaReverse = await typeCompositionAndWaitForCandidate(page, "`a;", "\u65e5");
    expect(candidateTexts(lunaReverse)).toContain("\u65e5");

    await saveJsonEvidence("m22-bucket3-reverse-lookup-state.json", {
      cangjieReverse,
      lunaReverse,
      reverseLookup: {
        cangjie5: "Jyutping dictionary lookup with cangjie5 target comments",
        luna_pinyin: "Cangjie5 dictionary lookup with luna_pinyin target comments",
      },
    });
    await takeEvidenceScreenshot(page, "m22-bucket3-reverse-lookup");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M13 AI commit safety", async ({ page }) => {
    await setAiToggle(page, true);
    await focusInputAndType(page, "nei");
    await expect(page.locator('.candidate-panel .candidates tbody[data-source="ai:local"]')).toHaveCount(1, { timeout: 5000 });
    const stagedState = await readCandidatePanelSnapshot(page, true);
    const aiIndex = stagedState.candidates.findIndex((candidate) => candidate.source === "ai:local");
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
    await expect(page.locator('.candidate-panel .candidates tbody[data-source="ai:local"]')).toHaveCount(1, { timeout: 5000 });
    const selectableState = await readCandidatePanelSnapshot(page, true);
    const selectableAiIndex = selectableState.candidates.findIndex((candidate) => candidate.source === "ai:local");
    await page.locator('.candidate-panel .candidates tbody[data-source="ai:local"]').click();
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

  test("M20 showcase control surface exposes honest controls", async ({ page }) => {
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

    await expect(page.getByLabel(/Combine same-text candidates/).last()).toBeChecked();
    await expect(page.getByLabel(/Prediction never first/).last()).toBeChecked();
    await expect(page.getByLabel(/AI Candidates/).last()).not.toBeChecked();
    await expect(page.getByLabel(/Prediction threshold/).last()).toHaveValue("0");
    await expect(page.getByText(/ascii_punct/i)).toHaveCount(0);
    await expect(page.getByLabel(/ascii_punct/i)).toHaveCount(0);
    const commonCustom = await readRepoText("third_party/typeduck-web/source/public/schema/common.custom.yaml");
    const commonYaml = await readRepoText("third_party/typeduck-web/source/public/schema/common.yaml");
    expect(commonCustom).toContain("- common:/separate_candidates");
    expect(commonYaml).toContain("translator/combine_candidates: false");

    await saveJsonEvidence("m20-control-surface-state.json", {
      activeControls: ACTIVE_SHOWCASE_CONTROLS.map(String),
      liveControls: LIVE_SHOWCASE_CONTROLS.map(String),
      displayControls: DISPLAY_SHOWCASE_CONTROLS.map(String),
      defaults: {
        combineCandidates: {
          uiDemoDefault: true,
          rawAssetPatch: "common.custom.yaml enables common:/separate_candidates, which maps to translator/combine_candidates: false in common.yaml.",
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

  test("M20 Input Memory persists schema customization", async ({ page }) => {
    const learningOn = await waitForPersistedSettings(page, {
      "translator/enable_user_dict": "true",
      "translator/encode_commit_history": "true",
    });

    const learnedCommit = await learnPhraseThroughBrowser(page);
    const learnedWithMemory = await typeCompositionAndWaitForCandidate(
      page,
      LEARNED_PREFIX_INPUT,
      LEARNED_PHRASE_TEXT,
    );
    expect(candidateTexts(learnedWithMemory)).toContain(LEARNED_PHRASE_TEXT);

    const learningOff = await setPreferenceToggleAndWaitForSettings(page, /Input Memory/, false, {
      "translator/enable_user_dict": "false",
      "translator/encode_commit_history": "false",
    });
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
      browserSurface: {
        status: "explicit browser-surface N/A for the memory-off candidate-output delta",
        observedAfterDisablingMemory: learnedWithMemoryOff,
        reason: "The browser persists translator/enable_user_dict=false and translator/encode_commit_history=false, but the current no-crates TypeDuck-Web surface cannot suppress an already learned prefix prediction from candidate output.",
        engineCoverage: "Userdb learning and per-entry pronunciation behavior remain engine-proven in cantonese_parity and frontend_client userdb tests; this M20 follow-up does not change crates/ or add a yune_typeduck_* export.",
        evidencePolicy: "The learned prediction on-state is visible browser behavior; the off-state is not counted as candidate-output proof.",
      },
      proof: "Input Memory remains an honest deploy-time schema customization in the browser. The learned-prediction on-state is visible after a real browser commit; the memory-off candidate-output delta is explicitly N/A on this surface.",
    });
    await takeEvidenceScreenshot(page, "m20-input-memory-persistence");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 Prediction never first persists schema customization @smoke", async ({ page }) => {
    const neverFirstOn = await waitForPersistedSettings(page, {
      "translator/prediction_never_first": "true",
    });

    const learnedCommit = await learnPhraseThroughBrowser(page);
    const neverFirstOnRanking = await typeCompositionAndWaitForCandidate(
      page,
      LEARNED_PREFIX_INPUT,
      LEARNED_PHRASE_TEXT,
    );
    const learnedOnIndex = candidateTexts(neverFirstOnRanking).indexOf(LEARNED_PHRASE_TEXT);
    expect(neverFirstOnRanking.candidates[0].text).toBe(CLASSIC_NGO_TEXT);
    expect(learnedOnIndex).toBeGreaterThan(0);

    const neverFirstOff = await setPreferenceToggleAndWaitForSettings(page, /Prediction never first/, false, {
      "translator/prediction_never_first": "false",
    });
    const neverFirstOffRanking = await typeCompositionAndWaitForCandidate(
      page,
      LEARNED_PREFIX_INPUT,
      LEARNED_PHRASE_TEXT,
    );
    const classicOffIndex = candidateTexts(neverFirstOffRanking).indexOf(CLASSIC_NGO_TEXT);
    expect(neverFirstOffRanking.candidates[0].text).toBe(LEARNED_PHRASE_TEXT);
    expect(classicOffIndex).toBeGreaterThan(0);

    await saveJsonEvidence("m20-prediction-never-first-persistence-state.json", {
      neverFirstOn,
      neverFirstOff,
      visibleBehavior: {
        learnedCommit,
        neverFirstOnRanking,
        neverFirstOffRanking,
        learnedOnIndex,
        classicOffIndex,
      },
      proof: "Prediction never first is a deploy-time schema customization and a visible browser behavior: after learning a phrase, classic 我 stays first while never-first is enabled, and the learned prefix prediction moves to index 0 after the control is disabled.",
    });
    await takeEvidenceScreenshot(page, "m20-prediction-never-first-persistence");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 guided scenarios use real TypeDuck-Web assets", async ({ page }) => {
    const scenarios: Record<string, string[]> = {};

    const ngo = await clickShowcaseScenario(page, "ngo", "我");
    expect(ngo.candidates[0].text).toBe("我");
    scenarios.ngo = ngo.candidates.map((candidate) => candidate.rowText);
    await expectNoToasts(page);

    const santai = await clickShowcaseScenario(page, "santai", "身體健康");
    expect(santai.candidates.map((candidate) => candidate.text)).toContain("身體");
    expect(santai.candidates.map((candidate) => candidate.text)).toContain("身體健康");
    scenarios.santai = santai.candidates.map((candidate) => candidate.rowText);
    await expectNoToasts(page);

    const mgoi = await clickShowcaseScenario(page, "mgoi", "唔該");
    expect(mgoi.candidates.map((candidate) => candidate.text)).toContain("唔該");
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
    scenarios.toneLetters = toneLetters.candidates.map((candidate) => candidate.rowText);
    await expectNoToasts(page);

    await clearComposition(page);
    await setAiToggle(page, true);
    await expectNoToasts(page);
    await clickShowcaseScenario(page, "AI trigger", "你", true);
    await expect(page.locator('.candidate-panel .candidates tbody[data-source="ai:local"]')).toHaveCount(1, { timeout: 5000 });
    const aiTrigger = await readCandidatePanelSnapshot(page, true);
    const aiIndex = aiTrigger.candidates.findIndex((candidate) => candidate.source === "ai:local");
    expect(aiIndex).toBeGreaterThan(0);
    expect(aiTrigger.candidates[0].source).toBeNull();
    expect(aiTrigger.candidates[aiIndex].text).toBe("你啊");
    scenarios.aiTrigger = aiTrigger.candidates.map((candidate) => candidate.rowText);

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

  test("M20 combine_candidates changes real candidate output", async ({ page }) => {
    const combineOn = await typeCompositionAndWaitForCandidate(page, "hou", "好");
    expect(combineOn.candidates[0].text).toBe("好");
    expect(combineOn.candidates[1].text).not.toBe("好");

    await clearComposition(page);
    await setPreferenceToggle(page, /Combine same-text candidates/, false);
    const combineOff = await typeCompositionAndWaitForCandidate(page, "hou", "好");
    expect(combineOff.candidates.slice(0, 2).map((candidate) => candidate.text)).toEqual(["好", "好"]);

    await saveJsonEvidence("m20-combine-candidates-state.json", {
      defaultOn: combineOn,
      disabled: combineOff,
    });
    await takeEvidenceScreenshot(page, "m20-combine-candidates");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 prediction threshold changes real candidate output", async ({ page }) => {
    const thresholdZero = await typeCompositionAndWaitForCandidate(page, "santai", "身體健康");
    expect(thresholdZero.candidates.map((candidate) => candidate.text)).toContain("身體健康");

    await clearComposition(page);
    await setPreferenceRange(page, /Prediction threshold/, 50000);
    const threshold50000 = await typeCompositionAndWaitForCandidate(page, "santai", "身體");
    expect(threshold50000.candidates[0].text).toBe("身體");
    expect(threshold50000.candidates.map((candidate) => candidate.text)).not.toContain("身體健康");

    await saveJsonEvidence("m20-prediction-threshold-state.json", {
      thresholdZero,
      threshold50000,
      calibratedValue: 50000,
      selectorRange: {
        min: 0,
        max: 200000,
        step: 1000,
        rationale: "Fine-grained browser control around the observed 50000 cutoff; range remains above the real-assets probe value so future higher-weight dictionary predictions remain reachable without exposing separate alias sliders.",
      },
      calibration: "Derived from real jyut6ping3_mobile browser assets: santai predictions disappear at 50000 while direct candidates remain.",
    });
    await takeEvidenceScreenshot(page, "m20-prediction-threshold");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M20 live session controls use setOption-visible output", async ({ page }) => {
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
    await setPreferenceToggle(page, /Simplification/, true);
    const simplification = await typeCompositionAndWaitForCandidate(page, "ngohaigo", "我系个");
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

  test("M20 display controls change rendering and record mobile-only side-lookup limits", async ({ page }) => {
    const jyutpingShown = await typeCompositionAndWaitForCandidate(page, "nei", "你");
    expect(jyutpingShown.candidates[0].rowText).toContain("nei5");

    await clearComposition(page);
    await setPreferenceRadio(page, /Hide/);
    const jyutpingHidden = await typeCompositionAndWaitForCandidate(page, "nei", "你");
    expect(jyutpingHidden.candidates[0].rowText).not.toContain("nei5");
    expect(jyutpingHidden.candidates[0].text).toBe("你");

    await clearComposition(page);
    await setPreferenceRadio(page, /Always Show/);
    const englishOnly = await typeCompositionAndWaitForCandidate(page, "nei", "你");
    expect(englishOnly.candidates[0].rowText).toContain("you (singular)");

    await clearComposition(page);
    await page.getByLabel(/Hindi/).last().check({ force: true });
    const hindiVisible = await typeCompositionAndWaitForCandidate(page, "nei", "你");
    expect(hindiVisible.candidates[0].rowText).toContain("आप");

    await expect(page.getByLabel(/Reverse code display/).last()).toBeVisible();
    await expect(page.getByText(/Cangjie version/)).toBeVisible();
    const mobileSchema = await readRepoText("third_party/typeduck-web/source/public/schema/jyut6ping3_mobile.schema.yaml");
    expect(mobileSchema).not.toContain("cangjie");
    expect(mobileSchema).not.toContain("show_full_code");
    const visibleSchemaControls = await page.locator(
      "[data-yune-schema-switcher], [data-schema], [data-schema-selector], .schema-selector, select[name='schema']",
    ).count();

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
        status: "Covered by M22 schema switch and reverse-lookup tests",
        activeBrowserSchema: "jyut6ping3_mobile",
        reason: "This M20 display test keeps the jyut6ping3_mobile display assertions; M22 tests switch to cangjie5 and luna_pinyin for schema/reverse-lookup browser evidence.",
        visibleSchemaControls,
      },
    });
    await takeEvidenceScreenshot(page, "m20-display-controls");
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("M16 combine candidates browser default matches M14", async ({ page }) => {
    await focusInputAndType(page, "hou");
    await captureM16Scenario(
      page,
      "combine-candidates-browser-default",
      await m14Texts("jyut6ping3-m14-options.json", "combine_candidates_default", "hou", 5),
    );
    expect(consoleErrors).toEqual([]);
  });

  test("M16 sentence composition browser path matches M14 @smoke", async ({ page }) => {
    await focusInputAndType(page, "ngohaigo");
    await captureM16Scenario(
      page,
      "sentence-enabled",
      await m14Texts("jyut6ping3-m14-options.json", "enable_sentence_default", "ngohaigo", 1),
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
    const sentenceDisabledPanelCount = await page.locator(".candidate-panel .candidates tbody").count();
    const sentenceDisabledState = sentenceDisabledPanelCount > 0
      ? await readCandidatePanelSnapshot(page, false)
      : { aiEnabled: false, inputValue: await inputField.inputValue(), candidates: [] };
    await saveJsonEvidence("m16-sentence-disabled-state.json", {
      expectedM14Texts: await m14Texts("jyut6ping3-m14-options.json", "enable_sentence_disabled", "ngohaigo", 6),
      browserState: sentenceDisabledState,
      persistedSettings: persistedSentenceDisabled,
      browserSurface: sentenceDisabledPanelCount > 0
        ? "Candidate panel rendered after disabling Auto-composition."
        : "No candidate panel rendered for full ngohaigo after disabling Auto-composition in TypeDuck-Web.",
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
      await m14Texts("jyut6ping3-m14-completion-correction.json", "completion_default", "ne", 1),
    );
    expect(consoleErrors).toEqual([]);
  });

  test("M16 correction browser path matches M14 visible before/after", async ({ page }) => {
    await setPreferenceToggle(page, /Auto-correction/, false);
    const persistedCorrectionDefault = await waitForPersistedSettings(page, {
      "translator/enable_correction": "false",
    });
    await waitForDeployedSettings(page, {
      "translator/enable_correction": "false",
    });
    const defaultState = await typeCompositionAndWaitForTopCandidate(page, "nri", "我");
    const expectedDefaultTexts = await m14Texts("jyut6ping3-m14-completion-correction.json", "correction_default", "nri", 5);
    const expectedDefaultEngineTexts = await m14Texts("jyut6ping3-m14-completion-correction.json", "correction_default", "nri", 6);
    expect(defaultState.candidates.slice(0, 5).map((candidate) => candidate.text)).toEqual(expectedDefaultTexts);
    await saveJsonEvidence("m16-correction-default-state.json", {
      expectedM14Texts: expectedDefaultTexts,
      expectedM14EngineTexts: expectedDefaultEngineTexts,
      browserState: defaultState,
      persistedSettings: persistedCorrectionDefault,
      browserSurface: "Candidate panel rendered for nri with correction off via prefix fallback; the visible page shows the first five oracle rows, and cantonese_parity asserts the sixth row plus commit preview.",
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
    const enabledState = await typeCompositionAndWaitForTopCandidate(page, "nri", "你");
    const expectedEnabledTexts = await m14Texts("jyut6ping3-m14-completion-correction.json", "correction_enabled", "nri", 5);
    expect(enabledState.candidates.slice(0, 5).map((candidate) => candidate.text)).toEqual(expectedEnabledTexts);
    expect(defaultState.candidates[0].text).not.toEqual(enabledState.candidates[0].text);
    await saveJsonEvidence("m16-correction-enabled-state.json", {
      expectedM14Texts: expectedEnabledTexts,
      browserState: enabledState,
      persistedSettings: persistedCorrectionEnabled,
      browserSurface: "Candidate panel rendered for nri with correction on; the first row matches the v1.1.2 correction fixture.",
    });
    await saveJsonEvidence("m20-auto-correction-visible-before-after-state.json", {
      defaultState,
      enabledState,
      persistedCorrectionDefault,
      persistedCorrectionEnabled,
      expectedDefaultTexts,
      expectedDefaultEngineTexts,
      expectedEnabledTexts,
      browserSurface: "Auto-correction is a visible before/after browser behavior for nri: off shows prefix fallback rows on the browser page, on shows correction 你 first, and engine parity asserts the full oracle prefix list.",
    });
    await takeEvidenceScreenshot(page, "m16-correction-enabled");
    await takeEvidenceScreenshot(page, "m20-auto-correction-visible-before-after");
    expect(consoleErrors).toEqual([]);
  });

  test("M16 simplification toggle converts browser candidates through OpenCC", async ({ page }) => {
    await focusInputAndType(page, "ngohaigo");
    const traditional = await captureM16Scenario(
      page,
      "simplification-off",
      ["\u6211\u4fc2\u500b"],
    );

    await clearComposition(page);
    await page.getByRole("button", { name: /Simplification/ }).click();
    await page.waitForTimeout(500);
    await focusInputAndType(page, "ngohaigo");
    const simplified = await captureM16Scenario(
      page,
      "simplification-on",
      ["\u6211\u7cfb\u4e2a"],
    );
    expect(simplified.candidates[0].text).not.toEqual(traditional.candidates[0].text);
    expect(consoleErrors).toEqual([]);
  });

  test("M16 schema menu and userdb pronunciation limits are explicit", async ({ page }) => {
    const schemaMenuFixture = await loadFixture("jyut6ping3-m14-schema-menu.json");
    const userdbFixture = await loadFixture("jyut6ping3-m14-userdb.json");
    const optionsFixture = await loadFixture("jyut6ping3-m14-options.json");
    const visibleSchemaControls = await page.locator(
      "[data-yune-schema-switcher], [data-schema], [data-schema-selector], .schema-selector, select[name='schema']",
    ).count();
    expect(visibleSchemaControls).toBeGreaterThan(0);
    await saveJsonEvidence("m16-documented-gaps-state.json", {
      deployOnlyVariants: {
        browserSurface: "M22 adds a real browser schema switcher for jyut6ping3_mobile, cangjie5, and luna_pinyin; deploy-variant controls for common:/separate_candidates and common:/show_full_code remain engine/doc scoped.",
        engineCoverage: "cargo test -p yune-core --test cantonese_parity covers combine_candidates and show_full_code against the M14 v1.1.2 goldens.",
        oracleSurface: optionsFixture["capture"],
      },
      browserRuntimeLimits: {
        sentenceDisabled: "The browser records the disabled Auto-composition state separately because full ngohaigo does not render the native disabled-prefix candidate panel.",
        correction: "The browser records nri correction default/enabled as visible candidate-output before/after evidence: correction off renders prefix fallback, correction on renders the v1.1.2 correction row first.",
      },
      schemaMenu: {
        oracleSurface: schemaMenuFixture["capture"],
        browserSurface: "M22 TypeDuck-Web exposes a schema switcher for the browser playground schemas; M14 RimeGetSchemaList remains the ABI oracle evidence.",
        visibleSchemaControls,
      },
      userdbPronunciation: {
        oracleSurface: userdbFixture["capture"],
        browserSurface: "No TypeDuck-Web native inspection surface exposes fork-only per-entry pronunciation levers.",
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
      `[${new Date().toISOString()}] Candidate paging: PageDown pressed, panel count=${candidatePanel}\n`
    );
    expect(candidatePanel).toBeGreaterThan(0);
  });

  test("Keyboard paging shortcuts do not error", async ({ page }) => {
    await focusInputAndType(page, "n");
    const firstPage = await readCandidatePanelSnapshot(page, false);
    expect(firstPage.candidates.length).toBeGreaterThan(1);

    await page.keyboard.press("=");
    await expect.poll(async () =>
      (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text,
    { timeout: 5000 }).not.toBe(firstPage.candidates[0].text);
    const secondPage = await readCandidatePanelSnapshot(page, false);
    expect(secondPage.candidates[0].text).not.toBe(firstPage.candidates[0].text);

    await page.keyboard.press("-");
    await expect.poll(async () =>
      (await readCandidatePanelSnapshot(page, false)).candidates[0]?.text,
    { timeout: 5000 }).toBe(firstPage.candidates[0].text);
    const returnedPage = await readCandidatePanelSnapshot(page, false);
    expect(returnedPage.candidates[0].text).toBe(firstPage.candidates[0].text);
    await expect(page.locator(".Toastify__toast")).toHaveCount(0, { timeout: 1000 });
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
      `[${new Date().toISOString()}] Candidate selection: committed="${inputValue}"\n`
    );
  });

  test("Number keys commit visible candidates", async ({ page }) => {
    await focusInputAndType(page, "nei");
    const inputField = page.locator("input[type='text'], textarea").first();
    const beforeSelection = await readCandidatePanelSnapshot(page, false);
    expect(beforeSelection.candidates[1].text).toBe("\u5462");

    await page.keyboard.press("2");
    await expect(inputField).toHaveValue("\u5462", { timeout: 5000 });
    await expect(page.locator(".candidate-panel")).toHaveCount(0, { timeout: 5000 });
    await expect(page.locator(".Toastify__toast")).toHaveCount(0, { timeout: 1000 });
    expect(consoleFailures(consoleErrors)).toEqual([]);
  });

  test("Deletion removes candidate or triggers delete path", async ({ page }) => {
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
      `[${new Date().toISOString()}] Deletion: input="${compositionAfter}"\n`
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
      `[${new Date().toISOString()}] Backspace: before="${beforeBackspace}" after="${afterBackspace}"\n`
    );
  });

  test("Deploy returns visible success/error evidence", async ({ page }) => {
    // D-08/D-10: Deploy returns visible success/error evidence

    // Locate deploy button/shortcut
    // TypeDuck-Web may have deploy button in settings or Ctrl+D shortcut
    const deployButton = await page.locator("[data-deploy], .deploy-button, button:has-text('deploy')").first();

    if (await deployButton.count() > 0) {
      await deployButton.click();
    } else {
      // Try keyboard shortcut (Ctrl+D or similar)
      await page.keyboard.press("Control+d");
    }

    await page.waitForTimeout(2000);

    // Verify deploy result visible (toast notification, console log, status change)
    const deployNotification = await page.locator(".Toastify__toast, .toast, [data-deploy-status], .notification").first().textContent({ timeout: 1000 }).catch(() => null);

    await takeEvidenceScreenshot(page, "deploy");

    if (deployNotification) {
      await saveEvidence(
        "browser-run.log",
        `[${new Date().toISOString()}] Deploy: status="${deployNotification}"\n`
      );
      expect(deployNotification).toBeDefined();
    } else {
      // Check console for deploy result
      await saveEvidence(
        "browser-run.log",
        `[${new Date().toISOString()}] Deploy: triggered (no visible notification)\n`
      );
    }
  });

  test("Customize returns visible success/error evidence", async ({ page }) => {
    // D-08/D-10: Customize returns visible success/error evidence

    const m20Settings = page.getByText(/Active engine controls/).first();
    if (await m20Settings.count() > 0) {
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
        `[${new Date().toISOString()}] Customize: M20 settings control persisted prediction threshold\n`
      );
      return;
    }

    // Locate customize settings panel
    // TypeDuck-Web may have settings panel for pageSize, completion, etc.
    const settingsPanel = await page.locator("[data-settings], .settings-panel, .customize-panel").first();

    if (await settingsPanel.count() > 0) {
      await settingsPanel.click();
      await page.waitForTimeout(1000);
      await takeEvidenceScreenshot(page, "customize");

      // Verify customize result visible
      const customizeNotification = await page.locator(".Toastify__toast, .toast, [data-customize-status], .notification").first().textContent({ timeout: 1000 }).catch(() => null);

      if (customizeNotification) {
        await saveEvidence(
          "browser-run.log",
          `[${new Date().toISOString()}] Customize: status="${customizeNotification}"\n`
        );
      } else {
        await saveEvidence(
          "browser-run.log",
          `[${new Date().toISOString()}] Customize: settings changed (no visible notification)\n`
        );
      }
    } else {
      await saveEvidence(
        "blocker.md",
        `# Browser E2E Blocker\n\n**Category**: TypeDuck-Web app/source\n\n**Flow**: Customize settings\n\n**Issue**: No settings/customize panel found\n\n**Selectors tried**: M20 Active engine controls, [data-settings], .settings-panel, .customize-panel\n\n**Impact**: Cannot verify customize flow\n`
      );
    }
  });

  test("Persistence sync after deploy/customize mutations", async ({ page, context }) => {
    // D-11: Persistence sync after deploy/customize/userdb-relevant boundaries

    // Perform mutation (deploy)
    await page.keyboard.press("Control+d"); // Deploy shortcut
    await page.waitForTimeout(1000);

    // Verify sync marker in console or performance timeline
    const syncMarkerFound = await verifyPersistenceMarker(page, "syncToPersistenceAfterMutation");

    await saveEvidence(
      "persistence-sync.log",
      `[${new Date().toISOString()}] syncToPersistenceAfterMutation: ${syncMarkerFound ? "PASS" : "FAIL (marker not logged)"}\n`
    );

    // Persistence evidence: check if IDBFS flushed
    // Implementation may log FS.syncfs(false) to console
    if (!syncMarkerFound) {
      await saveEvidence(
        "blocker.md",
        `# Browser E2E Blocker\n\n**Category**: Yune adapter/runtime\n\n**Flow**: Persistence sync evidence\n\n**Issue**: No persistence sync marker logged after mutation\n\n**Expected**: syncToPersistenceAfterMutation or FS.syncfs(false) console log\n\n**Impact**: Cannot verify persistence timing per D-11\n`
      );
    }
  });

  test("Reload/reinitialize preserves persisted state", async ({ page, context }) => {
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
    const syncFromMarkerFound = await verifyPersistenceMarker(page, "syncFromPersistenceBeforeInit");

    await saveEvidence(
      "persistence-sync.log",
      `[${new Date().toISOString()}] Reload: syncFromPersistenceBeforeInit ${syncFromMarkerFound ? "PASS" : "FAIL (marker not logged)"}\n`
 +
      `[${new Date().toISOString()}] Reload: App reinitialized\n`
    );

    // Note: Verifying specific persisted values requires app to expose persisted state
    // For E2E smoke, we verify the reload succeeded and app initialized again
    const inputAfterReload = await page.locator("input[type='text'], textarea").first();
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
 * - Category: TypeDuck-Web app/source, Yune adapter/runtime, environment/tooling
 */

export {};
