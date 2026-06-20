# M20 Web Demo Showcase Controls Implementation Plan

> **Status:** Active · **Milestone:** M20 (Web demo showcase controls) · **Updated:** 2026-06-20 · **Type:** execution plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the patched TypeDuck-Web app into Yune's canonical browser playground for demoing, stress-testing, and comparing the engine features already supported by M9, M13, M14-M16, and the FORK-PARITY backlog, without reopening M13 or changing the core ABI.

**Architecture:** Keep the existing TypeDuck-Web app and Yune seam, but treat the UI as a high-control engine workbench rather than a minimal end-user settings page. Runtime-changing controls must flow through the existing `customize()` schema-key path or `setOption()` session-option path; no new `RimeApi`, `RimeCandidate`, or `yune_typeduck_*` export is allowed unless a later task proves the existing paths cannot express the behavior. Every supported browser-safe engine feature should be reachable as either a live control, a deploy-time control, or a guided scenario; unsupported or deferred behavior must be visibly absent or explicitly labeled as deferred, never represented as working.

**Tech Stack:** Rust `yune-core` / `yune-rime-api`, Emscripten WASM, `@yune-ime/typeduck-runtime`, TypeDuck-Web React/TypeScript, Playwright.

---

## Scope

M20 is a web/demo milestone, parallel to Track 2 (M17-M19). It may run before M17, but it is not an M13 follow-up: M13 remains completed and archived as the default-off AI frontend exposure milestone.

For Yune development, this TypeDuck-Web build is the go-to playground. It should
let maintainers quickly toggle options, trigger edge cases, compare before/after
candidate behavior, capture browser evidence, and stress new engine features
with real assets. When a new engine feature becomes browser-safe, the default
expectation is to add either an active control or a guided scenario here; when a
feature is not browser-safe or not implemented, document the deferral instead of
leaving a confusing partial UI.

For M20, **browser-safe** means the behavior can be exercised through the
TypeDuck-Web worker with real checked-in browser assets, without changing the
upstream `RimeApi` table, widening `RimeCandidate`, requiring a native
platform-only frontend, requiring a remote provider, or collecting
security-sensitive host context. A browser-safe feature also needs deterministic
before/after evidence in Playwright or the manual smoke record.

M20 must preserve:

- M9 TypeDuck-Web real-assets browser behavior.
- M13 AI invariants: AI default-off, local-first, classic-first, provider-free `yune_typeduck_process_key`, provider work only through `stage_ai`, no `RimeCandidate` widening, no upstream `RimeApi` table change.
- M16 TypeDuck-Web Cantonese profile behavior and the committed fork-parity decisions.
- Upstream-first default behavior outside the TypeDuck-Web/profile surface.

M20 must be broad but honest. A visible engine/runtime toggle or slider is allowed only when it demonstrably changes candidate output, committed output, status output, or persisted schema configuration. Display-only controls are allowed, but they must live in their own group and prove a visible rendering difference. `ascii_punct` is therefore out of scope as an active toggle until M18 implements the real processor-level behavior.

---

## Control / Scenario Split

### Active Engine Controls

These controls are allowed in the UI because they are already wired to runtime behavior. Future supported engine features should be added to this table or to Guided Scenarios so the playground remains a living coverage surface, not a stale demo.

| UI control | Transport | Engine/config key | Default | Required proof |
|---|---|---|---|---|
| Auto-completion | `customize()` + `deploy()` | `translator/enable_completion` | On | Candidate/completion output changes |
| Auto-correction | `customize()` + `deploy()` | `translator/enable_correction` | Off | Correction row appears/disappears |
| Auto-composition | `customize()` + `deploy()` | `translator/enable_sentence` | On | Sentence row appears/disappears |
| Input memory | `customize()` + `deploy()` | `translator/enable_user_dict`, `translator/encode_commit_history` | On | Learned prediction behavior changes |
| AI candidates | `setAiEnabled()` | `yune_typeduck_set_ai_enabled` | Off | AI rows appear only after second pass |
| Combine same-text candidates | `customize()` + `deploy()` | `translator/combine_candidates` | On for `jyut6ping3_mobile` unless real-browser evidence proves otherwise | Same-text homographs group/split |
| Prediction never first | `customize()` + `deploy()` | `translator/prediction_never_first` | On | Learned prefix prediction cannot occupy index 0 |
| Prediction threshold | `customize()` + `deploy()` | `translator/prediction_weight_threshold` | `0` or empty | Threshold scale is derived from real loaded dictionary weights and filters prediction rows on real assets |
| ASCII mode | `setOption()` | `ascii_mode` | Off | Status changes and printable input bypasses IME |
| Full shape | `setOption()` | `full_shape` | Off | Punctuation/ASCII shape output changes |
| Simplification | `setOption()` | `simplification` | Off | `hk2s` simplifier output changes |
| Page size | `customize()` + `deploy()` | `page_size` | 6 | Candidate page count changes |

