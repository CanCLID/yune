# Phase 8: TypeScript Bridge And Runtime Package - Research

**Researched:** 2026-05-04 [VERIFIED: system current date]
**Domain:** TypeScript runtime wrapper for the Yune TypeDuck Emscripten C/WASM adapter [VERIFIED: 08-CONTEXT.md]
**Confidence:** HIGH for adapter contract, response ownership, lifecycle, and key constants; MEDIUM for exact package path because the repository has no existing JavaScript package convention [VERIFIED: 08-CONTEXT.md; VERIFIED: .planning/codebase/STACK.md]

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
## Implementation Decisions

### Wrapper Location And Tooling
- **D-01:** Add a minimal repository-owned TypeScript bridge package rather than embedding TypeScript snippets only in docs. The package should be small enough to test deterministically in Node without requiring a real Emscripten browser build.
- **D-02:** Because the repo currently has no JS/TS tooling files, Phase 8 may introduce the smallest necessary TypeScript test/build setup for the wrapper. Avoid broad frontend app scaffolding, bundler config for TypeDuck-Web, or browser E2E infrastructure.
- **D-03:** Keep the TypeScript bridge adapter-shaped and independent from `RimeApi`; it wraps only the Phase 7 `yune_typeduck_*` export list and must not expose librime-shaped function-table details to browser callers.

### TypeScript API Shape
- **D-04:** The wrapper should expose typed operations for `init`, `processKey`, `selectCandidate`, `deleteCandidate`, `flipPage`, `deploy`, `customize`, and `cleanup`, matching Phase 8 requirements TYPEDUCK-JS-01 through TYPEDUCK-JS-04.
- **D-05:** Treat the Emscripten `Module` object as an injected dependency with a narrow typed interface (`cwrap`/`UTF8ToString` or equivalent). Tests should use a fake module so Phase 8 does not depend on local Emscripten output or upstream TypeDuck-Web source.
- **D-06:** State pointers and response pointers should remain opaque numbers at the JS boundary. The wrapper may return small TypeScript classes/objects around those pointers, but callers should not manipulate raw C pointer lifetimes directly except through the wrapper contract.

### Response Ownership And JSON Parsing
- **D-07:** Centralize all response handling in one wrapper path: call the adapter operation, read `yune_typeduck_response_json`, copy/parse the JSON string, read handled state if needed, and always call `yune_typeduck_free_response` exactly once for non-null owned responses.
- **D-08:** Null response pointers and null JSON pointers are wrapper-level errors. The TypeScript layer should surface deterministic errors rather than fabricating empty candidate lists, because missing assets and lifecycle problems must stay visible before Phase 9 recovery work.
- **D-09:** Define TypeScript response types from the documented JSON response shape in `docs/typeduck-web-adapter.md`: `handled`, `commits`, optional/nullable `context`, optional/nullable `status`, and optional `error`. Keep parsing permissive enough for nullable context/status but strict enough to catch non-object or malformed JSON.

### Browser Key Mapping
- **D-10:** Keycode/mask mapping must be explicit in Phase 8 and covered by deterministic tests. The mapping should convert browser `KeyboardEvent`-like inputs into the integer keycode/mask pair passed to `yune_typeduck_process_key`.
- **D-11:** Start with the key paths needed by TypeDuck-Web integration: printable character keys, Enter/Backspace/Escape/Space, arrow keys, PageUp/PageDown, number selection keys, and common modifier masks. Avoid trying to model every platform/browser edge case before Phase 10 observes the real app seam.
- **D-12:** Keep the mapping function independently testable without DOM globals by accepting a narrow event-like object instead of requiring real `KeyboardEvent` instances.

### Runtime Lifecycle Contract
- **D-13:** The wrapper must make the one-active-process-global Yune/RIME service constraint visible. `cleanup` finalizes process-global service state through `yune_typeduck_cleanup`; callers should not create multiple simultaneous states with different dirs in one Module instance.
- **D-14:** The wrapper should guard against obvious misuse within one wrapper instance: operations after cleanup should fail deterministically, cleanup should be idempotent at the TypeScript layer, and init failure should throw or return a typed failure before any state object is exposed.
- **D-15:** Browser filesystem setup remains host responsibility. The wrapper can accept `sharedDataDir`, `userDataDir`, and `schemaId`, but it must not mount IDBFS, preload assets, sync persistence, or hide missing-asset failures in Phase 8.

### Claude's Discretion
- Choose exact package path, file names, and JS test runner during planning, as long as the result is minimal, deterministic, and does not turn Phase 8 into a browser app or TypeDuck-Web checkout.
- Prefer a fake Emscripten Module contract in tests over installing Emscripten or building WASM during Phase 8.
- Prefer small typed interfaces and focused tests over large generated bindings. Generated or build-system-specific TypeDuck-Web integration remains Phase 10.

### Deferred Ideas (OUT OF SCOPE)
## Deferred Ideas

