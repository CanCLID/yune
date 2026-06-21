# M24 TypeDuck-Web Dogfooding & Demo Hardening Implementation Plan

> **Status:** Active - **Milestone:** M24 (TypeDuck-Web dogfooding and demo hardening) - **Created:** 2026-06-21 - **Type:** running execution plan / issue ledger
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn real manual play-testing of the internal TypeDuck-Web playground into a tracked, evidence-backed hardening loop without reopening completed parity milestones by accident.

**Architecture:** M24 treats `third_party/typeduck-web/` as the active browser dogfooding surface. Browser loading, rendering, and UX defects can be fixed with browser evidence; candidate-set, candidate-order, or engine-semantic changes still require pinned oracle evidence from TypeDuck `v1.1.2` or upstream `1.17.0` before changing engine behavior. Each issue is classified first, then implemented in the narrowest owning layer.

**Tech Stack:** TypeDuck-Web React/TypeScript, Vite/Bun, Playwright, `@yune-ime/typeduck-runtime`, `yune-rime-api` C ABI, `yune-core`, TypeDuck `v1.1.2` oracle fixtures.

---

## Scope

M24 is an active dogfooding and hardening loop for the internal browser playground:

- **In scope:** first-load performance, visible loading state, candidate panel layout, comment rendering, dictionary panel ergonomics, browser schema-switch behavior, TypeDuck-Web runtime integration bugs, and browser evidence for user-visible fixes.
- **In scope with oracle evidence:** candidate set, candidate order, phrase/sentence fallback, correction, prediction, dictionary lookup payloads, and any behavior that changes engine output.
- **Out of scope:** changing the separately cloned or deployed `TypeDuck-HK/TypeDuck-Web` product; claiming live-site behavior as the hard oracle; widening default `RimeApi` or `RimeCandidate`; exposing unsupported controls as working toggles.

## Evidence Rules

- Save M24 browser evidence under `third_party/typeduck-web/e2e/results/m24-dogfooding/<issue-id>/`.
- For every browser-visible fix, capture before/after screenshots or JSON snapshots from the real app.
- For every engine-output fix, add or extend an oracle fixture under `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` or `crates/yune-core/tests/fixtures/upstream-1.17.0/` before implementation.
- Do not use `https://www.typeduck.hk/web/` as a hard oracle. It is a useful feel target only; any should-match behavior must be pinned through the TypeDuck `v1.1.2` fixture path.
- Keep completed M9/M13/M16/M20/M22 gates green after each fix.

## How To Add New Dogfooding Issues

Append a row to the ledger below using the next `M24-DOGFOOD-XX` id. Each row must name:

- the user-visible symptom,
- the current repro input or action,
- the classification,
- the owning files or tests to inspect first,
- the acceptance evidence needed before closing the row.

If a report is ambiguous, classify it as **Needs triage**, capture the screenshot/state first, and only then move it to browser integration, UI polish, engine correctness, unsupported/N/A, or future product integration.

## Running Issue Ledger

