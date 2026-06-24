# M24 TypeDuck-Web Dogfooding & Demo Hardening Implementation Plan

> **Status:** Complete - **Milestone:** M24 (TypeDuck-Web dogfooding and demo hardening) - **Created:** 2026-06-21 - **Closed:** 2026-06-21 - **Type:** archived execution plan / issue ledger
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Archive note:** This file preserves the original execution checklist. The unchecked task boxes below are historical instructions, not live work. The issue ledger rows marked `Closed`, the evidence under `apps/yune-web/e2e/results/m24-dogfooding/`, and the roadmap/requirements status are the authoritative M24 closeout record. Start a new scoped dogfooding plan for future browser-demo reports instead of editing this archived plan.

**Goal:** Close the first real manual play-testing batch for the internal TypeDuck-Web playground as a tracked, evidence-backed hardening loop without reopening completed parity milestones by accident.

**Architecture:** M24 treated `apps/yune-web/` as the browser dogfooding surface for this closed batch. Browser loading, rendering, and UX defects were fixed with browser evidence; candidate-set, candidate-order, or engine-semantic changes still required pinned oracle evidence from TypeDuck `v1.1.2` or upstream `1.17.0` before changing engine behavior. Each issue was classified first, then implemented in the narrowest owning layer.

**Tech Stack:** TypeDuck-Web React/TypeScript, Vite/Bun, Tailwind CSS, small local React components, Playwright, `@yune-ime/typeduck-runtime`, `yune-rime-api` C ABI, `yune-core`, TypeDuck `v1.1.2` oracle fixtures. M24 explicitly removes DaisyUI from the dogfood demo stack.

---

## Closeout Summary

M24 closed `M24-DOGFOOD-01` through `M24-DOGFOOD-13` and kept the work scoped to the internal TypeDuck-Web dogfood harness. The closed batch includes startup evidence, raw comment-control cleanup, compound-candidate detail layout, pinned TypeDuck `v1.1.2` candidate-order fixture coverage, Cantonese-first settings labels, simplified display-language controls, page-size wiring, horizontal/vertical candidate-menu display, explanatory status badges, real schema names, Jyutping Mandarin-pinyin reverse lookup, the full typeface picker, and DaisyUI removal.

The maintained TypeDuck-Web patch was regenerated at `apps/yune-web/patches/yune-web-runtime.patch`. Browser evidence is stored under `apps/yune-web/e2e/results/m24-dogfooding/`; future browser-demo issues should use a new plan and a new evidence scope unless they are explicitly auditing this archived baseline.

Screenshot evidence is a visual audit trail, while behavior is proved by the JSON/state snapshots, Rust fixture tests, runtime tests, and Playwright assertions named in each closed row. Do not treat a screenshot filename alone as proof of a distinct browser state.

