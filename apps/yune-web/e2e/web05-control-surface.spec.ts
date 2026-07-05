import { expect, test, type Page } from "@playwright/test";
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const APP_URL = process.env.YUNE_WEB_APP_URL || "http://localhost:5173";
const PUBLIC_APP_URL = process.env.YUNE_PUBLIC_DEMO_APP_URL || APP_URL;
const TIMEOUT_MS = 300000;
const repoRoot = path.resolve(process.cwd(), "../../..");
const evidenceRoot = path.join(
  repoRoot,
  "docs",
  "reports",
  "evidence",
  "web05-control-surface",
);
const baselinePath = path.join(evidenceRoot, "default-behavior-baseline.json");
const postPath = path.join(evidenceRoot, "default-behavior-post.json");
const controlsEvidencePath = path.join(evidenceRoot, "control-surface-evidence.json");
const publicDemoEvidencePath = path.join(evidenceRoot, "public-demo-gating-evidence.json");
const CAPTURE_BASELINE = process.env.WEB05_CAPTURE_BASELINE === "1";
const RUN_PUBLIC_DEMO_E2E = process.env.WEB05_PUBLIC_DEMO_E2E === "1";
const CAPTURE_LABEL = process.env.WEB05_CAPTURE_LABEL ?? null;
const SOURCE_COMMIT = process.env.WEB05_SOURCE_COMMIT ?? null;
const WASM_PATH = process.env.WEB05_WASM_PATH ?? null;

test.describe.configure({ mode: "serial" });
test.setTimeout(TIMEOUT_MS);

type CandidateSnapshot = {
  text: string | null;
  source: string | null;
  rowText: string;
};

type SmokeSnapshot = {
  schema: string;
  input: string;
  preedit: string;
  candidates: CandidateSnapshot[];
};

type DefaultBehaviorEvidence = {
  sourceLabel: string | null;
  sourceCommit: string | null;
  appUrl: string;
  capturedAt: string;
  wasmPath: string | null;
  wasmSha256: string | null;
  appVersion: string | null;
  snapshots: SmokeSnapshot[];
};

const smokeCases = [
  { schema: "jyut6ping3", inputs: ["ngo", "santai"] },
  { schema: "luna_pinyin", inputs: ["ni", "hao"] },
] as const;