There is only one prediction-threshold control. `prediction_frequency_threshold` and `prediction/frequency_threshold` are config aliases that feed the same `with_prediction_weight_threshold` engine setter; exposing two UI sliders would be dishonest. The implementation must not hard-code a `0..10` or `0..2` slider: it must first inspect the effective real-asset dictionary/candidate quality scale and choose a fine-grained sub-1 range, logarithmic selector, or discrete preset list that visibly filters prediction rows without hiding exact classic rows. Source dictionary weights and Yune candidate quality may use different scales, so the browser proof must come from the actual Yune-loaded real-assets path rather than source-row or synthetic-fixture assumptions.

### Display Controls

These controls are UI-only. They are still part of the playground, but their proof is a visible rendering change rather than an engine candidate/status change.

| UI control | Transport | UI target | Default | Required proof |
|---|---|---|---|---|
| Display languages | UI-only | dictionary-panel column filter | English | Dictionary panel columns change |
| Candidate Jyutping | UI-only | candidate rendering | Existing default | Candidate-panel code/comment display changes |
| Reverse code display | UI-only | dictionary/candidate rendering | Existing default | Reverse-code visibility changes |
| Cangjie version | Existing app setting until mapped | Cangjie side-lookup UI/config | Existing default | Must be mapped to real config or removed/relabeled before M20 completion |

### Guided Scenarios

These are feature-launch buttons or compact scenario rows, not toggles:

| Scenario | Input/action | Expected demo point |
|---|---|---|
| Baseline Cantonese | Type `nei` | Real `jyut6ping3_mobile` dictionary candidates render |
| Long-entry prediction | Type `santai` | Long entry `身體健康` can appear, with ranking controlled by prediction controls |
| Prediction never first | Commit/learn a long phrase, then type `ngo` | Classic `我` remains index 0 when never-first is on |
| Fuzzy/Cantonese tolerance | Type `m` and `mgoi` | Real dictionary fuzzy/容錯 path is active |
| Letter-to-tone | Type tone-letter inputs using `v`, `x`, `q` | TypeDuck profile tone-letter algebra is active |
| Correction | Type a known typo covered by M14/M15 evidence | Correction appears only when correction is on |
| Reverse lookup / dictionary panel | Hover/select candidates | Source-row labels and dictionary columns render from engine output |
| Show full code | Use a browser-reachable Cangjie side lookup such as `` `c...; `` | If the active browser schema has a `cangjie` namespace, `cangjie/show_full_code` changes side-lookup comments; if `jyut6ping3_mobile` remains the only browser schema, record this as N/A for the mobile-only surface rather than a fake control |
| Hide lone schema | Open switcher/schema state in the one-schema browser surface | Schema switcher remains hidden when only one schema is available; if there is no visible switcher UI, record the browser-surface N/A with the existing engine coverage |
| Per-entry userdb pronunciation | Commit a multi-syllable phrase, retype its prefix | If browser userdb inspection is available, pronunciation recovery is visible; otherwise record as engine-proven but browser-inspection-limited |
| AI second pass | Toggle AI on, type deterministic trigger | Classic row renders first; AI-labeled row arrives second and is explicit-select only |
| Unsupported punctuation | `ascii_punct` | Do not expose as working; label only as deferred M18 in docs/evidence if mentioned |

---

## File Structure

Implementation should touch only these paths unless a task explicitly discovers a blocker:

- Create `third_party/typeduck-web/AGENTS.md`: patch discipline, browser evidence, honest-control rules.
- Create `packages/yune-typeduck-runtime/AGENTS.md`: runtime wrapper/ABI safety rules.
- Modify `third_party/typeduck-web/source/src/types.ts`: extend preference types for new controls.
- Modify `third_party/typeduck-web/source/src/consts.ts`: add defaults, especially `predictionNeverFirst: true`.
- Modify `third_party/typeduck-web/source/src/App.tsx`: route new schema controls to `customize()`/`deploy()` and session controls to `setOption()`.
- Modify `third_party/typeduck-web/source/src/Preferences.tsx`: render controls in compact groups without exposing `ascii_punct`.
- Modify `third_party/typeduck-web/source/src/Inputs.tsx`: add a small fine-grained numeric/range input or discrete selector only if existing controls cannot express the measured prediction-threshold scale cleanly.
- Modify `third_party/typeduck-web/yune-integration/adapter.ts`: map new preferences to schema keys.
- Modify `third_party/typeduck-web/yune-integration/adapter-filesystem.test.ts`: cover new customize mappings.
- Modify `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`: add control honesty and guided scenario evidence.
- Modify `third_party/typeduck-web/e2e/yune-browser-smoke.md`: update the manual smoke path for M20.
- Modify `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`: regenerate the maintained TypeDuck-Web source patch after source changes.
- Modify docs only if execution finds a real scope correction.

Do not touch `crates/yune-rime-api/src/abi.rs`, the `RimeApi` table layout, or `RimeCandidate`.

---

## Task 1: Local Agent Instructions

**Files:**
- Create: `third_party/typeduck-web/AGENTS.md`
- Create: `packages/yune-typeduck-runtime/AGENTS.md`

- [ ] **Step 1: Create TypeDuck-Web instructions**

Create `third_party/typeduck-web/AGENTS.md`:

```markdown
# TypeDuck-Web Yune Integration Guide

