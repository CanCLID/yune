# TypeDuck-Web Integration Findings

> **Status:** Finished · **Milestone:** M9 (TypeDuck-Web browser validation) · **Closed:** 2026-06-18 · **Type:** findings / reference

This document records findings from integrating Yune with the upstream TypeDuck-Web browser application.

## Current Recommendation

**Recommendation: GO WITH CONDITIONS** — the TypeDuck-Web browser path is proven
for classic real-asset RIME flows, so AI-native frontend exposure may proceed
only behind the separate M11 gating policy: AI remains off by default in real
frontends until provider/ranking/privacy behavior is proven through the CLI
slice and explicitly enabled. HR-5 proves the browser can run the real TypeDuck
`jyut6ping3_mobile` matrix end to end: real candidates render, paging changes
pages, Space commits the highlighted candidate, `ngohaigo` commits as a phrase,
candidate deletion works, deploy/customize/persistence/reload pass, and
dictionary-panel comments render from the v1.1.2 oracle-shaped payload. The
post-review closeout fixed the `{Control_L}` modifier-key banner that appeared
before `{Control+Delete}` and recaptured delete/backspace evidence with no
visible runtime-error banner. A later post-review regression fixed partial-final
sentence completion, matching the TypeDuck v1.1.2 oracle for `ngohaig -> 我係個`
and recapturing clean browser evidence in `hr5-final-ngohaig-state.json`. HR-6
also locks the shared reverse-lookup comment
joiner and schema-prompt bytes. Clean-checkout dictionary-comment byte parity is
guaranteed by the committed `cantonese_parity` fixture; the browser-shaped
native rich-comment integration test runs its byte assertion only when the local
TypeDuck v1.1.2 oracle build assets are present.

> **Historical scope.** The Phase 10 blocker tables below describe the
> 2026-05-05 validation attempt, before WI-1b produced a loadable
> Emscripten JS/WASM module. The current WI-4 browser evidence is the
> Phase 17 matrix in this file's final summary; the HR-1 note below supersedes
> its echo-backed composition/candidate rows.

## HR-1 Real-Assets Browser Smoke

**Date**: 2026-06-18
**Status**: PASS for the real-assets candidate gate; full E2E matrix still open.

**What changed**:
- TypeDuck-Web worker now selects `jyut6ping3_mobile` and dictionary
  `jyut6ping3`, with deployed `schema/build/default.yaml` and
  `schema/build/jyut6ping3_mobile.schema.yaml` preloaded before init.
- `RimeGetContext` no longer rejects the mobile schema's
  `menu/alternative_select_keys: "\x00"` sentinel when exporting context; the
  sentinel remains available internally for selector behavior, but browser JSON
  receives `select_keys: null`.
- The Emscripten build uses a larger/growing memory configuration for the real
  asset load.

**Proof**:
- Direct Node/Emscripten artifact instantiation with real assets returns
  `schema: "jyut6ping3_mobile"`, `input: "nei"`, `preedit: "nei"`,
  `select_keys: null`, and candidates beginning `你`, `呢`, `尼`.
- Live browser at `http://127.0.0.1:5173/web/?debug&realAssets=1` renders the
  candidate panel for `nei` with `1.你`, `2.呢`, `3.尼`, followed by real
  TypeDuck dictionary candidates.
- Browser logs record `initialized: true` and a `{i}` `processKey` success with
  `isComposing: true`, `inputBuffer.before: "nei"`, and candidates beginning
  `你`, `呢`, `尼`.
- HR-1b committed the browser proof under
  `third_party/typeduck-web/e2e/results/`: `browser-run.log`,
  `browser-console.json`, `dom-snapshot-candidates.txt`, `blocker.md`, and
  `screenshot-real-assets-nei.png`. These artifacts supersede the old
  echo-backed WI-4 results for the real-assets candidate gate.

**Still open after HR-1**:
- `setOption` was still throwing from the adapter stub; HR-2 resolves that gap.
- Browser `deploy()` returned `false`; HR-3 resolves that gap.
- HR-4 proves live-worker persistence sync before init, after customize/deploy,
  and after a real reload.
- Paging, deletion, and dictionary-panel comment bytes must be re-run against
  real assets in HR-5.

---

## HR-2 setOption Export And Adapter Path

**Date**: 2026-06-18
**Status**: PASS for the `setOption` gap; full E2E matrix still open.

**What changed**:
- Added the `yune_typeduck_set_option` C export, retained it in the Emscripten
  linker anchor, and added it to `scripts/typeduck-exports.txt`.
- Added `TypeDuckRuntime.setOption` and the TypeScript binding signature.
- Replaced the TypeDuck-Web adapter's throwing `setOption` stub with a call into
  the active runtime.

**Proof**:
- `cargo test -p yune-rime-api --test typeduck_web` includes
  `typeduck_adapter_set_option_updates_session_status`.
- `npm --prefix packages/yune-typeduck-runtime test` covers binding/runtime
  forwarding.
- `vitest run third_party/typeduck-web/yune-integration` covers adapter
  forwarding.
- `third_party/typeduck-web/e2e/results/set-option-browser.log` records a live
  browser reload where startup `setOption` posts receive success responses, no
  option-error toast is visible, and no browser error logs are emitted.

---

## HR-3 Deploy With Real Assets

**Date**: 2026-06-18
**Status**: PASS for browser deploy with real TypeDuck assets; full E2E matrix
still open.

**What changed**:
- Added the plain `jyut6ping3.schema.yaml` to the TypeDuck-Web worker preload
  set. The real `default.custom.yaml` workspace path can reach this source
  schema during deployment even though the active browser schema is
  `jyut6ping3_mobile`.
- Tightened the native `typeduck_web` contract test so its "browser real assets"
  setup mirrors the curated browser preload list instead of copying the whole
  schema directory.

**Proof**:
- Node replay with the exact Emscripten artifact returned `deploy: 0` when the
  curated preload list omitted `jyut6ping3.schema.yaml`, and `deploy: 1` after
  adding it.
- `third_party/typeduck-web/e2e/results/deploy-browser.log` records a live
  browser run at `http://127.0.0.1:5173/web/?debug&realAssets=1` with
  `initialized: true`, `customize` result `true`, `deploy` posted, and `deploy`
  result `true`.
- `cargo test -p yune-rime-api --test typeduck_web` passes 10 tests, including
  deploy-after-init and deploy-after-customize with the browser-shaped real
  asset set.

**Still open after HR-3**:
- HR-4 must prove live-worker persistence sync before init and after mutations,
  plus reload survival. HR-4 resolves this.
- HR-5 must rerun paging, deletion, and dictionary-panel comment bytes against
  the real assets.

---

## HR-4 Live Persistence And Reload Survival

**Date**: 2026-06-18
**Status**: PASS for live-worker persistence sync and reload survival; full E2E
matrix still open.

**What changed**:
- The TypeDuck-Web adapter now emits structured persistence diagnostics around
  `syncFromPersistenceBeforeInit`, runtime init, and every
  `syncToPersistenceAfterMutation` boundary used by init, customize, deploy,
  commit, select, and delete paths.
- The worker forwards those diagnostics to the main page as debug diagnostics,
  and the main page records them in the DOM under
  `data-yune-persistence-diagnostics` for browser-captured evidence.
- `deploy()` and `customize()` now call the runtime operation and then perform
  the same persistence sync with a reason-tagged marker, so the live worker path
  exposes the exact after-mutation boundary.

**Proof**:
- `third_party/typeduck-web/e2e/results/persistence-sync.log` records a fresh
  browser origin where first load starts with no persisted
  `/rime/jyut6ping3_mobile.custom.yaml`, then startup `customize` writes
  `page_size: '6'`, and `deploy` syncs it to persistence.
- The same log records a real page reload where
  `syncFromPersistenceBeforeInit:pass` sees the persisted custom config with
  `pageSize: "'6'"` before `runtime:init`.
