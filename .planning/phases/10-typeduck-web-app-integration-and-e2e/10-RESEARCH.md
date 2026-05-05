# Phase 10: TypeDuck-Web App Integration And E2E - Research

**Researched:** 2026-05-05 [VERIFIED: system current date]
**Domain:** Upstream TypeDuck-Web browser app integration, Emscripten/WASM worker bridge replacement, and real browser E2E validation [VERIFIED: .planning/ROADMAP.md]
**Confidence:** MEDIUM-HIGH — HIGH for local Yune adapter/runtime contracts and observed TypeDuck-Web seam; MEDIUM for exact patch strategy because no Phase 10 CONTEXT.md locked decisions exist and the real Yune Emscripten output is unavailable locally. [VERIFIED: local file reads; VERIFIED: local tool probes]

<user_constraints>
## User Constraints

### Locked Decisions
Phase 10 CONTEXT.md exists at `.planning/phases/10-typeduck-web-app-integration-and-e2e/10-CONTEXT.md` and locks the integration boundary, upstream source handling, Yune bridge integration shape, browser E2E validation, and AI-native recommendation criteria. [VERIFIED: context created before copying this research into the main workspace]

### Claude's Discretion
No Phase 10 CONTEXT.md exists, so implementation details not fixed by `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, prior phase handoff docs, or project source remain planner discretion. [VERIFIED: Read returned file does not exist; VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: .planning/ROADMAP.md]

### Deferred Ideas (OUT OF SCOPE)
No Phase 10 CONTEXT.md exists, so deferred ideas are taken from roadmap/project state: AI provider, ranking, context, memory, privacy behavior, plugin compatibility, and a new product GUI frontend remain deferred until TypeDuck-Web integration produces a go/no-go recommendation. [VERIFIED: .planning/ROADMAP.md; VERIFIED: .planning/STATE.md]
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TYPEDUCK-E2E-01 | The upstream TypeDuck-Web repository is cloned or vendored in a reproducible test location, and its current librime/WASM bridge seam is identified. [VERIFIED: .planning/REQUIREMENTS.md] | Pin `TypeDuck-HK/TypeDuck-Web` at commit `03f9afd2cf6ca75653197f2193f24d1cd0adbd83`, then document the seam across `src/rime.ts`, `src/worker.ts`, `wasm/api.cpp`, and `scripts/build_wasm.ts`. [VERIFIED: gh api; CITED: github.com/TypeDuck-HK/TypeDuck-Web] |
| TYPEDUCK-E2E-02 | TypeDuck-Web is patched or configured so its input-engine binding calls the Yune TypeScript bridge instead of the original librime bridge. [VERIFIED: .planning/REQUIREMENTS.md] | Preserve upstream `Actions` and listener API where possible, but replace the worker-side `Module.ccall('process_key', ...)` path with `@yune-ime/typeduck-runtime` `TypeDuckRuntime`, `keyEventToRimeKey`, and filesystem helpers. [VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts; VERIFIED: packages/yune-typeduck-runtime/src/filesystem.ts] |
| TYPEDUCK-E2E-03 | Real TypeDuck-Web browser validation covers composition, candidate paging, selection, deletion, commit output, deploy, customize, and persistence smoke flows. [VERIFIED: .planning/REQUIREMENTS.md] | Use Playwright Test with `webServer`, `baseURL`, isolated `page` fixtures, locators, web-first assertions, trace/video/screenshot artifacts, and app-specific selectors around textarea, candidate table, page buttons, preferences, and reload persistence. [CITED: https://playwright.dev/docs/intro; CITED: https://playwright.dev/docs/test-webserver; VERIFIED: upstream CandidatePanel.tsx and App.tsx via WebFetch] |
| TYPEDUCK-E2E-04 | Integration findings end with a go/no-go recommendation for exposing AI-native behavior through real frontends. [VERIFIED: .planning/REQUIREMENTS.md] | Record reproducible blockers separately from implementation failures, then base the recommendation on whether real app flows work, whether adapter mismatches are bounded, and whether persistence/userdb behavior is trustworthy enough for future AI-native exposure. [VERIFIED: .planning/ROADMAP.md; ASSUMED] |
</phase_requirements>

## Summary

Phase 10 is an integration phase, not a new input engine or AI feature phase. [VERIFIED: .planning/ROADMAP.md] The upstream app seam is clear: TypeDuck-Web exposes a main-thread `Rime` API in `src/rime.ts`, queues calls through `new Worker('./worker.js')`, implements actions in `src/worker.ts`, loads Emscripten glue with `importScripts('rime.js')`, and calls librime-shaped native functions through `Module.ccall`. [VERIFIED: upstream src/rime.ts via WebFetch; VERIFIED: upstream src/worker.ts via WebFetch] The native bridge currently compiled by upstream is `wasm/api.cpp`, whose extern C exports include `init`, `set_option`, `process_key`, `select_candidate`, `delete_candidate`, `flip_page`, `customize`, and `deploy`. [VERIFIED: upstream wasm/api.cpp via WebFetch]

The Yune side has a different but now stable TypeDuck-shaped contract: 11 `yune_typeduck_*` exports, a TypeScript runtime package at `packages/yune-typeduck-runtime`, centralized response copying/freeing, browser key mapping from `KeyboardEvent.key`, and DOM-free filesystem/IDBFS helpers. [VERIFIED: scripts/typeduck-exports.txt; VERIFIED: packages/yune-typeduck-runtime/src/module.ts; VERIFIED: packages/yune-typeduck-runtime/src/response.ts; VERIFIED: packages/yune-typeduck-runtime/src/filesystem.ts] The patch should adapt contracts at the TypeDuck-Web worker/main-thread boundary rather than widening Rust to mimic upstream librime functions by default. [ASSUMED]

The main planning risk is that the local machine cannot currently build the real Yune Emscripten artifact: `wasm32-unknown-emscripten`, `emcc`, and `emar` are unavailable in the active PATH/toolchain probes, while Node, npm, Bun, git, gh, and the package-local TypeScript/Vitest suite are available. [VERIFIED: local tool probes; VERIFIED: npm package test/build] Therefore the plan should distinguish three validation levels: static seam inspection and patch construction, deterministic Yune runtime package tests, and real browser Playwright E2E that runs only after the Yune `rime.js`/WASM artifact or a documented blocker is available. [ASSUMED; VERIFIED: scripts/typeduck-wasm-build.sh]

**Primary recommendation:** Clone/pin upstream TypeDuck-Web reproducibly, preserve its `Actions`/listener API, replace the worker's librime `ccall` implementation with a Yune adapter layer using `@yune-ime/typeduck-runtime`, add Playwright E2E at the app boundary, and require every unrun browser flow to produce a reproducible blocker record plus a go/no-go recommendation. [ASSUMED; VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: .planning/ROADMAP.md]

## Project Constraints (from CLAUDE.md)

No project-level `CLAUDE.md` file exists at `CLAUDE.md`, so no CLAUDE.md directives apply. [VERIFIED: Read returned file does not exist]

Project skill directories `.claude/skills/` and `.agents/skills/` do not exist in `.`, so no project skill rules apply. [VERIFIED: Bash ls]

Project constraints from planning/source context that still bind this phase: keep Rust `lib.rs`/`main.rs` as facade/orchestration glue, treat runtime resource IDs as logical IDs rather than paths, preserve local-first frontend validation before AI-native work, and keep AI provider/ranker/privacy behavior deferred until the Phase 10 recommendation. [VERIFIED: .planning/STATE.md; VERIFIED: docs/typeduck-web-adapter.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Clone/vendor TypeDuck-Web | Build / Tooling | Browser / Client | Reproducible checkout/pinning is a build/test fixture concern before app patching starts. [VERIFIED: .planning/REQUIREMENTS.md] |
| Replace librime/WASM bridge | Browser worker / Client | Emscripten Module + Rust adapter | Upstream app already isolates engine calls in `src/worker.ts` and `src/rime.ts`; Yune should replace that seam rather than UI components first. [VERIFIED: upstream src/worker.ts via WebFetch; VERIFIED: upstream src/rime.ts via WebFetch] |
| TypeDuck-Web `Actions` compatibility | Browser / Client | TypeScript adapter layer | Upstream UI expects `Actions` methods returning `RimeResult` and listener events; the adapter layer should translate Yune `TypeDuckResponse` into that shape. [VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/response.ts] |
| Keyboard event conversion | Browser / Client | TypeScript runtime package | Browser events enter through TypeDuck-Web UI; Yune wrapper accepts event-like `key`/modifier data or numeric keycode/mask. [VERIFIED: upstream CandidatePanel.tsx via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts; CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key] |
| Virtual FS and persistence | Browser worker / Client storage | TypeScript runtime package | Upstream mounts IDBFS in the worker; Yune Phase 9 helpers expose mount/sync/preload primitives for that same host tier. [VERIFIED: upstream src/worker.ts via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/filesystem.ts; CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html] |
| Native composition/candidate behavior | Rust adapter / WASM backend | Browser adapter layer | `yune-rime-api` owns session, key processing, candidate actions, deploy/customize, and JSON state; browser code should adapt outputs, not reimplement engine logic. [VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts] |
| Browser E2E validation | Test / Browser automation | TypeDuck-Web UI + worker | Playwright should exercise user-observable UI flows through a real browser page, not only fake Module unit tests. [CITED: https://playwright.dev/docs/intro; VERIFIED: .planning/REQUIREMENTS.md] |
| AI-native go/no-go recommendation | Documentation / Product decision | Browser validation evidence | The roadmap requires a recommendation after TypeDuck-Web integration; AI-native implementation remains out of scope. [VERIFIED: .planning/ROADMAP.md] |

## Standard Stack

### Core

| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| TypeDuck-Web upstream | `TypeDuck-HK/TypeDuck-Web` main commit `03f9afd2cf6ca75653197f2193f24d1cd0adbd83`, authored 2024-11-17T02:48:01Z [VERIFIED: gh api] | Real upstream browser app under test. | Phase requirement explicitly requires upstream TypeDuck-Web clone/vendor and seam identification. [VERIFIED: .planning/REQUIREMENTS.md] |
| `@yune-ime/typeduck-runtime` | Local private package, TypeScript `^6.0.3`, Vitest `^4.1.5` [VERIFIED: packages/yune-typeduck-runtime/package.json] | Yune TS wrapper, response ownership, key mapping, FS/IDBFS helpers. | It is the Phase 8/9 handoff package intended for TypeDuck-Web callers. [VERIFIED: docs/typeduck-web-adapter.md] |
| Playwright Test | `@playwright/test` 1.59.1, npm modified 2026-05-05T06:02:31.612Z [VERIFIED: npm registry] | Real browser E2E harness for app flows. | Playwright Test provides browser automation, isolation, locators, web-first assertions, traces, and `webServer` integration. [CITED: https://playwright.dev/docs/intro; CITED: https://playwright.dev/docs/test-webserver] |
| Bun | Local 1.3.11; upstream TypeDuck-Web uses Bun scripts [VERIFIED: local tool probe; VERIFIED: upstream package.json via WebFetch] | Install/run upstream TypeDuck-Web scripts such as `bun run worker`, `bun run start`, and build helpers. | Upstream package scripts use `bun` for custom build steps and Vite/esbuild workflow. [VERIFIED: upstream package.json via WebFetch] |
| Vite | Upstream `^5.2.11`; npm latest 8.0.10 modified 2026-04-23T05:20:12.534Z [VERIFIED: upstream package.json via WebFetch; VERIFIED: npm registry] | Serve/build TypeDuck-Web. | Upstream uses Vite start/build/preview and has `base: '/web/'`. [VERIFIED: upstream package.json via WebFetch; VERIFIED: upstream vite.config.ts via WebFetch] |
| Emscripten Module runtime | Generated by Emscripten; local `emcc` unavailable [VERIFIED: local tool probe] | Provides JS glue, `cwrap`/`ccall`, `UTF8ToString`, `FS`, IDBFS, and WASM loading. | Emscripten officially documents JS calls into compiled C and runtime method/export requirements. [CITED: https://emscripten.org/docs/api_reference/preamble.js.html; CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html] |

### Supporting

| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| React | Upstream `^18.3.1`; npm latest 19.2.5 modified 2026-05-04T16:50:01.772Z [VERIFIED: upstream package.json via WebFetch; VERIFIED: npm registry] | Existing TypeDuck-Web UI. | Preserve unless upstream patch requires minimal selectors or event payload changes. [ASSUMED] |
| esbuild | Upstream `^0.21.4`; npm latest 0.28.0 modified 2026-04-02T20:38:59.211Z [VERIFIED: upstream package.json via WebFetch; VERIFIED: npm registry] | Bundles `src/worker.ts` to `public/worker.js`. | Use through upstream `bun run worker` / `bun run build` scripts. [VERIFIED: upstream package.json via WebFetch] |
| TypeScript | Yune local `^6.0.3`; upstream TypeDuck-Web `^5.4.5` [VERIFIED: local package.json; VERIFIED: upstream package.json via WebFetch] | Static typing for Yune runtime and TypeDuck-Web patch. | Use each package's existing version unless dependency resolution requires pinning. [ASSUMED] |
| GitHub CLI (`gh`) | Local 2.89.0 [VERIFIED: local tool probe] | Pin and inspect upstream repository metadata without scraping. | Use for reproducible clone metadata and source URL capture. [ASSUMED] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Patch TypeDuck-Web worker to use Yune runtime | Add Rust exports named `init`, `process_key`, etc. | Worker patch keeps Yune's TypeDuck-shaped contract stable; native compatibility shim may be useful only if worker isolation or bundling makes TS runtime import impossible. [ASSUMED; VERIFIED: scripts/typeduck-exports.txt] |
| Playwright | Cypress/WebDriver/custom Puppeteer | Playwright has official `webServer`, isolated contexts, locators, and multi-browser support; no project evidence favors another E2E stack. [CITED: https://playwright.dev/docs/intro; CITED: https://playwright.dev/docs/test-webserver; ASSUMED] |
| Vendor TypeDuck-Web into repo | Clone into an ignored/test work directory | Vendoring makes patches reviewable but can bloat history; cloned pinned worktree keeps reproducibility without committing upstream source. Planner should choose based on desired review artifact. [ASSUMED] |
| Keep upstream simulated key-sequence strings | Change UI/worker payload to event-like key data | Yune wrapper already maps `KeyboardEvent.key`/modifiers; parsing upstream `{BackSpace}` strings would be extra compatibility code. [VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts; VERIFIED: upstream CandidatePanel.tsx via WebFetch; ASSUMED] |
| Browser-only manual validation | Automated Playwright flows plus blocker artifacts | Requirement demands real browser validation; automation makes regressions reproducible and supports go/no-go evidence. [VERIFIED: .planning/REQUIREMENTS.md; ASSUMED] |

**Installation:**

```bash
# In the upstream TypeDuck-Web checkout after it is cloned/pinned
bun install
bun add -d @playwright/test@1.59.1
bunx playwright install chromium

