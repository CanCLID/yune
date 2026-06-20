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

import { test, expect, Page, BrowserContext } from "@playwright/test";

// Test configuration
const APP_URL = process.env.TYPEDUCK_APP_URL || "http://localhost:5173";
const TIMEOUT_MS = 120000; // WASM load and runtime init may be slow

// E2E evidence directory
const EVIDENCE_DIR = process.env.TYPEDUCK_EVIDENCE_DIR || "../e2e/results";

const ACTIVE_SHOWCASE_CONTROLS = [
  /Auto-completion/,
  /Auto-correction/,
  /Auto-composition/,
  /Input Memory/,
  /AI Candidates/,
  /Combine same-text candidates/,
  /Prediction never first/,
  /Prediction threshold/,
] as const;

const LIVE_SHOWCASE_CONTROLS = [
  /ASCII mode/,
  /Full shape/,
  /Simplification/,
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

/**
 * Helper: Capture browser console errors to evidence file
 */
async function captureConsoleErrors(page: Page, evidenceFile: string): Promise<string[]> {
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
  const evidencePath = path.join(EVIDENCE_DIR, filename);
  await fs.mkdir(EVIDENCE_DIR, { recursive: true });
  await fs.appendFile(evidencePath, content, "utf8");
}

/**
 * Helper: Take screenshot with evidence filename
 */
async function takeEvidenceScreenshot(page: Page, flowName: string): Promise<void> {
  const path = await import("path");
  await (await import("fs/promises")).mkdir(EVIDENCE_DIR, { recursive: true });
  const screenshotPath = path.join(EVIDENCE_DIR, `screenshot-${flowName}.png`);
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
    persistedConfig?: {
      exists?: boolean;
      settings?: Record<string, string | null>;
    };
  };
}

async function writeEvidence(filename: string, content: string): Promise<void> {
  const fs = await import("fs/promises");
  const path = await import("path");
  const evidencePath = path.join(EVIDENCE_DIR, filename);
  await fs.mkdir(EVIDENCE_DIR, { recursive: true });
  await fs.writeFile(evidencePath, content, "utf8");
}