- The log's assertions are all PASS: before-init sync precedes runtime init,
  customize syncs after mutation, deploy syncs after mutation, and reload
  restores persisted state before re-init.

**Resolved by HR-5**:
- Paging, deletion, phrase commit, reload, and dictionary-panel comment bytes now
  have real-assets browser evidence.

---

## HR-5 Real-Assets E2E Matrix

**Date**: 2026-06-18
**Status**: PASS for the real-assets browser matrix; HR-7 final recommendation
recorded as GO WITH CONDITIONS.

**What changed**:
- The dictionary lookup filter install path now parses TypeDuck lookup
  dictionaries in their real code-first `payload<TAB>text` shape, then matches
  candidates after the translator form-feed prefix. Browser candidates now carry
  dictionary-panel payload bytes instead of raw code-only comments.
- TypeDuck-Web candidate and dictionary-panel table markup now uses valid table
  children, removing React dev-console nesting warnings from the HR-5 browser
  evidence.
- The final matrix ran against live `jyut6ping3_mobile` browser assets and live
  worker persistence diagnostics.

**Proof**:
- `third_party/typeduck-web/e2e/results/hr5-real-assets-matrix.json` records
  PASS for composition, candidate list, paging, selection, Space commit,
  long-press candidate deletion, deploy, customize, persistence sync, reload,
  and dictionary-panel comments, with zero browser warning/error console entries
  in the final capture.
- Post-review delete/backspace recaptures replaced the stale banner-bearing
  artifacts. `hr5-final-delete-state.json` now records the root cause: the page
  sent `{Control_L}` immediately before `{Control+Delete}`; the adapter now
  treats pure modifier keydowns as pass-through, so the first `ngo` candidate is
  removed without a visible runtime-error banner. `hr5-final-backspace-state.json`
  also records `ngo` mutating to `ng` with no banner.
- Screenshot evidence was captured as
  `third_party/typeduck-web/e2e/results/screenshot-hr5-dictionary-panel.png`
  and `third_party/typeduck-web/e2e/results/screenshot-hr5-after-delete.png`.
- `cargo test -p yune-rime-api --test typeduck_web
  typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments` asserts
  the first `nei` candidate comment against the v1.1.2 fixture when the local
  `target/typeduck-oracle/v1.1.2/rime-user/build` assets are present. If those
  ignored local oracle-build assets are absent, the test emits an explicit skip
  reason instead of passing against the degraded fallback. The committed
  clean-checkout byte-parity guarantee is
  `cargo test -p yune-core --test cantonese_parity`, which uses
  `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json`.
- The matrix artifact's reload row proves a real reload restored
  `/rime/jyut6ping3_mobile.custom.yaml` with `pageSize: "'6'"` before runtime
  init.

---

## HR-6 Shared Reverse-Lookup Parity

**Date**: 2026-06-18
**Status**: PASS for the reverse-lookup `"; "` joiner and schema-name prompt
oracle cases; the five broader Cantonese parity goldens remain explicitly
BLOCKED pending dedicated v1.1.2 captures.

**What changed**:
- Added `crates/yune-core/tests/fixtures/typeduck-v1.1.2/reverse-lookup-prompt.json`,
  captured from the local TypeDuck-HK/librime v1.1.2 binary with a scratch
  schema. The fixture records the prompt/preedit bytes for input `` `huo `` and
  the selected candidate comment `ho; huo`.
- Added core parity tests that lock the fixture metadata and assert
  `ReverseLookupTranslator` joins multiple target pronunciations with `"; "`
  against the v1.1.2 oracle expectation.
- Loaded affix-segmentor `tips` into the Rime API schema path and exposed the
  prompt-shaped preedit through `RimeGetContext`, with an ABI test asserting
  preedit, cursor, commit preview, candidate text, and candidate comment against
  the same v1.1.2 fixture.

**Proof**:
- `cargo test -p yune-core --test cantonese_parity` passes the four active
  oracle-backed tests and reports the five uncaptured Cantonese cases as
  ignored blockers.
- `cargo test -p yune-rime-api
  select_schema_affix_prompt_matches_typeduck_v112_reverse_lookup_fixture_commit_preview`
  passes, proving the C ABI context path emits the oracle prompt/comment bytes.

**Still blocked**:
- `options_combine_candidates_show_full_code_enable_sentence_parity`
- `completion_prediction_and_enable_completion_parity`
- `correction_minimal_distance_and_m_abbreviation_parity`
- `schema_menu_hiding_parity`
- `per_entry_userdb_pronunciation_parity`

Those tests remain `#[ignore = "blocked: ..."]` with `panic!()` bodies. They are
not silent skips and should only be activated after dedicated TypeDuck v1.1.2
oracle fixtures are captured.

---

## Plan 10-01: Upstream seam inspection

**Date**: 2026-05-05
**Upstream Commit**: 03f9afd2cf6ca75653197f2193f24d1cd0adbd83
**Status**: Seam identified and documented (no source patching performed)

### Seam Overview

TypeDuck-Web uses a worker-based architecture where:
- Main thread (`src/rime.ts`) creates a Worker and queues action calls
- Worker (`src/worker.ts`) loads Emscripten-generated `rime.js`, initializes librime C++ bridge, and processes actions through `Module.ccall`
- Native bridge (`wasm/api.cpp`) implements librime-shaped C functions that forward to librime API
- UI (`src/CandidatePanel.tsx`) captures keyboard events and sends simulated key sequences

### Key Seam Files

#### 1. `src/worker.ts` — Primary Replacement Seam

**Role**: Worker implementation that bridges main-thread Actions to native librime calls

**Module Initialization** (lines 97-125):
- Defines `globalThis.Module` with `onRuntimeInitialized`, `printErr`, `locateFile`
- Loads `rime.js` via `importScripts("rime.js")`
- Waits for runtime initialization before processing actions

**Filesystem/Persistence** (lines 55-59, 111-116):
- Mounts IDBFS at `/rime` (RIME_USER_DIR)
- Uses `Module.FS.syncfs(direction === "read")` for persistence
- Syncs read before init, syncs write after commit/deploy
- **Pattern**: `syncUserDirectory("read")` → `Module.ccall("init")` → `syncUserDirectory("write")`

**Action Calls** (lines 61-93):
- `setOption`: `Module.ccall("set_option", null, ["string", "number"], [option, +value])`
- `processKey`: `Module.ccall("process_key", "string", ["string"], [input])` → returns JSON string parsed as RimeResult
- `selectCandidate`: `Module.ccall("select_candidate", "string", ["number"], [index])`
- `deleteCandidate`: `Module.ccall("delete_candidate", "string", ["number"], [index])`
- `flipPage`: `Module.ccall("flip_page", "string", ["boolean"], [backward])`
- `customize`: `Module.ccall("customize", "boolean", ["number", "number"], [pageSize, options])`
- `deploy`: `Module.ccall("deploy", "boolean", [], [])`

**Notifications** (lines 35-49):
- `globalThis.onRimeNotification` dispatches listener events (deploy, schema, option)
- Callbacks: `deployStatusChanged`, `schemaChanged`, `optionChanged`, `initialized`

**Yune Replacement Strategy**: Replace `importScripts("rime.js")` with Yune Emscripten artifact, replace `Module.ccall` calls with `@yune-ime/typeduck-runtime` `TypeDuckRuntime` methods, preserve Actions interface and listener events.

#### 2. `src/rime.ts` — Main-Thread Worker Queue

**Role**: Facade that creates Worker and queues action calls

**Worker Bridge** (lines 40-67):
- Creates `new Worker("./worker.js")`
- Queues one action at a time (serial execution)
- Posts `{ name, args }` messages to worker
- Receives `{ type: "success", result }` or `{ type: "error", error }` or `{ type: "listener", name, args }`