# In Yune runtime package, only if dependencies are absent
npm --prefix packages/yune-typeduck-runtime install
```
[VERIFIED: npm registry for Playwright version; CITED: https://playwright.dev/docs/intro; ASSUMED for exact upstream package-manager command]

**Version verification:** Recommended package versions were verified with `npm view @playwright/test version time.modified --json`, `npm view playwright version time.modified --json`, `npm view vite version time.modified --json`, `npm view react version time.modified --json`, `npm view bun-types version time.modified --json`, and `npm view esbuild version time.modified --json`. [VERIFIED: npm registry]

## Architecture Patterns

### System Architecture Diagram

```text
Browser user input in TypeDuck-Web UI
  |
  v
CandidatePanel / Preferences / Toolbar
  |-- keydown/keyup, candidate click, long-press delete, page buttons
  |-- preferences changes trigger customize + deploy
  v
TypeDuck-Web main-thread Rime API (src/rime.ts)
  |-- queue one action at a time
  |-- listener events: initialized, deployStatusChanged, schemaChanged, optionChanged
  v
Worker bridge (src/worker.ts or Yune replacement worker)
  |-- wait for Emscripten Module readiness
  |-- mount/sync IDBFS or equivalent
  |-- preload explicit Yune schema assets
  |-- TypeDuckRuntime.init(Module, paths/schema)
  v