| ID | Status | Classification | User-visible issue | First repro / evidence | Owning surfaces | Close criteria |
|---|---|---|---|---|---|---|
| M24-DOGFOOD-01 | Open | Browser integration / performance | First visit to `http://localhost:5173/web/` remains on `載入中 Loading...` for too long. | Fresh browser tab to `/web/`; user observed a long loading period before the IME becomes usable. | `third_party/typeduck-web/source/src/worker.ts`, `third_party/typeduck-web/source/src/yune-integration/adapter.ts`, `third_party/typeduck-web/source/src/yune-integration/assets.ts`, `packages/yune-typeduck-runtime/`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts` | Timing trace separates WASM download, module creation, asset loading, persistence sync, and runtime init; local first-load budget is enforced or the exact unavoidable bottleneck is documented with byte/time evidence. |
| M24-DOGFOOD-02 | Open | Browser integration / comment rendering | Long phrase candidates show a literal `\f` before Jyutping on following single-character candidates, while single-character input does not show it. | Type a long phrase such as `jigaajiusihaa`; screenshot shows candidates like `以 \fji5`. | `third_party/typeduck-web/source/src/CandidateInfo.ts`, `third_party/typeduck-web/source/src/Candidate.tsx`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`, candidate comments from `crates/yune-rime-api/tests/typeduck_web.rs` | No visible literal `\f`, `\r`, or `\v` appears in candidate rows; dictionary-rich comments still parse into entries; single-character and phrase inputs both render cleanly. |
| M24-DOGFOOD-03 | Open | UI polish / candidate layout | Compound-candidate dictionary glosses render horizontally next to the candidate text, making `而家要思考(compound 詞組) 而家 (now) 要 (want; need) 思考 (think; ponder)` read like one confusing inline candidate. | Type `jigaajiusihaa`; screenshot shows the first highlighted candidate widened across the horizontal row. | `third_party/typeduck-web/source/src/Candidate.tsx`, `third_party/typeduck-web/source/src/CandidatePanel.tsx`, `third_party/typeduck-web/source/src/DictionaryPanel.tsx`, `third_party/typeduck-web/source/src/index.css` | Main candidate row stays compact; detailed English/gloss content moves below the candidate or into the dictionary/detail panel; before/after screenshots prove no horizontal overflow or misleading inline gloss. |
| M24-DOGFOOD-04 | Open | Engine correctness / oracle recheck | For `jigaajiusihaa`, after the first compound candidate the next candidates are single characters, while the user-observed live TypeDuck behavior appears to prefer word entries such as `而家`, `依家`, `宜家` before single characters. | User compared the internal playground with `https://www.typeduck.hk/web/`; live product appears to show word candidates in positions 2-3. | `scripts/capture-typeduck-jyutping.ps1`, `crates/yune-core/tests/cantonese_parity.rs`, `crates/yune-core/src/translator/mod.rs`, `crates/yune-core/src/dictionary/`, `crates/yune-rime-api/tests/typeduck_web.rs`, M21 source-aware evidence under `third_party/typeduck-web/e2e/results/m21-product-comparison/` | A pinned TypeDuck `v1.1.2` fixture or a documented version-skew decision determines the expected row order; Yune either matches the fixture with active tests or records the live-site behavior as non-oracle product skew. |
| M24-DOGFOOD-05 | Open | UI polish / settings localization and help text | Settings and developer controls mix Cantonese/English and many labels are English-only, so a new developer cannot tell what active engine controls or live session controls do. Cantonese should come first for all labels; active engine and live session toggles need short description text. Display controls need Cantonese-first labels but no extra descriptions. | Browser comment on `/web/` settings area: selected controls include `Active engine controls`, `Live session controls`, `Display controls`, `Yune inspector`, `Schema`, and English-only toggle labels such as `ASCII mode`, `Full shape`, `Prediction threshold`, and `Dictionary exclude`. | `third_party/typeduck-web/source/src/Preferences.tsx`, `third_party/typeduck-web/source/src/Inputs.tsx`, `third_party/typeduck-web/source/src/Toolbar.tsx`, `third_party/typeduck-web/source/src/SchemaSwitcher.tsx`, `third_party/typeduck-web/source/src/App.tsx`, `third_party/typeduck-web/source/src/YuneInspector.tsx`, `third_party/typeduck-web/e2e/yune-typeduck.spec.ts` | All visible settings/developer labels use Cantonese-first bilingual text; active-engine and live-session toggles show concise helper copy; display controls remain compact without helper paragraphs; before/after screenshots prove the settings page stays readable at desktop and narrow widths. |

---

## Task 1: Baseline Capture And M24 Evidence Harness

**Files:**
- Modify: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Create evidence during execution: `third_party/typeduck-web/e2e/results/m24-dogfooding/`

- [ ] **Step 1: Add an M24 evidence scope helper**

Add a small helper near the existing evidence helpers so M24 screenshots and JSON do not mix with M20/M22 artifacts:

```ts
const M24_EVIDENCE_DIR = "m24-dogfooding";

async function saveM24Json(issueId: string, filename: string, payload: unknown): Promise<void> {
  await saveJsonEvidence(`${M24_EVIDENCE_DIR}/${issueId}/${filename}`, payload);
}

async function takeM24Screenshot(page: Page, issueId: string, filename: string): Promise<void> {
  await takeEvidenceScreenshot(page, `${M24_EVIDENCE_DIR}/${issueId}/${filename}`);
}
```