This subtree is the browser demo/integration surface for Yune. The canonical source is the maintained patch in `patches/yune-typeduck-runtime.patch` plus the checked-out upstream app under `source/`.

## Rules

- Keep TypeDuck-Web changes in the upstream app patch unless a file is intentionally Yune-owned (`yune-integration/`, `e2e/`, `README.yune-source.md`).
- After editing `source/`, regenerate and reverse-check `patches/yune-typeduck-runtime.patch`.
- Preserve the native TypeDuck-Web fallback shape expected by the app: `Actions.processKey`, `stageAi`, candidate action methods, `customize`, `deploy`, and `setOption`.
- Use `customize()` for schema/deploy-time keys and `setOption()` for live session options. Do not add a new WASM export when the existing transport works.
- Every visible engine control must change candidate output, committed output, status output, or persisted config; display controls must change visible rendering. Do not expose `ascii_punct` as a working toggle until M18 implements the processor behavior.
- AI remains default-off, local-only, classic-first, and second-pass only. Do not move provider work into `processKey`.
- Browser validation must use real assets and committed Playwright/manual evidence.
```

- [ ] **Step 2: Create runtime-package instructions**

Create `packages/yune-typeduck-runtime/AGENTS.md`:

```markdown
# Yune TypeDuck Runtime Guide

This package wraps the Emscripten `yune_typeduck_*` API for TypeDuck-Web.

## Rules