- Browser virtual filesystem layout creation, schema/dictionary asset preload, IDBFS sync, missing asset recovery, and stale deployed config recovery remain Phase 9.
- Cloning upstream TypeDuck-Web, identifying its current librime/WASM bridge seam, patching it to use Yune, and running real browser flows remain Phase 10.
- Multi-instance isolation beyond one active process-global Yune/RIME service remains out of scope for this milestone unless a later TypeDuck-Web integration blocker requires it.
- AI-native provider, ranking, context, memory, privacy, and frontend exposure behavior remains deferred until TypeDuck-Web integration produces a go/no-go recommendation.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TYPEDUCK-JS-01 | A TypeScript wrapper exposes init, process-key, candidate action, deploy, customize, and cleanup operations. [VERIFIED: .planning/REQUIREMENTS.md] | Bind the 11 canonical `yune_typeduck_*` symbols and expose a typed wrapper class/object with only TypeDuck-shaped operations. [VERIFIED: scripts/typeduck-exports.txt; VERIFIED: typeduck_web.rs] |
| TYPEDUCK-JS-02 | The wrapper centralizes JSON parsing and pairs every owned adapter response with `yune_typeduck_free_response`. [VERIFIED: .planning/REQUIREMENTS.md] | Implement one `readResponse`/`withResponse` helper using `try/finally`, and test free-on-success plus free-on-parse-error. [VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: typeduck_web.rs tests] |
| TYPEDUCK-JS-03 | Browser keycode/mask mapping is explicit and covered by deterministic tests. [VERIFIED: .planning/REQUIREMENTS.md] | Mirror the key constants and modifier bit positions from `key_table.rs`; map from `KeyboardEvent.key`, not deprecated `keyCode`. [VERIFIED: key_table.rs; CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key; CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode] |
| TYPEDUCK-JS-04 | Runtime lifecycle documentation makes the one-active-process-global-service constraint visible to TypeDuck-Web callers. [VERIFIED: .planning/REQUIREMENTS.md] | Document one active state per Module instance, host-owned MEMFS/IDBFS, init-time asset failures, cleanup finalization, and wrapper misuse guards. [VERIFIED: 08-CONTEXT.md; VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: typeduck_web.rs] |
</phase_requirements>

## Summary

Phase 8 should create a minimal TypeScript runtime package that wraps the Phase 7 TypeDuck C/WASM adapter, not the librime-shaped `RimeApi` function table. [VERIFIED: 08-CONTEXT.md; VERIFIED: scripts/typeduck-exports.txt] The wrapper should accept an injected Emscripten Module with `cwrap` and `UTF8ToString`, bind only the 11 `yune_typeduck_*` symbols, and expose typed operations for init, key processing, candidate selection/deletion, paging, deploy, customize, and cleanup. [VERIFIED: 08-CONTEXT.md; VERIFIED: typeduck_web.rs; CITED: emscripten.org/docs/api_reference/preamble.js.html]

The critical correctness property is response ownership. [VERIFIED: docs/typeduck-web-adapter.md] Rust response-producing operations allocate an owned `YuneTypeDuckResponse`, `yune_typeduck_response_json` returns the JSON C string pointer inside that response, and JS must copy the string before calling `yune_typeduck_free_response`. [VERIFIED: typeduck_web.rs; VERIFIED: docs/typeduck-web-adapter.md] Therefore the wrapper must not expose response pointers to application code; every response-producing operation must route through one helper that frees exactly once for every non-null response pointer, including malformed JSON and validation-error paths. [VERIFIED: 08-CONTEXT.md; VERIFIED: typeduck_web.rs tests; ASSUMED]

The second critical property is lifecycle visibility. [VERIFIED: 08-CONTEXT.md] `yune_typeduck_cleanup` consumes the raw state pointer and finalizes process-global RIME service state, while Phase 8 explicitly does not implement multi-instance isolation, MEMFS/IDBFS setup, asset preload, persistence sync, TypeDuck-Web app patching, browser E2E, or AI-native behavior. [VERIFIED: typeduck_web.rs; VERIFIED: 08-CONTEXT.md; VERIFIED: docs/typeduck-web-adapter.md]

