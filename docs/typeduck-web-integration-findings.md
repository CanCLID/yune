# TypeDuck-Web Integration Findings

This document records findings from integrating Yune with the upstream TypeDuck-Web browser application.

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
*Updated: 2026-05-05T15:10:00Z*