- Keep the wrapper a thin transport layer. It must not implement candidate ranking, AI provider logic, or schema semantics in TypeScript.
- Pair every owned native response with `yune_typeduck_free_response`.
- Preserve key mapping tests when changing keyboard behavior.
- Preserve lifecycle safety: operations after cleanup must fail in TypeScript before reusing a native pointer.
- Do not widen `RimeCandidate`, reorder `RimeApi`, or add exports for UI-only convenience.
- AI provider work belongs only behind `stageAi`; `processKeyboardEvent` must remain classic-first.
```

- [ ] **Step 3: Verify no accidental patch churn**

Run:

```powershell
git status --short
```

Expected: only the two new `AGENTS.md` files are changed for this task.

- [ ] **Step 4: Commit**

```powershell
git add third_party/typeduck-web/AGENTS.md packages/yune-typeduck-runtime/AGENTS.md
git commit -m "docs: add TypeDuck-Web agent guides"
```

---

## Task 2: Preference Model And Adapter Mapping

**Files:**
- Modify: `third_party/typeduck-web/source/src/types.ts`
- Modify: `third_party/typeduck-web/source/src/consts.ts`
- Modify: `third_party/typeduck-web/yune-integration/adapter.ts`
- Modify: `third_party/typeduck-web/yune-integration/adapter-filesystem.test.ts`

- [ ] **Step 1: Extend preference types**

In `third_party/typeduck-web/source/src/types.ts`, extend `RimePreferences`:

```ts
export interface RimePreferences {
	pageSize: number;
	enableCompletion: boolean;
	enableCorrection: boolean;
	enableSentence: boolean;
	enableLearning: boolean;
	enableAI: boolean;
	combineCandidates: boolean;
	predictionNeverFirst: boolean;
	predictionThreshold: number;
	isAsciiMode: boolean;
	isFullShape: boolean;
	isSimplification: boolean;
	isCangjie5: boolean;
}
```

- [ ] **Step 2: Add defaults**

In `third_party/typeduck-web/source/src/consts.ts`, update `DEFAULT_PREFERENCES`:

```ts
export const DEFAULT_PREFERENCES: Preferences = {
	displayLanguages: new Set([Language.Eng]),
	mainLanguage: Language.Eng,
	pageSize: 6,
	isHeiTypeface: false,
	showRomanization: ShowRomanization.Always,
	enableCompletion: true,
	enableCorrection: false,
	enableSentence: true,
	enableLearning: true,
	enableAI: false,
	combineCandidates: true,
	predictionNeverFirst: true,
	predictionThreshold: 0,
	isAsciiMode: false,
	isFullShape: false,
	isSimplification: false,
	showReverseCode: true,
	isCangjie5: true,
};
```

`predictionNeverFirst` must default to `true` because the current TypeDuck-Web schema patch already opts into `prediction_never_first: true`; M20 must not silently change demo behavior.

`combineCandidates` must default to `true` only after a fresh real-browser check confirms that this preserves the current shipped `jyut6ping3_mobile` behavior. If that check proves the current asset default is already separate, use the real default and update this plan/evidence rather than silently flipping demo behavior.

- [ ] **Step 3: Extend adapter preference type**

In `third_party/typeduck-web/yune-integration/adapter.ts`, extend `RimePreferences`:

```ts
export interface RimePreferences {
  pageSize?: number;
  enableCompletion?: boolean;
  enableCorrection?: boolean;
  enableSentence?: boolean;
  enableLearning?: boolean;
  enableAI?: boolean;
  combineCandidates?: boolean;
  predictionNeverFirst?: boolean;
  predictionThreshold?: number;
  isCangjie5?: boolean;
  /** Pre-2024 options encoding */
  options?: number;
}
```

- [ ] **Step 4: Map prediction controls through `customize()`**

In `customize(preferences)`, after the boolean customization loop, add:

```ts
  if (preferences.combineCandidates !== undefined) {
    customizeSetting(
      "translator/combine_candidates",
      preferences.combineCandidates ? "true" : "false",
    );
  }

  if (preferences.predictionNeverFirst !== undefined) {
    customizeSetting(
      "translator/prediction_never_first",
      preferences.predictionNeverFirst ? "true" : "false",
    );
  }

  if (preferences.predictionThreshold !== undefined) {
    customizeSetting(
      "translator/prediction_weight_threshold",
      String(preferences.predictionThreshold),
    );
  }
```

Do not add a separate `prediction_frequency_threshold` UI mapping; that key is an alias of the same engine value.

- [ ] **Step 5: Add adapter test coverage**

In `third_party/typeduck-web/yune-integration/adapter-filesystem.test.ts`, extend the existing customize-call test so the fake runtime receives these additional calls:

```ts
await customize({
  pageSize: 6,
  enableCompletion: true,
  enableCorrection: false,
  enableSentence: true,
  enableLearning: true,
  combineCandidates: true,
  predictionNeverFirst: true,
  predictionThreshold: 0.05,
});