**Primary recommendation:** Add `packages/yune-typeduck-runtime/` as a small private TypeScript package with strict TS compilation, Vitest fake-Module tests, an injected `EmscriptenTypeDuckModule` interface, one centralized response reader/freeing helper, explicit RIME key constants/mask mapping, and documentation that preserves the one-active-service and host-owned-filesystem constraints. [ASSUMED; VERIFIED: 08-CONTEXT.md; VERIFIED: npm registry; CITED: www.typescriptlang.org/tsconfig/; CITED: vitest.dev/guide/]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Bind `yune_typeduck_*` exports | Browser / Client | API / Backend (`yune-rime-api`) | The TypeScript wrapper runs in browser/JS host code and binds the Rust adapter symbols compiled by Emscripten. [VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: typeduck_web.rs] |
| Parse/free adapter responses | Browser / Client | API / Backend (`YuneTypeDuckResponse`) | Rust owns response allocation, while JavaScript must copy JSON and free each non-null response pointer once. [VERIFIED: typeduck_web.rs; VERIFIED: docs/typeduck-web-adapter.md] |
| Browser key mapping | Browser / Client | API / Backend key table | Browser events enter at the JS tier, but the adapter expects RIME/X11 integer keycode and mask values. [VERIFIED: key_table.rs; VERIFIED: typeduck_web.rs] |
| Lifecycle misuse guard | Browser / Client | API / Backend process-global runtime | Rust cleanup finalizes process-global state; TypeScript can make cleanup idempotent and reject calls after cleanup within one wrapper instance. [VERIFIED: typeduck_web.rs; VERIFIED: docs/typeduck-web-adapter.md] |
| Virtual filesystem and persistence | Browser / Client | Storage / IndexedDB | Phase 8 exposes paths but leaves MEMFS/IDBFS mounting, asset preload, and sync orchestration to Phase 9 host code. [VERIFIED: 08-CONTEXT.md; VERIFIED: docs/typeduck-web-adapter.md] |
| RIME session/schema behavior | API / Backend | Browser / Client wrapper | `yune-rime-api` owns init, deploy, schema selection, session operations, JSON serialization, and cleanup semantics. [VERIFIED: typeduck_web.rs] |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| TypeScript | 6.0.3; npm modified 2026-04-16T23:38:57.055Z | Compile the wrapper and emit declaration files. | TypeScript supports strict checking and declaration emit for library APIs. [VERIFIED: npm registry; CITED: www.typescriptlang.org/tsconfig/] |
| Vitest | 4.1.5; npm modified 2026-04-23T10:30:30.524Z | Run deterministic wrapper, fake Module, response ownership, lifecycle, and key mapping tests in Node. | Vitest is a maintained test framework with package-script execution and `vitest run`. [VERIFIED: npm registry; CITED: vitest.dev/guide/] |
| Emscripten Module runtime methods | Generated by Emscripten output; no npm package | Provide `cwrap` for C function wrappers and `UTF8ToString` for heap UTF-8 string copying. | Emscripten officially documents `cwrap`, `ccall`, `UTF8ToString`, `EXPORTED_FUNCTIONS`, and `EXPORTED_RUNTIME_METHODS`. [CITED: emscripten.org/docs/api_reference/preamble.js.html] |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| @types/node | 24.10.15 latest observed for Node 24 major; npm modified 2026-04-10T03:40:44.753Z | Node type definitions for package-local tests or scripts if Node globals are referenced. | Use only for tests/build tooling; wrapper runtime source should remain browser-compatible and should not depend on Node APIs. [VERIFIED: npm registry; ASSUMED] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| TypeScript package | Docs-only JS snippets | Rejected because D-01 requires a repository-owned package and deterministic tests. [VERIFIED: 08-CONTEXT.md] |
| Vitest fake-Module tests | Real Emscripten/browser tests | Rejected for Phase 8 because D-05 prefers fake modules; Phase 10 owns real TypeDuck-Web/browser flow validation. [VERIFIED: 08-CONTEXT.md; VERIFIED: ROADMAP.md] |
| Emscripten `cwrap` wrapper | wasm-bindgen-generated wrapper | Rejected because Phase 7 locked `wasm32-unknown-emscripten` plus Emscripten export flags as the browser build contract. [VERIFIED: 07-CONTEXT.md; VERIFIED: docs/typeduck-web-adapter.md] |
| `RimeApi` bindings | TypeDuck-shaped `yune_typeduck_*` bindings | Rejected because D-03 forbids exposing librime-shaped function-table details to browser callers. [VERIFIED: 08-CONTEXT.md] |

**Installation:**
```bash
npm install --save-dev typescript@6.0.3 vitest@4.1.5 @types/node@24.10.15
```
[VERIFIED: npm registry]

**Version verification:** Package versions and publish metadata were checked with `npm view typescript version time.modified engines --json`, `npm view vitest version time.modified engines --json`, and `npm view @types/node@24 version time.modified engines --json`. [VERIFIED: npm registry]

## Architecture Patterns

### System Architecture Diagram

```text
Browser caller / KeyboardEvent-like input
        |
        v
TypeDuck TS runtime wrapper
        |
        +--> keyEventToRimeKey(event) -> { keycode, mask }
        |
        +--> lifecycle guard -> live opaque state pointer or deterministic error
        |
        v
Injected Emscripten Module
        |
        +--> cwrap('yune_typeduck_*', ...)
        +--> UTF8ToString(jsonPtr)
        |
        v
Emscripten-exported Rust adapter symbols
        |
        v
`yune-rime-api` process-global runtime + session
        |
        v
Owned YuneTypeDuckResponse pointer
        |
        v
readResponse / withResponse
        |
        +--> null response? throw
        +--> response_json(response) -> null? throw
        +--> UTF8ToString(jsonPtr) -> JSON.parse -> shape validation
        +--> finally free_response(response)
        |
        v
Typed TypeDuckResponse to browser caller
```
[VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: typeduck_web.rs; CITED: emscripten.org/docs/api_reference/preamble.js.html]

### Recommended Project Structure

```text
packages/
└── yune-typeduck-runtime/          # Minimal TypeScript bridge package [ASSUMED]
    ├── package.json                # Private package metadata, build/test scripts, dev deps [ASSUMED]
    ├── tsconfig.json               # strict, declaration, rootDir src, outDir dist [CITED: www.typescriptlang.org/tsconfig/]
    ├── src/
    │   ├── index.ts                # Public exports [ASSUMED]
    │   ├── module.ts               # Emscripten Module interface and cwrap bindings [CITED: emscripten.org/docs/api_reference/preamble.js.html]
    │   ├── response.ts             # Response types, parser, read/free helper [VERIFIED: docs/typeduck-web-adapter.md]
    │   ├── typeduck.ts             # Runtime wrapper and lifecycle guard [VERIFIED: 08-CONTEXT.md]
    │   └── keys.ts                 # RIME keycode/mask constants and mapper [VERIFIED: key_table.rs]
    └── test/
        ├── fake-module.ts          # Fake Emscripten Module [VERIFIED: 08-CONTEXT.md]
        ├── response.test.ts        # Response parse/free/null/malformed tests [VERIFIED: typeduck_web.rs tests]
        ├── typeduck.test.ts        # Init, operation, Bool operation, cleanup tests [VERIFIED: typeduck_web.rs]
        └── keys.test.ts            # Browser event-like key mapping tests [VERIFIED: key_table.rs]
```