Reported verification for closeout: `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, focused M24 Rust tests, `npm --prefix packages/yune-typeduck-runtime test`, `npm --prefix packages/yune-typeduck-runtime run build`, `npm --prefix apps/yune-web/source run build`, M24 Playwright E2E with 13 passing tests, TypeDuck-Web patch reverse/forward checks, and `git diff --check`.

---

## Scope

M24 was a dogfooding and hardening batch for the internal browser playground:

- **In scope:** first-load performance, visible loading state, candidate panel layout, comment rendering, dictionary panel ergonomics, browser schema-switch behavior, TypeDuck-Web runtime integration bugs, and browser evidence for user-visible fixes.
- **In scope with oracle evidence:** candidate set, candidate order, phrase/sentence fallback, correction, prediction, dictionary lookup payloads, and any behavior that changes engine output.
- **In scope for frontend stack hardening:** keep the dogfood demo on Vite + React + Tailwind CSS + small local components only; remove DaisyUI and do not add a replacement UI framework. The replacement UI does **not** need to preserve the original DaisyUI visual style; it should be minimalistic, elegant, readable, and built for engine dogfooding.
- **Out of scope:** changing the separately cloned or deployed `TypeDuck-HK/TypeDuck-Web` product; claiming live-site behavior as the hard oracle; widening default `RimeApi` or `RimeCandidate`; exposing unsupported controls as working toggles; turning the dogfood demo into a product site or design-system project.

## Execution Order

The issue ledger is append-only, but implementation should follow this order unless a fresh blocker makes a different sequence necessary:

1. **Evidence harness first:** Task 1. All later browser fixes must use the same M24 evidence directory and helper path.
2. **Browser/runtime correctness:** Tasks 2, 3, 5, 8, and 12 (`M24-DOGFOOD-01`, `02`, `04`, `07`, `11`). These either affect loading, rendered candidate data, runtime settings, or candidate output and should establish the honest baseline before broad UI work.
3. **Shared settings and display structure:** Tasks 6, 7, 9, and 14 (`M24-DOGFOOD-05`, `06`, `08`, `13`). These reshape common controls, grouping, and local component primitives; doing them before smaller polish avoids repeated UI churn.
4. **Focused display polish:** Tasks 4, 10, 11, and 13 (`M24-DOGFOOD-03`, `09`, `10`, `12`). These can land after the broader settings/component structure is stable.
5. **Final regression and docs closeout:** Task 15. Do not archive M24 until the current dogfooding batch is intentionally closed.

## Worker Guardrails

- Keep the web dogfood stack simple: **Vite + React + Tailwind CSS + small local components**.
- Do not add shadcn, MUI, Radix, another Tailwind component kit, a router framework, a CSS-in-JS stack, or a broad design system.
- Remove DaisyUI in `M24-DOGFOOD-13`; until that task lands, avoid adding new DaisyUI-specific classes to new work.
- Do not preserve DaisyUI styling for its own sake. Preserve behavior, accessibility, and test hooks, while allowing the UI to become more minimalistic and elegant.
- Reuse and improve local components in `apps/yune-web/source/src/Inputs.tsx` and local CSS in `apps/yune-web/source/src/index.css` as logical upstream-app edit targets, but do not treat `source/` edits as landed until the tracked patch is regenerated and checked.
- `apps/yune-web/source/` is a gitignored upstream checkout. The committed Yune source of truth is `apps/yune-web/patches/yune-web-runtime.patch`, plus intentionally Yune-owned files under `apps/yune-web/yune-integration/`, `apps/yune-web/e2e/`, and `apps/yune-web/yune-web.lock.json`.
- Every task that lists `apps/yune-web/source/...` names the file to edit in the local checkout while developing. The same slice must also update the maintained patch, and the final staged diff must not rely on untracked `source/` changes.
- Browser-visible claims require Playwright screenshots or JSON evidence from the real `/web/` app.
- Engine-output changes require pinned oracle fixture evidence. Do not use the live deployed TypeDuck site as the hard oracle.
- Commit in small slices. Keep implementation commits scoped to the issue being closed.

## Patch-Layer Execution Rule

M24 workers may edit `apps/yune-web/source/` while testing the local app, but that directory is not vendored in this repository. Before any browser UI or TypeDuck-Web source change is considered complete:

1. Regenerate `apps/yune-web/patches/yune-web-runtime.patch` from the patched upstream checkout.
2. Reverse-check the patch from `apps/yune-web/source/`:

   ```powershell
   git apply --reverse --check ..\patches\yune-web-runtime.patch
   ```

3. Forward-check the patch on a clean source checkout reset to `apps/yune-web/yune-web.lock.json`:

   ```powershell
   git apply --check ..\patches\yune-web-runtime.patch
   ```

4. Stage only the tracked artifacts for the slice: the patch, Yune-owned integration files, Playwright tests/evidence, Rust/runtime files, docs, and lock metadata when the upstream source pin changes.

If a task only edits `source/` and does not update the patch, it is not complete.

## Minimal UI Design Direction

This dogfood app is a debugging workbench, not a product landing page. The M24 UI should be minimalistic, elegant, and dense enough for repeated engine testing. It does not need to preserve the original DaisyUI look.

- Use one-control-per-job local primitives: `Field`, `Switch`, `Checkbox`, `RadioList`, `Segmented`, `Slider`, `Button`, `IconButton`, `Tag`, and `Section`.
- Put Cantonese first in visible labels, with English secondary where helpful.
- Give active engine controls and live session controls one short help line each. Keep display controls compact without explanatory paragraphs.
- Separate settings into clear groups: active engine controls, live session controls, display controls, web frontend controls, and Yune inspector/debug controls.
- Display-language selection is checklist-only; derive any primary dictionary/comment display internally from selected language order.
- Candidate rows stay compact. Long dictionary glosses and multilingual explanations belong below the selected/hovered candidate or in the dictionary/detail panel, not inline inside the horizontal candidate strip.
- Candidate menu orientation is a frontend-only display setting. Switching horizontal/vertical layout must not change engine output, ranking, page size, or ABI behavior.
- Status badges should be labeled diagnostic tags with a short status-strip hint, not raw booleans.
- The typeface picker should be a radio list of full font-family names. Do not show ambiguous `Sung`/`Hei` category toggles.

## Evidence Rules

- Save M24 browser evidence under `apps/yune-web/e2e/results/m24-dogfooding/<issue-id>/`.
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
| --- | --- | --- | --- | --- | --- | --- |
| M24-DOGFOOD-01 | Closed | Browser integration / performance | First visit to `http://localhost:5173/web/` remains on `載入中 Loading...` for too long. | Fresh browser tab to `/web/`; user observed a long loading period before the IME becomes usable. | `apps/yune-web/source/src/worker.ts`, `apps/yune-web/source/src/yune-integration/adapter.ts`, `apps/yune-web/source/src/yune-integration/assets.ts`, `packages/yune-typeduck-runtime/`, `apps/yune-web/e2e/yune-typeduck.spec.ts` | Closed with startup marker and resource evidence under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-01/`; owned by Playwright `M24 startup timing trace records loading phases`. |
| M24-DOGFOOD-02 | Closed | Browser integration / comment rendering | Long phrase candidates show a literal `\f` before Jyutping on following single-character candidates, while single-character input does not show it. | Type a long phrase such as `jigaajiusihaa`; screenshot shows candidates like `以 \fji5`. | `apps/yune-web/source/src/CandidateInfo.ts`, `apps/yune-web/source/src/Candidate.tsx`, `apps/yune-web/e2e/yune-typeduck.spec.ts`, candidate comments from `crates/yune-rime-api/tests/typeduck_web.rs` | Closed with clean candidate-row evidence under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-02/`; owned by Playwright `M24 phrase comments render without raw control markers`. |
| M24-DOGFOOD-03 | Closed | UI polish / candidate layout | Compound-candidate dictionary glosses render horizontally next to the candidate text, making `而家要思考(compound 詞組) 而家 (now) 要 (want; need) 思考 (think; ponder)` read like one confusing inline candidate. | Type `jigaajiusihaa`; screenshot shows the first highlighted candidate widened across the horizontal row. | `apps/yune-web/source/src/Candidate.tsx`, `apps/yune-web/source/src/CandidatePanel.tsx`, `apps/yune-web/source/src/DictionaryPanel.tsx`, `apps/yune-web/source/src/index.css` | Closed with compact row and dictionary-panel evidence under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-03/`; owned by Playwright `M24 compound candidate rows stay compact with details in the dictionary panel`. |
| M24-DOGFOOD-04 | Closed | Engine correctness / oracle recheck with version-skew risk | For `jigaajiusihaa`, after the first compound candidate the next candidates are single characters, while the user-observed live TypeDuck behavior appears to prefer word entries such as `而家`, `依家`, `宜家` before single characters. | User compared the internal playground with `https://www.typeduck.hk/web/`; live product appears to show word candidates in positions 2-3. Current Yune behavior is already influenced by the M21-pinned `prediction_candidate_limit=1` TypeDuck profile rule, so this must be fixture-first. | `scripts/capture-typeduck-jyutping.ps1`, `crates/yune-core/tests/cantonese_parity.rs`, `crates/yune-core/src/translator/mod.rs`, `crates/yune-core/src/dictionary/`, `crates/yune-rime-api/tests/typeduck_web.rs`, M21 source-aware evidence under `apps/yune-web/e2e/results/m21-product-comparison/` | Closed with pinned TypeDuck `v1.1.2` fixture `jyut6ping3-m24-dogfooding.json` and browser order evidence under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-04/`; owned by `typeduck_v112_m24_dogfooding_fixture_is_locked`, `m24_jigaajiusihaa_order_matches_typeduck_v112_dogfood_fixture`, and Playwright `M24 jigaajiusihaa order is recorded against the current pinned expectation`. |
| M24-DOGFOOD-05 | Closed | UI polish / settings localization and help text | Settings and developer controls mix Cantonese/English and many labels are English-only, so a new developer cannot tell what active engine controls or live session controls do. Cantonese should come first for all labels; active engine and live session toggles need short description text. Display controls need Cantonese-first labels but no extra descriptions. | Browser comment on `/web/` settings area: selected controls include `Active engine controls`, `Live session controls`, `Display controls`, `Yune inspector`, `Schema`, and English-only toggle labels such as `ASCII mode`, `Full shape`, `Prediction threshold`, and `Dictionary exclude`. | `apps/yune-web/source/src/Preferences.tsx`, `apps/yune-web/source/src/Inputs.tsx`, `apps/yune-web/source/src/Toolbar.tsx`, `apps/yune-web/source/src/SchemaSwitcher.tsx`, `apps/yune-web/source/src/App.tsx`, `apps/yune-web/source/src/YuneInspector.tsx`, `apps/yune-web/e2e/yune-typeduck.spec.ts` | Closed with desktop and narrow settings evidence under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-05/`; owned by Playwright `M24 settings labels are Cantonese-first and grouped by engine, session, display, and frontend`. |
| M24-DOGFOOD-06 | Closed | UI polish / display-language control semantics | The display-language fieldset shows both radio buttons and checkboxes, making it unclear whether the radio or checklist controls dictionary/comment language display. The visible UI should be a checklist only. | Browser comment on `/web/` display controls: `Display languages` shows five radio buttons on the left, five checkboxes on the right, and an arrow row for `主要語言 Main Language`. | `apps/yune-web/source/src/Preferences.tsx`, `apps/yune-web/source/src/Inputs.tsx`, `apps/yune-web/source/src/CandidateInfo.ts`, `apps/yune-web/source/src/DictionaryPanel.tsx`, `apps/yune-web/e2e/yune-typeduck.spec.ts` | Closed with checklist and candidate evidence under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-06/`; owned by Playwright `M24 display languages use a checklist with deterministic primary language`. |
| M24-DOGFOOD-07 | Closed | Browser integration / customize page-size wiring | The `每頁候選詞數量 No. of Candidates Per Page` slider appears not to control candidate page size; typing after selecting a smaller value still shows more candidates than selected. | Browser comment on `/web/` settings area: user changed the candidate-number control, then typed input whose candidate row clearly exceeded the selected page size. | `apps/yune-web/source/src/Preferences.tsx`, `apps/yune-web/source/src/App.tsx`, `apps/yune-web/source/src/yune-integration/adapter.ts`, `apps/yune-web/e2e/yune-typeduck.spec.ts`, `crates/yune-rime-api/tests/typeduck_web.rs`, `crates/yune-rime-api/src/typeduck_web.rs`, `crates/yune-rime-api/src/context_api.rs` | Closed with page-size 4/10 evidence under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-07/`; owned by `typeduck_adapter_real_assets_page_size_customize_limits_context_page` and Playwright `M24 candidate page-size slider limits the visible candidate page`. |
| M24-DOGFOOD-08 | Closed | UI polish / frontend candidate-menu layout | The playground has no control for horizontal versus vertical candidate menu layout. Users familiar with RIME expect a menu style choice, but this web setting should be clearly grouped as a frontend display preference rather than an engine/schema control. | Browser comment on `/web/` settings area: user requested a horizontal/vertical candidate list control and clearer grouping that distinguishes engine controls from web frontend controls. | `apps/yune-web/source/src/Preferences.tsx`, `apps/yune-web/source/src/types.ts`, `apps/yune-web/source/src/consts.ts`, `apps/yune-web/source/src/CandidatePanel.tsx`, `apps/yune-web/source/src/Candidate.tsx`, `apps/yune-web/source/src/index.css`, `apps/yune-web/e2e/yune-typeduck.spec.ts` | Closed with horizontal/vertical screenshots and identical engine-output signatures under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-08/`; owned by Playwright `M24 candidate menu layout is a frontend-only horizontal or vertical setting`. |
| M24-DOGFOOD-09 | Closed | UI polish / engine status strip explanation | The status strip under the schema switcher shows raw badges such as `jyut6ping3_mobile`, `enabled`, `not traditional`, and `Chinese` with no hint explaining what the strip is or what each value means. | Browser comment on `/web/` status strip: selected `jyut6ping3_mobile enabled not traditional Chinese`; user requested a UI hint for what this is. | `apps/yune-web/source/src/YuneStatusStrip.tsx`, `apps/yune-web/source/src/App.tsx`, `apps/yune-web/e2e/yune-typeduck.spec.ts`, `apps/yune-web/source/src/index.css` | Closed with status JSON and screenshot under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-09/`; owned by Playwright `M24 engine status strip explains labeled state`. |
| M24-DOGFOOD-10 | Closed | UI polish / schema switcher names | The schema switcher uses English-ish labels such as `Jyutping`, `Cangjie 5`, and `Luna Pinyin`, even though the bundled schema YAML has real names: `粵語拼音`, `倉頡五代`, and `普通話`. The UI should show the real schema names where possible, with romanized/English helper text only as secondary text. | Browser comment on `/web/` schema switcher: user asked whether the real names should be `粵拼` / `倉頡五代` instead of English spellings. Local schema check: `jyut6ping3_mobile.schema.yaml` has `schema/name: 粵語拼音`, `cangjie5.schema.yaml` has `schema/name: 倉頡五代`, and `luna_pinyin.schema.yaml` has `schema/name: 普通話`. | `apps/yune-web/source/src/consts.ts`, `apps/yune-web/source/src/SchemaSwitcher.tsx`, `apps/yune-web/source/schema/*.schema.yaml`, `apps/yune-web/source/src/YuneStatusStrip.tsx`, `apps/yune-web/e2e/yune-typeduck.spec.ts` | Closed with schema-switcher screenshot under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-10/`; owned by Playwright `M24 schema switcher uses bundled real schema names`. |
| M24-DOGFOOD-11 | Closed | Browser integration / reverse lookup dogfood | The web dogfood does not visibly support the expected Jyutping Mandarin-pinyin lookup flow: with Jyutping active, typing the `luna_pinyin` affix path, for example `` `pzhe ``, should offer `這` as a candidate. Bare `` `zhe `` is the generic reverse-lookup tag in the full Jyutping schema, not the Mandarin pinyin affix path. | User feedback on `/web/`: expected backtick + Mandarin pinyin to show `這`; local schema check shows `jyut6ping3.schema.yaml` uses `luna_pinyin` prefix `` `p `` and a `;` suffix, while `jyut6ping3_mobile` does not expose that full recognizer/translator path. Core and ABI have reverse-lookup translator/filter tests, and `typeduck_web.rs` already proves browser app assets can reverse lookup for Cangjie/Luna schemas. | `apps/yune-web/source/schema/jyut6ping3_mobile.schema.yaml`, `apps/yune-web/source/schema/jyut6ping3.schema.yaml`, `apps/yune-web/source/schema/luna_pinyin.*`, `apps/yune-web/source/src/consts.ts`, `apps/yune-web/source/src/App.tsx`, `apps/yune-web/source/src/yune-integration/adapter.ts`, `apps/yune-web/e2e/yune-typeduck.spec.ts`, `crates/yune-rime-api/tests/typeduck_web.rs`, `crates/yune-rime-api/src/typeduck_web.rs`, `crates/yune-rime-api/src/schema_install.rs` | Closed with `pzhe` JSON/screenshot under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-11/`; owned by `typeduck_adapter_browser_app_assets_load_jyutping_mandarin_pinyin_reverse_lookup` and Playwright `M24 Jyutping reverse lookup accepts Mandarin pinyin affix input`. |
| M24-DOGFOOD-12 | Closed | UI polish / Chinese typeface picker | The `中文字體 Chinese Typeface` control is a two-button `宋體 Sung` / `黑體 Hei` toggle backed by `isHeiTypeface`, which is ambiguous and hides the actual font family. It should become a radio list using full font family names. | Browser comment on `/web/` display controls: user selected `中文字體 Chinese Typeface 宋體 Sung 黑體 Hei` and requested a radio list with full names. Official Google Fonts pages confirm the requested family names: Chocolate Classical Sans, Iansui, LXGW WenKai Mono TC, LXGW WenKai TC, WDXL Lubrifont TC, Chiron GoRound TC, Chiron Hei HK, and Chiron Sung HK. | `apps/yune-web/source/src/types.ts`, `apps/yune-web/source/src/consts.ts`, `apps/yune-web/source/src/hooks.ts`, `apps/yune-web/source/src/Preferences.tsx`, `apps/yune-web/source/src/App.tsx`, `apps/yune-web/source/src/Candidate.tsx`, `apps/yune-web/source/src/DictionaryPanel.tsx`, `apps/yune-web/source/src/index.css`, `apps/yune-web/source/tailwind.config.ts`, `apps/yune-web/e2e/yune-typeduck.spec.ts` | Closed with font-resource JSON and screenshot under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-12/`; owned by Playwright `M24 Chinese typeface picker applies full family names to visible Chinese surfaces`. |
| M24-DOGFOOD-13 | Closed | UI stack simplification / DaisyUI removal | The dogfood demo still depends on DaisyUI even though the intended stack is Vite + React + Tailwind CSS + small local components. DaisyUI classes such as `btn`, `toggle`, `radio`, `textarea`, `badge`, and `join` are used across the UI, and `tailwind.config.ts` imports the DaisyUI plugin. | User confirmed the desired stack should be simple because this is a debugging and stress-testing dogfood demo, not a large product site. The replacement UI does not need to keep the current DaisyUI look; it should be minimalistic and elegant while staying readable for debugging. Local code check: `apps/yune-web/source/package.json` lists `daisyui`, `tailwind.config.ts` imports and configures it, and local components currently wrap DaisyUI classes. | `apps/yune-web/source/package.json`, `apps/yune-web/source/tailwind.config.ts`, `apps/yune-web/source/src/Inputs.tsx`, `apps/yune-web/source/src/index.css`, `apps/yune-web/source/src/Toolbar.tsx`, `apps/yune-web/source/src/ThemeSwitcher.tsx`, `apps/yune-web/source/src/YuneFeatureShowcase.tsx`, `apps/yune-web/source/src/App.tsx`, `apps/yune-web/source/src/Preferences.tsx`, `apps/yune-web/source/src/CandidatePanel.tsx`, `apps/yune-web/e2e/yune-typeduck.spec.ts` | Closed with local Tailwind screenshot under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-13/`; owned by Playwright `M24 dogfood UI uses only local Tailwind components` and `npm.cmd --prefix apps/yune-web/source run build`. |

---

## Task 1: Baseline Capture And M24 Evidence Harness

**Files:**

- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Create evidence during execution: `apps/yune-web/e2e/results/m24-dogfooding/`

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
npm.cmd --prefix apps/yune-web/source run build
```