expect(module.calls("yune_typeduck_customize")).toEqual([
  [1, "jyut6ping3_mobile", "page_size", "6"],
  [1, "jyut6ping3_mobile", "translator/enable_completion", "true"],
  [1, "jyut6ping3_mobile", "translator/enable_correction", "false"],
  [1, "jyut6ping3_mobile", "translator/enable_sentence", "true"],
  [1, "jyut6ping3_mobile", "translator/enable_user_dict", "true"],
  [1, "jyut6ping3_mobile", "translator/encode_commit_history", "true"],
  [1, "jyut6ping3_mobile", "translator/combine_candidates", "true"],
  [1, "jyut6ping3_mobile", "translator/prediction_never_first", "true"],
  [1, "jyut6ping3_mobile", "translator/prediction_weight_threshold", "0.05"],
]);
```

Adjust the expected schema id if the existing fake fixture uses another id.

- [ ] **Step 6: Run focused runtime tests**

```powershell
npm --prefix packages/yune-typeduck-runtime test
```

Expected: PASS. The TypeDuck-Web source checkout has build scripts but no package-local test script; the adapter mapping is verified by the Playwright honesty gate in Task 5.

- [ ] **Step 7: Commit**

```powershell
git add third_party/typeduck-web/source/src/types.ts third_party/typeduck-web/source/src/consts.ts third_party/typeduck-web/yune-integration/adapter.ts third_party/typeduck-web/yune-integration/adapter-filesystem.test.ts
git commit -m "feat: wire TypeDuck-Web demo preferences"
```

---

## Task 3: UI Controls

**Files:**
- Modify: `third_party/typeduck-web/source/src/App.tsx`
- Modify: `third_party/typeduck-web/source/src/Preferences.tsx`
- Modify: `third_party/typeduck-web/source/src/Inputs.tsx` if needed

- [ ] **Step 1: Add a compact range helper if needed**

If `Preferences.tsx` needs a reusable numeric range control, add this to `Inputs.tsx`:

```tsx
interface RangeProps {
	label: string;
	min: number;
	max: number;
	step: number;
	value: number;
	setValue(value: number): void;
}

export function Range({ label, min, max, step, value, setValue }: RangeProps) {
	return <label className="label block">
		<div className="flex items-center gap-3">
			<span className="text-lg text-base-content-200 flex-1">{label}</span>
			<span className="badge badge-outline">{value}</span>
		</div>
		<input
			type="range"
			className="range range-primary range-sm"
			min={min}
			max={max}
			step={step}
			value={value}
			onChange={event => setValue(Number(event.target.value))} />
	</label>;
}
```

- [ ] **Step 2: Route deploy-time controls**

In `App.tsx`, include new deploy-time controls in the first customize effect:

```tsx
const {
	pageSize,
	enableCompletion,
	enableCorrection,
	enableSentence,
	enableLearning,
	enableAI,
	combineCandidates,
	predictionNeverFirst,
	predictionThreshold,
	isAsciiMode,
	isFullShape,
	isSimplification,
	isCangjie5,
	isHeiTypeface,
} = preferences;
```

Then pass prediction controls to `Rime.customize`:

```tsx
const success = await Rime.customize({
	pageSize,
	enableCompletion,
	enableCorrection,
	enableSentence,
	enableLearning,
	combineCandidates,
	predictionNeverFirst,
	predictionThreshold,
	isCangjie5,
});
```

Update the effect dependency list to include `combineCandidates`, `predictionNeverFirst`, and `predictionThreshold`.

- [ ] **Step 3: Route live session controls**

Add a separate effect in `App.tsx`:

```tsx
const [optionStatus, updateOptionStatus] = useReducer((n: number) => n + 1, 0);

useEffect(() =>
	runAsyncTask(async () => {
		let type: "warning" | "error" | undefined;
		try {
			await Rime.setOption("ascii_mode", isAsciiMode);
			await Rime.setOption("full_shape", isFullShape);
			await Rime.setOption("simplification", isSimplification);
		}
		catch {
			type = "error";
		}
		if (type) {
			notify(type, "Apply live options", "applying the live options");
		}
		updateOptionStatus();
	}), [isAsciiMode, isFullShape, isSimplification, updateOptionStatus, runAsyncTask]);