**Actions API** (lines 75-88):
- Dynamically registers `setOption`, `processKey`, `selectCandidate`, `deleteCandidate`, `flipPage`, `customize`, `deploy`
- Each action returns Promise resolving to action result

**Listeners** (lines 105-110):
- `subscribe(type, callback)` registers listeners
- Types: `deployStatusChanged`, `schemaChanged`, `optionChanged`, `initialized`

**Yune Replacement Strategy**: Preserve facade and queue behavior; patch worker implementation only.

#### 3. `src/types.ts` — Actions and RimeResult Interface

**Role**: TypeScript interfaces defining action signatures and result shapes

**Actions Interface** (lines 16-24):
```typescript
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

**RimeResult Shape** (lines 26-54):
- Composing state: `{ isComposing: true, inputBuffer: { before, active, after }, page, isLastPage, highlightedIndex, candidates: [{ label?, text, comment? }] }`
- Non-composing state: `{ isComposing: false }`
- Payload: `{ success: boolean, committed?: string }`

**Listener Types** (lines 64-69):
- `deployStatusChanged: [status: "start" | "success" | "failure"]`
- `schemaChanged: [id: string, name: string]`
- `optionChanged: [option: string, value: boolean]`
- `initialized: [success: boolean]`

**Yune Replacement Strategy**: Preserve Actions interface; translate `TypeDuckResponse` to `RimeResult` shape in worker adapter.

#### 4. `wasm/api.cpp` — Native C++ Bridge

**Role**: Librime-shaped C exports called by Emscripten Module.ccall

**Exports** (lines 97-166):
- `bool init()` — Initialize librime with `/usr/share/rime-data` shared dir, `/rime` user dir, create session
- `void set_option(const char* option, int value)` — Set session option via librime API
- `const char* process_key(const char* input)` — Calls `rime->simulate_key_sequence(session_id, input)` and returns JSON result
- `const char* select_candidate(int index)` — Select candidate on current page, return JSON
- `const char* delete_candidate(int index)` — Delete candidate on current page, return JSON
- `const char* flip_page(bool backward)` — Change page, return JSON
- `bool customize(int page_size, int options)` — Customize default/common settings via RimeLeversApi
- `bool deploy()` — Restart librime with maintenance thread

**Key Observation**: `process_key` accepts string input and calls `simulate_key_sequence`, which is different from Yune's keycode/mask approach.

**Yune Replacement Strategy**: Yune native adapter uses `yune_typeduck_*` exports with different signatures; adapter layer must translate between upstream string input and Yune keycode/mask.

#### 5. `scripts/build_wasm.ts` — Emscripten Build Script

**Role**: Defines Emscripten compile/link flags for WASM artifact

**Exported Functions** (lines 5-12):
```typescript
const exportedFunctions = [
  "_init",
  "_set_option",
  "_process_key",
  "_select_candidate",
  "_delete_candidate",
  "_flip_page",
  "_customize",
  "_deploy",
].join();
```

**Runtime Methods** (line 22):
```typescript
-s EXPORTED_RUNTIME_METHODS=["ccall","FS"]
```

**Preload** (line 23):
```typescript
--preload-file schema@/usr/share/rime-data
```

**Output** (line 25):
```typescript
-o public/rime.js
```

**Yune Replacement Strategy**: Yune uses different exports (`yune_typeduck_*`) and runtime methods (UTF8ToString); must ensure Yune artifact provides compatible Module interface and FS/IDBFS.

#### 6. `src/CandidatePanel.tsx` — Keyboard Event Handling

**Role**: UI component that captures keyboard input and calls Rime.processKey

**Keyboard Flow** (lines 124-130, 133-137):
- `document.addEventListener("keydown", onKeyDown)`
- `document.addEventListener("keyup", onKeyUp)`
- `processKey(`{${key}}`, event.key)` — sends string sequences like `{BackSpace}`
- `processKey(`{Release+${key}}`)` — sends release sequences

**Key Sequence Format**:
- Printable keys sent directly (e.g., `a`, `b`)
- Special keys wrapped in braces (e.g., `{BackSpace}`, `{Enter}`, `{Escape}`)
- Release events prefixed (e.g., `{Release+BackSpace}`)

**Yune Replacement Strategy**: Yune uses `keyEventToRimeKey` mapping from `KeyboardEvent.key` to keycode/mask; must either patch CandidatePanel to call `processKeyboardEvent(event)` or add a compatibility adapter parsing string sequences.

### Librime/WASM Seam Call Flow

```
User types in textarea
  |
  v
CandidatePanel.tsx keydown/keyup handlers
  |-- build key sequence string: `{BackSpace}`, `a`, `{Release+Enter}`
  |-- call Rime.processKey(input)
  v
Main-thread src/rime.ts facade
  |-- queue action message
  |-- postMessage to worker
  v
Worker src/worker.ts implementation
  |-- await loadRime (importScripts("rime.js"))
  |-- Module.FS.mkdir("/rime")
  |-- Module.FS.mount(IDBFS, {}, "/rime")
  |-- Module.FS.syncfs(true) // read
  |-- Module.ccall("init", "boolean", [], [])
  |-- Module.FS.syncfs(false) // write
  |-- on action:
  |   |-- Module.ccall("process_key", "string", ["string"], [input])
  |   |-- JSON.parse(result) -> RimeResult
  |   |-- if committed: syncUserDirectory("write")
  |-- postMessage back to main thread
  v
Emscripten-generated rime.js Module
  |-- ccall resolves to C functions
  |-- FS/IDBFS available
  v
Native wasm/api.cpp exports
  |-- process_key(const char* input)
  |   |-- rime->simulate_key_sequence(session_id, input)
  |   |-- build JSON result (success, committed, isComposing, inputBuffer, candidates)
  |-- return const char* JSON string
  v
Librime C++ API
  |-- RimeApi function table
  |-- Session, context, candidates, deployment
  v
Worker parses JSON, returns to main thread
  |
  v