### Pattern 1: Narrow Emscripten Module Injection
**What:** Accept only the Emscripten methods the wrapper needs: `cwrap` and `UTF8ToString`. [VERIFIED: 08-CONTEXT.md; CITED: emscripten.org/docs/api_reference/preamble.js.html]
**When to use:** Use when binding generated Emscripten output after the runtime is ready; `cwrap` itself can be called before runtime initialization, but calling the returned wrapper must wait until runtime readiness. [CITED: emscripten.org/docs/api_reference/preamble.js.html]
**Example:**
```typescript
// Source: Emscripten docs [CITED: emscripten.org/docs/api_reference/preamble.js.html]
export type EmscriptenCType = 'number' | 'string' | 'boolean' | 'array' | null;

export interface EmscriptenTypeDuckModule {
  cwrap(
    ident: string,
    returnType: EmscriptenCType,
    argTypes: EmscriptenCType[],
  ): (...args: unknown[]) => unknown;
  UTF8ToString(ptr: number, maxBytesToRead?: number, ignoreNul?: boolean): string;
}
```

### Pattern 2: Bind Only Canonical Adapter Symbols
**What:** Bind exactly the symbols in `scripts/typeduck-exports.txt`. [VERIFIED: scripts/typeduck-exports.txt]
**When to use:** Use during wrapper construction; fail fast if any required binding is absent or not callable. [ASSUMED]
**Example:**
```typescript
// Source: scripts/typeduck-exports.txt and typeduck_web.rs signatures [VERIFIED: scripts/typeduck-exports.txt; VERIFIED: typeduck_web.rs]
export const TYPEDUCK_EXPORTS = [
  'yune_typeduck_init',
  'yune_typeduck_process_key',
  'yune_typeduck_select_candidate',
  'yune_typeduck_delete_candidate',
  'yune_typeduck_flip_page',
  'yune_typeduck_deploy',
  'yune_typeduck_customize',
  'yune_typeduck_cleanup',
  'yune_typeduck_response_json',
  'yune_typeduck_response_handled',
  'yune_typeduck_free_response',
] as const;
```

### Pattern 3: Central Response Ownership Helper
**What:** One helper reads, parses, validates, and frees every non-null response pointer. [VERIFIED: 08-CONTEXT.md; VERIFIED: typeduck_web.rs tests]
**When to use:** Use for `processKey`, `selectCandidate`, `deleteCandidate`, and `flipPage`; do not use for `deploy` and `customize` because those return `Bool`. [VERIFIED: typeduck_web.rs]
**Example:**
```typescript
// Source: docs/typeduck-web-adapter.md suggested JS flow and Rust test helper [VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: crates/yune-rime-api/tests/typeduck_web.rs]
function readResponse(responsePtr: number, bindings: TypeDuckBindings): TypeDuckResponse {
  if (responsePtr === 0) {
    throw new TypeDuckResponseError('TypeDuck adapter returned null response');
  }
  try {
    const jsonPtr = bindings.responseJson(responsePtr);
    if (jsonPtr === 0) {
      throw new TypeDuckResponseError('TypeDuck adapter returned null response JSON');
    }
    const text = bindings.module.UTF8ToString(jsonPtr);
    const parsed = JSON.parse(text) as unknown;
    return parseTypeDuckResponse(parsed, bindings.responseHandled(responsePtr) !== 0);
  } finally {
    bindings.freeResponse(responsePtr);
  }
}
```

### Pattern 4: Wrapper-Owned Lifecycle Guard
**What:** Hide raw state pointers behind a wrapper object; make cleanup idempotent; reject all other calls after cleanup. [VERIFIED: 08-CONTEXT.md]
**When to use:** Use for all initialized runtime objects because `cleanup` consumes the raw pointer via Rust `Box::from_raw`. [VERIFIED: typeduck_web.rs]
**Example:**
```typescript
// Source: typeduck_web.rs cleanup ownership [VERIFIED: typeduck_web.rs]
export class TypeDuckRuntime {
  private statePtr: number;
  private cleanedUp = false;

  cleanup(): void {
    if (this.cleanedUp) return;
    this.cleanedUp = true;
    const ptr = this.statePtr;
    this.statePtr = 0;
    this.bindings.cleanup(ptr);
  }

  private requireLiveState(): number {
    if (this.cleanedUp || this.statePtr === 0) {
      throw new TypeDuckLifecycleError('TypeDuck runtime has been cleaned up');
    }
    return this.statePtr;
  }
}
```

### Pattern 5: DOM-Free Key Mapping
**What:** Accept a small event-like type and map `event.key` to RIME keycodes plus modifier masks. [VERIFIED: 08-CONTEXT.md; VERIFIED: key_table.rs]
**When to use:** Use in browser adapters and Node tests without depending on DOM globals. [VERIFIED: 08-CONTEXT.md]
**Example:**
```typescript
// Source: key_table.rs and MDN KeyboardEvent.key [VERIFIED: key_table.rs; CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key]
export interface TypeDuckKeyboardEventLike {
  key: string;
  shiftKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
}

export const RIME_KEY = {
  Backspace: 0xff08,
  Tab: 0xff09,
  Enter: 0xff0d,
  Escape: 0xff1b,
  Delete: 0xffff,
  ArrowLeft: 0xff51,
  ArrowUp: 0xff52,
  ArrowRight: 0xff53,
  ArrowDown: 0xff54,
  PageUp: 0xff55,
  PageDown: 0xff56,
  Space: 0x20,
} as const;

export const RIME_MASK = {
  Shift: 1 << 0,
  Lock: 1 << 1,
  Control: 1 << 2,
  Alt: 1 << 3,
  Super: 1 << 26,
  Hyper: 1 << 27,
  Meta: 1 << 28,
  Release: 1 << 30,
} as const;
```