```

If `optionStatus` is not consumed, keep it only if needed to retrigger a child component; otherwise remove it before commit.

- [ ] **Step 4: Render controls**

In `Preferences.tsx`, add visible controls for:

```tsx
<Toggle label="Prediction never first" checked={prefs.predictionNeverFirst} setChecked={prefs.setPredictionNeverFirst} />
<Toggle label="Combine same-text candidates" checked={prefs.combineCandidates} setChecked={prefs.setCombineCandidates} />
<Range label="Prediction threshold" min={thresholdRange.min} max={thresholdRange.max} step={thresholdRange.step} value={prefs.predictionThreshold} setValue={prefs.setPredictionThreshold} />
<Toggle label="ASCII mode" checked={prefs.isAsciiMode} setChecked={prefs.setIsAsciiMode} />
<Toggle label="Full shape" checked={prefs.isFullShape} setChecked={prefs.setIsFullShape} />
<Toggle label="Simplification" checked={prefs.isSimplification} setChecked={prefs.setIsSimplification} />
```

Do not ship placeholder threshold bounds. Before finalizing the UI, measure the real loaded WASM candidate-quality scale and set `thresholdRange` to a useful fine-grained range, logarithmic scale, or small preset list. The evidence must prove that the chosen cutoff filters prediction rows while exact classic rows remain available.

Keep `ascii_punct` absent from the active controls. If the UI mentions it, the text must say it is deferred to M18 and must not render a toggle.

- [ ] **Step 5: Build the app**

From `third_party/typeduck-web/source`, run the app's normal build command used by the current checkout:

```powershell
npm run build
```

If the checkout uses Bun-only scripts, run:

```powershell
bun run build
```

Expected: TypeScript/build PASS. If the command is absent, record the exact package script blocker in the M20 evidence directory and rely on Playwright plus runtime package build.

- [ ] **Step 6: Commit**

```powershell
git add third_party/typeduck-web/source/src/App.tsx third_party/typeduck-web/source/src/Preferences.tsx third_party/typeduck-web/source/src/Inputs.tsx
git commit -m "feat: add TypeDuck-Web showcase controls"
```

---

## Task 4: Guided Scenarios

**Files:**
- Create or modify: `third_party/typeduck-web/source/src/YuneFeatureShowcase.tsx`
- Modify: `third_party/typeduck-web/source/src/App.tsx`
- Modify: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`

- [ ] **Step 1: Create scenario data**

Create `YuneFeatureShowcase.tsx` with compact scenario buttons. Keep the content concise; do not add long tutorial copy.

```tsx
interface Scenario {
	label: string;
	input: string;
}

const SCENARIOS: Scenario[] = [
	{ label: "nei", input: "nei" },
	{ label: "santai", input: "santai" },
	{ label: "m", input: "m" },
	{ label: "mgoi", input: "mgoi" },
	{ label: "tone letters", input: "seov" },
	{ label: "AI trigger", input: "santai" },
];
```

- [ ] **Step 2: Implement scenario dispatch**

Use real keyboard events against the textarea so the normal TypeDuck-Web key path runs:

```tsx
function keyCodeFor(char: string): string {
	if (/^[a-z]$/.test(char)) {
		return `Key${char.toUpperCase()}`;
	}
	return char;
}

function sendPrintable(textArea: HTMLTextAreaElement, char: string) {
	textArea.dispatchEvent(new KeyboardEvent("keydown", {
		key: char,
		code: keyCodeFor(char),
		bubbles: true,
		cancelable: true,
	}));
	textArea.dispatchEvent(new KeyboardEvent("keyup", {
		key: char,
		code: keyCodeFor(char),
		bubbles: true,
		cancelable: true,
	}));
}
```

Scenario buttons should focus the textarea, clear any current composition with Escape, then dispatch each input character.

- [ ] **Step 3: Render the panel**

In `App.tsx`, render the scenario panel near the input surface:

```tsx
{textArea && <YuneFeatureShowcase textArea={textArea} />}
```

Keep it compact and work-surface-like; no landing-page hero or marketing layout.

- [ ] **Step 4: Add Playwright scenario checks**

In `yune-typeduck.spec.ts`, add assertions that clicking scenario buttons drives the real candidate panel. Required minimum:

```ts
await page.getByRole("button", { name: "santai" }).click();
await expect(page.locator(".candidate-panel")).toContainText("身體");
await expect(page.locator(".candidate-panel")).toContainText("身體健康");
```

Add a fuzzy scenario assertion:

```ts
await clearComposition(page);
await page.getByRole("button", { name: "mgoi" }).click();
await expect(page.locator(".candidate-panel")).toContainText("唔該");
```

- [ ] **Step 5: Commit**

```powershell
git add third_party/typeduck-web/source/src/YuneFeatureShowcase.tsx third_party/typeduck-web/source/src/App.tsx third_party/typeduck-web/e2e/yune-typeduck.spec.ts
git commit -m "feat: add TypeDuck-Web guided scenarios"
```

---

## Task 5: Control Honesty Tests