- [ ] **Step 2: Add a reusable long-phrase capture helper**

Use the same input path the user sees:

```ts
async function captureM24Phrase(page: Page, issueId: string, input: string, expectedTopText: string): Promise<CandidatePanelSnapshot> {
  const state = await typeCompositionAndWaitForTopCandidate(page, input, expectedTopText);
  await saveM24Json(issueId, `${input}-state.json`, state);
  await takeM24Screenshot(page, issueId, `${input}-candidate-panel`);
  return state;
}
```

- [ ] **Step 3: Run the browser smoke before changing behavior**

Run:

```powershell
npm.cmd --prefix third_party/typeduck-web/source run build
```

Expected: the worker bundle and Vite build complete without TypeScript errors.

- [ ] **Step 4: Commit the evidence-harness-only slice if it is useful independently**

Stage only the files changed for M24 evidence plumbing:

```powershell
git add -- third_party/typeduck-web/e2e/yune-typeduck.spec.ts
git commit -m "test: add M24 TypeDuck-Web dogfooding evidence helpers"
```

## Task 2: M24-DOGFOOD-01 Startup Loading Performance

**Files:**
- Modify: `third_party/typeduck-web/source/src/worker.ts`
- Modify: `third_party/typeduck-web/source/src/yune-integration/adapter.ts`
- Modify: `third_party/typeduck-web/source/src/yune-integration/assets.ts`
- Inspect first, then modify only if the timing trace points there: `packages/yune-typeduck-runtime/src/*`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Add a startup timing trace**

Instrument named phases without changing behavior:

```ts
type StartupPhase = {
  phase: string;
  startedAt: number;
  finishedAt: number;
  durationMs: number;
};

const startupTrace: StartupPhase[] = [];

async function measureStartupPhase<T>(phase: string, action: () => Promise<T>): Promise<T> {
  const startedAt = performance.now();
  try {
    return await action();
  } finally {
    const finishedAt = performance.now();
    startupTrace.push({ phase, startedAt, finishedAt, durationMs: finishedAt - startedAt });
  }
}
```

Wrap these phases in `worker.ts`: `import-yune-js`, `create-module`, `mount-persistence`, `load-text-assets`, `load-binary-assets`, and `select-default-schema`. In `adapter.ts`, emit sub-phases for `syncFromPersistenceBeforeInit`, `prepareTypeDuckFilesystem`, `TypeDuckRuntime.init`, and `syncToPersistenceAfterMutation`.

- [ ] **Step 2: Post timing diagnostics to the page**

After `dispatch("initialized", true)`, post a diagnostic:

```ts
postMessage({
  type: "diagnostic",
  source: "m24-startup",
  marker: {
    phase: "startup:complete",
    schemaId: activeSchemaId,
    trace: startupTrace,
    timestamp: new Date().toISOString(),
  },
});
```

- [ ] **Step 3: Add a Playwright characterization test**

Add a test that records the current baseline before optimization:

```ts
test("M24 startup timing trace records loading phases", async ({ page }) => {
  const markers: unknown[] = [];
  page.on("console", message => {
    if (message.type() === "error") {
      markers.push({ consoleError: message.text() });
    }
  });
  await page.goto(APP_URL, { timeout: TIMEOUT_MS, waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
  const resources = await page.evaluate(() =>
    performance.getEntriesByType("resource")
      .filter(entry => /yune-typeduck|schema|\.bin/.test(entry.name))
      .map(entry => ({
        name: entry.name,
        duration: entry.duration,
        transferSize: "transferSize" in entry ? (entry as PerformanceResourceTiming).transferSize : 0,
      })),
  );
  await saveM24Json("M24-DOGFOOD-01", "startup-resources.json", { resources, markers });
  expect(resources.some(resource => resource.name.includes("yune-typeduck.wasm"))).toBe(true);
});
```

- [ ] **Step 4: Optimize only the measured slow phases**