### Anti-Patterns to Avoid
- **Returning raw response pointers:** Application code must not own response lifetimes because every non-null owned response needs exactly one free. [VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: typeduck_web.rs]
- **Calling `cleanup` twice on the raw pointer:** Rust consumes the pointer with `Box::from_raw`; TypeScript must guard this. [VERIFIED: typeduck_web.rs]
- **Using `KeyboardEvent.keyCode`:** MDN marks `keyCode` deprecated and implementation-dependent; use `KeyboardEvent.key` for semantic key mapping. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode; CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key]
- **Exposing `RimeApi`:** Phase 8 must wrap only TypeDuck adapter symbols, not librime-shaped function-table details. [VERIFIED: 08-CONTEXT.md]
- **Pulling filesystem/E2E work forward:** Phase 8 must not mount IDBFS, preload assets, patch TypeDuck-Web, or add real browser E2E. [VERIFIED: 08-CONTEXT.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Type declarations | Handwritten `.d.ts` separate from implementation | TypeScript `declaration: true` | TypeScript officially emits `.d.ts` files from source. [CITED: www.typescriptlang.org/tsconfig/] |
| Unit test harness | Custom Node assertion scripts | Vitest | Vitest provides package-script test execution and `vitest run`. [CITED: vitest.dev/guide/] |
| Emscripten calls | Custom lookup/heap string machinery | `Module.cwrap` and `Module.UTF8ToString` | Emscripten documents `cwrap` for native JS wrappers and `UTF8ToString` for null-terminated UTF-8 heap strings. [CITED: emscripten.org/docs/api_reference/preamble.js.html] |
| Response ownership | Per-method parse/free code | One `readResponse`/`withResponse` helper | Centralization makes exact free pairing testable for success and error paths. [VERIFIED: 08-CONTEXT.md; VERIFIED: typeduck_web.rs tests; ASSUMED] |
| Keyboard mapping | Deprecated `keyCode` tables | `KeyboardEvent.key` to explicit RIME constants | `key` accounts for modifiers, locale, and layout; `keyCode` is deprecated and unreliable. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key; CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode] |
| Filesystem persistence | Wrapper-owned MEMFS/IDBFS policy | Phase 9 host orchestration | Phase 8 explicitly leaves asset preload and persistence sync to browser host work. [VERIFIED: 08-CONTEXT.md] |

**Key insight:** Phase 8 is an ownership-safe TypeScript facade over a fixed adapter contract, not a browser runtime platform; keep C pointer ownership, key mapping, and lifecycle misuse visible and testable. [VERIFIED: 08-CONTEXT.md; VERIFIED: typeduck_web.rs; ASSUMED]

## Common Pitfalls

### Pitfall 1: Leaking responses on malformed JSON
**What goes wrong:** A thrown `JSON.parse` or shape-validation error skips `yune_typeduck_free_response`. [VERIFIED: typeduck_web.rs; ASSUMED]
**Why it happens:** Response parsing is duplicated per operation instead of centralized in a `try/finally`. [ASSUMED]
**How to avoid:** Parse and validate inside a `try` whose `finally` always frees the non-null response pointer. [VERIFIED: typeduck_web.rs tests; ASSUMED]
**Warning signs:** Tests cover successful freeing but not malformed JSON freeing. [ASSUMED]

### Pitfall 2: Treating null pointers as empty responses
**What goes wrong:** Missing assets, null state, or lifecycle misuse becomes an apparently valid empty response. [VERIFIED: typeduck_web.rs; VERIFIED: 08-CONTEXT.md]
**Why it happens:** `free_response(NULL)` is a no-op and may make null paths look harmless. [VERIFIED: docs/typeduck-web-adapter.md]
**How to avoid:** Throw deterministic wrapper errors for null response and null JSON pointers. [VERIFIED: 08-CONTEXT.md]
**Warning signs:** Wrapper returns `{ handled: false, commits: [] }` when operation response is `0`. [ASSUMED]

### Pitfall 3: Double-cleaning a Rust state pointer
**What goes wrong:** Calling `yune_typeduck_cleanup` twice with the same pointer would pass freed memory back into Rust. [VERIFIED: typeduck_web.rs]
**Why it happens:** Browser teardown often has multiple cleanup paths. [ASSUMED]
**How to avoid:** Store cleanup state in TypeScript, zero the pointer before/while cleaning up, and make subsequent `cleanup()` calls no-ops. [VERIFIED: 08-CONTEXT.md; ASSUMED]
**Warning signs:** Raw state pointer is public or remains non-zero after cleanup. [ASSUMED]

