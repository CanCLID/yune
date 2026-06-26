# M41 yune-web Startup Optimization Plan

> **Status:** Active - **Milestone:** M41 (yune-web startup optimization) -
> **Created:** 2026-06-26 - **Type:** browser-harness performance plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce real-browser startup latency for the tracked
`apps/yune-web/` harness using fresh post-M40 evidence, without making native
engine-performance claims from browser data or regressing typing behavior.

**Architecture:** M41 is a browser harness milestone, not an engine milestone.
It starts with a production-build baseline that splits UI shell, asset
delivery/cache, worker/WASM startup, virtual filesystem/persistence, deploy
reuse, schema selection, first key-to-paint, and browser memory. Optimization
then targets the measured top owner only, with `luna_pinyin` and
`jyut6ping3_mobile` kept as separate browser scenarios.

**Tech Stack:** `apps/yune-web/` Vite/React app, `apps/yune-web/src/worker.ts`,
`apps/yune-web/src/rime.ts`, `apps/yune-web/src/yune-integration/*`,
`apps/yune-web/public-demo/`, Playwright, Chromium CDP metrics, browser
Performance APIs, Windows process working-set sampling, and evidence under
`apps/yune-web/e2e/results/m41-yune-web-startup-optimization/`.

---

## Rationale

M40 closes the native Track A long-row owner, but it intentionally does not
claim web startup or public-demo speed. The root-cause report now records the
next work boundary: web harness startup must be measured and optimized as a
separate browser/application problem.

Earlier browser evidence suggests startup pain can come from browser shell,
asset delivery, worker/WASM startup, cache behavior, virtual filesystem,
schema deploy/reuse, and schema selection. The next milestone must not guess.
It must first produce a post-M40 browser baseline and then optimize the largest
measured owner.

## Non-Negotiable Scope Rules

- Work in the tracked `apps/yune-web/` app. Do not edit
  `apps/yune-web/source/` or regenerate the retired patch unless a plan task
  explicitly changes that rule.
- Do not touch `crates/` or `packages/` unless the browser evidence proves the
  top owner is inside the runtime/engine boundary and the plan is updated with
  a narrow justification.
- Keep `yune-web` visible naming and public-demo identity intact.
- AI remains default-off, local-only, classic-first, and outside startup
  optimization unless a failing startup test proves the AI path is active.
- Native M40 numbers are an engine baseline only. M41 claims require
  real-browser evidence.

## Closeout Gates

- `M41-YWEB-01` (fresh baseline): final evidence includes post-M40 production
  baselines for tracked `apps/yune-web` build and public-demo build, with
  real-worker and mock-worker modes.