Expected: the worker bundle and Vite build complete without TypeScript errors.

- [ ] **Step 4: Commit the evidence-harness-only slice if it is useful independently**

Stage only the files changed for M24 evidence plumbing:

```powershell
git add -- apps/yune-web/e2e/yune-typeduck.spec.ts
git commit -m "test: add M24 TypeDuck-Web dogfooding evidence helpers"
```

## Task 2: M24-DOGFOOD-01 Startup Loading Performance

**Files:**

- Modify: `apps/yune-web/source/src/worker.ts`
- Modify: `apps/yune-web/source/src/yune-integration/adapter.ts`
- Modify: `apps/yune-web/source/src/yune-integration/assets.ts`
- Inspect first, then modify only if the timing trace points there: `packages/yune-typeduck-runtime/src/*`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

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

- Modify: `apps/yune-web/source/src/CandidateInfo.ts`
- Inspect first, then modify only if parsing cannot be fixed in `CandidateInfo.ts`: `apps/yune-web/source/src/Candidate.tsx`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`
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
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 phrase candidates"
```

Expected: no visible `\f`, `\r`, or `\v` in candidate rows.

## Task 4: M24-DOGFOOD-03 Candidate Gloss Layout

**Files:**

- Modify: `apps/yune-web/source/src/Candidate.tsx`
- Modify: `apps/yune-web/source/src/CandidatePanel.tsx`
- Modify: `apps/yune-web/source/src/DictionaryPanel.tsx`
- Modify: `apps/yune-web/source/src/index.css`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

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
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 compound gloss layout"
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
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Treat the live site as a clue, not the oracle**

Record the user observation as:

```text
Input: jigaajiusihaa
Internal playground observed: top compound candidate, then single-character rows.
Live product observed by user: word candidates such as 而家 / 依家 / 宜家 appear before single-character rows.
Hard-oracle status: not captured yet for this exact input.
Existing Yune constraint: M21 pinned `jyut6ping3` prediction behavior to one long prediction before single-character rows via `prediction_candidate_limit=1`.
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

Add a focused test in `crates/yune-core/tests/cantonese_parity.rs`. Extend the existing fixture constants and helpers in the same style as `m21_closeout_fixture()`, `m21_closeout_case(...)`, and `selected_candidate_text(...)`:

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

This uses the production `typeduck_jyut6ping3_mobile_engine(false)` helper; do not invent a parallel engine harness.

- [ ] **Step 4: Diagnose source ordering before changing code**

Use existing source-aware diagnostics from the M21 pattern to classify candidates as sentence, table, prediction, user, correction, or fallback. The fix should answer one of these questions with evidence:

- Are word entries present in the dictionary but filtered out?
- Are word entries present but ranked after single-character entries?
- Does the sentence fallback return only one compound row and suppress the rest of the word path?
- Is the live product behavior version skew rather than TypeDuck `v1.1.2` behavior?
- Does the M21 prediction-candidate limit already explain the current order?

- [ ] **Step 5: Implement only the oracle-backed ordering fix**

If TypeDuck `v1.1.2` expects word candidates before single-character rows, adjust the narrow TypeDuck profile path. Keep default upstream `luna_pinyin` behavior untouched and keep TypeDuck-specific tuning behind explicit profile config. If TypeDuck `v1.1.2` matches the current compound-then-single-character order, document the deployed-site difference as version skew and do not change ranking.

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

- Modify: `apps/yune-web/source/src/Preferences.tsx`
- Modify: `apps/yune-web/source/src/Inputs.tsx`
- Modify: `apps/yune-web/source/src/Toolbar.tsx`
- Modify: `apps/yune-web/source/src/SchemaSwitcher.tsx`
- Modify: `apps/yune-web/source/src/App.tsx`
- Inspect first, then modify only if the inspector panel remains visible in the same developer settings flow: `apps/yune-web/source/src/YuneInspector.tsx`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

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

Save after screenshots under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-05/`, including a narrow viewport. Verify that the new helper text does not overlap switches, sliders, or display-control groups.

## Task 7: M24-DOGFOOD-06 Simplify Display-Language Selection

**Files:**

- Modify: `apps/yune-web/source/src/Preferences.tsx`
- Modify: `apps/yune-web/source/src/Inputs.tsx`
- Inspect first, then modify only if primary-definition behavior becomes unclear after removing visible radios: `apps/yune-web/source/src/CandidateInfo.ts`
- Inspect first, then modify only if dictionary-panel primary-definition behavior becomes unclear after removing visible radios: `apps/yune-web/source/src/DictionaryPanel.tsx`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Capture the current confusing control state**

Save the current display-language fieldset before changing behavior:

```ts
test("M24 display language selector uses one clear control type", async ({ page }) => {
  await page.goto(APP_URL, { timeout: TIMEOUT_MS, waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
  await takeM24Screenshot(page, "M24-DOGFOOD-06", "display-languages-before");

  const fieldset = page.locator("fieldset").filter({ hasText: "Display languages" });
  await expect(fieldset.getByRole("radio")).toHaveCount(5);
  await expect(fieldset.getByRole("checkbox")).toHaveCount(5);
});
```

Expected before the fix: the test documents that both radio and checkbox controls are visible.

- [ ] **Step 2: Replace the visible radio-plus-checkbox component with a checklist**

Remove `RadioCheckbox` from the display-language fieldset. Add a checkbox-only shared input if one does not already exist:

```tsx
export function Checkbox({ label, checked, setChecked }: CheckboxProps) {
  return <label className="cursor-pointer label gap-2">
    <span className="text-lg text-base-content-200 flex-1">{label}</span>
    <input
      type="checkbox"
      className="checkbox checkbox-primary"
      {...NO_AUTO_FILL}
      checked={checked}
      onChange={event => setChecked(event.target.checked)} />
  </label>;
}
```

Use that checkbox-only control for each `LANGUAGE_LABELS` entry:

```tsx
<fieldset className="border border-base-300 rounded px-3">
  <legend className="text-xl text-base-content mb-1 px-2">顯示語言 Display Languages</legend>
  {(Object.entries(LANGUAGE_LABELS) as [Language, string][]).map(([language, label]) =>
    <Checkbox
      key={language}
      label={label}
      checked={prefs.displayLanguages.has(language)}
      setChecked={checked => toggleDisplayLanguage(language, checked)} />
  )}
</fieldset>
```

If `RadioCheckbox` is no longer used anywhere after this change, delete it from `Inputs.tsx` instead of leaving dead UI code.

- [ ] **Step 3: Keep at least one display language selected**

Update `toggleDisplayLanguage` so the last checked language cannot be removed:

```ts
function toggleDisplayLanguage(language: Language, checked: boolean) {
  const newDisplayLanguages = new Set(prefs.displayLanguages);
  if (checked) {
    newDisplayLanguages.add(language);
  }
  else if (newDisplayLanguages.size > 1) {
    newDisplayLanguages.delete(language);
  }
  prefs.setDisplayLanguages(newDisplayLanguages);
}
```

This keeps the checklist honest: it chooses which dictionary/comment languages are displayed, and it never leaves the UI in an empty-language state.

- [ ] **Step 4: Preserve deterministic primary-definition behavior internally**

The current dictionary panel uses `prefs.mainLanguage` to choose the primary definition line. After removing the visible radio controls, keep that behavior deterministic without exposing a second control:

```ts
const orderedDisplayLanguages = (Object.keys(LANGUAGE_LABELS) as Language[])
  .filter(language => newDisplayLanguages.has(language));

if (!newDisplayLanguages.has(prefs.mainLanguage) && orderedDisplayLanguages[0]) {
  prefs.setMainLanguage(orderedDisplayLanguages[0]);
}
```

Checking a new language should not automatically steal primary status. Unchecking the current primary language should move primary status to the first still-checked language in the stable `LANGUAGE_LABELS` order. If the final implementation derives primary language instead of storing it, update `CandidateInfo.ts` and `DictionaryPanel.tsx` together so both surfaces use the same rule.

- [ ] **Step 5: Add browser assertions for the simplified control**

Replace the characterization assertions with the expected final behavior:

```ts
const fieldset = page.locator("fieldset").filter({ hasText: "顯示語言 Display Languages" });
await expect(fieldset.getByRole("radio")).toHaveCount(0);
await expect(fieldset.getByRole("checkbox")).toHaveCount(5);
await expect(fieldset).not.toContainText("主要語言 Main Language");
```

Then verify the checklist actually controls visible dictionary/comment languages:

```ts
await fieldset.getByLabel("印地語 Hindi").check();
await fieldset.getByLabel("英語 English").uncheck();
await captureM24Phrase(page, "M24-DOGFOOD-06", "jigaajiusihaa", "而家要思考");
await takeM24Screenshot(page, "M24-DOGFOOD-06", "display-languages-after-hindi");
```

This checks Hindi first, then unchecks English, so the at-least-one-language guard remains covered.

- [ ] **Step 6: Verify layout after the simplification**

Save after screenshots under `apps/yune-web/e2e/results/m24-dogfooding/M24-DOGFOOD-06/` for desktop and narrow viewports. The fieldset should show one checkbox column/list only, with no radio column, no arrow row, and no ambiguous "main language" hint.

## Task 8: M24-DOGFOOD-07 Candidate Page-Size Control Wiring

**Files:**

- Modify: `apps/yune-web/source/src/yune-integration/adapter.ts`
- Inspect first, then modify only if the React state/effect is not firing: `apps/yune-web/source/src/App.tsx`
- Inspect first, then modify only if the slider emits the wrong value: `apps/yune-web/source/src/Preferences.tsx`
- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Modify: `crates/yune-rime-api/tests/typeduck_web.rs`
- Inspect first, then modify only if the bridge cannot customize the deployed key correctly: `crates/yune-rime-api/src/typeduck_web.rs`
- Inspect first, then modify only if the deployed context ignores the customized key: `crates/yune-rime-api/src/context_api.rs`

- **Known root-cause hypothesis:** this is not primarily a React wiring bug. The current patch/integration adapter maps the page-size preference through `customizeSetting("page_size", ...)`, while the deployed schema/context path reads `menu/page_size`. Prove or disprove that key mismatch first.

- [ ] **Step 1: Capture the browser failure against the real settings control**

Add a focused Playwright test that sets the slider to 4, types an input with more than four candidates, and records both the visible candidate rows and the runtime context:

```ts
test("M24 candidate page-size slider limits the visible candidate page", async ({ page }) => {
  await page.goto(APP_URL, { timeout: TIMEOUT_MS, waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
  await setPreferenceRange(page, /No\. of Candidates Per Page|每頁候選詞數量/, 4);
  await waitForPersistedSettings(page, { "menu/page_size": "4" });

  const state = await captureM24Phrase(page, "M24-DOGFOOD-07", "jigaajiusihaa", "而家要思考");
  await saveM24Json("M24-DOGFOOD-07", "page-size-4-state.json", state);
  await takeM24Screenshot(page, "M24-DOGFOOD-07", "page-size-4-candidates");

  expect(state.candidates.length).toBeLessThanOrEqual(4);
});
```

Expected before the fix: either `waitForPersistedSettings` cannot find `menu/page_size: "4"`, or the captured candidate list contains more than four visible rows.