Use the trace to choose the narrow fix:

- If binary asset loading dominates, load only `jyut6ping3_mobile` assets at startup and lazily load `cangjie5` / `luna_pinyin` artifacts on schema switch.
- If module creation dominates, check whether the dev server serves the 36.6 MB WASM with the expected cache headers and whether the generated JS glue can avoid repeated startup work.
- If persistence sync dominates, narrow IDBFS sync to the user/build paths the default schema needs.
- If default schema deploy dominates, keep deployed assets warm across reloads and avoid re-preparing unchanged shared assets.

- [ ] **Step 5: Add a budget assertion after optimization**

After the measured fix lands, enforce:

```ts
expect(totalReadyMs).toBeLessThanOrEqual(5000);
```

For cold-cache CI/browser runs where the WASM transfer alone exceeds 5000 ms, record `wasmTransferMs` and enforce the budget on `totalReadyMs - wasmTransferMs` instead.

## Task 3: M24-DOGFOOD-02 Literal `\f` Comment-Control Leakage

**Files:**
- Modify: `third_party/typeduck-web/source/src/CandidateInfo.ts`
- Inspect first, then modify only if parsing cannot be fixed in `CandidateInfo.ts`: `third_party/typeduck-web/source/src/Candidate.tsx`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Reference: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json`

- [ ] **Step 1: Capture the failing browser state**

Use `jigaajiusihaa` and save the raw row text:

```ts
test("M24 phrase candidates do not show raw comment control bytes", async ({ page }) => {
  const state = await captureM24Phrase(page, "M24-DOGFOOD-02", "jigaajiusihaa", "而家要思考");
  expect(state.candidates.map(candidate => candidate.rowText).join(" ")).not.toMatch(/\\[frv]/);
});
```

Expected before the fix: this fails because a row contains visible `\f`.

- [ ] **Step 2: Fix comment parsing at the browser boundary**

Keep the raw engine comment payload intact. Normalize only the UI parser so control separators are consumed before rendering:

```ts
const visibleControlText = /\\[frv]/;
```

The fix should make `CandidateInfo` handle all of these shapes:

- `jyutping-only` comment,
- `\v` reverse lookup prefix,
- `note\fjyutping`,
- `\fjyutping`,
- `note\f\r1,...`,
- `\f\r1,...`.

- [ ] **Step 3: Prove dictionary-rich comments still parse**

Extend the same test or add a second one asserting the first compound candidate still exposes dictionary entries:

```ts
expect(state.candidates[0].rowText).toContain("而家");
expect(state.candidates[0].rowText).toContain("思考");
```

- [ ] **Step 4: Run the focused web gate**

Run:

```powershell
npm.cmd --prefix third_party/typeduck-web/source run build
npx --prefix third_party/typeduck-web/source playwright test third_party/typeduck-web/e2e/yune-typeduck.spec.ts -g "M24 phrase candidates"
```

Expected: no visible `\f`, `\r`, or `\v` in candidate rows.

## Task 4: M24-DOGFOOD-03 Candidate Gloss Layout

**Files:**
- Modify: `third_party/typeduck-web/source/src/Candidate.tsx`
- Modify: `third_party/typeduck-web/source/src/CandidatePanel.tsx`
- Modify: `third_party/typeduck-web/source/src/DictionaryPanel.tsx`
- Modify: `third_party/typeduck-web/source/src/index.css`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Capture current overcrowding**

Use the same repro as the user screenshot:

```ts
test("M24 compound gloss layout stays readable", async ({ page }) => {
  const state = await captureM24Phrase(page, "M24-DOGFOOD-03", "jigaajiusihaa", "而家要思考");
  expect(state.candidates[0].rowText).toContain("compound");
});
```

Before the layout fix this records the overcrowded horizontal row as evidence.

- [ ] **Step 2: Split compact candidate identity from verbose glosses**

Keep the horizontal candidate list focused on:

- label,
- candidate text,
- Jyutping,
- short type labels such as `(compound 詞組)`,
- info marker when dictionary details exist.

Move English gloss strings such as `now`, `want; need`, and `think; ponder` below the candidate or into the dictionary/detail panel. Do not remove the data from `CandidateInfo`; change only presentation.

- [ ] **Step 3: Add a visible layout assertion**

Assert the candidate panel does not render verbose English glosses inline in the first row:

```ts
const firstRow = page.locator(".candidate-panel .candidates tbody").first();
await expect(firstRow).toContainText("而家要思考");
await expect(firstRow).not.toContainText("think; ponder");
```

Then assert the detail surface still contains the gloss when the candidate is hovered or selected:

```ts
await firstRow.hover();
await expect(page.locator(".dictionary-panel")).toContainText("think; ponder");
```

- [ ] **Step 4: Verify desktop-width and narrow-width screenshots**

Run the M24 Playwright test twice:

```powershell
npx --prefix third_party/typeduck-web/source playwright test third_party/typeduck-web/e2e/yune-typeduck.spec.ts -g "M24 compound gloss layout"
```

Capture screenshots at default desktop width and a narrow viewport. The candidate panel must not overflow into an unreadable single line.

## Task 5: M24-DOGFOOD-04 `jigaajiusihaa` Word-Candidate Ordering

**Files:**
- Modify or add fixture: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m24-dogfooding.json`
- Modify: `scripts/capture-typeduck-jyutping.ps1`
- Modify: `crates/yune-core/tests/cantonese_parity.rs`
- Inspect first, then modify only if the pinned fixture proves an engine-ordering bug: `crates/yune-core/src/translator/mod.rs`
- Inspect first, then modify only if the pinned fixture proves dictionary lookup/weight data is missing or misread: `crates/yune-core/src/dictionary/`
- Test: `crates/yune-rime-api/tests/typeduck_web.rs`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Treat the live site as a clue, not the oracle**