### Pitfall 4: Runtime method/export mismatch
**What goes wrong:** `cwrap` or `UTF8ToString` is unavailable, or adapter functions are optimized away from generated Emscripten output. [CITED: emscripten.org/docs/api_reference/preamble.js.html]
**Why it happens:** External JS use requires `EXPORTED_RUNTIME_METHODS`, and callable C functions require `EXPORTED_FUNCTIONS`. [CITED: emscripten.org/docs/api_reference/preamble.js.html]
**How to avoid:** Keep docs and wrapper binding names aligned with Phase 7 export flags and `scripts/typeduck-exports.txt`. [VERIFIED: docs/typeduck-web-adapter.md; VERIFIED: scripts/typeduck-exports.txt]
**Warning signs:** Wrapper binds symbols not listed in `scripts/typeduck-exports.txt`. [VERIFIED: scripts/typeduck-exports.txt; ASSUMED]

### Pitfall 5: Deprecated keyboard APIs
**What goes wrong:** Browser, system, Shift, Alt, and layout differences produce wrong keycodes. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode]
**Why it happens:** `keyCode` is system/implementation-dependent and deprecated. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode]
**How to avoid:** Use `event.key` and explicit tested mapping to RIME constants. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key; VERIFIED: key_table.rs]
**Warning signs:** Tests construct events with only `keyCode`. [ASSUMED]

### Pitfall 6: Phase-scope creep
**What goes wrong:** Wrapper implementation starts handling MEMFS/IDBFS setup, browser persistence, TypeDuck-Web patching, or AI ranking. [VERIFIED: 08-CONTEXT.md]
**Why it happens:** Those features are adjacent to browser integration but assigned to Phase 9, Phase 10, or the future AI-native milestone. [VERIFIED: ROADMAP.md; VERIFIED: 08-CONTEXT.md]
**How to avoid:** Keep wrapper inputs to Module plus paths/schema ID; document host responsibilities instead of implementing them. [VERIFIED: 08-CONTEXT.md]
**Warning signs:** Phase 8 adds Playwright, clones TypeDuck-Web, mounts IDBFS, or adds provider/ranker APIs. [ASSUMED]

## Code Examples

### Typed Response Shape
```typescript
// Source: docs/typeduck-web-adapter.md JSON response shape [VERIFIED: docs/typeduck-web-adapter.md]
export interface TypeDuckCandidate {
  text: string;
  comment: string;
}

export interface TypeDuckContext {
  input: string;
  preedit: string;
  caret: number;
  highlighted: number;
  page_size: number;
  page_no: number;
  is_last_page: boolean;
  select_keys: string | null;
  select_labels: string[];
  candidates: TypeDuckCandidate[];
}

export interface TypeDuckStatus {
  schema_id: string;
  schema_name: string;
  is_disabled: boolean;
  is_composing: boolean;
  is_ascii_mode: boolean;
  is_full_shape: boolean;
  is_simplified: boolean;
  is_traditional: boolean;
  is_ascii_punct: boolean;
}

export interface TypeDuckResponse {
  handled: boolean;
  commits: string[];
  context: TypeDuckContext | null;
  status: TypeDuckStatus | null;
  error?: string;
}
```

### Public Wrapper API Shape
```typescript
// Source: 08-CONTEXT.md D-04 and typeduck_web.rs signatures [VERIFIED: 08-CONTEXT.md; VERIFIED: typeduck_web.rs]
export interface TypeDuckInitOptions {
  sharedDataDir: string;
  userDataDir: string;
  schemaId: string;
}

export interface RimeKey {
  keycode: number;
  mask: number;
}

export class TypeDuckRuntime {
  static init(module: EmscriptenTypeDuckModule, options: TypeDuckInitOptions): TypeDuckRuntime;
  processKey(keycode: number, mask?: number): TypeDuckResponse;
  processKeyboardEvent(event: TypeDuckKeyboardEventLike): TypeDuckResponse;
  selectCandidate(index: number): TypeDuckResponse;
  deleteCandidate(index: number): TypeDuckResponse;
  flipPage(backward?: boolean): TypeDuckResponse;
  deploy(): boolean;
  customize(configId: string, key: string, value: string): boolean;
  cleanup(): void;
}
```