async function writeJson(file: string, value: unknown): Promise<void> {
  await mkdir(path.dirname(file), { recursive: true });
  await writeFile(file, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

async function readJson<T>(file: string): Promise<T> {
  return JSON.parse(await readFile(file, "utf8")) as T;
}

async function runtimeArtifact(): Promise<{
  wasmPath: string | null;
  wasmSha256: string | null;
}> {
  if (WASM_PATH) {
    const data = await readFile(WASM_PATH);
    return {
      wasmPath: WASM_PATH,
      wasmSha256: createHash("sha256").update(data).digest("hex"),
    };
  }
  const candidates = [
    path.join(repoRoot, "apps", "yune-web", "public", "yune-web.wasm"),
    path.join(repoRoot, "target", "wasm32-unknown-emscripten", "debug", "yune-web.wasm"),
  ];
  for (const file of candidates) {
    try {
      const data = await readFile(file);
      return {
        wasmPath: file,
        wasmSha256: createHash("sha256").update(data).digest("hex"),
      };
    } catch {
      // Try the next known runtime artifact location.
    }
  }
  return { wasmPath: null, wasmSha256: null };
}

async function openApp(page: Page, url = APP_URL): Promise<void> {
  await page.goto(url, { waitUntil: "domcontentloaded", timeout: TIMEOUT_MS });
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

async function clearComposition(page: Page): Promise<void> {
  await page.keyboard.press("Escape").catch(() => undefined);
  await expect(page.locator(".candidate-panel")).toHaveCount(0, {
    timeout: 5000,
  });
  await page.locator("textarea").first().fill("");
}

async function selectSchema(page: Page, schema: string): Promise<void> {
  await clearComposition(page);
  const switcher = page.locator("[data-yune-schema-switcher]");
  await expect(switcher).toBeVisible({ timeout: 5000 });
  const select = switcher.locator("select");
  if (await select.count()) {
    await select.selectOption(schema);
  } else {
    await switcher.locator(`input[value="${schema}"]`).check({ force: true });
  }
  await expect
    .poll(
      () =>
        page.evaluate(
          () => document.documentElement.dataset.yuneActiveSchema ?? null,
        ),
      { timeout: TIMEOUT_MS },
    )
    .toBe(schema);
  await openAppReady(page);
}

async function openAppReady(page: Page): Promise<void> {
  await page.waitForFunction(
    () => document.documentElement.dataset.yuneLoading === "false",
    undefined,
    { timeout: TIMEOUT_MS },
  );
}

async function typeAndSnapshot(
  page: Page,
  schema: string,
  input: string,
): Promise<SmokeSnapshot> {
  await clearComposition(page);
  const textArea = page.locator("textarea").first();
  await textArea.focus();
  await textArea.type(input, { delay: 60 });
  await expect(page.locator(".candidate-panel .candidate-row").first()).toBeVisible({
    timeout: 10000,
  });
  const candidates = await page
    .locator(".candidate-panel .candidate-row")
    .evaluateAll((rows) =>
      rows.slice(0, 6).map((row) => ({
        text: row.getAttribute("data-candidate-text"),
        source: row.getAttribute("data-source"),
        rowText: row.textContent?.replace(/\s+/g, " ").trim() ?? "",
      })),
    );
  const preedit = await page.locator(".candidate-preedit").first().innerText();
  return { schema, input, preedit, candidates };
}

async function captureDefaultBehavior(page: Page): Promise<DefaultBehaviorEvidence> {
  await openApp(page);
  const snapshots: SmokeSnapshot[] = [];
  for (const smokeCase of smokeCases) {
    await selectSchema(page, smokeCase.schema);
    for (const input of smokeCase.inputs) {
      snapshots.push(await typeAndSnapshot(page, smokeCase.schema, input));
    }
  }
  const artifact = await runtimeArtifact();
  return {
    sourceLabel: CAPTURE_LABEL,
    sourceCommit: SOURCE_COMMIT,
    appUrl: APP_URL,
    capturedAt: new Date().toISOString(),
    wasmPath: artifact.wasmPath,
    wasmSha256: artifact.wasmSha256,
    appVersion: await page.evaluate(
      () => document.documentElement.dataset.yuneRimeVersion ?? null,
    ),
    snapshots,
  };
}

test("WEB-05 same-WASM default behavior baseline/post comparison", async ({
  page,
}) => {
  const current = await captureDefaultBehavior(page);
  if (CAPTURE_BASELINE) {
    await writeJson(baselinePath, current);
    return;
  }
  const baseline = await readJson<typeof current>(baselinePath);
  await writeJson(postPath, current);
  expect(current.wasmSha256).toBe(baseline.wasmSha256);
  expect(current.appVersion).toBe(baseline.appVersion);
  expect(current.snapshots).toEqual(baseline.snapshots);
});

test("WEB-05 deploy and diagnostics controls expose observable state", async ({
  page,
}) => {
  await openApp(page);
  await expect(page.locator("[data-yune-control-surface]")).toBeVisible();
  await expect(page.locator("[data-yune-deploy-status-view]")).toContainText(
    /start|success|failure|idle/i,
  );
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.dataset.yuneDeployStatus ?? null,
      ),
    )
    .not.toBeNull();

  await page.locator("[data-yune-control-redeploy]").click();
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.dataset.yuneDeployStatus ?? null,
      ),
      { timeout: TIMEOUT_MS },
    )
    .toBe("success");

  await expect(page.locator("[data-yune-deploy-cache-view]")).toBeVisible();
  await page.locator("[data-yune-control-invalidate-deploy-cache]").click();
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.dataset.yuneDeployCacheFresh ?? null,
      ),
      { timeout: 10000 },
    )
    .toBe("false");
  const invalidatedDeployCacheFresh = await page.evaluate(
    () => document.documentElement.dataset.yuneDeployCacheFresh ?? null,
  );
  await page.locator("[data-yune-control-redeploy]").click();
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.dataset.yuneDeployStatus ?? null,
      ),
      { timeout: TIMEOUT_MS },
    )
    .toBe("success");
  await expect(page.locator("[data-yune-persistence-diagnostics-panel]")).toContainText(
    /schema:deploy|deploy:cache|runtime:init/,
  );
  await expect(page.locator("[data-yune-injected-assets]")).toBeVisible();

  await writeJson(controlsEvidencePath, {
    deployStatus: await page.evaluate(
      () => document.documentElement.dataset.yuneDeployStatus ?? null,
    ),
    invalidatedDeployCacheFresh,
    deployCacheFresh: await page.evaluate(
      () => document.documentElement.dataset.yuneDeployCacheFresh ?? null,
    ),
    persistenceDiagnosticsCount: await page.evaluate(() => {
      const raw = document.documentElement.dataset.yunePersistenceDiagnostics;
      return raw ? JSON.parse(raw).length : 0;
    }),
    injectedAssets: await page
      .locator("[data-yune-injected-asset]")
      .evaluateAll((rows) => rows.map((row) => row.textContent?.trim())),
  });
});