Record the user observation as:

```text
Input: jigaajiusihaa
Internal playground observed: top compound candidate, then single-character rows.
Live product observed by user: word candidates such as 而家 / 依家 / 宜家 appear before single-character rows.
Hard-oracle status: not captured yet for this exact input.
```

- [ ] **Step 2: Capture TypeDuck `v1.1.2` for the exact input**

Extend `scripts/capture-typeduck-jyutping.ps1` with a dogfooding variant that captures at least:

```powershell
@("jigaajiusihaa", "jigaa", "jiusihau", "jigaajiu")
```

Write the output to:

```text
crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m24-dogfooding.json
```

The fixture must record selected candidate texts, comments, highlighted index, page size, schema id, active option profile, and capture command.

- [ ] **Step 3: Add a failing parity assertion before changing ranking**

Add a focused test in `crates/yune-core/tests/cantonese_parity.rs`. Extend the
existing fixture constants and helpers in the same style as
`m21_closeout_fixture()`, `m21_closeout_case(...)`, and
`selected_candidate_text(...)`:

```rust
const M24_DOGFOOD_ORACLE: &str =
    include_str!("fixtures/typeduck-v1.1.2/jyut6ping3-m24-dogfooding.json");

fn m24_dogfooding_fixture() -> Value {
    serde_json::from_str(M24_DOGFOOD_ORACLE)
        .expect("TypeDuck v1.1.2 M24 dogfooding fixture should be valid JSON")
}

fn m24_dogfooding_case<'a>(fixture: &'a Value, variant: &str, input: &str) -> &'a Value {
    fixture["cases"]
        .as_array()
        .expect("M24 dogfooding cases should be an array")
        .iter()
        .find(|case| case["variant"] == variant && case["input"] == input)
        .unwrap_or_else(|| {
            panic!("M24 dogfooding fixture should capture variant {variant} input {input}")
        })
}

#[test]
fn m24_jigaajiusihaa_word_candidates_match_typeduck_v112() {
    let fixture = m24_dogfooding_fixture();
    let case = m24_dogfooding_case(&fixture, "default_combined", "jigaajiusihaa");
    let expected = (0..5)
        .map(|index| selected_candidate_text(case, index))
        .collect::<Vec<_>>();

    let mut engine = typeduck_jyut6ping3_mobile_engine(false);
    engine.set_input("jigaajiusihaa");
    let actual = engine
        .context()
        .candidates
        .iter()
        .take(5)
        .map(|candidate| candidate.text.as_str())
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
}
```