### Fake Module Test Cases
```typescript
// Source: Vitest docs and Phase 8 fake-module decision [CITED: vitest.dev/guide/; VERIFIED: 08-CONTEXT.md]
import { describe, expect, it } from 'vitest';

it('frees response on malformed JSON', () => {
  const fake = new FakeTypeDuckModule();
  fake.queueResponse(101, 201, '{not json');
  const runtime = TypeDuckRuntime.init(fake.module, {
    sharedDataDir: '/rime/shared',
    userDataDir: '/rime/user',
    schemaId: 'typeduck_luna',
  });

  expect(() => runtime.processKey(0x61, 0)).toThrow(TypeDuckResponseError);
  expect(fake.freeResponseCalls).toEqual([101]);
});
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `KeyboardEvent.keyCode` | `KeyboardEvent.key` | Current MDN marks `keyCode` deprecated. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode] | Phase 8 should test `key`-based mapping. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key] |
| wasm-bindgen-first wrapper | Emscripten C ABI via `cwrap` | Phase 7 locked `wasm32-unknown-emscripten`. [VERIFIED: 07-CONTEXT.md] | Phase 8 must wrap Emscripten symbols, not wasm-bindgen classes. [VERIFIED: 08-CONTEXT.md] |
| Raw JS snippets | Tested TypeScript package | Phase 8 D-01 locked package ownership. [VERIFIED: 08-CONTEXT.md] | Planner must create package files/tests, not docs only. [VERIFIED: 08-CONTEXT.md] |
| Real browser tests for wrapper logic | Fake Module tests in Node | Phase 8 D-05 locked fake-module preference. [VERIFIED: 08-CONTEXT.md] | Wrapper tests do not require Emscripten or TypeDuck-Web source. [VERIFIED: 08-CONTEXT.md] |

**Deprecated/outdated:**
- `KeyboardEvent.keyCode`: deprecated and not recommended for new code. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode]
- Raw response pointer exposure to app callers: Phase 8 requires centralized response ownership. [VERIFIED: 08-CONTEXT.md]
- Wrapper-owned browser filesystem setup: Phase 8 defers this to Phase 9. [VERIFIED: 08-CONTEXT.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `packages/yune-typeduck-runtime/` is the best package path. | Summary / Standard Stack / Project Structure | Planner may need to choose another path if maintainers prefer root-level JS tooling or a different package layout. |
| A2 | `@types/node` should be installed only if tests/scripts need Node globals. | Standard Stack | If Vitest config requires Node globals, planner should include it; if not, omit it. |
| A3 | The central helper should be named `readResponse` or `withResponse`. | Architecture Patterns / Code Examples | Helper name can change without affecting required ownership semantics. |
| A4 | Private package metadata is preferable initially. | Open Questions | If publication is intended, package naming and export maps need user confirmation. |

## Open Questions

1. **Package name and publication intent**
   - What we know: Phase 8 requires a repository-owned TypeScript package. [VERIFIED: 08-CONTEXT.md]
   - What's unclear: No source states the desired npm package name or publication policy. [VERIFIED: 08-CONTEXT.md]
   - Recommendation: Start with private package metadata and avoid publication-specific decisions until requested. [ASSUMED]

2. **Exact package path**
   - What we know: The repo currently has no existing JS package convention. [VERIFIED: .planning/codebase/STACK.md]
   - What's unclear: Maintainer preference between `packages/`, `crates/yune-rime-api/js/`, or another path is not documented. [VERIFIED: 08-CONTEXT.md]
   - Recommendation: Use `packages/yune-typeduck-runtime/` because it isolates JS package tooling from Rust crates. [ASSUMED]

3. **Runtime readiness handoff**
   - What we know: Emscripten docs state that `cwrap` can be called before runtime initialization but calling wrapped functions must wait until runtime is ready. [CITED: emscripten.org/docs/api_reference/preamble.js.html]
   - What's unclear: Phase 8 context does not define whether wrapper construction itself should enforce Emscripten readiness or assume caller constructs after readiness. [VERIFIED: 08-CONTEXT.md]
   - Recommendation: Document that callers construct/init the wrapper after the Emscripten Module is ready; do not add browser-specific readiness orchestration in Phase 8. [ASSUMED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | TypeScript/Vitest package tests | yes [VERIFIED: local command] | v24.14.1 [VERIFIED: local command] | None needed |
| npm | Installing/running TypeScript package scripts | yes [VERIFIED: local command] | 11.11.0 [VERIFIED: local command] | None needed |
| npx | Context7 CLI and optional package command execution | yes [VERIFIED: local command] | 11.11.0 [VERIFIED: local command] | Use npm scripts directly |
| Cargo | Existing Rust workspace verification and Phase 7 fallback context | yes [VERIFIED: local command] | 1.95.0 [VERIFIED: local command] | None needed |
| Rust `wasm32-unknown-emscripten` target | Real Emscripten build, not Phase 8 fake-Module tests | no [VERIFIED: local command] | — | Use fake Module tests and Phase 7 native fallback path [VERIFIED: 07-VERIFICATION.md] |
| Emscripten `emcc` | Real browser build, not Phase 8 fake-Module tests | no [VERIFIED: local command] | — | Use fake Module tests; Phase 7 script reports blocker [VERIFIED: 07-VERIFICATION.md] |

**Missing dependencies with no fallback:**
- None for Phase 8 wrapper/package research because fake Emscripten Module tests are the locked/preferred test path. [VERIFIED: 08-CONTEXT.md]

**Missing dependencies with fallback:**
- `wasm32-unknown-emscripten` and `emcc` are missing locally; fallback is Node/Vitest fake-Module tests plus existing Phase 7 native adapter fallback for the Rust contract. [VERIFIED: local command; VERIFIED: 07-VERIFICATION.md]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Phase 8 wrapper has no user authentication flow. [VERIFIED: 08-CONTEXT.md] |
| V3 Session Management | yes | Treat opaque adapter state as a lifecycle resource; expose cleanup and reject operations after cleanup. [VERIFIED: 08-CONTEXT.md; VERIFIED: typeduck_web.rs] |
| V4 Access Control | no | Phase 8 has no multi-user authorization boundary. [VERIFIED: 08-CONTEXT.md] |
| V5 Input Validation | yes | Validate parsed JSON shape, reject unsupported key mappings deterministically, and do not fabricate null-pointer responses. [VERIFIED: 08-CONTEXT.md; VERIFIED: docs/typeduck-web-adapter.md] |
| V6 Cryptography | no | Phase 8 has no cryptographic operation. [VERIFIED: 08-CONTEXT.md] |
| V8 Data Protection | yes | Do not introduce AI, remote calls, or browser persistence policy; Phase 8 only passes host-owned paths and response data. [VERIFIED: 08-CONTEXT.md; VERIFIED: .planning/PROJECT.md] |
| V12 File and Resources | yes | Do not mount filesystems or accept arbitrary resource behavior in the wrapper; browser filesystem setup remains host-owned Phase 9 work. [VERIFIED: 08-CONTEXT.md; VERIFIED: docs/typeduck-web-adapter.md] |

### Known Threat Patterns for TypeScript/Emscripten Wrapper

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Use-after-free/double-free of adapter pointers | Tampering / Denial of Service | Keep state/response pointers opaque, centralize response freeing, and make cleanup idempotent at the wrapper layer. [VERIFIED: typeduck_web.rs; VERIFIED: 08-CONTEXT.md] |
| Null pointer hidden as normal response | Tampering / Information Disclosure | Throw deterministic wrapper errors for null response and null JSON pointers. [VERIFIED: 08-CONTEXT.md] |
| Unsupported key input silently mapped to wrong command | Tampering | Use explicit key mapping and deterministic tests; reject unsupported keys. [VERIFIED: key_table.rs; CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key] |
| Browser filesystem side effects in wrapper | Tampering / Information Disclosure | Keep MEMFS/IDBFS mount and sync out of Phase 8; document host ownership. [VERIFIED: 08-CONTEXT.md] |
| Remote/cloud behavior accidentally added | Information Disclosure | Do not introduce AI-native or remote provider behavior in Phase 8. [VERIFIED: 08-CONTEXT.md; VERIFIED: .planning/PROJECT.md] |

## Sources

### Primary (HIGH confidence)
- `/Users/trenton/Projects/yune/.planning/phases/08-typescript-bridge-and-runtime-package/08-CONTEXT.md` - Phase 8 decisions, scope, deferred work, package/test/lifecycle constraints. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/.planning/REQUIREMENTS.md` - `TYPEDUCK-JS-01` through `TYPEDUCK-JS-04`. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/.planning/ROADMAP.md` - Phase 8 goal, success criteria, and planned slices. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/.planning/phases/07-wasm-build-and-export-contract/07-CONTEXT.md` - Emscripten target and Phase 7 handoff constraints. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/.planning/phases/07-wasm-build-and-export-contract/07-VERIFICATION.md` - Phase 7 verification and local Emscripten blocker evidence. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/scripts/typeduck-exports.txt` - Canonical 11-symbol adapter export list. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/docs/typeduck-web-adapter.md` - Export signatures, response ownership, JSON shape, lifecycle, filesystem, and Emscripten flags. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs` - Adapter implementation, pointer ownership, JSON serialization, cleanup behavior. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs` - Native response-copy/free pattern and lifecycle tests. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/key_table.rs` - RIME/X11 keycode constants and modifier bit positions. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/.planning/codebase/STACK.md` - Existing Rust stack and no JS package tooling. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md` - Layering, process-global state, ABI boundary, facade constraints. [VERIFIED: file read]
- `/Users/trenton/Projects/yune/.planning/codebase/TESTING.md` - Current testing patterns and no browser E2E harness. [VERIFIED: file read]
- `npm view typescript`, `npm view vitest`, `npm view @types/node@24` - current package versions and engine metadata. [VERIFIED: npm registry]