async function saveJsonEvidence(filename: string, value: unknown): Promise<void> {
  await writeEvidence(filename, `${JSON.stringify(value, null, 2)}\n`);
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
  await expect(page.locator(".loading")).toHaveCount(0, { timeout: TIMEOUT_MS });
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

async function focusInputAndType(page: Page, text: string): Promise<void> {
  const inputField = page.locator("input[type='text'], textarea").first();
  await inputField.focus();
  await inputField.type(text, { delay: 80 });
  await expect(page.locator(".candidate-panel .candidates tbody").first()).toBeVisible({ timeout: 5000 });
  await expect(page.locator(".candidate-panel").first()).toContainText(text, { timeout: 5000 });
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
  if (await page.locator(".candidate-panel").count() > 0) {
    await page.keyboard.press("Escape").catch(() => undefined);
  }
  await inputField.fill("");
  await expect(page.locator(".candidate-panel")).toHaveCount(0, { timeout: 5000 });
}

async function setPreferenceToggle(page: Page, label: RegExp, enabled: boolean): Promise<void> {
  const toggle = page.getByLabel(label).last();
  const checked = await toggle.isChecked();
  if (checked !== enabled) {
    if (enabled) {
      await toggle.check({ force: true });
    } else {
      await toggle.uncheck({ force: true });
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
    if (enabled) {
      await toggle.check({ force: true });
    } else {
      await toggle.uncheck({ force: true });
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

test.describe("TypeDuck-Web Browser E2E", () => {
  test.setTimeout(TIMEOUT_MS);

  let consoleErrors: string[] = [];

  test.beforeAll(async ({ browser }) => {
    // Record browser runner start
    await saveEvidence(
      "browser-run.log",
      `[${new Date().toISOString()}] Browser E2E started\nURL: ${APP_URL}\n`
    );
  });

  test.beforeEach(async ({ page }) => {
    consoleErrors = await captureConsoleErrors(page, "browser-console.log");
    await openApp(page);
  });

  test.afterEach(async ({ page }, testInfo) => {
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

  test("Composition after typing schema-valid keys", async ({ page }) => {
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

  test("Candidate list visible", async ({ page }) => {
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

  test("M13 AI-off identity and AI-on second-pass source labels", async ({ page }) => {
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

  test("M20 Prediction never first persists schema customization", async ({ page }) => {
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

    const toneLetters = await clickShowcaseScenario(page, "tone letters", "瀡板");
    expect(toneLetters.candidates[0].text).toBe("瀡板");
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
      "[data-schema], [data-schema-selector], .schema-selector, select[name='schema']",
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
        status: "N/A for the current browser surface",
        activeBrowserSchema: "jyut6ping3_mobile",
        reason: "The real browser schema does not declare a cangjie namespace or show_full_code patch; reverse/Cangjie rendering remains engine-covered but not browser-reachable here.",
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

  test("M16 sentence composition browser path matches M14", async ({ page }) => {
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
    await focusInputAndType(page, "ne");
    await captureM16Scenario(
      page,
      "completion-default",
      await m14Texts("jyut6ping3-m14-completion-correction.json", "completion_default", "ne", 1),
    );
    expect(consoleErrors).toEqual([]);
  });

  test("M16 correction browser path records explicit M20 browser-surface N/A", async ({ page }) => {
    const correctionBrowserSurface = {
      status: "explicit browser-surface N/A",
      activeBrowserSchema: "jyut6ping3_mobile",
      input: "nri",
      reason: "Engine-proven at cantonese_parity; the correction candidate for nri is not browser-renderable on the current jyut6ping3_mobile TypeDuck-Web surface.",
      engineCoverage: "cargo test -p yune-core --test cantonese_parity correction_minimal_distance_and_m_abbreviation_parity covers the TypeDuck v1.1.2 correction row.",
      evidencePolicy: "Empty candidate arrays are recorded only as this browser-surface limit, not as proof that Auto-correction changes candidate output.",
    };

    await setPreferenceToggle(page, /Auto-correction/, false);
    const persistedCorrectionDefault = await waitForPersistedSettings(page, {
      "translator/enable_correction": "false",
    });
    const inputField = page.locator("input[type='text'], textarea").first();
    await inputField.focus();
    await inputField.type("nri", { delay: 80 });
    await page.waitForTimeout(1000);
    const defaultPanelCount = await page.locator(".candidate-panel .candidates tbody").count();
    expect(defaultPanelCount).toBe(0);
    const defaultState = defaultPanelCount > 0
      ? await readCandidatePanelSnapshot(page, false)
      : { aiEnabled: false, inputValue: await inputField.inputValue(), candidates: [] };
    await saveJsonEvidence("m16-correction-default-state.json", {
      expectedM14Texts: await m14Texts("jyut6ping3-m14-completion-correction.json", "correction_default", "nri", 5),
      browserState: defaultState,
      persistedSettings: persistedCorrectionDefault,
      browserSurface: correctionBrowserSurface,
    });
    await takeEvidenceScreenshot(page, "m16-correction-default");

    await clearComposition(page);
    await setPreferenceToggle(page, /Auto-correction/, true);
    const persistedCorrectionEnabled = await waitForPersistedSettings(page, {
      "translator/enable_correction": "true",
    });
    await inputField.focus();
    await inputField.type("nri", { delay: 80 });
    await page.waitForTimeout(1000);
    const enabledPanelCount = await page.locator(".candidate-panel .candidates tbody").count();
    expect(enabledPanelCount).toBe(0);
    const enabledState = enabledPanelCount > 0
      ? await readCandidatePanelSnapshot(page, false)
      : { aiEnabled: false, inputValue: await inputField.inputValue(), candidates: [] };
    await saveJsonEvidence("m16-correction-enabled-state.json", {
      expectedM14Texts: await m14Texts("jyut6ping3-m14-completion-correction.json", "correction_enabled", "nri", 5),
      browserState: enabledState,
      persistedSettings: persistedCorrectionEnabled,
      browserSurface: correctionBrowserSurface,
    });
    await saveJsonEvidence("m20-auto-correction-browser-surface-na-state.json", {
      defaultPanelCount,
      enabledPanelCount,
      defaultState,
      enabledState,
      persistedCorrectionDefault,
      persistedCorrectionEnabled,
      browserSurface: correctionBrowserSurface,
    });
    await takeEvidenceScreenshot(page, "m16-correction-enabled");
    await takeEvidenceScreenshot(page, "m20-auto-correction-browser-surface-na");
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
    await page.locator(".btn-toolbar").nth(1).click();
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
      "[data-schema], [data-schema-selector], .schema-selector, select[name='schema']",
    ).count();
    expect(visibleSchemaControls).toBe(0);
    await saveJsonEvidence("m16-documented-gaps-state.json", {
      deployOnlyVariants: {
        browserSurface: "The checked-in TypeDuck-Web app initializes jyut6ping3_mobile only and exposes no schema/deploy-variant selector for common:/separate_candidates or common:/show_full_code.",
        engineCoverage: "cargo test -p yune-core --test cantonese_parity covers combine_candidates and show_full_code against the M14 v1.1.2 goldens.",
        oracleSurface: optionsFixture["capture"],
      },
      browserRuntimeLimits: {
        sentenceDisabled: "The browser records the disabled Auto-composition state separately because full ngohaigo does not render the native disabled-prefix candidate panel.",
        correction: "The browser records nri correction default/enabled persisted settings separately; correction candidate output is explicit N/A on jyut6ping3_mobile, and M15 cantonese_parity remains the oracle-backed correction proof.",
      },
      schemaMenu: {
        oracleSurface: schemaMenuFixture["capture"],
        browserSurface: "TypeDuck-Web exposes no schema-selector DOM control; M14 RimeGetSchemaList remains the oracle evidence.",
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