CandidatePanel renders result
```

### Yune Integration Gap Analysis

#### Contract Mismatch: String Input vs. Keycode/Mask

**Upstream**: `processKey(input: string)` sends key sequences like `{BackSpace}`, `a`
**Yune**: `processKeyboardEvent(event)` or `processKey(keycode, mask)` uses integer keycode/modifier mask

**Mitigation**: Either:
1. Patch `CandidatePanel.tsx` to call `Rime.processKeyboardEvent(event)` with event-like object (preferred for clarity)
2. Add compatibility adapter parsing string sequences to keycode/mask (less invasive but extra code)

#### Contract Mismatch: RimeResult vs. TypeDuckResponse

**Upstream**: `RimeResult` with `{ isComposing, inputBuffer?, page?, isLastPage?, highlightedIndex?, candidates?, success, committed? }`
**Yune**: `TypeDuckResponse` with `{ handled, commits, context?, status?, error? }` where context has `{ preedit, caret, candidates, select_labels, ... }`

**Mitigation**: Worker adapter layer must translate Yune response to upstream RimeResult shape before returning to main thread.

#### Missing Export: setOption

**Upstream**: `Actions.setOption(option: string, value: boolean)`
**Yune**: Current TypeDuck wrapper lacks `setOption` method

**Mitigation**: Determine if E2E flows require `setOption`; if yes, either map through customize/status or add native/wrapper support.

#### Persistence Timing

**Upstream**: Explicit `syncUserDirectory("read")` before init, `"write"` after commit/deploy
**Yune**: Phase 9 helpers `syncFromPersistenceBeforeInit`, `syncToPersistenceAfterMutation`, `deployAndSync`, `customizeAndSync`

**Mitigation**: Use Yune helpers in worker replacement; preserve sync boundaries.

#### Asset Preload

**Upstream**: Build script preloads `schema@/usr/share/rime-data`
**Yune**: Caller-owned assets via `prepareTypeDuckFilesystem`, `assertTypeDuckAssetsReady`

**Mitigation**: Yune worker must create shared/user/build layout with explicit assets before init; no build-time preload.

### Deferred Items (Per D-14)

The following are explicitly deferred and not part of this plan:

- AI-native provider calls, candidate generation, ranking policy
- AI-native context capture, memory, privacy controls
- New first-party Yune graphical frontend
- Multi-instance Yune/RIME service isolation
- Browser CDN/cache/service worker/storage quota policy

### Seam Inspection Summary

**Files Identified**: 6 key seam files documented with exact paths, line numbers, and call patterns
**Call Flow**: Main thread → Worker queue → Emscripten Module → Native exports → Librime API
**Contract Gaps**: String input vs keycode/mask, RimeResult vs TypeDuckResponse, missing setOption, different persistence helpers
**Replacement Seam**: `src/worker.ts` is primary replacement target; preserve `src/rime.ts` facade and `Actions` interface

**Next Plan**: 10-02 will implement minimal seam replacement using `@yune-ime/typeduck-runtime` and document any remaining blockers.

---

## Plan 10-02: Yune seam patch

**Date**: 2026-05-05
**Status**: Minimal patch generated, pending build gates and E2E asset configuration

### Patch Scope

Minimal patch touches two files only per D-03:

1. `src/worker.ts` — Replace librime WASM binding with Yune runtime adapter
   - Import Yune adapter functions from `yune-integration/adapter.js`
   - Import asset loader from `yune-integration/assets.js`
   - Replace `Module.ccall` calls with adapter exports (processKey, selectCandidate, deleteCandidate, flipPage, deploy, customize, setOption)
   - Replace `importScripts("rime.js")` with `importScripts("yune-typeduck.js")` (Yune WASM artifact)
   - Preserve notification dispatch, message handling, Actions interface
   - Add cleanup on worker termination
   - Preserve worker queue behavior and listener events

2. `package.json` — Add package alias for local resolution
   - Add `"@yune-ime/typeduck-runtime": "file:../../../packages/yune-typeduck-runtime"` dependency
   - Enables upstream worker to import Yune package without publishing

### Yune Integration Layer

Created `third_party/typeduck-web/yune-integration/` directory with:

1. **adapter.ts** — Yune seam adapter
   - Imports TypeDuckRuntime, keyEventToRimeKey, filesystem helpers per D-04
   - Enforces one-active-runtime-per-Module constraint per D-05
   - Translates TypeDuckResponse to upstream RimeResult shape
   - Parses upstream key sequence strings (e.g., `{BackSpace}`, `a`) to keyboard event-like objects
   - Delegates persistence sync to Yune helpers (syncFromPersistenceBeforeInit, syncToPersistenceAfterMutation, deployAndSync, customizeAndSync)
   - Documents setOption gap: throws error if called, requires Yune widening per D-07

2. **assets.ts** — Explicit asset contract
   - Requires TypeDuck-Web-owned default.yaml, schema YAML, dictionary YAML per D-06
   - Validates no synthetic/fake/placeholder asset content
   - Fails visibly when assets absent
   - Provides asset checklist for init verification

3. **README.md** — Integration instructions
   - Patch application steps
   - Lifecycle hooks (init, actions, cleanup)
   - Contract mismatch documentation
   - Known gaps and blockers
   - Deferred items per D-14

4. **package-alias.md** — Local package resolution methods
   - Package.json alias (preferred)
   - Vite resolve alias
   - Relative import fallback

### Contract Mismatches Addressed

1. **String Input vs Keycode/Mask**
   - Upstream: `processKey("{BackSpace}")`
   - Yune: `processKey(keycode, mask)`
   - Adapter: Parses string sequences to `TypeDuckKeyboardEventLike`, delegates to `keyEventToRimeKey`

2. **RimeResult vs TypeDuckResponse**
   - Upstream: `RimeResult` with `isComposing`, `inputBuffer`, `candidates`, `committed`
   - Yune: `TypeDuckResponse` with `handled`, `commits`, `context.preedit`, `context.candidates`
   - Adapter: Translates response fields to upstream shape

3. **Persistence Timing**
   - Upstream: `syncUserDirectory("read")` before init, `"write"` after commit/deploy
   - Yune: `syncFromPersistenceBeforeInit`, `syncToPersistenceAfterMutation` match pattern
   - Adapter: Uses Yune helpers in init and action flows

4. **Resolved setOption**
   - Upstream: `Actions.setOption(option, value)`
   - Yune: `TypeDuckRuntime.setOption` delegates to `yune_typeduck_set_option`
   - Adapter: Forwards option toggles to the active runtime

### Patch Generation

Patch file: `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`

Generated via:
```bash
cd third_party/typeduck-web/source
git diff src/worker.ts package.json > ../patches/yune-typeduck-runtime.patch
```

Patch contents:
- package.json: Add Yune package alias dependency
- src/worker.ts: Import Yune adapter, replace ccall with adapter calls, load Yune WASM artifact

### Known Gaps (Per D-07, D-09)

#### TypeDuck-Web app/source blockers

1. **Asset Configuration TODO**
   - Patched worker contains placeholder asset configuration: `content: ""`
   - Requires explicit TypeDuck-Web-owned YAML assets before init
   - E2E task must provide real default.yaml, schema YAML, dictionary YAML
   - Asset discovery mechanism needed (app config, CDN, bundled data)

2. **Yune WASM Artifact Naming**
   - Patch references `importScripts("yune-typeduck.js")`
   - Requires build task generating Yune Emscripten artifact with correct filename
   - locateFile path assumes artifact in `packages/yune-typeduck-runtime/dist/`

3. **setOption API Gap** (resolved by HR-2)
   - Adapter now forwards `setOption` through the TypeDuck runtime
   - Native/runtime/adapter tests cover the path
   - Browser startup smoke shows no option-error toast

#### Yune adapter/runtime mismatches

1. **Native/wrapper setOption** (resolved by HR-2)
   - TypeDuckRuntime exposes `setOption`
   - Upstream worker calls through the adapter without option-error toasts
   - Native/runtime/adapter tests cover the path

2. **customize Options Bitmap**
   - Upstream customize uses pageSize and options bitmap
   - Yune customize accepts (configId, key, value) strings
   - Adapter maps pageSize only; options bitmap handling incomplete

#### Environment/tooling blockers

1. **Bun Package Manager**
   - TypeDuck-Web uses Bun for install/build
   - Bun may be unavailable in local environment
   - Task 3 will document: command (`bun install`), missing executable, install hint, fallback evidence

2. **Yune Runtime Build**
   - Requires `npm --prefix packages/yune-typeduck-runtime run build`
   - Must pass before upstream worker can import Yune package

3. **Yune WASM Build**
   - Requires Phase 7 WASM artifact with `yune_typeduck_*` exports
   - Emscripten build chain may have blockers
   - Will document in Task 3 build gates

### Deferred Items (Per D-14)

The following remain deferred and are NOT part of this plan:

- AI-native provider calls, candidate generation, ranking policy
- AI-native context capture, memory, privacy controls
- New first-party Yune graphical frontend
- Multi-instance Yune/RIME service isolation
- Browser CDN/cache/service worker/storage quota policy

### Patch Verification

```bash
test -s third_party/typeduck-web/patches/yune-typeduck-runtime.patch
grep -Eq "@yune-ime/typeduck-runtime|yune-integration|TypeDuckRuntime" third_party/typeduck-web/patches/yune-typeduck-runtime.patch
! grep -E "^diff --git a/(node_modules|dist|build|\.next|coverage)/" third_party/typeduck-web/patches/yune-typeduck-runtime.patch
```

Expected: All checks pass.

### Next Steps

Task 3 will:
- Run upstream build/typecheck (Bun) and document blockers
- Run Yune runtime build (npm) and verify pass
- Record categorized blockers in findings
- Update README with build instructions or blocker evidence

---

## Plan 10-02: Build Gates

**Date**: 2026-05-05
**Status**: Build gates passed with documentation

### Repository-Owned Runtime Build

**Command**: `npm --prefix packages/yune-typeduck-runtime run build`

**Result**: PASSED
- TypeScript compilation successful
- Package builds without errors
- Adapter exports available for import

### Upstream Package Install

**Command**: `bun install` (from `third_party/typeduck-web/source`)

**Result**: PASSED
- Bun 1.3.11 available
- Dependencies resolved successfully
- Yune package alias resolved: `@yune-ime/typeduck-runtime@../../../packages/yune-typeduck-runtime`
- 458 packages installed

### Upstream Worker Build

**Command**: `bun run worker` (esbuild)

**Result**: PASSED
- Patched worker.ts compiles successfully
- Output: `public/worker.js` (3.4kb)
- Integration layer imports resolve correctly
- Build completes in ~1-4ms

### Upstream TypeScript Typecheck

**Command**: `bunx tsc --noEmit`

**Result**: PASSED (patched files only)
- No errors in `src/worker.ts`
- No errors in `src/yune-integration/adapter.ts`
- No errors in `src/yune-integration/assets.ts`
- Pre-existing errors in `scripts/build_lib.ts` and `scripts/build_native.ts` (Set.difference) ignored as out-of-scope per deviation rules

### Patch Refinement

**Action**: Regenerated patch to include TypeScript resolution fixes
- Added `tsconfig.json` modifications for path aliases (later reverted)
- Copied `yune-integration/` into `src/yune-integration/` for module resolution
- Fixed adapter.ts to use upstream types.ts imports instead of duplicate type definitions
- Fixed adapter.ts null checks and property access for TypeDuckContext
- Adjusted Module type conversion with `unknown` intermediate

**Final patch scope**:
- `package.json` — Yune package alias
- `src/worker.ts` — Yune adapter imports, runtime calls
- `tsconfig.json` — (removed, not needed after integration files moved to src)
- `src/yune-integration/adapter.ts` — Yune seam adapter
- `src/yune-integration/assets.ts` — Explicit asset contract
- `src/yune-integration/README.md` — Integration instructions
- `src/yune-integration/package-alias.md` — Local package resolution docs

### Build Gate Summary

**Repository runtime**: PASSED (npm build)
**Upstream package install**: PASSED (Bun available, alias resolved)
**Upstream worker build**: PASSED (esbuild compiles patched worker)
**Upstream typecheck**: PASSED (patched files error-free, pre-existing script errors out-of-scope)

**Blockers documented per D-09**:
- Bun available in environment — no blocker
- Yune runtime build passes — no blocker
- TypeScript errors resolved — no blocker
- Asset configuration TODO documented — blocker for E2E, not for build

### Categorized Blockers (Per D-12)

#### TypeDuck-Web app/source blockers

1. **Asset Configuration TODO**
   - Patched worker contains placeholder asset configuration: `defaultYaml: { type: "content", content: "" }`
   - Requires explicit TypeDuck-Web-owned YAML assets before runtime init
   - E2E task (Plan 10-03) must provide real assets or asset discovery mechanism
   - Not a build blocker; compiles successfully with placeholder

2. **Yune WASM Artifact Generation**
   - Patch references `importScripts("yune-typeduck.js")`
   - Requires Phase 7 WASM artifact with `yune_typeduck_*` exports
   - Artifact must be placed at expected path or `locateFile` adjusted
   - blocker for E2E, not for build (worker compiles with placeholder artifact path)

#### Yune adapter/runtime mismatches

1. **TypeDuckContext properties missing**
   - `comments` and `highlighted_candidate_index` not in current TypeDuckContext
   - Adapter maps to undefined/0 for compatibility
   - Does not block build or patch compilation
   - May affect E2E candidate comment/highlight behavior

2. **setOption API gap** (resolved by HR-2)
   - TypeDuckRuntime now exposes `setOption`
   - Adapter forwards upstream option toggles to the active runtime
   - Native/runtime/adapter tests cover the path

#### Environment/tooling blockers

**None** — All tooling available:
- Bun 1.3.11 installed and functional
- npm build passes
- TypeScript compiler resolves patched imports
- esbuild compiles worker successfully

### Deferred Items (Per D-14)

No deferred items implemented in build gates. AI-native behavior, new frontend, service isolation remain deferred as documented in Plan 10-02 seam patch section.

---

*Updated: 2026-05-05T16:45:00Z*
---

## Plan 10-03: Real browser E2E/smoke validation

**Date**: 2026-05-05
**Status**: Browser E2E spec created, pending browser runner execution

### Browser E2E Scaffolding (Task 1)

Created `third_party/typeduck-web/e2e/` with explicit asset/result instructions:

#### Assets README (D-06 enforcement)

**File**: `e2e/assets/README.md`

**Requirements**:
- TypeDuck-Web-owned YAML assets mandatory (default.yaml, schema.yaml, dictionary.yaml)
- NO fallback/dummy/placeholder schema or dictionary data
- Assets must come from TypeDuck-Web source, CDN, or documented upstream
- Validation rejects synthetic/test-only content
- Grep-gate verifies no forbidden substitute patterns

**Evidence**:
- `asset-sources.log` — Documented asset paths/URLs
- `asset-validation.log` — Runtime validation output
- Forbidden pattern check PASSED (no fallback schema/dictionary wording in scaffolding)

#### Results README (D-08/D-09/D-10/D-11 evidence)

**File**: `e2e/results/README.md`

**Required artifacts**:
- `browser-run.log` — Browser runner output with flow evidence
- `screenshot-*.png` — Screenshots for composition, candidates, paging, selection, persistence
- `persistence-sync.log` — D-11 timing markers (before init, after mutation, reload)
- `blocker.md` — Reproducible blockers with command/dependency/fallback

**Blocker format** (per D-09):
- Category: TypeDuck-Web app/source | Yune adapter/runtime | environment/tooling
- Exact command attempted
- Missing dependency/executable
- Install hint from upstream docs
- Fallback evidence (manual smoke or package-local tests)
- Impact: which D-08/D-10/D-11 flows blocked

#### Browser Smoke Procedure (D-08/D-09 fallback)

**File**: `e2e/yune-browser-smoke.md`

**Manual browser smoke steps**:
1. Apply patch (git apply patches/yune-typeduck-runtime.patch)
2. Install/build upstream (bun install, bun run worker)
3. Start dev server (bun run start)
4. Load explicit assets (per e2e/assets/README.md)
5. Test composition flow (type keys → verify preedit)
6. Test candidate paging (PageDown → page change)
7. Test candidate selection (selection key → commit text)
8. Test deletion flow (Delete/Backspace → composition mutation)
9. Test deploy flow (deploy action → success/error visible)
10. Test customize flow (customize action → success/error visible)
11. Test persistence sync (D-11 timing: before init, after mutation, reload/reinitialize)
12. Record evidence (screenshots, console logs, persistence markers)

**Evidence requirements**:
- Manual smoke MUST use real browser (not package-local fake tests)
- Persistence timing MUST verify sync-before-init, sync-after-mutation, reload-reinitialize

### Upstream Test Framework Discovery (Task 2)

**Discovery**: Upstream TypeDuck-Web has NO browser E2E test framework.

**Evidence**:
- package.json has NO test scripts
- NO test framework dependencies (Vitest, Jest, Playwright, Cypress)
- NO spec/test files found in upstream source
- Scripts are build-only (start, build, worker, wasm)

**Impact per Task 2 action**: Create standalone Playwright-compatible spec under `third_party/typeduck-web/e2e/` (not upstream source)

### Browser E2E Spec Created (Task 2)

**File**: `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`

**Spec coverage (per D-08/TYPEDUCK-E2E-03)**:

1. **Composition after typing schema-valid keys** (D-08/D-10)
   - Focus input field
   - Type schema-valid keys
   - Verify composition/preedit visible
   - Screenshot: screenshot-composition.png

2. **Candidate list visible** (D-08/D-10)
   - Type to trigger candidates
   - Verify candidate panel visible
   - Verify candidate count > 0
   - Screenshot: screenshot-candidates.png

3. **Candidate paging** (D-08/D-10)
   - Type to generate multiple candidates
   - Press PageDown
   - Verify page change (indicator or candidate set changed)
   - Screenshot: screenshot-candidate-paging.png

4. **Candidate selection → commit output** (D-08/D-10)
   - Type composition
   - Press selection key (1 or Space/Enter)
   - Verify committed text in output field
   - Screenshot: screenshot-candidate-selection.png

5. **Deletion removes candidate or triggers delete path** (D-08/D-10)
   - Type composition
   - Press Delete key
   - Verify deletion effect
   - Screenshot: screenshot-deletion.png

6. **Backspace mutates composition** (D-08/D-10)
   - Type composition
   - Press Backspace
   - Verify composition mutated (shorter or changed)

7. **Deploy returns visible success/error evidence** (D-08/D-10)
   - Trigger deploy action (button or Ctrl+D)
   - Verify deploy notification/status visible
   - Screenshot: screenshot-deploy.png

8. **Customize returns visible success/error evidence** (D-08/D-10)
   - Open settings panel
   - Change setting (pageSize)
   - Verify customize notification/status visible
   - Screenshot: screenshot-customize.png

9. **Persistence sync after deploy/customize mutations** (D-11)
   - Perform mutation (deploy)
   - Verify syncToPersistenceAfterMutation marker in console/timeline
   - Evidence: persistence-sync.log

10. **Reload/reinitialize preserves persisted state** (D-11)
    - Perform customization
    - Deploy to persist state
    - Reload page
    - Verify syncFromPersistenceBeforeInit marker
    - Verify app reinitialized with persisted state
    - Screenshot: screenshot-persistence-after-reload.png

**Evidence capture**:
- `browser-run.log` — Test results appended per flow
- `browser-console.log` — Console errors captured
- `persistence-sync.log` — Persistence timing markers
- `screenshot-*.png` — Visual evidence per flow
- `blocker.md` — Flows blocked by missing selectors/implementation

**Selector assumptions** (require E2E execution verification):
- Input field: `input[type='text'], textarea`
- Candidate panel: `[data-candidates], .candidate-panel, .candidate-list`
- Candidate items: `.candidate, [data-candidate]`
- Page indicator: `[data-page], .page-indicator`
- Deploy button: `[data-deploy], .deploy-button, button:has-text('deploy')` or Ctrl+D shortcut
- Settings panel: `[data-settings], .settings-panel, .customize-panel`
- Notifications: `.toast, [data-deploy-status], .notification`

**Testability gaps documented in blocker.md** (per D-09):
- Missing selectors → TypeDuck-Web app/source blocker
- Missing persistence markers → Yune adapter/runtime blocker
- Missing browser runner → Environment/tooling blocker

### Browser Runner Execution Status (Task 3 pending)

**Status**: Spec created, pending Playwright execution

**Prerequisites for Task 3 execution**:
1. Apply patch to upstream source
2. Build Yune WASM artifact (Phase 7 blocker if not built)
3. Build @yune-ime/typeduck-runtime package
4. Provide explicit TypeDuck-Web-owned YAML assets
5. Install Playwright in worktree or use manual browser smoke

**Expected blockers per D-09**:
- Yune WASM artifact not built (Phase 7 blocker)
- Asset configuration TODO in patched worker (placeholder YAML)
- Missing testability selectors (TypeDuck-Web UI)
- Persistence sync markers not logged (Yune adapter)

### Flow Pass/Fail Tracking (superseded by WI-4)

| Flow | D-08/D-10/D-11 Requirement | Status | Evidence |
|------|----------------------------|--------|----------|
| Composition | Schema-valid keys -> preedit visible | PASS | `e2e/results/browser-run.log`, `e2e/results/browser-console.json` |
| Candidate list | Visible after composition | PASS | `e2e/results/dom-snapshot-candidates.txt` |
| Candidate paging | PageDown -> page change | FAIL | PageDown accepted but page remains `0`, `isLastPage: true`; paging buttons disabled |
| Candidate selection | Selection key -> commit text | PASS | `browser-run.log`; pressing `1` commits `ba` |
| Deletion | Delete key -> candidate/composition change | FAIL | Historical echo-backed run left the same placeholder candidate; HR-5 must rerun this with real assets |
| Backspace mutation | Backspace -> composition shorter/changed | PASS | `browser-run.log`; same-session console shows `ba` -> `b` |
| Deploy | Deploy action -> visible success/error | FAIL | `browser-console.json`; deploy returns `false` |
| Customize | Customize action -> visible success/error | PASS | `browser-console.json`; customize returns `true` |
| Persistence sync | sync-after-mutation marker | FAIL | `persistence-sync.log`; no browser-visible sync markers and deploy fails |
| Persistence reload | sync-before-init + reload/reinitialize | FAIL | `persistence-sync.log`; reload survival not proven |
| Dictionary-panel comment rendering | v1.1.2 candidate comment bytes render | FAIL | HR-1b shows raw code-style comments such as `\fnei5`; HR-5 must assert dictionary-panel oracle bytes |

**Note**: WI-4 moved the flow matrix from blocked/pending to evidence-backed
PASS/FAIL. Screenshots were not available from the Codex browser wrapper; the
captured evidence is console JSON plus DOM snapshots.

---

*Updated: 2026-05-05T00:26:00Z*
*Plan: 10-03 (Real browser E2E/smoke validation)*

### Browser Execution Attempt (Task 3)

**Status**: BLOCKED

**Date**: 2026-05-05T00:30:00Z

#### Environment/Tooling Blockers

**Critical missing executables**:

1. **cargo** — Rust build tool
   - Command: `./scripts/typeduck-wasm-build.sh`
   - Error: `cargo: command not found`
   - Impact: Cannot build WASM, cannot run native tests

2. **rustup** — Rust toolchain manager
   - Command: `rustup target list --installed`
   - Error: `command not found: rustup`
   - Impact: Cannot install wasm32-unknown-emscripten target
   - Install: https://rustup.rs

3. **emcc** — Emscripten compiler
   - Command: `emcc --version`
   - Error: `emcc not found`
   - Impact: Cannot compile WASM/JS glue
   - Install: https://emscripten.org/docs/getting_started/downloads.html

#### WASM Artifact Blocker

**Required**: `yune-typeduck.js` + `yune-typeduck.wasm` (Phase 7 artifact)

**Patch dependency**: `src/worker.ts` calls `importScripts("yune-typeduck.js")`

**Build attempt**:
```bash
$ ./scripts/typeduck-wasm-build.sh
./scripts/typeduck-wasm-build.sh: line 130: cargo: command not found
```

**Impact**: Browser runtime cannot initialize without WASM artifact

#### Asset Configuration Blocker

**Patch placeholder** (`src/worker.ts` lines 246-251):
```typescript
const assetsConfig: ExplicitTypeDuckAssets = {
  defaultYaml: { type: "content", content: "" }, // Placeholder for E2E
  schemaYaml: { type: "content", content: "" },   // Placeholder for E2E
  dictionaryYaml: { type: "content", content: "" }, // Placeholder for E2E
};
```

**Impact**: Runtime init will fail with missing assets

**Resolution**: Provide explicit YAML assets per `e2e/assets/README.md`

#### Flow Execution Status (All BLOCKED)

| Flow | D-08/D-10/D-11 | Status | Blocker |
|------|----------------|--------|---------|
| Composition | Keys → preedit | BLOCKED | WASM missing |
| Candidate list | Visible | BLOCKED | WASM missing |
| Candidate paging | PageDown | BLOCKED | WASM missing |
| Candidate selection | Commit | BLOCKED | WASM missing |
| Deletion | Delete key | BLOCKED | WASM missing |
| Backspace mutation | Composition change | BLOCKED | WASM missing |
| Deploy | Success/error visible | BLOCKED | WASM missing |
| Customize | Success/error visible | BLOCKED | WASM missing |
| Persistence sync | sync-after-mutation | BLOCKED | WASM missing |
| Persistence reload | sync-before-init + reload | BLOCKED | WASM missing |

**Reason**: WASM artifact is prerequisite for all browser flows

#### Fallback Evidence

**Native fallback attempt** (per scripts/typeduck-wasm-build.sh):
```bash
Native fallback: cargo test -p yune-rime-api --test typeduck_web
Error: cargo: command not found
```

**Fallback BLOCKED**: Native tests also require cargo

**Evidence captured**:
- blocker.md — Documents missing cargo/rustup/emcc
- No browser-run.log (browser never ran)
- No screenshots (browser never ran)
- No persistence-sync.log (browser never ran)

#### Category Assignment (Per D-12)

**Environment/tooling** (primary blockers):
- cargo/rustup/emcc missing
- WASM artifact not built
- Native fallback blocked

**TypeDuck-Web app/source**:
- Asset configuration placeholder (needs explicit assets)

**Yune adapter/runtime**:
- Runtime JS built successfully (packages/yune-typeduck-runtime/dist/*.js)
- WASM artifact is Phase 7 build blocker (not adapter implementation)

#### Upstream Build Status

**Commands executed**:
- `bun install` — PASSED (Bun 1.3.11 available)
- `cp yune-integration/* source/src/yune-integration/` — PASSED (integration files copied)
- `git apply patches/yune-typeduck-runtime.patch` — PASSED (patch applied)
- `npm --prefix packages/yune-typeduck-runtime install typescript` — PASSED
- `npm --prefix packages/yune-typeduck-runtime run build` — PASSED (JS artifacts built)

**Commands blocked**:
- `./scripts/typeduck-wasm-build.sh` — BLOCKED (cargo missing)
- Playwright browser tests — BLOCKED (WASM artifact missing)
- Manual browser smoke — BLOCKED (WASM artifact missing)

#### Recommendation

**For Plan 10-04**:
1. Build WASM artifact in environment with cargo/rustup/emcc
2. Provide explicit TypeDuck-Web YAML assets
3. Run browser E2E spec or manual smoke
4. Update selectors based on actual TypeDuck-Web UI
5. Add persistence markers in Yune adapter for D-11 verification

**Per D-09**: Blocker documented with exact commands, missing dependencies, install hints, and fallback evidence. Browser E2E BLOCKED, not silently skipped.

---

*Updated: 2026-05-05T00:30:00Z*

---

## Final Phase 10 Evidence Summary

**Generated**: 2026-05-05T16:38:00Z
**Status**: Phase complete with blockers documented for WASM artifact generation

### Upstream Source and Seam

**Repository**: https://github.com/TypeDuck-HK/TypeDuck-Web.git
**Revision**: 03f9afd2cf6ca75653197f2193f24d1cd0adbd83 (main branch)
**Clone path**: third_party/typeduck-web/source
**Setup command**: bun install

**Seam files identified** (from 10-01):
- src/worker.ts — Primary replacement seam (Module.ccall → Yune adapter)
- src/rime.ts — Main-thread worker queue (preserve facade)
- src/types.ts — Actions and RimeResult interface (preserve contract)
- wasm/api.cpp — Librime C++ bridge (replaced by Yune native exports)
- scripts/build_wasm.ts — Emscripten build script (replaced by Yune WASM build)
- src/CandidatePanel.tsx — Keyboard event handling (preserve UI)

**Original librime/WASM call path**:
```
UI keyboard event → Rime.processKey(string) → Worker queue → Module.ccall("process_key") →
Emscripten Module → api.cpp::process_key(const char*) → librime::simulate_key_sequence →
RIME session → JSON result → Worker parse → Main thread render
```

### Yune Seam Patch Summary

**Patch file**: third_party/typeduck-web/patches/yune-typeduck-runtime.patch

**Minimal scope** (per D-03):
- package.json — Yune package alias dependency (@yune-ime/typeduck-runtime)
- src/worker.ts — Import Yune adapter, replace ccall with adapter exports, load Yune WASM artifact
- src/yune-integration/ — Integration layer (adapter.ts, assets.ts, README.md, package-alias.md)

**Contract mismatches addressed**:
1. String input → keycode/mask: Adapter parses `{BackSpace}` sequences to keyboard event-like objects
2. RimeResult vs TypeDuckResponse: Adapter translates Yune response to upstream shape
3. Persistence timing: Yune helpers match upstream sync boundaries (before init, after mutation)
4. setOption: HR-2 forwards option toggles through the TypeDuck runtime

**Build gates passed** (from 10-02):
- Repository runtime: npm build PASSED
- Upstream package install: Bun 1.3.11 PASSED
- Upstream worker build: esbuild PASSED (3.4kb output)
- TypeScript typecheck: PASSED (patched files error-free)

### E2E Behavior Matrix

**Browser E2E spec**: third_party/typeduck-web/e2e/yune-typeduck.spec.ts

**Coverage** (per D-08/TYPEDUCK-E2E-03):
1. Composition after schema-valid keys
2. Candidate list visible
3. Candidate paging (PageDown)
4. Candidate selection → commit
5. Deletion (Delete key)
6. Backspace mutation
7. Deploy returns success/error
8. Customize returns success/error
9. Persistence sync after mutation (D-11)
10. Persistence reload/reinitialize (D-11)

HR-5 also records phrase-commit regressions from the `ngohaigo` browser bug and
the later `ngohaig` partial-final sentence-completion bug as additional proof
rows.

**Flow status** (from HR-5 real-assets browser execution, 2026-06-18):

| Flow | D-08/D-10/D-11 Requirement | Status | Evidence/Blocker |
|------|----------------------------|--------|------------------|
| Composition | Schema-valid keys -> preedit visible | PASS | `hr5-final-nei-state.json`, `hr5-final-nei.png` show `nei` preedit with real candidates |
| Candidate list | Visible after composition | PASS | `hr5-final-nei-state.json` shows real `nei` candidates and no echo placeholder path |
| Candidate paging | PageDown -> page change | PASS | `hr5-final-paging-state.json`, before/after screenshots show `ngo` page advances |
| Candidate selection | Selection key -> commit text | PASS | `hr5-final-selection-space-state.json` records Space committing the highlighted candidate; number key `1` is not expected for this mobile schema because `alternative_select_keys` is the NUL sentinel |
| Phrase commit | Multi-syllable input -> phrase output | PASS | `hr5-final-phrase-state.json` records `ngohaigo` + Space committing `我係個` |
| Partial-final sentence completion | Multi-syllable prefix input -> completed phrase output | PASS | `hr5-final-ngohaig-state.json` records `ngohaig` rendering `1. 我係個` and Space committing `我係個` with no runtime-error banner or warning/error logs |
| Deletion | Delete key -> candidate/composition change | PASS | `hr5-final-delete-state.json` shows `{Control+Delete}` removes the first `ngo` candidate after the `{Control_L}` banner fix; no runtime-error banner is present |
| Backspace mutation | Backspace -> composition shorter/changed | PASS | `hr5-final-backspace-state.json`, `hr5-final-backspace.png` show `ngo` mutating to `ng` with no runtime-error banner |
| Deploy | Deploy action -> visible success/error | PASS | `hr5-final-diagnostics-before-reload.json` records live-worker deploy success diagnostics |
| Customize | Customize action -> visible success/error | PASS | `hr5-final-diagnostics-before-reload.json` records live-worker customize success diagnostics |
| Persistence sync | sync-after-mutation marker | PASS | `hr5-final-diagnostics-before-reload.json` records after-mutation sync markers for customize/deploy |
| Persistence reload | sync-before-init + reload/reinitialize | PASS | `hr5-final-reload-state.json`, `hr5-final-reload.png` prove a real reload restores `/rime/jyut6ping3_mobile.custom.yaml` before runtime init |
| Dictionary-panel comment rendering | v1.1.2 candidate comment bytes render | PASS | `hr5-final-nei-state.json` shows rendered dictionary-panel fields; `cantonese_parity` is the committed byte-parity guarantee, while `typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments` byte-asserts the browser-shaped rich path only when local oracle build assets are present |

**Reason**: The WASM/browser initialization blocker is cleared, real assets now
render real Chinese candidates, `setOption` no longer errors, deploy returns
true with the real workspace assets, live persistence sync plus reload survival
are proven, and HR-5 captures PASS evidence for the formerly open
paging/deletion/dictionary-comment rows. HR-6 now covers the reverse-lookup
joiner and prompt oracle cases, with five broader Cantonese goldens documented
as explicit capture blockers. HR-7 closes M9 with a GO WITH CONDITIONS
recommendation for gated AI-native frontend exposure.

**Evidence captured**:
- `third_party/typeduck-web/e2e/results/hr5-real-assets-matrix.log`
- `third_party/typeduck-web/e2e/results/hr5-final-nei-state.json`
- `third_party/typeduck-web/e2e/results/hr5-final-nei.png`
- `third_party/typeduck-web/e2e/results/hr5-final-paging-state.json`
- `third_party/typeduck-web/e2e/results/hr5-final-paging-before.png`
- `third_party/typeduck-web/e2e/results/hr5-final-paging-after.png`
- `third_party/typeduck-web/e2e/results/hr5-final-selection-space-state.json`
- `third_party/typeduck-web/e2e/results/hr5-final-selection-space-before.png`
- `third_party/typeduck-web/e2e/results/hr5-final-selection-space-after.png`
- `third_party/typeduck-web/e2e/results/hr5-final-phrase-state.json`
- `third_party/typeduck-web/e2e/results/hr5-final-phrase-before.png`
- `third_party/typeduck-web/e2e/results/hr5-final-phrase-after.png`
- `third_party/typeduck-web/e2e/results/hr5-final-delete-state.json`
- `third_party/typeduck-web/e2e/results/hr5-final-delete-before.png`
- `third_party/typeduck-web/e2e/results/hr5-final-delete-after.png`
- `third_party/typeduck-web/e2e/results/hr5-final-backspace-state.json`
- `third_party/typeduck-web/e2e/results/hr5-final-backspace.png`
- `third_party/typeduck-web/e2e/results/hr5-final-diagnostics-before-reload.json`
- `third_party/typeduck-web/e2e/results/hr5-final-reload-state.json`
- `third_party/typeduck-web/e2e/results/hr5-final-reload.png`
- `third_party/typeduck-web/e2e/results/browser-run.log`
- `third_party/typeduck-web/e2e/results/browser-console.json`
- `third_party/typeduck-web/e2e/results/dom-snapshot-candidates.txt`
- `third_party/typeduck-web/e2e/results/screenshot-real-assets-nei.png`
- `third_party/typeduck-web/e2e/results/persistence-sync.log`
- `third_party/typeduck-web/e2e/results/deploy-browser.log`
- `third_party/typeduck-web/e2e/results/blocker.md`

---

## Final blocker taxonomy

Phase 10 blockers categorized per D-12 with status, evidence, affected requirement, and AI-native frontend exposure impact.

### TypeDuck-Web app/source blockers

| Blocker | Status | Evidence | Affected Requirement | Blocks AI-native frontend? |
|---------|--------|----------|----------------------|---------------------------|
| Candidate DOM nesting warning | resolved | HR-5 patched `Candidate.tsx`/`DictionaryPanel.tsx` table markup and captured zero warning/error console entries in `hr5-real-assets-matrix.json` | TYPEDUCK-E2E-03 | NO |
| Browser reload evidence gap | resolved | HR-4 `persistence-sync.log` | TYPEDUCK-E2E-03 | NO — reload survival is browser-proven |

**Explanation**: The app now loads explicit assets and the generated
`yune-typeduck.js` / `.wasm` artifact. The HR-5 patch also removes the invalid
candidate/dictionary table markup warnings from the browser evidence.

### Yune adapter/runtime mismatches

| Blocker | Status | Evidence | Affected Requirement | Blocks AI-native frontend? |
|---------|--------|----------|----------------------|---------------------------|
| Dictionary comment oracle gap | resolved | HR-5 browser evidence renders dictionary-panel fields; `cantonese_parity` is the committed clean-checkout byte guarantee; the browser-shaped native real-assets test byte-asserts the first `nei` rich comment only when local v1.1.2 oracle build assets are present, otherwise it emits an explicit skip reason; HR-6 locks reverse-lookup `"; "` joining and schema-prompt bytes against a v1.1.2 fixture | TYPEDUCK-E2E-03, WI-6 | NO |
| `{Control_L}` pre-delete runtime-error banner | resolved | Adapter now ignores pure modifier keydowns before modified key chords; `hr5-final-delete-state.json` recaptures `{Control+Delete}` deleting the first `ngo` candidate with no visible runtime-error banner | TYPEDUCK-E2E-03 | NO |
| Candidate paging/deletion real-assets evidence | resolved | HR-5 `hr5-real-assets-matrix.log` and screenshots prove paging and `{Control+Delete}` deletion with real assets; post-review clean recapture proves no banner remains | TYPEDUCK-E2E-03 | NO |
| persistence sync/reload evidence | resolved | HR-4 `persistence-sync.log`, adapter diagnostic test | TYPEDUCK-E2E-03 | NO — live worker and real reload path are proven |
| setOption startup evidence | resolved | `set-option-browser.log`, native/runtime/adapter tests | D-07, TYPEDUCK-E2E-03 | NO — startup option toggles no longer throw |
| deploy returns false | resolved | `deploy-browser.log`, native browser-shaped asset tests | TYPEDUCK-E2E-03 | NO — deploy returns true with real assets |

**Explanation**: Adapter shape bugs are fixed, `setOption` and deploy now pass in
the browser, the core composition -> candidate -> commit seam works, and HR-5
closes the real-assets paging/deletion/dictionary-comment browser matrix gaps.

### Environment/tooling blockers

| Blocker | Status | Evidence | Affected Requirement | Blocks AI-native frontend? |
|---------|--------|----------|----------------------|---------------------------|
| Full-matrix screenshot coverage pending | resolved | HR-5 captured `hr5-final-*.png`, `hr5-final-*-state.json`, and `hr5-real-assets-matrix.log` | WI-4/HR-5 evidence | NO |

**Explanation**: Cargo, rustup, Emscripten, and the loadable WASM/JS artifact are
available locally. The real-assets browser matrix executed and its evidence is
captured in the repo.

---

**Total current blockers**: 5 non-browser Cantonese parity capture blockers
left as explicit ignored tests. The HR-5 real-browser matrix and post-review
delete/backspace recaptures captured zero warning/error entries after the
TypeDuck-Web DOM nesting and modifier-key fixes.

**Blocking AI-native frontend exposure**: HR-5 leaves no browser-matrix blocker
for composition/candidates/paging/selection/deletion/phrase commit/deploy/
persistence/reload/dictionary comments. Exposure is GO WITH CONDITIONS: keep
AI-native behavior disabled by default in real frontends until the separate M11
provider/ranking/privacy contracts are proven and explicitly enabled.

---

*Findings consolidated: 2026-06-18*
*Phase: 17 / M9 (TypeDuck-Web Browser Validation)*
*Requirement coverage: TYPEDUCK-E2E-01, TYPEDUCK-E2E-02, TYPEDUCK-E2E-03, TYPEDUCK-E2E-04*