Yune TypeScript runtime package
  |-- keyEventToRimeKey(event) -> { keycode, mask }
  |-- TypeDuckRuntime.processKey/select/delete/flip/deploy/customize
  |-- read TypeDuckResponse JSON and free native response pointer
  |-- syncfs(false) after deploy/customize/userdb boundaries
  v
Emscripten-generated Yune rime.js + WASM
  |-- exports yune_typeduck_* symbols
  |-- exposes cwrap, UTF8ToString, FS/IDBFS
  v
Rust yune-rime-api TypeDuck adapter
  |-- process-global RIME service + session
  |-- schema/dictionary/userdb/deploy/customize behavior
  v
TypeDuckResponse adapter output
  |
  v
Worker translates to upstream RimeResult shape
  |
  v
TypeDuck-Web UI renders composition/candidates/commits/status
  |
  v
Playwright E2E observes UI and persistence across reload/new context
```
[VERIFIED: upstream src/rime.ts via WebFetch; VERIFIED: upstream src/worker.ts via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts; VERIFIED: packages/yune-typeduck-runtime/src/filesystem.ts; CITED: https://playwright.dev/docs/intro]

### Recommended Project Structure

```text
.planning/phases/10-typeduck-web-app-integration-and-e2e/
├── 10-RESEARCH.md                         # This research output [VERIFIED: current task]
├── typeduck-web-upstream.json              # Optional clone metadata/blocker record [ASSUMED]
└── findings.md or verification notes        # Optional final Phase 10 go/no-go evidence [ASSUMED]

external/ or target/integration/
└── TypeDuck-Web/                            # Pinned upstream checkout at 03f9afd... [ASSUMED]
    ├── src/rime.ts                          # Main-thread action queue/listeners [VERIFIED: upstream src/rime.ts]
    ├── src/worker.ts                        # Replace librime ccall implementation [VERIFIED: upstream src/worker.ts]
    ├── src/types.ts                         # Keep Actions/RimeResult compatibility [VERIFIED: upstream src/types.ts]
    ├── src/CandidatePanel.tsx               # Add stable selectors or event payload if needed [VERIFIED: upstream CandidatePanel.tsx]
    ├── src/App.tsx                          # Initialization/customize/deploy UI flows [VERIFIED: upstream App.tsx]
    ├── scripts/build_wasm.ts                # Original librime build reference, likely bypassed/replaced [VERIFIED: upstream scripts/build_wasm.ts]
    ├── public/rime.js                       # Generated Yune Emscripten JS glue location or alias [ASSUMED]
    ├── playwright.config.ts                 # New E2E config with webServer/baseURL [CITED: https://playwright.dev/docs/test-webserver]
    └── e2e/typeduck-yune.spec.ts            # New real browser scenarios [ASSUMED]

packages/yune-typeduck-runtime/
├── src/typeduck.ts                          # Existing Yune wrapper [VERIFIED: local file]
├── src/response.ts                          # Existing response parser/free helper [VERIFIED: local file]
├── src/keys.ts                              # Existing browser key mapper [VERIFIED: local file]
└── src/filesystem.ts                        # Existing FS/IDBFS helpers [VERIFIED: local file]
```

### Pattern 1: Preserve upstream `Actions`, replace worker internals

**What:** Keep TypeDuck-Web's public `Rime` object and listener events stable while swapping the worker's implementation from librime `Module.ccall` calls to Yune `TypeDuckRuntime` calls. [VERIFIED: upstream src/rime.ts via WebFetch; VERIFIED: upstream src/types.ts via WebFetch]

**When to use:** Use when the UI contract is mostly adequate and the engine bridge is the only intended replacement. [ASSUMED]

**Example:**

```typescript
// Source: upstream src/types.ts and Yune TypeDuckRuntime [VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts]
const actions: Actions = {
  async processKey(input) {
    const event = parseOrCarryEvent(input); // Planner should prefer event payloads over string parsing. [ASSUMED]
    const response = runtime.processKeyboardEvent(event);
    await syncAfterCommitBoundaryIfNeeded(response);
    return toRimeResult(response);
  },
  async selectCandidate(index) {
    return toRimeResult(runtime.selectCandidate(index));
  },
  async deleteCandidate(index) {
    return toRimeResult(runtime.deleteCandidate(index));
  },
  async flipPage(backward) {
    return toRimeResult(runtime.flipPage(backward));
  },
  async customize(preferences) {
    return customizePreferencesWithYune(runtime, Module.FS, preferences);
  },
  async deploy() {
    return deployAndSync(runtime, Module.FS);
  },
};
```

### Pattern 2: Translate `TypeDuckResponse` to upstream `RimeResult`

**What:** Convert Yune's `{ handled, commits, context, status, error }` response into TypeDuck-Web's `{ success, committed?, isComposing, inputBuffer?, page?, isLastPage?, highlightedIndex?, candidates? }` shape. [VERIFIED: packages/yune-typeduck-runtime/src/response.ts; VERIFIED: upstream src/types.ts via WebFetch]

**When to use:** Use before returning any worker result to existing UI code. [VERIFIED: upstream CandidatePanel.tsx via WebFetch]

**Example:**

```typescript
// Source: Yune response.ts and upstream TypeDuck-Web types.ts [VERIFIED: local response.ts; VERIFIED: upstream src/types.ts via WebFetch]
function toRimeResult(response: TypeDuckResponse): RimeResult {
  const committed = response.commits.length > 0 ? response.commits.join("") : undefined;
  const base = { success: response.error === undefined, ...(committed ? { committed } : {}) };
  if (response.context === null || response.status?.is_composing !== true) {
    return { ...base, isComposing: false };
  }
  return {
    ...base,
    isComposing: true,
    inputBuffer: { before: "", active: response.context.preedit, after: "" },
    page: response.context.page_no,
    isLastPage: response.context.is_last_page,
    highlightedIndex: response.context.highlighted,
    candidates: response.context.candidates.map((candidate, index) => ({
      label: response.context?.select_labels[index] ?? undefined,
      text: candidate.text,
      comment: candidate.comment || undefined,
    })),
  };
}
```

### Pattern 3: Prefer event-like payloads over deprecated keycode or RIME sequence parsing

**What:** Upstream TypeDuck-Web currently builds strings like `{BackSpace}` or printable key sequences for `simulate_key_sequence`, while Yune's wrapper maps `KeyboardEvent.key` and modifiers to integer keycode/mask. [VERIFIED: upstream CandidatePanel.tsx via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts]

**When to use:** Use when patching `CandidatePanel.tsx` and `src/types.ts` is acceptable; otherwise write a small compatibility parser for the upstream string format and treat it as a temporary adapter. [ASSUMED]

**Example:**

```typescript
// Source: MDN KeyboardEvent.key and Yune TypeDuckRuntime.processKeyboardEvent [CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key; VERIFIED: local typeduck.ts]
await Rime.processKeyboardEvent?.({
  key: event.key,
  shiftKey: event.shiftKey,
  ctrlKey: event.ctrlKey,
  altKey: event.altKey,
  metaKey: event.metaKey,
  type: event.type,
});
```

### Pattern 4: Playwright config owns dev server startup and `/web/` navigation

**What:** TypeDuck-Web uses Vite with `base: '/web/'`, so E2E should navigate to `/web/` and let Playwright start/reuse the Bun/Vite server. [VERIFIED: upstream vite.config.ts via WebFetch; CITED: https://playwright.dev/docs/test-webserver]

**When to use:** Use for all browser E2E tests. [ASSUMED]

**Example:**

```typescript
// Source: Playwright webServer docs and TypeDuck-Web vite.config.ts [CITED: https://playwright.dev/docs/test-webserver; VERIFIED: upstream vite.config.ts via WebFetch]
import { defineConfig } from "@playwright/test";