- [ ] **Step 2: Add a runtime bridge regression before changing browser code**

In `crates/yune-rime-api/tests/typeduck_web.rs`, add a real-assets test that proves the TypeDuck-Web bridge can customize the same deployed key the context reader uses:

```rust
#[test]
fn typeduck_adapter_real_assets_page_size_customize_limits_context_page() {
    let _guard = test_guard();
    let runtime =
        TypeDuckRuntime::create_with_schema("browser-real-page-size-customize", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    let key = CString::new("menu/page_size").expect("custom key should be valid");
    let value = CString::new("4").expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);

    let composing = process_input(state, "jigaajiusihaa");
    assert_eq!(composing["context"]["page_size"], Value::from(4));
    let candidates = composing["context"]["candidates"]
        .as_array()
        .expect("candidate page should be an array");
    assert!(
        candidates.len() <= 4,
        "customized page size should limit candidates, got {}",
        candidates.len()
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}
```

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_web typeduck_adapter_real_assets_page_size_customize_limits_context_page
```

Expected before the fix: this should pass if the bridge already supports `menu/page_size`; if it fails, fix the bridge/runtime before changing the browser adapter.

- [ ] **Step 3: Use the deployed schema key from the browser adapter**

`context_menu_settings(...)` reads `menu/page_size` from the deployed schema. The browser adapter currently maps page size through `customizeSetting("page_size", ...)`; update it to customize the deployed path:

```ts
if (preferences.pageSize !== undefined) {
  customizeSetting("menu/page_size", String(preferences.pageSize));
}
```

Do not change unrelated customization keys in the same slice. If the older flat `page_size` key is still needed for compatibility with a different frontend, support both explicitly and prove which one the current browser uses with persisted-settings evidence.

- [ ] **Step 4: Verify customize, deploy, and fresh composition ordering**

The React effect in `App.tsx` already calls `Rime.customize(...)` and then `Rime.deploy()`. Verify the M24 browser test waits for deploy completion before typing. If the test is flaky because composition starts before deploy finishes, add a page-visible or worker diagnostic wait that observes the customize/deploy completion used by existing `waitForPersistedSettings(...)` helpers.

Do not make the candidate renderer slice the full candidate array as the main fix. The runtime context should already expose only the current page. UI slicing is acceptable only as a defensive assertion after runtime `context.page_size` is correct.

- [ ] **Step 5: Add page-size roundtrip coverage for multiple values**

Extend the Playwright test to cover at least 4 and 10:

```ts
for (const pageSize of [4, 10] as const) {
  await setPreferenceRange(page, /No\. of Candidates Per Page|每頁候選詞數量/, pageSize);
  await waitForPersistedSettings(page, { "menu/page_size": String(pageSize) });
  const state = await typeCompositionAndWaitForTopCandidate(page, "jigaajiusihaa", "而家要思考");
  await saveM24Json("M24-DOGFOOD-07", `page-size-${pageSize}-state.json`, state);
  await takeM24Screenshot(page, "M24-DOGFOOD-07", `page-size-${pageSize}-candidates`);
  expect(state.candidates.length).toBeLessThanOrEqual(pageSize);
}
```

- [ ] **Step 6: Run focused gates**

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_web typeduck_adapter_real_assets_page_size_customize_limits_context_page
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 candidate page-size"
```

Expected: the runtime reports `context.page_size` equal to the selected value, the browser evidence shows no more visible candidates than selected, and the persisted settings JSON records `menu/page_size`.

## Task 9: M24-DOGFOOD-08 Frontend Candidate-Menu Layout Control

**Files:**

- Modify: `apps/yune-web/source/src/consts.ts`
- Modify: `apps/yune-web/source/src/types.ts`
- Modify: `apps/yune-web/source/src/Preferences.tsx`
- Modify: `apps/yune-web/source/src/CandidatePanel.tsx`
- Modify: `apps/yune-web/source/src/Candidate.tsx`
- Modify: `apps/yune-web/source/src/index.css`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Capture the current single-layout behavior**

Add a browser characterization that captures the existing horizontal layout before adding the control:

```ts
test("M24 candidate menu layout can switch between horizontal and vertical", async ({ page }) => {
  await page.goto(APP_URL, { timeout: TIMEOUT_MS, waitUntil: "domcontentloaded" });
  await waitForAppReady(page);

  const horizontal = await captureM24Phrase(page, "M24-DOGFOOD-08", "jigaajiusihaa", "而家要思考");
  await saveM24Json("M24-DOGFOOD-08", "horizontal-before-state.json", horizontal);
  await takeM24Screenshot(page, "M24-DOGFOOD-08", "horizontal-before");
});
```

Expected before the fix: there is no visible menu-layout control and the candidate panel always uses the existing horizontal table presentation.

- [ ] **Step 2: Add a frontend-only layout preference**

Define the preference in the interface layer, not `RimePreferences`, because it must not be sent through `Rime.customize(...)` or treated as schema behavior:

```ts
export const enum CandidateMenuLayout {
  Horizontal = "horizontal",
  Vertical = "vertical",
}

export const CANDIDATE_MENU_LAYOUT_LABELS: Record<CandidateMenuLayout, string> = {
  [CandidateMenuLayout.Horizontal]: "橫排 Horizontal",
  [CandidateMenuLayout.Vertical]: "直排 Vertical",
};
```

Add the default:

```ts
candidateMenuLayout: CandidateMenuLayout.Horizontal,
```

Add the type field to `InterfacePreferences`:

```ts
candidateMenuLayout: CandidateMenuLayout;
```

Do not add this field to `RimePreferences`, `customize(...)`, `yune-integration/adapter.ts`, or any C ABI/runtime response type.

- [ ] **Step 3: Put frontend controls in a visibly separate settings group**

Keep active engine controls and live session controls separate from frontend-only display preferences. In `Preferences.tsx`, rename or structure the display area so the user can see which controls affect the web UI only:

```tsx
<h4 className="font-semibold text-xl my-2">網頁顯示設定 Web Frontend Controls</h4>
```

Place `候選排版 Candidate Menu Layout` in this frontend group near page size, Chinese typeface, display languages, candidate Jyutping, reverse-code display, and Cangjie version. Do not move engine-affecting controls such as completion, correction, sentence mode, learning, prediction threshold, dictionary exclude, or live RIME options into this group.

- [ ] **Step 4: Add the segmented layout control**

Use the existing `Segment` pattern:

```tsx
<li>
  <div className="label gap-2">
    <span className="text-lg text-base-content-200">候選排版 Candidate Menu Layout</span>
    <div className="join">
      {(Object.entries(CANDIDATE_MENU_LAYOUT_LABELS) as [CandidateMenuLayout, string][]).map(([layout, label]) =>
        <Segment
          key={layout}
          name="candidateMenuLayout"
          label={label}
          state={prefs.candidateMenuLayout}
          setState={prefs.setCandidateMenuLayout}
          value={layout} />
      )}
    </div>
  </div>
</li>
```

The control is intentionally a web preference. It should persist through the existing preference hook like other interface preferences, but it should not trigger deploy or runtime customization.

- [ ] **Step 5: Pass layout into the candidate panel presentation**

Use the interface preference to add stable layout classes:

```tsx
return inputState && <CaretFollower
  textArea={textArea}
  className={`candidate-panel candidate-panel-${prefs.candidateMenuLayout}`}>
```

Keep candidate selection indices, digit-key selection, pagination, delete behavior, dictionary hover/touch behavior, and AI source badges unchanged.

- [ ] **Step 6: Implement vertical layout in CSS without changing candidate data**

Keep the existing horizontal layout as the default. Add vertical CSS that stacks candidates top-to-bottom while retaining compact candidate rows:

```css
.candidate-panel-horizontal {
  @apply flex-row;
}

.candidate-panel-horizontal .candidates {
  @apply table;
}

.candidate-panel-vertical {
  @apply flex-col whitespace-normal min-w-72 max-w-[min(28rem,calc(100vw-2rem))];
}

.candidate-panel-vertical .candidates {
  @apply block;
}

.candidate-panel-vertical .candidates tbody {
  @apply block rounded-md;
}

.candidate-panel-vertical .candidates tr {
  @apply flex items-baseline gap-2;
}

.candidate-panel-vertical .candidates td {
  @apply block;
}
```

If screenshots show overlap or clipping, revise only these bounded spacing utilities in this order: change `min-w-72` to `min-w-80`, change `max-w-[min(28rem,calc(100vw-2rem))]` to `max-w-[min(34rem,calc(100vw-2rem))]`, then change `gap-2` to `gap-3`. Keep the behavior constraints: vertical mode stacks candidates; horizontal mode remains visually equivalent to today; neither mode changes candidate order or count.

- [ ] **Step 7: Add browser assertions for both layouts**

Extend the Playwright test to switch the segmented control and assert the class plus visible content:

```ts
await expect(page.getByText("候選排版 Candidate Menu Layout")).toBeVisible();
await page.getByLabel("直排 Vertical").check();
const vertical = await typeCompositionAndWaitForTopCandidate(page, "jigaajiusihaa", "而家要思考");
await saveM24Json("M24-DOGFOOD-08", "vertical-state.json", vertical);
await takeM24Screenshot(page, "M24-DOGFOOD-08", "vertical");
await expect(page.locator(".candidate-panel-vertical")).toBeVisible();
expect(vertical.candidates.map(candidate => candidate.text)).toEqual(horizontal.candidates.map(candidate => candidate.text));

await page.getByLabel("橫排 Horizontal").check();
const horizontalAgain = await typeCompositionAndWaitForTopCandidate(page, "jigaajiusihaa", "而家要思考");
await takeM24Screenshot(page, "M24-DOGFOOD-08", "horizontal-after");
await expect(page.locator(".candidate-panel-horizontal")).toBeVisible();
expect(horizontalAgain.candidates.map(candidate => candidate.text)).toEqual(horizontal.candidates.map(candidate => candidate.text));
```

If the existing `Segment` helper does not expose labels through `getByLabel`, use `getByRole("radio", { name: "直排 Vertical" })` and keep the accessibility label intact.

- [ ] **Step 8: Run focused gates**

Run:

```powershell
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 candidate menu layout"
```

Expected: both screenshots render clearly, the candidate text order is identical across layouts, and no runtime/customize/persisted engine setting changes are required.

## Task 10: M24-DOGFOOD-09 Explain The Engine Status Strip

**Files:**

- Modify: `apps/yune-web/source/src/YuneStatusStrip.tsx`
- Inspect first, then modify only if spacing needs adjustment: `apps/yune-web/source/src/index.css`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Capture the current raw status strip**

Add a browser characterization test that types once to make the strip appear and captures the current raw badges:

```ts
test("M24 engine status strip explains schema and mode badges", async ({ page }) => {
  await page.goto(APP_URL, { timeout: TIMEOUT_MS, waitUntil: "domcontentloaded" });
  await waitForAppReady(page);
  await typeInputForStatus(page, "nei");

  const status = page.locator("[data-yune-status]");
  await expect(status).toBeVisible();
  await saveM24Json("M24-DOGFOOD-09", "status-strip-before.json", await readYuneStatus(page));
  await takeM24Screenshot(page, "M24-DOGFOOD-09", "status-strip-before");
});
```

Expected before the fix: the strip contains raw text such as `jyut6ping3_mobile`, `enabled`, `not traditional`, and `Chinese` without a visible label or helper hint.

- [ ] **Step 2: Add a label and hint inside `YuneStatusStrip`**