- `M41-YWEB-02` (scenario coverage): baseline and final runs include
  `luna_pinyin` and `jyut6ping3_mobile` startup scenarios. Track A typing rows
  include `hao`, `ni`, `zhongguo`,
  `ceshiyixiachangjushuruxingnengzenyang`,
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`,
  `cszysmsrsd`, and `zybfshmsru`. Track B rows include `hai`, `ngo`, `caksi`,
  `sihaacoenggeoisyujapgecukdou`, and
  `taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng`.
- `M41-YWEB-03` (owner attribution): startup is split into browser shell,
  resource transfer, worker script load, WASM fetch/instantiate, virtual
  filesystem/persistence, asset cache, schema deploy/reuse, schema select, and
  React ready-to-input.
- `M41-YWEB-04` (measured optimization): the final implementation reduces the
  measured top startup owner by at least `50%`, or records a measured blocker
  and reduces total ready-to-input by at least `30%` on the affected scenario.
- `M41-YWEB-05` (cache correctness): any `Cache.put` or browser cache failure
  is fixed or explicitly proven absent; the public-demo build must not be
  blocked by a cache exception.
- `M41-YWEB-06` (startup budgets): final cold real-worker ready-to-input
  medians are no worse than `1.5s` for `luna_pinyin` and `3.0s` for
  `jyut6ping3_mobile`, or the final report records why the budget is not yet
  reachable and names the remaining owner. Warm reload and warm new-page
  medians must improve from baseline.
- `M41-YWEB-07` (typing no-regression): first keydown-to-paint after ready does
  not regress more than `10%` for required Track A and Track B rows; incomplete
  pinyin rows are reported as behavior probes if they export no candidates.
- `M41-YWEB-08` (memory no-regression): Chromium JS heap, DOM/node counts,
  `performance.measureUserAgentSpecificMemory()` when available, and Windows
  process working set are recorded before/after/peak. Final memory must not
  regress more than `10%` unless the owner is attributed and accepted.
- `M41-YWEB-09` (public-demo honesty): public-demo evidence covers asset
  manifest, cache headers, service/cache behavior, startup readiness, and
  first typing. It does not claim native engine speed.
- `M41-YWEB-10` (quality gates): final closeout passes
  `npm.cmd --prefix apps/yune-web run build`,
  `npm.cmd --prefix apps/yune-web run build:public`,
  `npm.cmd --prefix apps/yune-web run typecheck`, the M41 startup benchmark,
  focused browser smoke, and `git diff --check`. Rust gates are required only
  if M41 touches Rust.

## File Responsibilities

- `apps/yune-web/e2e/yune-web-startup-benchmark.spec.ts`: create a focused
  Playwright startup benchmark separate from the broad smoke spec.
- `apps/yune-web/e2e/startup-benchmark/`: create if helper modules are needed
  for scenario definitions, browser memory sampling, resource aggregation, and
  dashboard generation.
- `apps/yune-web/e2e/results/m41-yune-web-startup-optimization/`: raw JSON,
  CSV, screenshots, dashboard markdown, and SVG visualizations.
- `apps/yune-web/src/worker.ts`: optimize only measured worker, asset, cache,
  deploy, or schema-selection owners.
- `apps/yune-web/src/rime.ts`: optimize only measured main-thread/worker
  queue, ready-state, clear-cache, or first-key-to-paint owners.
- `apps/yune-web/src/yune-integration/adapter.ts`: optimize only measured
  deploy/reuse, persistence, or filesystem owners.
- `apps/yune-web/src/yune-integration/assets.ts`: optimize only measured asset
  loading/validation owners.
- `apps/yune-web/public-demo/*`: optimize only measured public-demo packaging,
  asset, cache-header, or manifest owners.
- `docs/reports/yune-vs-librime-performance.md` and
  `docs/reports/yune-vs-librime-root-cause-analysis.md`: preserve the native
  engine boundary; add only a pointer to M41 browser evidence if needed.
- `docs/roadmap.md`, `docs/requirements.md`, `docs/decisions.md`, and
  `docs/ledgers/milestone-history.md`: update on closeout.

## Task 0 - Baseline Harness And Evidence Shape

**Files:**

- Create: `apps/yune-web/e2e/yune-web-startup-benchmark.spec.ts`
- Create: `apps/yune-web/e2e/startup-benchmark/scenarios.ts`
- Create: `apps/yune-web/e2e/startup-benchmark/metrics.ts`
- Create:
  `apps/yune-web/e2e/results/m41-yune-web-startup-optimization/phase-0-baseline/`

- [ ] **Step 0.1: Define benchmark scenarios**

Create `apps/yune-web/e2e/startup-benchmark/scenarios.ts` with:

```ts
export type StartupSchema = "luna_pinyin" | "jyut6ping3_mobile";

export type StartupMode =
  | "real-worker-cold"
  | "real-worker-warm-reload"
  | "real-worker-warm-new-page"
  | "mock-worker-cold"
  | "mock-worker-warm";

export interface StartupScenario {
  id: string;
  schema: StartupSchema;
  mode: StartupMode;
  publicDemo: boolean;
  samples: number;
  inputs: string[];
}

export const trackAInputs = [
  "hao",
  "ni",
  "zhongguo",
  "ceshiyixiachangjushuruxingnengzenyang",
  "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
  "cszysmsrsd",
  "zybfshmsru",
] as const;

export const trackBInputs = [
  "hai",
  "ngo",
  "caksi",
  "sihaacoenggeoisyujapgecukdou",
  "taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng",
] as const;

export const startupScenarios: StartupScenario[] = [
  { id: "tracked-luna-cold", schema: "luna_pinyin", mode: "real-worker-cold", publicDemo: false, samples: 10, inputs: [...trackAInputs] },
  { id: "tracked-luna-warm-reload", schema: "luna_pinyin", mode: "real-worker-warm-reload", publicDemo: false, samples: 20, inputs: [...trackAInputs] },
  { id: "tracked-luna-warm-new-page", schema: "luna_pinyin", mode: "real-worker-warm-new-page", publicDemo: false, samples: 20, inputs: [...trackAInputs] },
  { id: "tracked-jyut-cold", schema: "jyut6ping3_mobile", mode: "real-worker-cold", publicDemo: false, samples: 10, inputs: [...trackBInputs] },
  { id: "tracked-jyut-warm-reload", schema: "jyut6ping3_mobile", mode: "real-worker-warm-reload", publicDemo: false, samples: 20, inputs: [...trackBInputs] },
  { id: "tracked-jyut-warm-new-page", schema: "jyut6ping3_mobile", mode: "real-worker-warm-new-page", publicDemo: false, samples: 20, inputs: [...trackBInputs] },
  { id: "tracked-mock-cold", schema: "luna_pinyin", mode: "mock-worker-cold", publicDemo: false, samples: 10, inputs: ["hao"] },
  { id: "tracked-mock-warm", schema: "luna_pinyin", mode: "mock-worker-warm", publicDemo: false, samples: 20, inputs: ["hao"] },
  { id: "public-luna-cold", schema: "luna_pinyin", mode: "real-worker-cold", publicDemo: true, samples: 10, inputs: [...trackAInputs] },
  { id: "public-jyut-cold", schema: "jyut6ping3_mobile", mode: "real-worker-cold", publicDemo: true, samples: 10, inputs: [...trackBInputs] },
];
```

Expected: scenario definitions make both schema tracks explicit.

- [ ] **Step 0.2: Capture startup metrics**

Create `apps/yune-web/e2e/startup-benchmark/metrics.ts` with helpers that
collect:

```ts
export interface StartupSample {
  scenarioId: string;
  sampleIndex: number;
  url: string;
  schema: string;
  mode: string;
  readyToInputMs: number;
  startupCompleteMs?: number;
  phases: Array<{ phase: string; ms: number }>;
  navigation: Record<string, number>;
  resources: Array<{ name: string; initiatorType: string; transferSize: number; encodedBodySize: number; decodedBodySize: number; duration: number }>;
  cache: { hits: number; misses: number; errors: string[] };
  storageEstimate?: { usage?: number; quota?: number };
  browserMemory?: Record<string, number>;
  firstKeyToPaint: Array<{ input: string; ms: number; candidateCount?: number }>;
}
```

Expected: the runner can serialize bounded JSON and CSV for every sample.

- [ ] **Step 0.3: Add mock-worker mode**

In the Playwright spec, intercept `worker.js` for `mock-worker-*` scenarios and
serve a tiny worker that responds to startup, schema, and process-key calls
without loading WASM:

```js
self.postMessage({
  type: "listener",
  name: "persistenceDiagnostic",
  args: [{
    source: "yune-startup",
    marker: { phase: "startup:complete", totalMs: 1, markers: [{ phase: "mock-worker:start", ms: 0 }] }
  }]
});
self.onmessage = (event) => {
  const { name } = event.data;
  self.postMessage({
    type: "success",
    result: name === "processKey" ? { handled: true, candidates: [], context: { composition: "" } } : true,
    elapsedMs: 0,
  });
};
```

Expected: mock-worker startup isolates React/browser shell from worker/WASM and
asset costs.

- [ ] **Step 0.4: Run phase 0 baseline**

Run:

```powershell
npm.cmd --prefix apps\yune-web run build
npm.cmd --prefix apps\yune-web run build:public
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M41 STARTUP" --workers=1
```

Expected: raw baseline JSON/CSV and a baseline dashboard are written under
`phase-0-baseline/`.

## Task 1 - Name The Top Owner

**Files:**

- Modify: `apps/yune-web/e2e/startup-benchmark/metrics.ts`
- Create:
  `apps/yune-web/e2e/results/m41-yune-web-startup-optimization/phase-1-owner/owner-summary.md`

- [ ] **Step 1.1: Aggregate owner buckets**

Compute median and p95 for:

- browser shell without worker;
- worker script load;
- WASM JS and binary resource transfer;
- WASM instantiate/runtime init;
- schema asset transfer by file group;
- cache hit/miss/error counts;
- virtual filesystem/persistence sync;
- deploy cache hit/miss;
- schema selection;
- ready-to-input;
- first keydown-to-paint;
- memory before/after/peak.

Expected: one top current owner is named per scenario group.

- [ ] **Step 1.2: Decide optimization order**

Write `phase-1-owner/owner-summary.md` with:

```markdown
# M41 Phase 1 Owner Summary

| Scenario group | Top owner | Median | p95 | Planned fix |
| --- | ---: | ---: | ---: | --- |
| tracked luna cold | ... | ... | ... | ... |
| tracked jyut cold | ... | ... | ... | ... |
| public luna cold | ... | ... | ... | ... |
| public jyut cold | ... | ... | ... | ... |

Decision: optimize <owner> first because it is the largest repeated blocker.
```

Expected: no implementation begins until this owner table exists.

## Task 2 - Fix Cache And Asset Delivery Owners

**Files:**

- Modify: `apps/yune-web/src/worker.ts`
- Modify: `apps/yune-web/public-demo/build.mjs` or `build.ps1` only if the
  public-demo manifest/header owner is measured.
- Modify: `apps/yune-web/e2e/yune-web-startup-benchmark.spec.ts`

- [ ] **Step 2.1: Make cache writes safe**

If phase 1 records `Cache.put` errors, update `cachePublicAsset` in
`apps/yune-web/src/worker.ts` so it only caches successful same-origin basic
responses and records a diagnostic instead of blocking startup:

```ts
if (!response.ok || response.type === "opaque") {
  dispatch("persistenceDiagnostic", {
    source: "yune-startup",
    marker: { phase: "asset-cache:skip", path, status: response.status, responseType: response.type },
  });
  return responseAssetContent(response, path);
}

try {
  await cache.put(cacheRequest, response.clone());
} catch (error) {
  dispatch("persistenceDiagnostic", {
    source: "yune-startup",
    marker: { phase: "asset-cache:error", path, error: error instanceof Error ? error.message : String(error) },
  });
}
```

Expected: cache failures never block startup or public-demo readiness.

- [ ] **Step 2.2: Reduce transferred startup assets**

If resource attribution shows non-active schema assets dominate cold startup,
change startup loading so only the active schema and minimal reachable shared
dependencies load before ready-to-input. Defer non-active schema assets until
schema switch.

Expected: final resource table shows fewer startup bytes for the affected
schema without breaking schema switch smoke.

## Task 3 - Fix Worker/WASM And Deploy Reuse Owners

**Files:**

- Modify: `apps/yune-web/src/worker.ts`
- Modify: `apps/yune-web/src/yune-integration/adapter.ts`
- Modify: `apps/yune-web/src/yune-integration/assets.ts`
- Modify: `apps/yune-web/src/rime.ts`

- [ ] **Step 3.1: Preserve deployed reuse**

If deploy cache miss dominates warm startup, change the startup path so warm
reload/new-page reuse deployed assets when signatures match and skips schema
deploy work.

Expected: warm scenarios record `deploy:cache-hit` and reduced
ready-to-input.

- [ ] **Step 3.2: Split shell-ready from engine-ready without faking input**

If React shell blocks while the worker initializes, expose a separate shell
ready marker but keep input disabled until engine ready:

```ts
window.dispatchEvent(new CustomEvent("yune:shell-ready", { detail: { at: performance.now() } }));
```

Expected: reports can distinguish visual shell paint from true ready-to-input.

## Task 4 - Typing And Memory No-Regression

**Files:**

- Modify: `apps/yune-web/e2e/yune-web-startup-benchmark.spec.ts`
- Modify: app files touched by Tasks 2-3 if first-key-to-paint regresses.

- [ ] **Step 4.1: Compare first key-to-paint**

For each required input, type after ready and record:

- keydown timestamp;
- worker queue wait;
- worker processing;
- main-thread response mapping;
- React state update;
- paint proxy;
- candidate count.

Expected: no required row regresses more than `10%` from phase 0 unless the
owner is named and fixed.

- [ ] **Step 4.2: Compare browser memory**

Record:

- CDP `Performance.getMetrics`;
- DOM node counts;
- JS heap used/total;
- `performance.measureUserAgentSpecificMemory()` when available;
- Windows process working set keyed to the Playwright user-data-dir.

Expected: memory no-regression gate is backed by data.

## Task 5 - Final Evidence And Docs Closeout

**Files:**

- Create:
  `apps/yune-web/e2e/results/m41-yune-web-startup-optimization/phase-4-final/`
- Create:
  `apps/yune-web/e2e/results/m41-yune-web-startup-optimization/final-gates.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Modify: `docs/decisions.md`
- Modify: `docs/ledgers/milestone-history.md`
- Move on closeout:
  `docs/plans/active/m41-plan-yune-web-startup-optimization.md` to
  `docs/plans/completed/m41-plan-yune-web-startup-optimization.md`

- [ ] **Step 5.1: Run final gates**

Run:

```powershell
npm.cmd --prefix apps\yune-web run build
npm.cmd --prefix apps\yune-web run build:public
npm.cmd --prefix apps\yune-web run typecheck
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M41 STARTUP" --workers=1
npm.cmd --prefix apps\yune-web\e2e run test:e2e:smoke -- --workers=1
git diff --check
```

Expected: all pass. Run Rust gates only if Rust files changed.

- [ ] **Step 5.2: Write final dashboard**

Write `final-gates.md` with:

- commit SHA and dirty state;
- build mode and URL;
- browser version;
- sample counts;
- cold/warm/mock rows;
- startup owner stacked chart;
- asset transfer/cache table;
- memory before/after/peak table;
- first-key-to-paint table;
- top-owner verdict and final remaining owner.

Expected: a future session can decide the next optimization from the dashboard
without reading raw JSON first.

## Out Of Scope

| Area | Reason |
| --- | --- |
| Native engine parity claims | Covered by M40 reports; M41 is browser-only. |
| Product repository work | `apps/yune-web` is this repo's harness, not the separate TypeDuck-Web product repo. |
| New AI UX | AI remains default-off and outside startup optimization. |
| Broad UI redesign | Only measured startup/typing owners are in scope. |
| Rust engine rewrites | Require a separate engine plan unless M41 evidence proves a narrow runtime boundary blocker. |

## What Success Looks Like

- A reader can see exactly why yune-web startup was slow before M41.
- The top measured owner is reduced, not guessed around.
- Public-demo cache/startup errors are fixed or proven absent.
- `luna_pinyin` and `jyut6ping3_mobile` browser startup are both covered.
- First typing after ready remains responsive.
- Browser memory does not drift silently.
- The final report clearly separates browser harness wins from native engine
  wins.