export default defineConfig({
  webServer: {
    command: "bun run start --host 127.0.0.1 --port 5173",
    url: "http://127.0.0.1:5173/web/",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
  use: {
    baseURL: "http://127.0.0.1:5173",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
});
```

### Anti-Patterns to Avoid

- **Rewriting the TypeDuck-Web UI first:** The observed seam is already `src/rime.ts`/`src/worker.ts`; start there and modify UI only for payload shape or stable selectors. [VERIFIED: upstream src/rime.ts via WebFetch; VERIFIED: upstream src/worker.ts via WebFetch; ASSUMED]
- **Bypassing `readTypeDuckResponse`:** The Yune wrapper frees every non-null response pointer in a centralized `finally`; direct worker `cwrap` calls risk leaks or double-free mistakes. [VERIFIED: packages/yune-typeduck-runtime/src/response.ts]
- **Using deprecated `KeyboardEvent.keyCode`:** MDN marks `keyCode` deprecated and implementation-dependent; use `event.key` or Yune's existing event-like mapper. [CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode; CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key]
- **Treating missing Emscripten as test success:** The existing build script reports `TypeDuck WASM build blocked:` and runs native fallback; Phase 10 should record the blocker instead of pretending browser E2E passed. [VERIFIED: scripts/typeduck-wasm-build.sh]
- **Adding AI behavior during frontend plumbing:** The roadmap defers AI provider/ranking/context/privacy work until after the Phase 10 recommendation. [VERIFIED: .planning/ROADMAP.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Browser automation | Custom Puppeteer scripts or shell screenshots | Playwright Test | Playwright Test officially provides test runner, browser support, isolation, locators, web-first assertions, and `webServer`. [CITED: https://playwright.dev/docs/intro; CITED: https://playwright.dev/docs/test-webserver] |
| Yune C/WASM bindings | Per-action raw `Module.cwrap` calls in app code | `@yune-ime/typeduck-runtime` | The local package centralizes binding, response parsing/freeing, lifecycle guard, key mapping, and FS helpers. [VERIFIED: packages/yune-typeduck-runtime/src/module.ts; VERIFIED: packages/yune-typeduck-runtime/src/response.ts; VERIFIED: packages/yune-typeduck-runtime/src/filesystem.ts] |
| Browser persistence backend | Custom IndexedDB serializer for RIME files | Emscripten IDBFS or equivalent with `FS.syncfs` | Emscripten documents IDBFS persistence and `syncfs(true/false)` directions. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html] |
| Keyboard translation | Deprecated `keyCode` table or large string-sequence parser | `KeyboardEvent.key` via Yune `keyEventToRimeKey` | `event.key` reflects modifiers/layout/locale and Yune already has a tested mapper. [CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts] |
| Upstream repo discovery | Ad hoc zip downloads without provenance | `git clone` plus pinned commit metadata | The requirement calls for reproducible clone/vendor; commit pinning makes source state auditable. [VERIFIED: .planning/REQUIREMENTS.md; ASSUMED] |
| Blocker tracking | Free-text vague notes | Structured blocker records with command, environment, expected, actual, logs, and next action | Phase 10 success criteria require reproducible blocker recording. [VERIFIED: .planning/ROADMAP.md; ASSUMED] |

**Key insight:** The hardest part is contract translation at a worker boundary: upstream TypeDuck-Web expects a librime simulator-style `Actions` API and `RimeResult`, while Yune exposes a safer TypeDuck-specific runtime with explicit response ownership, keycode/mask mapping, and host-owned FS sync. [VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts; VERIFIED: packages/yune-typeduck-runtime/src/response.ts]

## Common Pitfalls

### Pitfall 1: Contract mismatch hidden behind same action names

**What goes wrong:** `processKey`, `customize`, `deploy`, and candidate actions appear similar but return/accept different shapes between upstream librime and Yune. [VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: packages/yune-typeduck-runtime/src/typeduck.ts]
**Why it happens:** Upstream `processKey(input: string)` forwards simulated RIME key sequences to `simulate_key_sequence`, while Yune expects numeric keycode/mask or a browser event-like object. [VERIFIED: upstream wasm/api.cpp via WebFetch; VERIFIED: local typeduck.ts]
**How to avoid:** Add a single adapter module that translates `TypeDuckResponse` to `RimeResult` and chooses one key input strategy before touching UI behavior broadly. [ASSUMED]
**Warning signs:** Worker returns Yune `TypeDuckResponse` directly to `CandidatePanel.tsx`, or UI checks `response.context` instead of `result.isComposing`. [ASSUMED]

### Pitfall 2: Missing `setOption` parity

**What goes wrong:** Upstream app calls `Rime.setOption(option, value)`, but the current Yune TypeDuck wrapper exports no `setOption` method. [VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: local typeduck.ts; VERIFIED: local module.ts]
**Why it happens:** Phase 8 locked operations around init/process/candidate/deploy/customize/cleanup, not the full upstream librime option API. [VERIFIED: .planning/phases/08-typescript-bridge-and-runtime-package/08-RESEARCH.md]
**How to avoid:** Determine whether TypeDuck-Web UI requires `setOption` for Phase 10 test flows; if required, either map supported options through `customize`/status or record a focused adapter-gap blocker before adding native exports. [ASSUMED]
**Warning signs:** Preferences or toolbar flows silently no-op options but E2E still claims full customize/deploy coverage. [ASSUMED]

### Pitfall 3: Worker bundling cannot import the local runtime package

**What goes wrong:** Upstream `esbuild src/worker.ts --outdir=public` may not resolve the local Yune package or generated Emscripten glue path without alias/link/package configuration. [VERIFIED: upstream package.json via WebFetch; ASSUMED]
**Why it happens:** TypeDuck-Web is a separate upstream app with its own Bun/esbuild/Vite dependency graph, and `@yune-ime/typeduck-runtime` is a private local package. [VERIFIED: local package.json; VERIFIED: upstream package.json via WebFetch]
**How to avoid:** Plan a deterministic integration path: workspace link, packed tarball, relative file dependency, or copied `dist` package, and record which one was used. [ASSUMED]
**Warning signs:** Patch works only through developer-local absolute imports. [ASSUMED]

### Pitfall 4: Generated `rime.js` export/runtime mismatch

**What goes wrong:** Browser code cannot call `cwrap`, `UTF8ToString`, `FS`, or the `yune_typeduck_*` symbols because Emscripten optimized them away or did not export runtime methods. [CITED: https://emscripten.org/docs/api_reference/preamble.js.html; VERIFIED: scripts/typeduck-wasm-build.sh]
**Why it happens:** Emscripten requires JS-callable C functions in `EXPORTED_FUNCTIONS` and external runtime helpers in `EXPORTED_RUNTIME_METHODS`. [CITED: https://emscripten.org/docs/api_reference/preamble.js.html]
**How to avoid:** Use `scripts/typeduck-exports.txt` and `scripts/typeduck-wasm-build.sh`; ensure Phase 10 includes `FS`/IDBFS availability if the generated artifact must support browser persistence. [VERIFIED: local scripts; CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html; ASSUMED]
**Warning signs:** Worker code assumes `Module.FS` exists but Phase 7 flags only include `ccall,cwrap,UTF8ToString`; planner should verify whether FS export/link flags need update for real TypeDuck-Web use. [VERIFIED: local typeduck-wasm-build.sh; ASSUMED]

### Pitfall 5: `FS.syncfs` direction reversal or missing persistence smoke

**What goes wrong:** Browser reload loses user data or stale config overwrites fresh runtime files. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html; VERIFIED: local filesystem.ts]
**Why it happens:** `FS.syncfs(true)` populates in-memory FS from persistence, while `FS.syncfs(false)` saves in-memory FS to persistence. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html]
**How to avoid:** Use `syncFromPersistenceBeforeInit`, `deployAndSync`, `customizeAndSync`, and `syncAfterUserDataChange` from the Yune package; E2E should include reload/new page persistence smoke if the artifact runs. [VERIFIED: local filesystem.ts; ASSUMED]
**Warning signs:** E2E covers composition in one page but never reloads or checks that deploy/customize/userdb changes survive. [ASSUMED]

### Pitfall 6: Brittle E2E selectors

**What goes wrong:** Tests fail from cosmetic CSS/layout changes rather than behavior regressions. [ASSUMED]
**Why it happens:** Upstream UI classes like `.flex`, `.join`, `.bg-accent`, and `.page-nav` are styling-oriented. [VERIFIED: upstream CandidatePanel.tsx via WebFetch]
**How to avoid:** Prefer accessible roles/labels; add `data-testid` only where semantics are unavailable for textarea, candidate rows, page buttons, deploy status, and preferences. [CITED: https://playwright.dev/docs/intro; ASSUMED]
**Warning signs:** Tests locate the nth `.join-item` without naming previous/next semantics or stable test IDs. [ASSUMED]

### Pitfall 7: Claiming real browser success when local WASM tooling is blocked

**What goes wrong:** Phase 10 reports app integration complete based only on Node/Vitest or native Rust fallback. [ASSUMED]
**Why it happens:** The Phase 7 fallback is useful but not a real browser/WebAssembly app flow. [VERIFIED: scripts/typeduck-wasm-build.sh; VERIFIED: .planning/REQUIREMENTS.md]
**How to avoid:** Separate results as: browser E2E passed, browser E2E blocked by named tool/artifact issue, or native/runtime fallback passed. [ASSUMED]
**Warning signs:** Final recommendation says GO without listing whether TypeDuck-Web was actually exercised in Playwright. [ASSUMED]

## Code Examples

Verified patterns from official and project sources.

### Low-level Yune wrapper flow inside a worker

```typescript
// Source: docs/typeduck-web-adapter.md and local runtime package [VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: local typeduck.ts; VERIFIED: local filesystem.ts]
await syncFromPersistenceBeforeInit(Module.FS);
prepareTypeDuckFilesystem(Module.FS, fsOptions);
assertTypeDuckAssetsReady(Module.FS, fsOptions);

const runtime = TypeDuckRuntime.init(Module, {
  sharedDataDir: fsOptions.sharedDataDir,
  userDataDir: fsOptions.userDataDir,
  schemaId: fsOptions.schemaId,
});

const response = runtime.processKeyboardEvent({
  key: event.key,
  shiftKey: event.shiftKey,
  ctrlKey: event.ctrlKey,
  altKey: event.altKey,
  metaKey: event.metaKey,
  type: event.type,
});
```

### Upstream worker action names to preserve

```typescript
// Source: upstream TypeDuck-Web src/types.ts [VERIFIED: upstream src/types.ts via WebFetch]
interface Actions {
  setOption(option: string, value: boolean): Promise<void>;
  processKey(input: string): Promise<RimeResult>;
  selectCandidate(index: number): Promise<RimeResult>;
  deleteCandidate(index: number): Promise<RimeResult>;
  flipPage(backward: boolean): Promise<RimeResult>;
  customize(preferences: RimePreferences): Promise<boolean>;
  deploy(): Promise<boolean>;
}
```

### Playwright smoke scenario skeleton

```typescript
// Source: Playwright docs for fixtures/locators/expect and TypeDuck-Web UI observations [CITED: https://playwright.dev/docs/intro; VERIFIED: upstream CandidatePanel.tsx via WebFetch]
import { expect, test } from "@playwright/test";

test("Yune TypeDuck composes, pages, selects, commits, and survives reload", async ({ page }) => {
  await page.goto("/web/");
  const input = page.getByRole("textbox");
  await expect(input).toBeEnabled();

  await input.pressSequentially("ba");
  await expect(page.locator(".candidate-panel")).toBeVisible();
  await expect(page.locator("table.candidates")).toBeVisible();

  await page.locator(".page-nav").last().click();
  await page.locator("table.candidates tr").first().click();
  await expect(input).not.toHaveValue("");

  await page.reload();
  await expect(input).toBeEnabled();
});
```

### Structured blocker record shape

```markdown
<!-- Source: Phase 10 roadmap blocker requirement [VERIFIED: .planning/ROADMAP.md] -->
### Blocker: missing Emscripten linker
- Command: `./scripts/typeduck-wasm-build.sh`
- Expected: Yune `rime.js`/`.wasm` artifact with `yune_typeduck_*` exports
- Actual: `TypeDuck WASM build blocked: missing Emscripten linker emcc on PATH.`
- Environment: macOS Darwin 25.3.0, Rust available via `/Users/trenton/.cargo/bin`, `wasm32-unknown-emscripten` missing
- Fallback run: `cargo test -p yune-rime-api --test typeduck_web`
- Next action: install/activate Emscripten SDK and Rust target, then rerun browser E2E
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Upstream TypeDuck-Web calls librime C++ bridge functions like `process_key` through `Module.ccall`. | Yune should call `yune_typeduck_*` through `@yune-ime/typeduck-runtime` and translate output to upstream `RimeResult`. | Yune Phases 7-9 created the export contract, TypeScript wrapper, and filesystem helpers before Phase 10. [VERIFIED: local phase 08/09 research; VERIFIED: local scripts/package] | Planner should adapt at the worker layer rather than clone librime's C++ API in Rust by default. [ASSUMED] |
| Upstream builds `public/rime.js` from `wasm/api.cpp` and preloads `schema@/usr/share/rime-data`. | Yune build script verifies `yune_typeduck_*` exports for `wasm32-unknown-emscripten`; browser FS assets are explicit caller-owned preloads. | Phase 7/9 handoff. [VERIFIED: local typeduck-wasm-build.sh; VERIFIED: local filesystem.ts; VERIFIED: docs/typeduck-web-adapter.md] | Phase 10 must decide where Yune `rime.js`/WASM and assets live inside TypeDuck-Web. [ASSUMED] |
| Upstream keyboard bridge sends simulated key-sequence strings. | Yune wrapper uses `KeyboardEvent.key` semantics and explicit keycode/mask mapping. | Phase 8 handoff. [VERIFIED: local typeduck.ts; CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key] | Planner should prefer event payload patch or a bounded compatibility parser. [ASSUMED] |
| Manual browser validation notes | Playwright E2E with artifacts and reproducible blockers | Phase 10 requirement. [VERIFIED: local requirements/roadmap; CITED: https://playwright.dev/docs/intro] | Planner should add automated browser coverage or explicit blocked test records. [ASSUMED] |

**Deprecated/outdated:**
- `KeyboardEvent.keyCode` for new key mapping is deprecated and unreliable. [CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode]
- Treating TypeDuck-Web integration as docs-only is outdated because Phase 10 requires a real app patch/configuration and browser validation or reproducible blockers. [VERIFIED: .planning/REQUIREMENTS.md]
- Treating native fallback tests as equivalent to browser app E2E is insufficient for `TYPEDUCK-E2E-03`. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: local typeduck-wasm-build.sh]

## Concrete Files Likely To Create Or Modify

| Path | Action | Purpose |
|------|--------|---------|
| `.planning/phases/10-typeduck-web-app-integration-and-e2e/10-RESEARCH.md` | Create | Phase 10 planning research. [VERIFIED: current task] |
| `.planning/phases/10-typeduck-web-app-integration-and-e2e/typeduck-web-upstream.json` | Create optional | Record repo URL, commit SHA, clone path, patch path, and source URLs for reproducibility. [ASSUMED] |
| `scripts/typeduck-wasm-build.sh` | Modify only if browser app needs `FS` runtime export or artifact path change | Current script exports `ccall,cwrap,UTF8ToString`; TypeDuck-Web worker also needs `FS`/IDBFS for persistence. [VERIFIED: local script; VERIFIED: upstream src/worker.ts via WebFetch; ASSUMED] |
| `scripts/typeduck-exports.txt` | Avoid unless native API gap is proven | Canonical export list already matches TypeScript wrapper. [VERIFIED: local file] |
| `packages/yune-typeduck-runtime/src/typeduck.ts` | Modify only if upstream gap requires wrapper expansion | Current wrapper lacks `setOption` and notification-style events. [VERIFIED: local file; VERIFIED: upstream src/types.ts via WebFetch] |
| `packages/yune-typeduck-runtime/src/filesystem.ts` | Reuse; modify only for TypeDuck-Web-specific integration blocker | Existing helpers cover mount, sync, asset preload, deploy/customize sync. [VERIFIED: local file] |
| `TypeDuck-Web/src/worker.ts` in pinned checkout | Modify | Primary replacement seam for Yune runtime, Module init, FS setup, response translation, and persistence sync. [VERIFIED: upstream src/worker.ts via WebFetch] |
| `TypeDuck-Web/src/rime.ts` in pinned checkout | Modify only if message payloads/listeners need change | Main-thread queue and listener API should be preserved where possible. [VERIFIED: upstream src/rime.ts via WebFetch] |
| `TypeDuck-Web/src/types.ts` in pinned checkout | Modify if adding event-like `processKeyboardEvent` or Yune-specific adapter types | Upstream `processKey(input: string)` does not match Yune event/keycode flow. [VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: local typeduck.ts] |
| `TypeDuck-Web/src/CandidatePanel.tsx` in pinned checkout | Modify if event-like payloads or stable test selectors are needed | UI owns keydown/keyup, candidate click/delete, page buttons. [VERIFIED: upstream CandidatePanel.tsx via WebFetch] |
| `TypeDuck-Web/src/App.tsx`, `Toolbar.tsx`, `Preferences.tsx` in pinned checkout | Modify only for stable selectors or customize/deploy status adaptation | E2E must cover initialization, preferences, customize, and deploy flows. [VERIFIED: upstream App.tsx via WebFetch] |
| `TypeDuck-Web/playwright.config.ts` and `TypeDuck-Web/e2e/*.spec.ts` | Create | Real browser E2E harness and scenarios. [CITED: https://playwright.dev/docs/test-webserver; ASSUMED] |
| `TypeDuck-Web/package.json` | Modify | Add Playwright scripts/dependency and possibly local `@yune-ime/typeduck-runtime` dependency/link. [VERIFIED: upstream package.json via WebFetch; ASSUMED] |

## Upstream TypeDuck-Web Seam Findings

| File | Observed Fact | Planning Implication |
|------|---------------|----------------------|
| `src/rime.ts` | Creates `new Worker('./worker.js')`, dynamically registers `setOption`, `processKey`, `selectCandidate`, `deleteCandidate`, `flipPage`, `customize`, and `deploy`, queues one action at a time, and routes listener messages. [VERIFIED: upstream src/rime.ts via WebFetch] | Preserve the `Rime` facade and queue if possible; patch worker implementation first. [ASSUMED] |
| `src/worker.ts` | Defines `globalThis.Module`, loads `rime.js` with `importScripts`, waits for `onRuntimeInitialized`, mounts `IDBFS` at `/rime`, syncs read/write, calls `Module.ccall`, and dispatches notifications. [VERIFIED: upstream src/worker.ts via WebFetch] | Main Yune replacement seam; must load Yune Emscripten glue and initialize `TypeDuckRuntime` after FS prep. [ASSUMED] |
| `src/types.ts` | `Actions` uses `processKey(input: string)` and returns `RimeResult`, while listeners include `initialized`, `deployStatusChanged`, `schemaChanged`, and `optionChanged`. [VERIFIED: upstream src/types.ts via WebFetch] | Add response and listener translation; identify missing `setOption`/notification parity. [ASSUMED] |
| `wasm/api.cpp` | Exports librime-shaped functions and forwards `process_key` strings to `simulate_key_sequence`. [VERIFIED: upstream wasm/api.cpp via WebFetch] | Do not assume Yune can accept upstream key-sequence strings without adapter work. [ASSUMED] |
| `scripts/build_wasm.ts` | Exports `_init`, `_set_option`, `_process_key`, `_select_candidate`, `_delete_candidate`, `_flip_page`, `_customize`, `_deploy`; runtime methods are `ccall` and `FS`; output is `public/rime.js`; schema preloaded to `/usr/share/rime-data`. [VERIFIED: upstream scripts/build_wasm.ts via WebFetch] | Yune artifact needs different exports and likely different asset/preload strategy. [ASSUMED] |
| `vite.config.ts` | Uses `base: '/web/'`, React SWC plugin, and build target `es2017`. [VERIFIED: upstream vite.config.ts via WebFetch] | Playwright should navigate to `/web/`. [ASSUMED] |
| `CandidatePanel.tsx` | Handles document `keydown`/`keyup`, calls `Rime.processKey`, `Rime.flipPage`, `Rime.selectCandidate`, and `Rime.deleteCandidate`, and renders `.candidate-panel`, `.page-nav`, and `table.candidates`. [VERIFIED: upstream CandidatePanel.tsx via WebFetch] | E2E can target composition/candidate flows here; add stable selectors if CSS selectors are too brittle. [ASSUMED] |
| `App.tsx` | Subscribes to `initialized` and `deployStatusChanged`, calls `Rime.customize(...)` and `Rime.deploy()` when preferences change, and renders textarea, toolbar, candidate panel, preferences, and notifications. [VERIFIED: upstream App.tsx via WebFetch] | E2E customize/deploy flows must account for automatic preference-triggered deployment. [ASSUMED] |

## Go / No-Go Recommendation Criteria

| Outcome | Criteria | Recommendation |
|---------|----------|----------------|
| GO | TypeDuck-Web runs in Playwright with Yune artifact; composition, paging, selection, deletion, commit, deploy, customize, and persistence smoke pass; blockers are only minor UI selector/build polish. [ASSUMED] | Start AI-native frontend exposure planning with TypeDuck-Web as a validated frontend host. [ASSUMED] |
| GO WITH CONDITIONS | Core composition/candidate/commit flows pass, but deploy/customize or persistence has bounded documented gaps that do not corrupt input behavior. [ASSUMED] | Start AI-native design only behind mock/local providers and keep frontend exposure disabled until gaps close. [ASSUMED] |
| NO-GO | Yune cannot be loaded in TypeDuck-Web worker, key/response contract mismatch requires native API expansion, or browser persistence/userdb behavior cannot be validated/recovered reproducibly. [ASSUMED] | Do not expose AI-native behavior through real frontends; plan a follow-up adapter/runtime stabilization phase. [ASSUMED] |
| BLOCKED | Missing `emcc`, `emar`, `wasm32-unknown-emscripten`, upstream source access, or browser artifact prevents real E2E from running. [VERIFIED: local tool probe for current Emscripten/Rust target blocker] | Record blocker with exact command/output and native/runtime fallback results; recommendation should be provisional. [ASSUMED] |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The best patch path is to preserve upstream `Actions`/listener API and replace worker internals first. | Summary / Architecture Patterns | If worker bundling or Module global constraints block this, planner may need a compatibility shim or main-thread adapter approach. |
| A2 | Exact TypeDuck-Web checkout location should be `external/` or `target/integration/` rather than committed vendored source. | Recommended Project Structure | If reviewers need patch diffs in git, vendoring or patch-file storage may be preferable. |
| A3 | Playwright is the right E2E stack for Phase 10. | Standard Stack / Don't Hand-Roll | If project maintainers prefer another browser test runner, package scripts and artifacts would change. |
| A4 | Event-like key payloads are preferable to parsing upstream simulated key-sequence strings. | Architecture Patterns / Pitfalls | If UI changes are undesirable, a bounded string parser may be less invasive. |
| A5 | `setOption` can be mapped, no-oped for noncritical paths, or recorded as a focused blocker rather than immediately expanding native exports. | Common Pitfalls | If toolbar/preferences depend on option state for required flows, native/wrapper expansion may be needed. |
| A6 | The Yune Emscripten artifact may need `FS` exported/available in addition to current `ccall,cwrap,UTF8ToString` runtime methods for TypeDuck-Web worker persistence. | Concrete Files / Pitfalls | If Emscripten exposes `FS` without adding it to `EXPORTED_RUNTIME_METHODS` in this build mode, script changes may be unnecessary; if not, browser runtime will fail. |
| A7 | Stable `data-testid` additions are acceptable in the upstream patch if accessible locators are insufficient. | Common Pitfalls / Browser Strategy | If the patch should avoid UI source changes, tests may need more brittle existing selectors. |
| A8 | The final go/no-go rubric should classify GO, GO WITH CONDITIONS, NO-GO, and BLOCKED. | Go / No-Go Recommendation Criteria | If the project expects only binary GO/NO-GO, the planner can collapse GO WITH CONDITIONS into conditions attached to GO. |

## Open Questions

1. **Where should the upstream TypeDuck-Web checkout live?**
   - What we know: Phase 10 requires a cloned or vendored reproducible test location. [VERIFIED: .planning/REQUIREMENTS.md]
   - What's unclear: Phase 10 CONTEXT.md requires a reproducible local integration location but leaves clone-vs-vendor-vs-scripted checkout to planning. [VERIFIED: `.planning/phases/10-typeduck-web-app-integration-and-e2e/10-CONTEXT.md`]
   - Recommendation: Use a pinned clone plus a small committed metadata/patch record unless the planner needs committed upstream source for review. [ASSUMED]

2. **Should TypeDuck-Web receive event-like keyboard payloads or keep `processKey(input: string)`?**
   - What we know: Upstream sends simulated strings; Yune wrapper maps event-like `key`/modifiers. [VERIFIED: upstream CandidatePanel.tsx via WebFetch; VERIFIED: local typeduck.ts]
   - What's unclear: The least invasive patch depends on whether changing `src/types.ts` and `CandidatePanel.tsx` is acceptable. [ASSUMED]
   - Recommendation: Prefer event-like payloads for correctness; keep a compatibility parser only if UI contract preservation is prioritized. [ASSUMED]

3. **Does Phase 10 require adding `setOption` to Yune native/TS adapter?**
   - What we know: Upstream `Actions` includes `setOption`, but local Yune wrapper does not. [VERIFIED: upstream src/types.ts via WebFetch; VERIFIED: local typeduck.ts]
   - What's unclear: Required E2E flows may not need toolbar option toggles beyond customize/deploy preferences. [ASSUMED]
   - Recommendation: Start with documented unsupported-option behavior; add native/wrapper support only if a required UI flow fails and cannot be mapped to `customize`. [ASSUMED]

4. **Can the real Yune Emscripten artifact be built in this environment?**
   - What we know: Local probes show no active `rustup` in PATH, home rustup target `wasm32-unknown-emscripten` missing, and `emcc`/`emar` unavailable. [VERIFIED: local tool probes]
   - What's unclear: The user may have Emscripten available outside the agent PATH or may accept installing it before execution. [ASSUMED]
   - Recommendation: Plan blocker-first validation: run `scripts/typeduck-wasm-build.sh`, capture output, and proceed to Playwright only if artifact exists. [VERIFIED: local script; ASSUMED]

5. **What exact schema/assets should TypeDuck-Web E2E preload?**
   - What we know: Yune helper requires `default.yaml`, `<schema>.schema.yaml`, `<dict>.dict.yaml`, build copies, and logical `schemaId`/`dictionaryId`. [VERIFIED: local filesystem.ts; VERIFIED: docs/typeduck-web-adapter.md]
   - What's unclear: Phase 10 context requires explicit TypeDuck-Web-owned assets but leaves the exact browser E2E fixture schema bundle to planning. [VERIFIED: `.planning/phases/10-typeduck-web-app-integration-and-e2e/10-CONTEXT.md`]
   - Recommendation: Use the smallest existing Yune TypeDuck fixture schema that native `typeduck_web` tests already exercise, then record any app-specific asset gap. [ASSUMED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | Local TS runtime verification and possible Playwright tooling | yes [VERIFIED: local tool probe] | v24.14.1 [VERIFIED: local tool probe] | — |
| npm | Local package install/test/build and registry verification | yes [VERIFIED: local tool probe] | 11.11.0 [VERIFIED: local tool probe] | — |
| Bun | Upstream TypeDuck-Web scripts | yes [VERIFIED: local tool probe] | 1.3.11 [VERIFIED: local tool probe] | Use npm only for Yune package; upstream scripts still expect Bun. [VERIFIED: upstream package.json via WebFetch] |
| git | Clone/pin upstream TypeDuck-Web | yes [VERIFIED: local tool probe] | Apple Git 2.50.1 [VERIFIED: local tool probe] | GitHub archive download with checksum, but clone is preferred. [ASSUMED] |
| gh | Verify upstream metadata/source | yes [VERIFIED: local tool probe] | 2.89.0 [VERIFIED: local tool probe] | `git ls-remote` and raw GitHub URLs. [ASSUMED] |
| TypeDuck runtime package deps | Yune wrapper tests/build | yes after npm install [VERIFIED: npm test/build passed] | TypeScript 6.0.3, Vitest 4.1.5 from local package resolution [VERIFIED: npm test/build output; VERIFIED: package.json] | — |
| Playwright browsers | Real browser E2E | unknown/not probed [ASSUMED] | `@playwright/test` latest 1.59.1 [VERIFIED: npm registry] | Install with Playwright browser install command before E2E. [CITED: https://playwright.dev/docs/intro] |
| Cargo | Native fallback tests / Yune build | yes via `/Users/trenton/.cargo/bin/cargo` [VERIFIED: local tool probe] | 1.95.0 [VERIFIED: local tool probe] | Use explicit `/Users/trenton/.cargo/bin/cargo` if PATH lacks Cargo. [VERIFIED: local tool probe] |
| rustup | Install/check Emscripten Rust target | not in active PATH; exists at `/Users/trenton/.cargo/bin/rustup` [VERIFIED: local tool probe] | 1.29.0 [VERIFIED: local tool probe] | Use explicit path or add `/Users/trenton/.cargo/bin` to PATH. [ASSUMED] |
| `wasm32-unknown-emscripten` Rust target | Real Yune WASM build | no [VERIFIED: local tool probe] | — | Native fallback `cargo test -p yune-rime-api --test typeduck_web`; install target for browser E2E. [VERIFIED: local script] |
| Emscripten `emcc` | Real Yune WASM build | no [VERIFIED: local tool probe] | — | Record blocker and run native fallback. [VERIFIED: local script] |
| Emscripten `emar` | Real Yune WASM build | no [VERIFIED: local tool probe] | — | Record blocker and run native fallback. [VERIFIED: local script] |

**Missing dependencies with no fallback:**
- Real browser app validation is blocked until a Yune Emscripten `rime.js`/WASM artifact exists; native fallback validates adapter behavior but does not satisfy real TypeDuck-Web browser E2E by itself. [VERIFIED: local tool probes; VERIFIED: .planning/REQUIREMENTS.md; ASSUMED]

**Missing dependencies with fallback:**
- `wasm32-unknown-emscripten`, `emcc`, and `emar` are missing; fallback is `scripts/typeduck-wasm-build.sh` blocker output plus native adapter tests. [VERIFIED: local tool probes; VERIFIED: local script]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | TypeDuck-Web/Yune integration has no authentication flow in Phase 10. [VERIFIED: roadmap/requirements scope] |
| V3 Session Management | yes | Treat Yune `TypeDuckRuntime` as one active process-global native runtime per Module instance and cleanup deterministically. [VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: local typeduck.ts] |
| V4 Access Control | no | Phase 10 does not introduce multi-user authorization boundaries. [VERIFIED: roadmap/requirements scope] |
| V5 Input Validation | yes | Validate schema/dictionary logical IDs, response envelopes, keyboard input mapping, and unsupported action behavior. [VERIFIED: local filesystem.ts; VERIFIED: local response.ts; CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key] |
| V6 Cryptography | no | Phase 10 does not introduce cryptographic operations, secrets, or tokens. [VERIFIED: roadmap/requirements scope] |
| V8 Data Protection | yes | Keep TypeDuck-Web validation local-first; do not add AI, remote calls, or cloud inference during Phase 10. [VERIFIED: roadmap deferred AI scope; VERIFIED: docs/typeduck-web-adapter.md] |
| V12 File and Resources | yes | Use Emscripten FS/IDBFS helpers, reject path-like resource IDs, and make persistence sync failures visible. [VERIFIED: local filesystem.ts; CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html] |

### Known Threat Patterns for TypeDuck-Web + Emscripten Integration

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via schema/dictionary IDs | Tampering | Use `isTypeDuckLogicalId` and caller-provided virtual paths; reject path-like IDs before joining. [VERIFIED: local filesystem.ts] |
| Use-after-free or double-free of response pointers | Denial of Service / Tampering | Use `readTypeDuckResponse` through `TypeDuckRuntime`; never expose raw response pointers to TypeDuck-Web UI. [VERIFIED: local response.ts] |
| Silent stale or lost browser persistence | Tampering / Repudiation | Wrap `FS.syncfs(true/false)` with named helpers and surface callback errors. [VERIFIED: local filesystem.ts; CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html] |
| Unsupported key mapped to wrong command | Tampering | Use `KeyboardEvent.key` and tested Yune key mapper; avoid deprecated `keyCode`. [CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key; CITED: https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode] |
| Accidental remote/AI data flow during frontend test | Information Disclosure | Keep Phase 10 scoped to classic frontend plumbing and final recommendation; do not add providers/rankers/context upload. [VERIFIED: roadmap deferred AI scope] |
| Worker/global Module confusion | Denial of Service | Initialize one Emscripten Module/runtime per worker and guard cleanup/idempotence through `TypeDuckRuntime`. [VERIFIED: upstream src/worker.ts via WebFetch; VERIFIED: local typeduck.ts] |

## Sources

### Primary (HIGH confidence)

- `.planning/REQUIREMENTS.md` — `TYPEDUCK-E2E-01` through `TYPEDUCK-E2E-04`. [VERIFIED: file read]
- `.planning/ROADMAP.md` — Phase 10 goal, dependencies, success criteria, planned slices, AI deferral. [VERIFIED: file read]
- `.planning/STATE.md` — current status and blockers/concerns. [VERIFIED: file read]
- `docs/typeduck-web-adapter.md` — Yune adapter/export/runtime/filesystem contract. [VERIFIED: file read]
- `scripts/typeduck-exports.txt` — canonical adapter export list. [VERIFIED: file read]
- `scripts/typeduck-wasm-build.sh` — build/export verification and blocker fallback behavior. [VERIFIED: file read]
- `packages/yune-typeduck-runtime/src/module.ts` — Emscripten Module interface and export binding signatures. [VERIFIED: file read]
- `packages/yune-typeduck-runtime/src/typeduck.ts` — Yune wrapper operations and lifecycle guard. [VERIFIED: file read]
- `packages/yune-typeduck-runtime/src/response.ts` — response shape parsing and free-in-finally ownership. [VERIFIED: file read]
- `packages/yune-typeduck-runtime/src/filesystem.ts` — FS/IDBFS helper API and sync wrappers. [VERIFIED: file read]
- `packages/yune-typeduck-runtime/package.json` — local TypeScript/Vitest package metadata. [VERIFIED: file read]
- TypeDuck-Web GitHub repository `https://github.com/TypeDuck-HK/TypeDuck-Web`, default branch `main`, commit `03f9afd2cf6ca75653197f2193f24d1cd0adbd83`. [VERIFIED: gh api]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/package.json` — upstream scripts/dependencies. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/src/rime.ts` — main-thread worker queue/listener seam. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/src/worker.ts` — worker Module/ccall/IDBFS/actions seam. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/src/types.ts` — upstream `Actions`, `RimeResult`, preferences, listeners. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/src/CandidatePanel.tsx` — keyboard, candidate, paging, selection/deletion flow. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/src/App.tsx` — initialization/customize/deploy UI flow. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/wasm/api.cpp` — upstream librime C++ bridge exports/response shape. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/scripts/build_wasm.ts` — upstream Emscripten build flags/artifacts/preload. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/vite.config.ts` — Vite base `/web/` and build target. [VERIFIED: WebFetch]
- `https://raw.githubusercontent.com/TypeDuck-HK/TypeDuck-Web/main/src/consts.ts` — upstream RIME key-name mapping. [VERIFIED: WebFetch]
- [Playwright docs: intro](https://playwright.dev/docs/intro) — Playwright Test browser/E2E capabilities. [CITED: official docs]
- [Playwright docs: webServer](https://playwright.dev/docs/test-webserver) — `webServer`, `baseURL`, `reuseExistingServer`, timeout behavior. [CITED: official docs]
- [Emscripten preamble.js API](https://emscripten.org/docs/api_reference/preamble.js.html) — `ccall`, `cwrap`, `UTF8ToString`, export/runtime method requirements. [CITED: official docs]
- [Emscripten File System API](https://emscripten.org/docs/api_reference/Filesystem-API.html) — `FS.mount`, IDBFS, `FS.syncfs(true/false)`. [CITED: official docs]
- [MDN KeyboardEvent.key](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key) — semantic key values and modifier/layout behavior. [CITED: official docs]
- [MDN KeyboardEvent.keyCode](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode) — deprecation and unreliability. [CITED: official docs]
- npm registry — `@playwright/test`, `playwright`, `vite`, `react`, `bun-types`, and `esbuild` versions. [VERIFIED: npm registry]

### Secondary (MEDIUM confidence)

- Context7 CLI `/microsoft/playwright` docs query — corroborated Playwright `webServer`, `baseURL`, traces, locators, isolation, and assertions. [VERIFIED: Context7 CLI]
- [Bun installation docs](https://bun.sh/docs/installation) — Bun binary/availability verification guidance. [CITED: official docs]

### Tertiary (LOW confidence)

- Assumptions listed in the Assumptions Log about exact checkout location, patch strategy, selector strategy, and go/no-go rubric. [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for current package versions, local tool availability, and upstream package scripts; MEDIUM for exact dependency installation commands inside the future checkout. [VERIFIED: npm registry; VERIFIED: local tool probes; VERIFIED: upstream package.json via WebFetch; ASSUMED]
- Architecture: HIGH for seam identification and Yune runtime contracts; MEDIUM for exact worker patch because it has not been applied against a real checkout in this session. [VERIFIED: upstream source via WebFetch; VERIFIED: local runtime files; ASSUMED]
- Pitfalls: HIGH for response ownership, key API deprecation, FS sync direction, and missing Emscripten toolchain; MEDIUM for `setOption`/notification impact until real UI flows are run. [VERIFIED: local response.ts; CITED: MDN; CITED: Emscripten docs; VERIFIED: local tool probes; ASSUMED]
- Security: MEDIUM-HIGH because local-first/file/resource/pointer risks are concrete, but no auth/crypto/remote data flow is in scope. [VERIFIED: local docs/runtime files; VERIFIED: roadmap scope]

**Research date:** 2026-05-05 [VERIFIED: system current date]
**Valid until:** 2026-05-12 for npm/Playwright/Bun/Vite version-sensitive details; 2026-06-04 for Yune adapter/runtime contracts unless `scripts/typeduck-exports.txt`, `typeduck-wasm-build.sh`, `packages/yune-typeduck-runtime`, or upstream TypeDuck-Web commit pin changes. [ASSUMED]