**Files:**
- Modify: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`
- Modify: `third_party/typeduck-web/e2e/yune-browser-smoke.md`

- [ ] **Step 1: Add an explicit list of active controls in the E2E**

In `yune-typeduck.spec.ts`, define the controls the browser is expected to expose:

```ts
const ACTIVE_SHOWCASE_CONTROLS = [
  /Auto-completion/,
  /Auto-correction/,
  /Auto-composition/,
  /Input Memory/,
  /AI Candidates/,
  /Combine same-text candidates/,
  /Prediction never first/,
  /Prediction threshold/,
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
```

- [ ] **Step 2: Assert every listed control exists**

```ts
for (const label of ACTIVE_SHOWCASE_CONTROLS) {
  await expect(page.getByLabel(label)).toBeVisible();
}
for (const label of DISPLAY_SHOWCASE_CONTROLS) {
  await expect(page.getByText(label)).toBeVisible();
}
await expect(page.getByLabel(/ascii_punct/i)).toHaveCount(0);
```

Keep these labels aligned with the **Display Controls** table. If a display control is implemented as an actual form input, assert it with `getByLabel()` using the same rendered label; otherwise use `getByText()` for static labels. Do not list `show_full_code` here unless it becomes a real browser-reachable display control; otherwise it remains a guided scenario.

- [ ] **Step 3: Assert every control changes observable behavior**

Add one test per control family:

- `Prediction never first`: learn or use the existing deterministic `ngo` setup; assert `我` remains first when on and a learned prefix prediction can move when off.
- `Combine same-text candidates`: use an M14-backed homograph input such as `hou`; assert same-text rows group when on and split when off.
- `Prediction threshold`: set threshold `0`, capture prediction rows for `santai`, then set the real-assets-derived cutoff and assert exact candidates such as `身體` remain while lower-weight prediction rows are filtered. Expect a fine sub-1 cutoff unless measurement proves otherwise.
- `ASCII mode`: enable, type printable letters, assert they enter the textarea instead of opening the candidate panel.
- `Full shape`: enable, type `/` or a punctuation key with known full-shape mapping, assert output/status changes.
- `Simplification`: enable, type an M16 simplification scenario, assert simplified output appears.
- Display controls: language columns, candidate Jyutping visibility, and reverse-code visibility must each show a before/after rendering change.
- `show_full_code`: if M20 adds a browser-reachable `jyut6ping3`/Cangjie side-lookup path, assert `cangjie/show_full_code` changes `` `c...; `` comments; otherwise assert the M20 evidence explicitly records it as not browser-reachable in the current mobile-only surface.
- Existing controls: keep or strengthen current completion/correction/sentence/learning/AI assertions.

Every assertion must compare before/after candidate text, commit text, status, persisted config, or display rendering. A test that only asserts `consoleErrors == []` is not sufficient for M20.

- [ ] **Step 4: Update manual smoke procedure**

In `yune-browser-smoke.md`, add an M20 section:

```markdown
### M20 Showcase Controls

1. Toggle each visible active control.
2. Toggle each visible display control.
3. Record the before/after candidate, status, commit, persisted-config, or rendering difference.
4. Confirm `ascii_punct` is not presented as a working toggle.
5. Run the guided `santai`, `mgoi`, `m`, tone-letter, show-full-code/N-A, hide-lone-schema/N-A, and AI-trigger scenarios.
6. Save screenshots and JSON state under `e2e/results/m20-showcase-controls/`.
```

- [ ] **Step 5: Commit**

```powershell
git add third_party/typeduck-web/e2e/yune-typeduck.spec.ts third_party/typeduck-web/e2e/yune-browser-smoke.md
git commit -m "test: prove TypeDuck-Web showcase controls"
```

---

## Task 6: Patch Regeneration And Reverse Check

**Files:**
- Modify: `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`

- [ ] **Step 1: Regenerate the maintained patch**

From the repository root, regenerate the TypeDuck-Web source patch using the same patch discipline already used in this subtree. If the exact local helper is not present, use a clean upstream source checkout in `third_party/typeduck-web/source` and produce a patch that includes the changed `source/src/*` files.

Expected: `third_party/typeduck-web/patches/yune-typeduck-runtime.patch` includes the M20 source changes and keeps the existing Yune integration edits.

- [ ] **Step 2: Reverse-check the patch**

Run from `third_party/typeduck-web/source`:

```powershell
git apply --reverse --check ..\patches\yune-typeduck-runtime.patch
```

Expected: command exits 0.

- [ ] **Step 3: Forward-check the patch on a clean source tree**

After resetting or recreating the source checkout to the recorded upstream state, run:

```powershell
git apply --check ..\patches\yune-typeduck-runtime.patch
```

Expected: command exits 0.

- [ ] **Step 4: Commit**

```powershell
git add third_party/typeduck-web/patches/yune-typeduck-runtime.patch
git commit -m "chore: refresh TypeDuck-Web showcase patch"
```

---

## Task 7: Final Verification

**Files:**
- Modify only evidence files under `third_party/typeduck-web/e2e/results/m20-showcase-controls/` if browser evidence is captured.

- [ ] **Step 1: Format**

```powershell
cargo fmt
```

Expected: PASS / no unexpected Rust changes.

- [ ] **Step 2: Native TypeDuck-Web fallback**

```powershell
cargo test -p yune-rime-api --test typeduck_web
```

Expected: PASS.

- [ ] **Step 3: Workspace tests**

```powershell
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 4: Clippy**

```powershell
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 5: TypeScript runtime tests**

```powershell
npm --prefix packages/yune-typeduck-runtime test
```

Expected: PASS.

- [ ] **Step 6: TypeScript runtime build**

```powershell
npm --prefix packages/yune-typeduck-runtime run build
```

Expected: PASS.

- [ ] **Step 7: Real TypeDuck-Web Playwright E2E**

Run the procedure in `third_party/typeduck-web/e2e/yune-browser-smoke.md`, including the M20 showcase controls and the existing M13/M16 scenarios.

Expected evidence directory:

```text
third_party/typeduck-web/e2e/results/m20-showcase-controls/
```

Required evidence:

- Browser run log.
- Console warning/error log.
- JSON state snapshots for active-control before/after checks.
- Screenshots for at least `santai`, `mgoi`, prediction-never-first, AI second pass, and one live `setOption()` control.
- `blocker.md` only if a real browser/tooling blocker occurs; it must not replace browser evidence when the browser can run.

- [ ] **Step 8: Whitespace check**

```powershell
git diff -- crates/yune-rime-api/src/abi.rs crates/yune-rime-api/src/api_table.rs crates/yune-rime-api/src/candidate_api.rs
```

Expected: no diff. M20 UI/playground work must not alter the upstream `RimeApi` table, `RimeCandidate`, or ABI-owned candidate structs.

- [ ] **Step 9: Whitespace check**

```powershell
git diff --check
```

Expected: no whitespace errors.

- [ ] **Step 10: Commit browser evidence and any remaining checked files**

```powershell
git add third_party/typeduck-web/e2e/results/m20-showcase-controls
git add docs/plans/m20-plan-web-demo-showcase-controls.md docs/roadmap.md docs/requirements.md
git commit -m "test: record TypeDuck-Web showcase evidence"
```

Stage explicit paths only. If earlier tasks already committed the plan/docs, omit those paths here. Do not stage unrelated root docs, scratch files, local browser cache, `node_modules`, or untracked generated files outside the intended evidence directory.

---

## Completion Criteria

M20 is complete when:

- TypeDuck-Web is documented and built as Yune's canonical browser playground for demoing and stress-testing supported engine behavior.
- The TypeDuck-Web UI exposes the new controls without `ascii_punct` as a fake active toggle.
- `combineCandidates` uses the verified shipped default for `jyut6ping3_mobile`, `predictionNeverFirst` defaults on, and the UI has one measured real-assets-scaled prediction threshold control.
- Display-only controls are grouped separately from engine/runtime controls and have rendering assertions.
- Guided scenarios demonstrate static/default-on Cantonese features, `show_full_code` reachability/N-A, and browser-inspection limits without representing them as toggles.
- Local AGENTS files document patch/runtime rules.
- Every visible active and display control has a real before/after assertion.
- The TypeDuck-Web maintained patch reverse-checks cleanly.
- The final diff leaves `abi.rs`, `api_table.rs`, and `candidate_api.rs` unchanged.
- Full Rust, TypeScript, and real-browser gates pass or record a precise blocker.

When complete, archive this plan under `docs/plans/archive/` with a scoped docs commit.