This uses the production `typeduck_jyut6ping3_mobile_engine(false)` helper; do
not invent a parallel engine harness.

- [ ] **Step 4: Diagnose source ordering before changing code**

Use existing source-aware diagnostics from the M21 pattern to classify candidates as sentence, table, prediction, user, correction, or fallback. The fix should answer one of these questions with evidence:

- Are word entries present in the dictionary but filtered out?
- Are word entries present but ranked after single-character entries?
- Does the sentence fallback return only one compound row and suppress the rest of the word path?
- Is the live product behavior version skew rather than TypeDuck `v1.1.2` behavior?

- [ ] **Step 5: Implement only the oracle-backed ordering fix**

If TypeDuck `v1.1.2` expects word candidates before single-character rows, adjust the narrow TypeDuck profile path. Keep default upstream `luna_pinyin` behavior untouched and keep TypeDuck-specific tuning behind explicit profile config.

- [ ] **Step 6: Add browser evidence**

Add a Playwright assertion for the visible playground after the engine test passes:

```ts
test("M24 jigaajiusihaa shows word candidates before single-character fallback", async ({ page }) => {
  const state = await captureM24Phrase(page, "M24-DOGFOOD-04", "jigaajiusihaa", "而家要思考");
  expect(state.candidates.slice(1, 4).map(candidate => candidate.text)).toEqual(["而家", "依家", "宜家"]);
});
```

If the pinned fixture differs from the live product, replace the expected list with the pinned fixture output and record the live product difference as version skew in the evidence JSON.

## Task 6: M24-DOGFOOD-05 Cantonese-First Settings Labels And Toggle Help

**Files:**
- Modify: `third_party/typeduck-web/source/src/Preferences.tsx`
- Modify: `third_party/typeduck-web/source/src/Inputs.tsx`
- Modify: `third_party/typeduck-web/source/src/Toolbar.tsx`
- Modify: `third_party/typeduck-web/source/src/SchemaSwitcher.tsx`
- Modify: `third_party/typeduck-web/source/src/App.tsx`
- Inspect first, then modify only if the inspector panel remains visible in the same developer settings flow: `third_party/typeduck-web/source/src/YuneInspector.tsx`
- Test: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Capture the current settings readability problem**

Save desktop and narrow-width screenshots of the settings area before changing labels:

```ts
test("M24 settings labels are Cantonese-first and documented", async ({ page }) => {
  await page.goto(APP_URL, { timeout: TIMEOUT_MS, waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
  await takeM24Screenshot(page, "M24-DOGFOOD-05", "settings-labels-before");
});
```

Also capture a text snapshot of the headings and labels so the regression test can compare specific strings rather than relying only on screenshots.

- [ ] **Step 2: Add structured helper text support for controls that need it**

Extend the shared input components instead of hard-coding helper paragraphs in `Preferences.tsx`:

```ts
interface ControlCopy {
  label: string;
  description?: string;
}
```

Allow `Toggle` and `Range` to accept `description?: string`. Render the helper copy under the label in smaller text, keeping the switch/range aligned and ensuring the description does not force the row to overflow on narrow widths.

- [ ] **Step 3: Localize headings and all visible settings labels Cantonese-first**

Use Cantonese-first bilingual text for settings labels. Cover at least:

- `Schema` -> `方案 Schema`,
- `Yune inspector` -> `Yune 檢查器 Yune Inspector`,
- `Active engine controls` -> `輸入引擎控制 Active Engine Controls`,
- `Live session controls` -> `即時輸入狀態 Live Session Controls`,
- `Display controls` -> `顯示設定 Display Controls`,
- `Prediction threshold` -> `預測門檻 Prediction Threshold`,
- `Dictionary exclude` -> `字典排除 Dictionary Exclude`,
- `ASCII mode`, `Full shape`, `Simplification`, `Traditionalization`, `Extended charset`, and `Disabled`,
- display-control labels such as `Candidate Jyutping`, `Reverse code display`, and `Cangjie version`.