test("WEB-05 option controls synchronize through optionChanged", async ({
  page,
}) => {
  await openApp(page);
  const asciiPunct = page.locator("[data-yune-control-ascii-punct]");
  await expect(asciiPunct).toBeVisible();
  await asciiPunct.check();
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.dataset.yuneOptionAsciiPunct ?? null,
      ),
    )
    .toBe("true");
  await expect(asciiPunct).toBeChecked();
  const editor = page.locator("[data-yune-dictionary-exclude-editor]");
  await expect(editor).toBeVisible();
  await editor.fill("alpha,beta");
  await expect
    .poll(() =>
      page.evaluate(
        () => document.documentElement.dataset.yuneDictionaryExcludeCount ?? null,
      ),
    )
    .toBe("2");
});

test("WEB-05 inspector renders parsed debug fields", async ({ page }) => {
  await openApp(page);
  await page.locator("[data-yune-inspector-toggle] input").check();
  await typeAndSnapshot(page, "jyut6ping3", "ngo");
  await expect(page.locator("[data-yune-inspector='panel']")).toBeVisible();
  await expect(page.locator("[data-yune-inspector-segment-source]").first()).toBeVisible();
  await expect(page.locator("[data-yune-inspector-prediction-threshold]").first()).toBeVisible();
  await expect(page.locator("[data-yune-inspector-prediction-above-threshold]").first()).toBeVisible();
  await expect(page.locator("[data-yune-inspector-candidate-preedit]").first()).toBeVisible();
  await expect(page.locator("[data-yune-inspector-candidate-ai-confidence]").first()).toBeVisible();
});

test("WEB-05 dev-power controls are visible only in the harness", async ({
  page,
}) => {
  await openApp(page);
  await expect(page.locator("[data-yune-raw-response-viewer]")).toBeVisible();
  await expect(page.locator("[data-yune-freeform-set-option]")).toBeVisible();
  await expect(page.locator("[data-yune-freeform-customize]")).toBeVisible();
  await expect(page.locator("[data-yune-freeform-customize-warning]")).toContainText(
    /deployed config|已部署設定/i,
  );
  await expect(page.locator("[data-yune-debug-url-reference]")).toBeVisible();

  await page.locator("[data-yune-freeform-customize] input").first().fill("");
  await page.locator("[data-yune-freeform-customize-submit]").click();
  await expect(page.locator("[data-yune-action-error-history]")).toContainText(
    /customizeValue|config ID|key/i,
    { timeout: 10000 },
  );
});

test("WEB-05 public demo hides debug and admin controls", async ({ page }) => {
  test.skip(!RUN_PUBLIC_DEMO_E2E, "Set WEB05_PUBLIC_DEMO_E2E=1 for public demo pass.");
  await openApp(page, PUBLIC_APP_URL);
  const hiddenSelectors = [
    "[data-yune-control-redeploy]",
    "[data-yune-control-invalidate-deploy-cache]",
    "[data-yune-persistence-diagnostics-panel]",
    "[data-yune-injected-assets]",
    "[data-yune-raw-response-viewer]",
    "[data-yune-freeform-set-option]",
    "[data-yune-freeform-customize]",
    "[data-yune-debug-url-reference]",
    "[data-yune-action-error-history]",
  ];
  for (const selector of hiddenSelectors) {
    await expect(page.locator(selector)).toHaveCount(0);
  }
  const forbiddenActions = [
    "deployCacheSnapshot",
    "injectedAssetsManifest",
    "invalidateDeployCache",
  ];
  const publicDataSurface = await page.evaluate(() => {
    const root = document.documentElement;
    const rawActions = root.dataset.yuneActionDiagnostics;
    const actionDiagnostics = rawActions ? JSON.parse(rawActions) as { action?: string }[] : [];
    return {
      lastActionResult: root.dataset.yuneLastActionResult ?? null,
      deployCacheFresh: root.dataset.yuneDeployCacheFresh ?? null,
      actionNames: actionDiagnostics.map((diagnostic) => diagnostic.action ?? ""),
    };
  });
  expect(publicDataSurface.lastActionResult).toBeNull();
  expect(publicDataSurface.deployCacheFresh).toBeNull();
  for (const action of forbiddenActions) {
    expect(publicDataSurface.actionNames).not.toContain(action);
  }
  await writeJson(publicDemoEvidencePath, {
    hiddenSelectors,
    forbiddenActions,
    publicDataSurface,
  });
});