### Primary documentation (HIGH confidence)
- [Emscripten preamble.js API reference](https://emscripten.org/docs/api_reference/preamble.js.html) - `ccall`, `cwrap`, `UTF8ToString`, `EXPORTED_FUNCTIONS`, and `EXPORTED_RUNTIME_METHODS`. [CITED: emscripten.org/docs/api_reference/preamble.js.html]
- [TypeScript TSConfig Reference](https://www.typescriptlang.org/tsconfig/) - strict checking, declaration emit, rootDir/outDir/module/target guidance. [CITED: www.typescriptlang.org/tsconfig/]
- [Vitest Guide](https://vitest.dev/guide/) - package script and `vitest run` usage; current docs show v4.1.5 and Node >=20. [CITED: vitest.dev/guide/]
- [MDN KeyboardEvent.key](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key) - semantic key values, printable characters, special key names, Shift/layout effects. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key]
- [MDN KeyboardEvent.keyCode](https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode) - deprecation and implementation-dependence. [CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode]

### Secondary (MEDIUM confidence)
- Context7 CLI results for `/microsoft/typescript` and `/vitest-dev/vitest` - library documentation discovery and supporting examples. [VERIFIED: Context7 CLI]

### Tertiary (LOW confidence)
- None. [VERIFIED: research assessment]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for versions and tool capabilities; MEDIUM for exact package path. [VERIFIED: npm registry; CITED: www.typescriptlang.org/tsconfig/; CITED: vitest.dev/guide/; ASSUMED]
- Architecture: HIGH because the adapter contract, lifecycle, and response ownership were verified in source and docs. [VERIFIED: typeduck_web.rs; VERIFIED: docs/typeduck-web-adapter.md]
- Pitfalls: HIGH for pointer/lifecycle/keyCode pitfalls; MEDIUM for inferred developer-error warning signs. [VERIFIED: typeduck_web.rs; CITED: developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/keyCode; ASSUMED]
- Security: MEDIUM because the phase is a local wrapper with no auth/crypto, but pointer ownership and input validation risks are concrete. [VERIFIED: 08-CONTEXT.md; VERIFIED: typeduck_web.rs]

**Research date:** 2026-05-04 [VERIFIED: system current date]
**Valid until:** 2026-05-11 for npm/Emscripten/TypeScript/Vitest version-sensitive details; adapter-contract findings remain valid until `typeduck_web.rs`, `scripts/typeduck-exports.txt`, or `docs/typeduck-web-adapter.md` changes. [ASSUMED]