Keep the Cantonese term first for every bilingual label. Do not add explanatory descriptions to display controls unless a later user report specifically asks for it.

- [ ] **Step 4: Add short explanations for active engine controls**

Add one-line helper copy for active engine controls. Keep each description concise enough to scan in the settings panel:

```text
自動完成 Auto-completion - 用輸入開頭搵候選字詞。
自動校正 Auto-correction - 容許常見錯碼或近音修正。
自動組詞 Auto-composition - 將多段輸入砌成長詞句候選。
輸入記憶 Input Memory - 用本機輸入記錄改善常用候選排序。
AI 候選 AI Candidates - 顯示本機 AI staging 候選。
合併相同候選 Combine Same-Text Candidates - 合併同字候選，避免重複。
預測不排第一 Prediction Never First - 保持預測候選不會排第一。
預測門檻 Prediction Threshold - 調高門檻先顯示分數較高預測。
字典排除 Dictionary Exclude - 臨時隱藏測試用字典項目。
```

Treat the English half as a fallback label, not the primary explanation.

- [ ] **Step 5: Add short explanations for live session controls**

Add one-line helper copy for live session controls:

```text
英文模式 ASCII Mode - 直接輸入英文字母，暫停中文組字。
全形 Full Shape - 使用全形英文字母及標點。
簡化 Simplification - 將候選或輸出轉成簡體。
繁化 Traditionalization - 將候選或輸出轉成繁體。
擴展字集 Extended Charset - 顯示較少見或擴展漢字候選。
停用 Disabled - 暫停輸入法處理，保留原始按鍵。
```

If a control is a visible no-op for the current schema, keep the label but route the implementation to the existing status/output mechanism so it remains honest instead of silently implying unsupported behavior.

- [ ] **Step 6: Add browser assertions and screenshots**

Add a focused Playwright check:

```ts
await expect(page.getByText("輸入引擎控制 Active Engine Controls")).toBeVisible();
await expect(page.getByText("即時輸入狀態 Live Session Controls")).toBeVisible();
await expect(page.getByText("顯示設定 Display Controls")).toBeVisible();
await expect(page.getByText("用輸入開頭搵候選字詞。")).toBeVisible();
await expect(page.getByText("直接輸入英文字母，暫停中文組字。")).toBeVisible();
```

Save after screenshots under `third_party/typeduck-web/e2e/results/m24-dogfooding/M24-DOGFOOD-05/`, including a narrow viewport. Verify that the new helper text does not overlap switches, sliders, or display-control groups.

## Task 7: M24 Regression Sweep And Closeout Discipline

**Files:**
- Modify: this plan as issue rows close
- Modify if requirements become durable: `docs/requirements.md`
- Modify when M24 closes: `docs/roadmap.md`

- [ ] **Step 1: Run focused gates for the touched layer**

For browser-only fixes:

```powershell
npm.cmd --prefix third_party/typeduck-web/source run build
npx --prefix third_party/typeduck-web/source playwright test third_party/typeduck-web/e2e/yune-typeduck.spec.ts -g "M24"
```

For runtime bridge fixes:

```powershell
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
cargo test -p yune-rime-api --test typeduck_web
```

For engine behavior fixes:

```powershell
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
```

- [ ] **Step 2: Run broad gates before closing a batch**

Run:

```powershell
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm.cmd --prefix third_party/typeduck-web/source run build
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
```

- [ ] **Step 3: Update the running ledger**

For each closed row, change `Status` from `Open` to `Closed`, add the evidence directory, and state which test owns the regression.

- [ ] **Step 4: Add requirements only for durable product/demo contracts**

Do not add requirement IDs for every small bug. Add `M24-DOGFOOD-*` requirements only when a finding becomes a durable contract, such as startup budget, no raw comment-control rendering, or candidate-detail layout behavior.

- [ ] **Step 5: Archive M24 only after the dogfooding batch is intentionally closed**

When the current batch is complete, move this plan to `docs/plans/archive/`, update `docs/roadmap.md`, and keep future dogfooding rounds as a new plan or a reopened M24 continuation only if the scope is still the same browser playground.