Keep the component small and self-contained. Add a visible label and one-line explanation before the badges:

```tsx
const statusHint = "顯示目前輸入引擎狀態：方案、啟用狀態、繁化狀態同中英模式。";

return <section className="my-3 text-sm" data-yune-status aria-label="輸入引擎狀態 Engine Status">
  <div className="mb-1 font-semibold text-base-content">輸入引擎狀態 Engine Status</div>
  <p className="mb-2 text-xs text-base-content-300">{statusHint}</p>
  <div className="flex flex-wrap gap-2">
    {/* badges stay here */}
  </div>
</section>;
```

This is UI explanation only. Do not change `YuneStatusSnapshot`, runtime status fields, or worker response parsing.

- [ ] **Step 3: Replace raw badge text with labeled Cantonese-first text**

Keep the existing `data-yune-status-*` attributes so current tests and future diagnostics still find the values, but make the visible text explain each badge:

```tsx
<span className="badge badge-outline" data-yune-status-schema>
  方案 {status.schema_name || status.schema_id}
</span>
<span className="badge badge-outline" data-yune-status-disabled={status.is_disabled}>
  狀態 {status.is_disabled ? "停用 Disabled" : "啟用 Enabled"}
</span>
<span className="badge badge-outline" data-yune-status-traditional={status.is_traditional}>
  繁化 {status.is_traditional ? "開 On" : "關 Off"}
</span>
<span className="badge badge-outline" data-yune-status-ascii={status.is_ascii_mode}>
  模式 {status.is_ascii_mode ? "英文 ASCII" : "中文 Chinese"}
</span>
```

If `status.schema_name` is empty or unavailable for a schema, fall back to `status.schema_id` exactly as shown.

- [ ] **Step 4: Add browser assertions for the clearer copy**

Extend the M24 status test to assert the label, hint, and labeled badges:

```ts
await expect(page.getByText("輸入引擎狀態 Engine Status")).toBeVisible();
await expect(page.getByText("顯示目前輸入引擎狀態：方案、啟用狀態、繁化狀態同中英模式。")).toBeVisible();
await expect(page.locator("[data-yune-status-schema]")).toContainText(/方案/);
await expect(page.locator("[data-yune-status-disabled]")).toContainText(/狀態/);
await expect(page.locator("[data-yune-status-traditional]")).toContainText(/繁化/);
await expect(page.locator("[data-yune-status-ascii]")).toContainText(/模式/);
await takeM24Screenshot(page, "M24-DOGFOOD-09", "status-strip-after");
```

Keep the existing `readYuneStatus(...)` helper valid by preserving `data-yune-status-schema`, `data-yune-status-disabled`, `data-yune-status-traditional`, and `data-yune-status-ascii`.

- [ ] **Step 5: Verify the strip updates after live option changes**

Add a short assertion that the labels remain understandable after toggling live options:

```ts
await setPreferenceToggle(page, /ASCII mode|英文模式/, true);
await typeInputForStatus(page, "abc");
await expect(page.locator("[data-yune-status-ascii]")).toContainText("英文 ASCII");

await setPreferenceToggle(page, /Traditionalization|繁化/, true);
await typeInputForStatus(page, "nei");
await expect(page.locator("[data-yune-status-traditional]")).toContainText("開 On");
```

If the label-localization task has already renamed the toggles, use the Cantonese-first labels in the regex and keep the English fallback in the same regex.

- [ ] **Step 6: Run focused gates**

Run:

```powershell
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 engine status strip"
```

Expected: the strip is labeled and explained, status badges still update from the runtime status object, and existing status tests continue to locate the same `data-yune-status-*` attributes.

## Task 11: M24-DOGFOOD-10 Show Real Schema Names In The Switcher

**Files:**

- Modify: `apps/yune-web/source/src/consts.ts`
- Modify: `apps/yune-web/source/src/SchemaSwitcher.tsx`
- Inspect first, then modify only if needed for agreement: `apps/yune-web/source/src/YuneStatusStrip.tsx`
- Read-only oracle metadata: `apps/yune-web/source/schema/jyut6ping3_mobile.schema.yaml`, `apps/yune-web/source/schema/cangjie5.schema.yaml`, `apps/yune-web/source/schema/luna_pinyin.schema.yaml`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Capture the current schema switcher labels**

Add a characterization test that records the current English-ish switcher before changing the UI:

```ts
test("M24 schema switcher shows real schema names", async ({ page }) => {
  await page.goto(APP_URL, { timeout: TIMEOUT_MS, waitUntil: "domcontentloaded" });
  await waitForAppReady(page);

  const switcher = page.locator("[data-yune-schema-switcher]");
  await expect(switcher).toBeVisible();
  await expect(switcher).toContainText("Jyutping");
  await expect(switcher).toContainText("Cangjie 5");
  await expect(switcher).toContainText("Luna Pinyin");
  await takeM24Screenshot(page, "M24-DOGFOOD-10", "schema-switcher-before");
});
```

Expected before the fix: the schema names are shown as `Jyutping`, `Cangjie 5`, and `Luna Pinyin`, while the real bundled schema names are only available in YAML/status metadata.

- [ ] **Step 2: Make schema option metadata carry checked real names**

Update `SCHEMA_OPTIONS` so each entry records the real `schema/name` value from the bundled YAML. Do not guess from the schema ID.

```ts
export interface SchemaOption {
  id: RimeSchemaId;
  label: string;
  shortLabel: string;
  schemaName: string;
  romanizationLabel: string;
  reverseLookup: string;
}

export const SCHEMA_OPTIONS: readonly SchemaOption[] = [
  {
    id: "jyut6ping3_mobile",
    schemaName: "粵語拼音",
    romanizationLabel: "Jyutping",
    shortLabel: "粵拼 Jyutping",
    label: "粵語拼音 Jyutping",
    reverseLookup: "`nei; -> jyut6ping3 reverse lookup",
  },
  {
    id: "cangjie5",
    schemaName: "倉頡五代",
    romanizationLabel: "Cangjie 5",
    shortLabel: "倉頡五代",
    label: "倉頡五代 Cangjie 5",
    reverseLookup: "`nei; -> Jyutping lookup with Cangjie comments",
  },
  {
    id: "luna_pinyin",
    schemaName: "普通話",
    romanizationLabel: "Luna Pinyin",
    shortLabel: "普通話",
    label: "普通話 Luna Pinyin",
    reverseLookup: "`a; -> Cangjie lookup with Pinyin comments",
  },
];
```

Before finalizing the exact labels, re-open the three schema YAML files and verify the `schema/name` values. If a future schema is added, update the YAML and `SCHEMA_OPTIONS` metadata in the same slice.

- [ ] **Step 3: Render Cantonese/Chinese-first schema controls**

Update `SchemaSwitcher.tsx` so the visible legend and segmented labels are user-readable:

```tsx
<legend className="mb-2 font-semibold">方案 Schema</legend>
```

Use `schema.label` for the accessible label. If the existing `Segment` helper only accepts a string label, keep the combined bilingual string there. If richer rendering is already available, the primary visible text should be `schema.schemaName` and the romanized/English label should be visually secondary.

Do not show `jyut6ping3_mobile`, `cangjie5`, or `luna_pinyin` as the primary visible label in the switcher.

- [ ] **Step 4: Keep the status strip and switcher consistent**

If M24-DOGFOOD-09 has already landed, make sure the status strip uses `status.schema_name || schema.schemaName || status.schema_id` so it shows the same real schema name as the switcher whenever the runtime reports it. Preserve the diagnostic `data-yune-status-schema` attribute.

After switching schemas, assert the user-facing name remains consistent:

```ts
await page.getByRole("radio", { name: /倉頡五代/ }).check();
await typeInputForStatus(page, "a");
await expect(page.locator("[data-yune-schema-switcher]")).toContainText("倉頡五代");
await expect(page.locator("[data-yune-status-schema]")).toContainText(/倉頡五代|cangjie5/);
```

If the status strip task has not landed yet, keep this assertion inside the same M24 test but allow the `cangjie5` fallback until M24-DOGFOOD-09 closes.

- [ ] **Step 5: Add final browser assertions and screenshot**

Extend the characterization test after the fix:

```ts
await expect(page.locator("[data-yune-schema-switcher]")).toContainText("方案 Schema");
await expect(page.getByRole("radio", { name: /粵語拼音 Jyutping/ })).toBeVisible();
await expect(page.getByRole("radio", { name: /倉頡五代 Cangjie 5/ })).toBeVisible();
await expect(page.getByRole("radio", { name: /普通話 Luna Pinyin/ })).toBeVisible();
await takeM24Screenshot(page, "M24-DOGFOOD-10", "schema-switcher-after");
```

If the segmented helper does not expose a radio role, use `getByText(...)` for the visible assertions and separately preserve whatever accessibility role the helper currently uses.

- [ ] **Step 6: Run focused gates**

Run:

```powershell
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 schema switcher"
```

Expected: the schema switcher uses checked real schema names, the selected schema is still controllable, the status strip remains diagnostic-friendly, and the screenshot shows readable labels at the existing desktop width.

## Task 12: M24-DOGFOOD-11 Add Jyutping Reverse Lookup To The Web Dogfood

**Files:**

- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Modify: `crates/yune-rime-api/tests/typeduck_web.rs`
- Modify after the failing native/browser tests identify the gap: `apps/yune-web/source/schema/jyut6ping3_mobile.schema.yaml`
- Inspect before modifying schema behavior: `apps/yune-web/source/schema/jyut6ping3.schema.yaml`, `apps/yune-web/source/schema/luna_pinyin.schema.yaml`, `apps/yune-web/source/schema/luna_pinyin.dict.yaml`
- Modify only for visible examples/help text: `apps/yune-web/source/src/consts.ts`, `apps/yune-web/source/src/App.tsx`
- Modify only if runtime routing drops the tagged segment or assets: `apps/yune-web/source/src/yune-integration/adapter.ts`, `crates/yune-rime-api/src/typeduck_web.rs`, `crates/yune-rime-api/src/schema_install.rs`

- [ ] **Step 1: Add a failing browser test for the user-visible Jyutping flow**

The test should use the visible Jyutping schema, type the Mandarin pinyin affix path from the full Jyutping schema, and assert only the behavior we know from the report: `這` is present somewhere on the page. Use `` `pzhe `` for the pinyin path; do not use bare `` `zhe `` unless a separate fixture proves the generic reverse-lookup tag should also be wired.

```ts
test("M24 Jyutping reverse lookup accepts Mandarin pinyin", async ({ page }) => {
  await selectSchema(page, /Jyutping|粵語拼音/);

  const inputField = page.locator("input[type='text'], textarea").first();
  await clearComposition(page);
  await inputField.focus();
  await inputField.type("`pzhe", { delay: 120 });

  await expect.poll(async () => {
    const state = await readCandidatePanelSnapshot(page, false);
    return state.candidates.map((candidate) => candidate.text);
  }, { timeout: 10000 }).toContain("這");

  const state = await readCandidatePanelSnapshot(page, false);
  await saveM24Json("M24-DOGFOOD-11", "jyutping-reverse-lookup-pzhe.json", state);
  await takeM24Screenshot(page, "M24-DOGFOOD-11", "jyutping-reverse-lookup-pzhe");
});
```

Expected before the fix: this fails because the visible Jyutping schema does not produce `這` for `` `pzhe `` in the browser dogfood.

- [ ] **Step 2: Add a native `typeduck_web` bridge test for the same packaged-assets path**

Add this next to `typeduck_adapter_browser_app_assets_load_m22_schemas_and_reverse_lookup()` so the browser failure is not hidden by React rendering:

```rust
#[test]
fn typeduck_adapter_browser_app_assets_load_jyutping_mandarin_reverse_lookup() {
    let _guard = test_guard();
    let runtime =
        TypeDuckRuntime::create_with_schema("browser-app-jyutping-reverse-lookup", "jyut6ping3_mobile");
    runtime.write_browser_app_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null(), "jyut6ping3_mobile should initialize from browser app assets");

    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);
    let reverse = process_input(state, "`pzhe");
    let texts = reverse["context"]["candidates"]
        .as_array()
        .expect("candidate list should be an array")
        .iter()
        .map(|candidate| candidate["text"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert!(
        texts.contains(&"這"),
        "jyut6ping3_mobile reverse lookup should expose 這 for `pzhe, got {texts:?}"
    );

    drop(response_json(unsafe {
        yune_typeduck_process_key(state, 0xff1b, 0)
    }));
    let normal = process_input(state, "nei");
    assert_eq!(
        normal["context"]["candidates"][0]["text"],
        serde_json::Value::String("你".to_owned()),
        "normal Jyutping composition must stay intact"
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}
```

Run the native test before implementation:

```powershell
cargo test -p yune-rime-api --test typeduck_web typeduck_adapter_browser_app_assets_load_jyutping_mandarin_reverse_lookup
```

Expected before the fix: the test fails on the `texts.contains(&"這")` assertion, while the existing Cangjie/Luna reverse-lookup test documents that the lower-level browser assets can already support reverse lookup.

- [ ] **Step 3: Wire reverse lookup into the visible Jyutping web schema without changing normal composition**

Keep `jyut6ping3_mobile` as the visible Jyutping schema unless a fresh TypeDuck `v1.1.2` fixture proves the product uses a different schema for the web dogfood. Extend only the Mandarin pinyin affix path by borrowing the full Jyutping `luna_pinyin` declarations:

```yaml
schema:
  schema_id: jyut6ping3_mobile
  name: 粵語拼音
  dependencies:
    - luna_pinyin

engine:
  segmentors:
    - ascii_segmentor
    - matcher
    - affix_segmentor@luna_pinyin
    - abc_segmentor
    - punct_segmentor
    - fallback_segmentor
  translators:
    - punct_translator
    - script_translator
    - script_translator@luna_pinyin
  filters:
    - dictionary_lookup_filter
    - simplifier

luna_pinyin:
  tag: luna_pinyin
  dictionary: luna_pinyin
  enable_sentence: false
  enable_user_dict: false
  encode_commit_history: false
  always_show_comments: true
  prefix: "`p"
  suffix: ";"
  tips: 〔普通話〕
  preedit_format:
    - xform/([nl])v/$1ü/
    - xform/([nl])ue/$1üe/
    - xform/([jqxy])v/$1u/
  comment_format:
    - xform/^/\v/
```

Preserve the existing `translator` block, `menu` patch, and mobile patches in `jyut6ping3_mobile.schema.yaml`. If the deployed/generated schema already defines `engine` through `template:/`, place the added segmentor/translator items in the source YAML and verify the deployed schema includes them after build.

- [ ] **Step 4: Make the visible web UI teach the feature**

Update the Jyutping schema option and the quick example controls so users can discover the feature:

```ts
{
  id: "jyut6ping3_mobile",
  label: "粵語拼音 Jyutping",
  reverseLookup: "`pzhe -> 這（普通話反查）",
}
```

If M24-DOGFOOD-10 has already expanded `SchemaOption`, keep the same fields and only update the Jyutping `reverseLookup` text. Add a small Cantonese-first example button near the existing examples:

```tsx
<button type="button" className="yd-button" onClick={() => runScenario("`pzhe")}>
  反查 `pzhe
</button>
```

Use the existing scenario helper used by the `nei`, `ngo`, `santai`, `m`, `mgoi`, `tone letters`, and `AI trigger` buttons. Do not add a second input path.

- [ ] **Step 5: Prove normal Jyutping behavior did not regress**

Extend the browser test after the reverse lookup assertion:

```ts
await clearComposition(page);
const normal = await typeCompositionAndWaitForTopCandidate(page, "nei", "你");
expect(normal.candidates[0].text).toBe("你");

await clearComposition(page);
const phrase = await typeCompositionAndWaitForTopCandidate(page, "jigaajiusihaa", "而家要思考");
expect(phrase.candidates[0].text).toBe("而家要思考");
```

This is a regression guard for the visible dogfood path. Ranking questions beyond the top preserved phrase remain owned by M24-DOGFOOD-04.

- [ ] **Step 6: Run focused gates**

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_web typeduck_adapter_browser_app_assets_load_jyutping_mandarin_reverse_lookup
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 Jyutping reverse lookup"
```

Expected: `jyut6ping3_mobile` still initializes from the browser app assets, `` `pzhe `` exposes `這`, normal Jyutping composition stays green, and the screenshot under `M24-DOGFOOD-11` shows the reverse-lookup candidate in the real web panel.

## Task 13: M24-DOGFOOD-12 Replace The Sung/Hei Toggle With A Full Typeface Picker

**Files:**

- Modify: `apps/yune-web/source/src/types.ts`
- Modify: `apps/yune-web/source/src/consts.ts`
- Modify: `apps/yune-web/source/src/hooks.ts`
- Modify: `apps/yune-web/source/src/Preferences.tsx`
- Modify: `apps/yune-web/source/src/App.tsx`
- Modify: `apps/yune-web/source/src/Candidate.tsx`
- Modify: `apps/yune-web/source/src/DictionaryPanel.tsx`
- Modify: `apps/yune-web/source/src/index.css`
- Modify: `apps/yune-web/source/tailwind.config.ts`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Add a failing browser test for the new typeface picker**

Add a Playwright test that expects a radio list with the eight full family names and no binary `宋體 Sung` / `黑體 Hei` segmented control:

```ts
const EXPECTED_CHINESE_TYPEFACES = [
  "Chocolate Classical Sans",
  "Iansui",
  "LXGW WenKai Mono TC",
  "LXGW WenKai TC",
  "WDXL Lubrifont TC",
  "Chiron GoRound TC",
  "Chiron Hei HK",
  "Chiron Sung HK",
] as const;

test("M24 Chinese typeface picker uses full font family names", async ({ page }) => {
  const picker = page.locator("[data-yune-chinese-typeface]");
  await expect(picker).toBeVisible();
  await expect(picker).toContainText("中文字款 Chinese Typeface");
  await expect(picker).not.toContainText("宋體 Sung");
  await expect(picker).not.toContainText("黑體 Hei");

  for (const family of EXPECTED_CHINESE_TYPEFACES) {
    await expect(picker.getByLabel(family)).toBeVisible();
  }

  await picker.getByLabel("Iansui").check();
  await expect(page.locator("textarea")).toHaveAttribute("data-chinese-typeface", "iansui");
  await focusInputAndType(page, "nei", "你");
  await expect(page.locator(".candidate-panel [data-chinese-typeface='iansui']").first()).toBeVisible();

  const textareaFont = await page.locator("textarea").evaluate((element) =>
    window.getComputedStyle(element).fontFamily,
  );
  expect(textareaFont).toContain("Iansui");

  await saveM24Json("M24-DOGFOOD-12", "typeface-picker-font-resources.json", {
    selected: "Iansui",
    textareaFont,
    fontResources: await page.evaluate(() =>
      performance.getEntriesByType("resource")
        .filter((entry) => /fonts\.(googleapis|gstatic)\.com/.test(entry.name))
        .map((entry) => ({ name: entry.name, duration: entry.duration })),
    ),
  });
  await takeM24Screenshot(page, "M24-DOGFOOD-12", "typeface-picker-iansui");
});
```

Expected before the fix: the locator is missing or the test finds only the old binary `宋體 Sung` / `黑體 Hei` control.

- [ ] **Step 2: Replace the boolean preference with a font-family ID**

In `types.ts`, replace `isHeiTypeface: boolean` with an explicit union:

```ts
export type ChineseTypefaceId =
  | "chocolate-classical-sans"
  | "iansui"
  | "lxgw-wenkai-mono-tc"
  | "lxgw-wenkai-tc"
  | "wdxl-lubrifont-tc"
  | "chiron-goround-tc"
  | "chiron-hei-hk"
  | "chiron-sung-hk";

export interface InterfacePreferences {
  displayLanguages: Set<Language>;
  mainLanguage: Language;
  chineseTypeface: ChineseTypefaceId;
  showRomanization: ShowRomanization;
  showReverseCode: boolean;
}
```

Delete all new references to `isHeiTypeface` in TypeScript. The old localStorage key is handled only by the migration in Step 4.

- [ ] **Step 3: Add checked font metadata in `consts.ts`**

Add the exact Google Fonts family names as data, and preserve the old default by making `Chiron Sung HK` the default family:

```ts
import type { ChineseTypefaceId, Preferences, RimeSchemaId } from "./types";

export interface ChineseTypefaceOption {
  id: ChineseTypefaceId;
  family: string;
  label: string;
  className: string;
  googleFontsUrl: string;
}

export const CHINESE_TYPEFACE_OPTIONS: readonly ChineseTypefaceOption[] = [
  {
    id: "chocolate-classical-sans",
    family: "Chocolate Classical Sans",
    label: "Chocolate Classical Sans",
    className: "font-chinese-chocolate-classical-sans",
    googleFontsUrl: "https://fonts.google.com/specimen/Chocolate+Classical+Sans",
  },
  {
    id: "iansui",
    family: "Iansui",
    label: "Iansui",
    className: "font-chinese-iansui",
    googleFontsUrl: "https://fonts.google.com/specimen/Iansui",
  },
  {
    id: "lxgw-wenkai-mono-tc",
    family: "LXGW WenKai Mono TC",
    label: "LXGW WenKai Mono TC",
    className: "font-chinese-lxgw-wenkai-mono-tc",
    googleFontsUrl: "https://fonts.google.com/specimen/LXGW+WenKai+Mono+TC",
  },
  {
    id: "lxgw-wenkai-tc",
    family: "LXGW WenKai TC",
    label: "LXGW WenKai TC",
    className: "font-chinese-lxgw-wenkai-tc",
    googleFontsUrl: "https://fonts.google.com/specimen/LXGW+WenKai+TC",
  },
  {
    id: "wdxl-lubrifont-tc",
    family: "WDXL Lubrifont TC",
    label: "WDXL Lubrifont TC",
    className: "font-chinese-wdxl-lubrifont-tc",
    googleFontsUrl: "https://fonts.google.com/specimen/WDXL+Lubrifont+TC",
  },
  {
    id: "chiron-goround-tc",
    family: "Chiron GoRound TC",
    label: "Chiron GoRound TC",
    className: "font-chinese-chiron-goround-tc",
    googleFontsUrl: "https://fonts.google.com/specimen/Chiron+GoRound+TC",
  },
  {
    id: "chiron-hei-hk",
    family: "Chiron Hei HK",
    label: "Chiron Hei HK",
    className: "font-chinese-chiron-hei-hk",
    googleFontsUrl: "https://fonts.google.com/specimen/Chiron+Hei+HK",
  },
  {
    id: "chiron-sung-hk",
    family: "Chiron Sung HK",
    label: "Chiron Sung HK",
    className: "font-chinese-chiron-sung-hk",
    googleFontsUrl: "https://fonts.google.com/specimen/Chiron+Sung+HK",
  },
];

export const CHINESE_TYPEFACE_BY_ID = Object.fromEntries(
  CHINESE_TYPEFACE_OPTIONS.map((option) => [option.id, option]),
) as Record<ChineseTypefaceId, ChineseTypefaceOption>;
```

Then update `DEFAULT_PREFERENCES`:

```ts
chineseTypeface: "chiron-sung-hk",
```

- [ ] **Step 4: Migrate old `isHeiTypeface` storage in `hooks.ts`**

Map the old boolean to the closest new family and ignore malformed stored values:

```ts
import { CHINESE_TYPEFACE_BY_ID, DEFAULT_PREFERENCES, Language } from "./consts";
import type { ChineseTypefaceId, Preferences } from "./types";

function defaultChineseTypeface(): ChineseTypefaceId {
  if (typeof window === "undefined") {
    return DEFAULT_PREFERENCES.chineseTypeface;
  }
  const stored = window.localStorage.getItem("chineseTypeface");
  if (stored && stored in CHINESE_TYPEFACE_BY_ID) {
    return stored as ChineseTypefaceId;
  }
  const legacy = window.localStorage.getItem("isHeiTypeface");
  return legacy === "true" ? "chiron-hei-hk" : "chiron-sung-hk";
}
```

Inside `usePreferences()`, use the migrated default only for this key:

```ts
const effectiveDefaultValue = key === "chineseTypeface"
  ? defaultChineseTypeface()
  : defaultValue;
const [optionValue, setOptionValue] = useLocalStorageState(
  key,
  {
    defaultValue: effectiveDefaultValue,
    serializer: key === "displayLanguages"
      ? {
        stringify: languages => [...languages as Set<Language>].join(),
        parse: values => new Set(values.split(",").map(value => value.trim() as Language)),
      }
      : typeof effectiveDefaultValue === "string"
      ? {
        stringify: String,
        parse: value => value in CHINESE_TYPEFACE_BY_ID || key !== "chineseTypeface"
          ? value
          : DEFAULT_PREFERENCES.chineseTypeface,
      }
      : JSON,
  },
);
```

This keeps existing users on a visually similar default without storing the obsolete boolean again.

- [ ] **Step 5: Render a radio list in `Preferences.tsx`**

Replace the segmented toggle with a fieldset. Keep it in display controls and do not add helper paragraphs there:

```tsx
<li>
  <fieldset className="border border-base-300 rounded px-3 pb-2 mb-1" data-yune-chinese-typeface>
    <legend className="text-xl text-base-content my-2 px-2">中文字款 Chinese Typeface</legend>
    <div className="grid gap-2 sm:grid-cols-2">
      {CHINESE_TYPEFACE_OPTIONS.map(option =>
        <Radio
          key={option.id}
          name="chineseTypeface"
          label={option.label}
          state={prefs.chineseTypeface}
          setState={prefs.setChineseTypeface}
          value={option.id} />
      )}
    </div>
  </fieldset>
</li>
```

Import `CHINESE_TYPEFACE_OPTIONS` from `consts.ts`. The visible option text must be the full family name, not a generic category such as `Sung`, `Hei`, or `Kai`.

- [ ] **Step 6: Apply the selected family to all Chinese text surfaces**

In `App.tsx`, derive the class once:

```ts
const {
  chineseTypeface,
} = preferences;
const chineseTypefaceClass = CHINESE_TYPEFACE_BY_ID[chineseTypeface].className;
```

Use it on the textarea:

```tsx
<textarea
  className={`block w-full min-h-64 my-6 textarea textarea-bordered text-lg px-3 ${chineseTypefaceClass}`}
  data-chinese-typeface={chineseTypeface}
  ref={setTextArea}
  {...NO_AUTO_FILL} />
```

In `Candidate.tsx` and `DictionaryPanel.tsx`, replace every `prefs.isHeiTypeface ? "font-hei" : "font-sung"` and `font-kai-fallback-*` branch with the selected option class:

```ts
const chineseTypefaceClass = CHINESE_TYPEFACE_BY_ID[prefs.chineseTypeface].className;
```

Add `data-chinese-typeface={prefs.chineseTypeface}` to the rendered candidate Hanzi cell and dictionary headword/table cells so the browser test can prove the selected family reaches all visible Chinese surfaces.

- [ ] **Step 7: Wire Google Fonts without blocking IME startup**

In `index.css`, import the families with `display=swap`:

```css
@import url("https://fonts.googleapis.com/css2?family=Chocolate+Classical+Sans&family=Iansui&family=LXGW+WenKai+Mono+TC&family=LXGW+WenKai+TC&family=WDXL+Lubrifont+TC&family=Chiron+GoRound+TC&family=Chiron+Hei+HK&family=Chiron+Sung+HK&display=swap");
```

In `tailwind.config.ts`, add named font families:

```ts
"chinese-chocolate-classical-sans": ['"Chocolate Classical Sans"', "var(--font-sans)", "var(--font-emoji)"],
"chinese-iansui": ['"Iansui"', "var(--font-sans)", "var(--font-emoji)"],
"chinese-lxgw-wenkai-mono-tc": ['"LXGW WenKai Mono TC"', "var(--font-sans)", "var(--font-emoji)"],
"chinese-lxgw-wenkai-tc": ['"LXGW WenKai TC"', "var(--font-sans)", "var(--font-emoji)"],
"chinese-wdxl-lubrifont-tc": ['"WDXL Lubrifont TC"', "var(--font-sans)", "var(--font-emoji)"],
"chinese-chiron-goround-tc": ['"Chiron GoRound TC"', "var(--font-sans)", "var(--font-emoji)"],
"chinese-chiron-hei-hk": ['"Chiron Hei HK"', "var(--font-sans)", "var(--font-emoji)"],
"chinese-chiron-sung-hk": ['"Chiron Sung HK"', "var(--font-serif)", "var(--font-emoji)"],
```

Do not render each radio label with its own font preview in this first slice; doing so would encourage the browser to load all eight large Chinese fonts on first paint. The first performance pass should load only the CSS and the selected family.

- [ ] **Step 8: Run focused gates**

Run:

```powershell
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 Chinese typeface"
```

Expected: the picker lists all eight full family names, selecting `Iansui` changes the computed textarea font and candidate/dictionary `data-chinese-typeface` markers, the old Sung/Hei toggle is absent, and the screenshot under `M24-DOGFOOD-12` proves the control is readable.

## Task 14: M24-DOGFOOD-13 Remove DaisyUI And Keep Local Tailwind Components Only

**Files:**

- Modify: `apps/yune-web/source/package.json`
- Modify: `apps/yune-web/source/tailwind.config.ts`
- Modify: `apps/yune-web/source/src/index.css`
- Modify: `apps/yune-web/source/src/Inputs.tsx`
- Modify: `apps/yune-web/source/src/Toolbar.tsx`
- Modify: `apps/yune-web/source/src/ThemeSwitcher.tsx`
- Modify: `apps/yune-web/source/src/YuneFeatureShowcase.tsx`
- Modify when shared component replacement touches callers: `apps/yune-web/source/src/App.tsx`, `apps/yune-web/source/src/Preferences.tsx`, `apps/yune-web/source/src/CandidatePanel.tsx`, `apps/yune-web/source/src/Candidate.tsx`, `apps/yune-web/source/src/DictionaryPanel.tsx`, `apps/yune-web/source/src/YuneStatusStrip.tsx`, `apps/yune-web/source/src/YuneInspector.tsx`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Add a failing stack-boundary test**

Add a Playwright test that checks package metadata, Tailwind config, source class strings, and basic UI availability:

```ts
test("M24 dogfood UI uses only local Tailwind components", async ({ page }) => {
  const packageJson = JSON.parse(await readRepoText("apps/yune-web/source/package.json")) as {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
  };
  expect(packageJson.dependencies?.daisyui).toBeUndefined();
  expect(packageJson.devDependencies?.daisyui).toBeUndefined();

  const tailwindConfig = await readRepoText("apps/yune-web/source/tailwind.config.ts");
  expect(tailwindConfig).not.toMatch(/\bdaisyui\b/i);
  expect(tailwindConfig).not.toContain("DaisyUIConfig");

  const filesToScan = [
    "apps/yune-web/source/src/Inputs.tsx",
    "apps/yune-web/source/src/Toolbar.tsx",
    "apps/yune-web/source/src/ThemeSwitcher.tsx",
    "apps/yune-web/source/src/YuneFeatureShowcase.tsx",
    "apps/yune-web/source/src/App.tsx",
    "apps/yune-web/source/src/Preferences.tsx",
    "apps/yune-web/source/src/CandidatePanel.tsx",
    "apps/yune-web/source/src/Candidate.tsx",
    "apps/yune-web/source/src/DictionaryPanel.tsx",
    "apps/yune-web/source/src/YuneStatusStrip.tsx",
    "apps/yune-web/source/src/YuneInspector.tsx",
    "apps/yune-web/source/src/index.css",
  ];
  const forbiddenDaisyUiClasses = /\b(btn|toggle|radio|checkbox|range|textarea|badge|join|tooltip|link|loading)(?:-[A-Za-z0-9_:[\]/.%#]+)?\b/;
  for (const file of filesToScan) {
    const source = await readRepoText(file);
    const classSnippets = source.match(/className\s*=\s*(?:"[^"]*"|'[^']*'|`[^`]*`|\{`[^`]*`\}|\{"[^"]*"\}|\{'[^']*'\})/g) ?? [];
    for (const snippet of classSnippets) {
      expect(snippet, `${file}: ${snippet}`).not.toMatch(forbiddenDaisyUiClasses);
    }
  }

  await expect(page.getByRole("button", { name: /ASCII mode|中/ })).toBeVisible();
  await expect(page.locator("[data-yune-schema-switcher]")).toBeVisible();
  await expect(page.locator("[data-yune-status]")).toBeVisible({ timeout: 10000 });
  await focusInputAndType(page, "nei", "你");
  await expect(page.locator(".candidate-panel")).toBeVisible();
  await takeM24Screenshot(page, "M24-DOGFOOD-13", "local-tailwind-components");
});
```

Expected before the fix: this fails because `package.json` lists `daisyui`, `tailwind.config.ts` imports/configures it, and source files contain DaisyUI class names.

- [ ] **Step 2: Move DaisyUI theme tokens into local Tailwind/CSS**

In `tailwind.config.ts`, remove:

```ts
import daisyui from "daisyui";
import type { Config as DaisyUIConfig } from "daisyui";
plugins: [daisyui],
daisyui: { /* themes */ } satisfies DaisyUIConfig,
```

Replace the theme color surface with local CSS-variable-backed colors:

```ts
theme: {
  fontFamily: {
    "serif": ["var(--font-serif)", "var(--font-emoji)"],
    "sans": ["var(--font-sans)", "var(--font-emoji)"],
    "geometric": ["var(--font-geometric)", "var(--font-sans)", "var(--font-emoji)"],
    "sung": ["var(--font-sung)", "var(--font-serif)", "var(--font-emoji)"],
    "hei": ["var(--font-hei)", "var(--font-sans)", "var(--font-emoji)"],
    "kai-fallback-sung": ["var(--font-kai)", "var(--font-sung)", "var(--font-serif)", "var(--font-emoji)"],
    "kai-fallback-hei": ["var(--font-kai)", "var(--font-hei)", "var(--font-sans)", "var(--font-emoji)"],
    "devanagari": ["var(--font-devanagari)", "var(--font-sans)", "var(--font-emoji)"],
    "arabic": ["var(--font-arabic)", "var(--font-sans)", "var(--font-emoji)"],
  },
  colors: {
    transparent: "transparent",
    current: "currentColor",
    primary: "rgb(var(--primary) / <alpha-value>)",
    "primary-content": "rgb(var(--primary-content) / <alpha-value>)",
    "primary-content-200": "rgb(var(--primary-content-200) / <alpha-value>)",
    "primary-content-300": "rgb(var(--primary-content-300) / <alpha-value>)",
    "primary-content-400": "rgb(var(--primary-content-400) / <alpha-value>)",
    "primary-content-500": "rgb(var(--primary-content-500) / <alpha-value>)",
    secondary: "rgb(var(--secondary) / <alpha-value>)",
    "secondary-content": "rgb(var(--secondary-content) / <alpha-value>)",
    accent: "rgb(var(--accent) / <alpha-value>)",
    "accent-content": "rgb(var(--accent-content) / <alpha-value>)",
    neutral: "rgb(var(--neutral) / <alpha-value>)",
    "neutral-content": "rgb(var(--neutral-content) / <alpha-value>)",
    "base-100": "rgb(var(--base-100) / <alpha-value>)",
    "base-200": "rgb(var(--base-200) / <alpha-value>)",
    "base-300": "rgb(var(--base-300) / <alpha-value>)",
    "base-400": "rgb(var(--base-400) / <alpha-value>)",
    "base-500": "rgb(var(--base-500) / <alpha-value>)",
    "base-content": "rgb(var(--base-content) / <alpha-value>)",
    "base-content-200": "rgb(var(--base-content-200) / <alpha-value>)",
    "base-content-300": "rgb(var(--base-content-300) / <alpha-value>)",
    "base-content-400": "rgb(var(--base-content-400) / <alpha-value>)",
  },
},
plugins: [],
```

In `index.css`, define the theme variables DaisyUI used to supply:

```css
:root,
:root[data-theme="light"] {
  color-scheme: light;
  --primary: 10 130 250;
  --primary-content: 248 251 255;
  --primary-content-200: 229 240 255;
  --primary-content-300: 212 230 255;
  --primary-content-400: 202 225 255;
  --primary-content-500: 135 195 255;
  --secondary: 212 235 255;
  --secondary-content: 6 89 167;
  --accent: 199 227 255;
  --accent-content: 5 65 125;
  --neutral: 229 235 241;
  --neutral-content: 33 67 97;
  --base-100: 255 255 255;
  --base-200: 249 250 251;
  --base-300: 236 238 241;
  --base-400: 222 225 227;
  --base-500: 181 183 185;
  --base-content: 0 22 53;
  --base-content-200: 75 88 105;
  --base-content-300: 67 89 117;
  --base-content-400: 47 82 120;
}

:root[data-theme="dark"] {
  color-scheme: dark;
  --primary: 4 101 198;
  --primary-content: 248 251 255;
  --primary-content-200: 229 240 255;
  --primary-content-300: 212 230 255;
  --primary-content-400: 202 225 255;
  --primary-content-500: 69 141 213;
  --secondary: 16 63 106;
  --secondary-content: 211 224 236;
  --accent: 16 75 138;
  --accent-content: 221 236 255;
  --neutral: 38 50 62;
  --neutral-content: 197 207 211;
  --base-100: 11 18 31;
  --base-200: 28 35 42;
  --base-300: 52 58 68;
  --base-400: 70 77 87;
  --base-500: 116 122 129;
  --base-content: 255 255 255;
  --base-content-200: 214 224 235;
  --base-content-300: 207 219 232;
  --base-content-400: 197 212 228;
}
```

- [ ] **Step 3: Replace DaisyUI classes with local component classes**

In `index.css`, add local component classes under `@layer components`:

```css
@layer components {
  .yd-anchor {
    @apply text-primary no-underline transition-colors hover:underline;
  }
  .yd-button {
    @apply inline-flex items-center justify-center rounded border border-primary px-3 py-1.5 text-sm font-medium text-primary transition-colors hover:bg-accent focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-base-100 disabled:pointer-events-none disabled:opacity-50;
  }
  .yd-button-active {
    @apply bg-primary text-primary-content hover:bg-primary;
  }
  .yd-icon-button {
    @apply inline-flex size-10 items-center justify-center rounded border border-base-300 bg-base-300 text-xl text-base-content transition-colors hover:border-base-400 hover:bg-base-400 focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-base-100;
  }
  .yd-segment-group {
    @apply inline-flex overflow-hidden rounded border border-primary;
  }
  .yd-segment {
    @apply inline-flex items-center justify-center border-r border-primary px-3 py-1.5 text-sm font-medium text-primary last:border-r-0 hover:bg-accent focus-within:outline focus-within:outline-2 focus-within:outline-primary;
  }
  .yd-segment-active {
    @apply bg-primary text-primary-content hover:bg-primary;
  }
  .yd-switch {
    @apply h-5 w-9 cursor-pointer appearance-none rounded-full border border-base-500 bg-base-300 transition-colors checked:border-primary checked:bg-primary focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-base-100;
  }
  .yd-choice {
    @apply size-5 cursor-pointer appearance-none rounded-full border border-primary bg-transparent checked:border-[0.35rem] checked:bg-base-100 focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-base-100;
  }
  .yd-check {
    @apply size-5 cursor-pointer appearance-none rounded border border-primary bg-transparent checked:bg-primary focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-base-100;
  }
  .yd-slider {
    @apply h-2 w-full cursor-pointer appearance-none rounded-full bg-base-300 accent-primary;
  }
  .yd-input-area {
    @apply block w-full rounded border border-base-300 bg-base-100 text-base-content shadow-sm focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary;
  }
  .yd-pill {
    @apply inline-flex min-w-6 items-center justify-center rounded-full border border-base-400 px-2 py-0.5 text-xs text-base-content;
  }
}
```

Then update `Inputs.tsx`:

```tsx
export function Toggle({ label, checked, setChecked }: CheckboxProps) {
  return <label className="flex cursor-pointer items-center gap-2 py-1">
    <span className="flex-1 text-lg text-base-content-200">{label}</span>
    <input
      type="checkbox"
      className="yd-switch"
      {...NO_AUTO_FILL}
      checked={checked}
      onChange={event => setChecked(event.target.checked)} />
  </label>;
}

export function Radio<T>({ name, label, state, setState, value }: RadioProps<T>) {
  return <label className="flex cursor-pointer items-center gap-2 py-1">
    <input
      type="radio"
      name={name}
      className="yd-choice"
      {...NO_AUTO_FILL}
      checked={state === value}
      onChange={() => setState(value)} />
    <span className="flex-1 text-lg text-base-content-200">{label}</span>
  </label>;
}

export function Segment<T>({ name, label, state, setState, value }: RadioProps<T>) {
  const active = state === value;
  return <label className={`yd-segment${active ? " yd-segment-active" : ""}`}>
    <input
      type="radio"
      className="sr-only"
      name={name}
      {...NO_AUTO_FILL}
      checked={active}
      onChange={() => setState(value)} />
    {label}
  </label>;
}
```

Apply the same local classes to `Range` and `RadioCheckbox`: use `yd-pill`, `yd-slider`, `yd-choice`, and `yd-check`.

- [ ] **Step 4: Update callers that currently depend on DaisyUI class composition**

Replace these common patterns:

```tsx
<div className="join">...</div>
```

with:

```tsx
<div className="yd-segment-group">...</div>
```

Replace:

```tsx
className="btn-toolbar join-item"
```

with:

```tsx
className="yd-icon-button"
```

Replace:

```tsx
className="btn btn-outline btn-primary btn-sm join-item"
```

with:

```tsx
className="yd-button"
```

Replace the textarea classes in `App.tsx`:

```tsx
className={`yd-input-area min-h-64 my-6 px-3 text-lg ${chineseTypefaceClass}`}
```

Update `ThemeSwitcher.tsx` to remove the DaisyUI attribution comment and use the local toggle class:

```tsx
<input
  type="checkbox"
  checked={theme === "dark"}
  onChange={() => setTheme(theme === "dark" ? "light" : "dark")}
  className="yd-switch row-start-1 col-start-1 col-span-2" />
```

Keep existing ARIA labels, input types, and test hooks. The worker may simplify spacing, borders, sizing, and visual treatment instead of matching DaisyUI's old appearance, as long as the result is minimalistic, elegant, readable, and browser-tested.

- [ ] **Step 5: Remove the dependency and verify no DaisyUI identifiers remain**

Remove `daisyui` from `devDependencies` in `apps/yune-web/source/package.json`.

Run:

```powershell
rg -n "\bdaisyui\b|DaisyUIConfig|\b(btn|toggle|radio|checkbox|range|textarea|badge|join|tooltip|link|loading)(-[A-Za-z0-9_:[\]/.%#]+)?\b" apps/yune-web/source/package.json apps/yune-web/source/tailwind.config.ts apps/yune-web/source/src
```

Expected: no DaisyUI dependency/config/imports and no DaisyUI class tokens remain. If this search catches native HTML words in prose or `type="radio"`, narrow the check in the Playwright test to class strings, but keep the command output reviewed manually.

- [ ] **Step 6: Run focused gates**

Run:

```powershell
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24 dogfood UI uses only local Tailwind components"
```

Expected: the app builds without DaisyUI, the smoke UI remains visible, candidate composition still works, and the screenshot under `M24-DOGFOOD-13` proves the local Tailwind components render correctly in a minimalistic, elegant layout.

## Task 15: M24 Regression Sweep And Closeout Discipline

**Files:**

- Modify: this plan as issue rows close
- Modify when TypeDuck-Web source changes: `apps/yune-web/patches/yune-web-runtime.patch`
- Modify when Yune-owned integration changes: `apps/yune-web/yune-integration/*`
- Modify if requirements become durable: `docs/requirements.md`
- Modify when M24 closes: `docs/roadmap.md`

- [ ] **Step 1: Run focused gates for the touched layer**

For browser-only fixes:

```powershell
npm.cmd --prefix apps/yune-web/source run build
npx --prefix apps/yune-web/source playwright test apps/yune-web/e2e/yune-typeduck.spec.ts -g "M24"
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

- [ ] **Step 2: Regenerate and check the TypeDuck-Web patch if any source files changed**

If any task edited `apps/yune-web/source/...`, regenerate the maintained patch and run both checks from `apps/yune-web/source`:

```powershell
git apply --reverse --check ..\patches\yune-web-runtime.patch
git apply --check ..\patches\yune-web-runtime.patch
```

Expected: both commands exit 0 after the source checkout is in the matching state for the direction being checked. The staged diff must include `apps/yune-web/patches/yune-web-runtime.patch`; untracked ignored `source/` edits alone are not a valid M24 deliverable.

- [ ] **Step 3: Run broad gates before closing a batch**

Run:

```powershell
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm.cmd --prefix apps/yune-web/source run build
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
```

- [ ] **Step 4: Update the running ledger**

For each closed row, change `Status` from `Open` to `Closed`, add the evidence directory, and state which test owns the regression.

- [ ] **Step 5: Add requirements only for durable product/demo contracts**

Do not add requirement IDs for every small bug. Add `M24-DOGFOOD-*` requirements only when a finding becomes a durable contract, such as startup budget, no raw comment-control rendering, or candidate-detail layout behavior.

- [ ] **Step 6: Archive M24 only after the dogfooding batch is intentionally closed**

When the current batch is complete, move this plan to `docs/plans/archive/`, update `docs/roadmap.md`, and keep future dogfooding rounds as a new plan or a reopened M24 continuation only if the scope is still the same browser playground.
