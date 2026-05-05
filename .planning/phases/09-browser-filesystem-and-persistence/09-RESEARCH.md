# Phase 9: Browser Filesystem And Persistence - Research

**Researched:** 2026-05-05  
**Domain:** Browser-hosted Emscripten filesystem orchestration for the Yune TypeDuck TypeScript runtime  
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

## Implementation Decisions

### Browser Filesystem Host Shape
- **D-01:** Add browser filesystem orchestration beside the Phase 8 TypeScript runtime package rather than in Rust core. Phase 9 should expose a small host-side helper layer that prepares Emscripten `FS`/`IDBFS` state before `TypeDuckRuntime.init`.
- **D-02:** Keep the helper DOM-free and TypeDuck-Web-app-free. It should be testable with a fake Emscripten filesystem/module in Node/Vitest, similar to the Phase 8 fake Module tests, rather than requiring a real browser or upstream TypeDuck-Web checkout.
- **D-03:** Use explicit caller-provided paths for `sharedDataDir`, `userDataDir`, and `schemaId`, carrying forward the Phase 8 runtime options. The helper may derive `userDataDir/build`, but it must not invent hidden global paths or support multiple simultaneous process-global services.

### Virtual Filesystem Layout And Asset Preload
- **D-04:** The prepared layout must include `shared_data_dir`, `user_data_dir`, and `user_data_dir/build` before init. These path names should be treated as browser virtual filesystem paths passed to the existing adapter, not as arbitrary native filesystem paths.
- **D-05:** Asset preload should be explicit and schema-scoped: preload `default.yaml`, `<schema>.schema.yaml`, and the selected `<dict>.dict.yaml` into `shared_data_dir`, and ensure deployed/preloaded `default.yaml` and `<schema>.schema.yaml` exist under `user_data_dir/build` before init.
- **D-06:** Do not hide missing preload data by fabricating placeholder schema or dictionary files. Missing assets should remain a deterministic setup/init failure so the recovery path can tell callers what is absent.
- **D-07:** Resource identifiers remain logical IDs. Browser helper code must not accept path-like schema or dictionary IDs that include traversal, absolute path syntax, or platform separators before joining virtual paths.

### Persistence Sync Policy
- **D-08:** Treat persistence sync as an explicit host-owned operation around the runtime: sync from persistent storage before init, and sync back after deploy, customize, cleanup when needed, and any userdb-changing flows that Phase 9 can observe or document.
- **D-09:** Use IDBFS as the primary documented Emscripten persistence target, but keep the helper abstraction narrow enough that tests can use a fake sync backend and docs can say “IDBFS or equivalent.”
- **D-10:** Sync failures should be surfaced as deterministic TypeScript errors, not swallowed. Callers need to know whether init is using fresh persistent data, stale in-memory data, or no persisted data.
- **D-11:** Phase 9 may provide convenience wrappers that call `runtime.deploy()` or `runtime.customize(...)` and then sync, but it should not change the underlying Phase 8 `TypeDuckRuntime` ownership/freeing contract.

### Failure And Recovery Behavior
- **D-12:** Missing assets, failed sync, and stale deployed config should have focused tests or documented repros. The expected behavior is visible failure plus actionable recovery instructions, not silent best-effort continuation.
- **D-13:** Stale deployed config recovery should prefer rerunning explicit preload/deploy/sync flows before init where possible. If recovery requires a live runtime, document the order clearly so callers do not initialize with incomplete state.
- **D-14:** Keep recovery paths local-first and deterministic. Do not add network fetch, remote asset discovery, or TypeDuck-Web-specific app policy in this phase.

### Claude's Discretion
- Choose exact TypeScript file names and helper API shape during planning, as long as the result stays small, package-local, and deterministic under Vitest.
- Prefer a fake Emscripten `FS`/`IDBFS` contract in tests over real browser automation. Real TypeDuck-Web/browser validation remains Phase 10.
- Prefer documenting userdb sync boundaries if the TypeScript wrapper cannot directly observe every native userdb mutation without widening the adapter contract.

### Deferred Ideas (OUT OF SCOPE)
- Upstream TypeDuck-Web clone, source seam identification, app patching, and real browser E2E remain Phase 10.
- Network asset fetching, CDN/cache policy, service worker integration, and TypeDuck-Web-specific application state remain out of scope unless Phase 10 requires them.
- Native adapter API expansion for richer userdb mutation notifications is deferred unless Phase 9 planning finds no safe way to document or wrap sync timing with the current exports.
- Multi-instance isolation beyond one active process-global Yune/RIME service remains out of scope for this milestone.
- AI-native provider, ranking, context, memory, privacy, and frontend exposure behavior remains deferred until TypeDuck-Web integration produces a go/no-go recommendation.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TYPEDUCK-FS-01 | Browser setup creates expected shared/user/build layout before adapter init. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Use the package-local filesystem helper to call `FS.mkdirTree`/equivalent for caller-provided `sharedDataDir`, `userDataDir`, and derived `${userDataDir}/build`; mirror the native adapter's layout guard. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.mkdirTree] |
| TYPEDUCK-FS-02 | Schema and dictionary assets can be preloaded before adapter init. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Preload explicit asset content for `default.yaml`, `<schema>.schema.yaml`, selected `<dict>.dict.yaml`, plus deployed build copies of `default.yaml` and `<schema>.schema.yaml`; reject invalid logical IDs before path joining. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] |
| TYPEDUCK-FS-03 | IDBFS or equivalent persistence syncs before init and after deploy/customize/userdb mutations. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Wrap `FS.syncfs(true, cb)` before init and `FS.syncfs(false, cb)` after deploy/customize/cleanup/userdb-changing flows; expose deterministic errors from callback failures. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] |
| TYPEDUCK-FS-04 | Missing assets, failed sync, and stale deployed config recovery paths are documented/tested where possible. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Add focused Vitest cases with fake `FS`/sync behavior and update `docs/typeduck-web-adapter.md` with deterministic recovery order; do not require real browser E2E. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
</phase_requirements>

## Summary

Phase 9 should implement a small, DOM-free filesystem/persistence helper inside `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime`, composing with the existing `TypeDuckRuntime.init`, `deploy`, `customize`, and `cleanup` methods instead of replacing the Phase 8 wrapper or changing native exports. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] The existing package already has strict TypeScript, package-local Vitest tests, a fake Emscripten module pattern, and an export barrel, so Phase 9 should add source/test files there without root JS workspace or browser-app scaffolding. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json] [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/test/fake-module.ts]

The native browser adapter currently refuses to initialize unless shared/default, shared/schema, build/default, build/schema, and the schema's referenced dictionary asset are present, and it rejects non-logical schema/dictionary IDs. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] Phase 9 should duplicate those preflight checks in TypeScript so callers see deterministic setup errors before a process-global RIME service is started. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs] Persistence should use Emscripten's documented `FS.syncfs(true, callback)` before init and `FS.syncfs(false, callback)` after file-mutating flows; callback errors must become typed TypeScript errors. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs]

**Primary recommendation:** Add `src/filesystem.ts` plus fakeable Vitest tests and docs that prepare the Emscripten virtual layout, preload explicit schema assets, mount/sync IDBFS or an equivalent backend, and provide sync-after wrappers for `deploy`/`customize` while documenting userdb mutation boundaries. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]

## Project Constraints (from CLAUDE.md)

No project-level `/Users/trenton/Projects/yune/CLAUDE.md` file was found, so no additional CLAUDE.md directives apply. [VERIFIED: Read /Users/trenton/Projects/yune/CLAUDE.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Browser virtual filesystem layout creation | Browser / Client host runtime | TypeScript runtime package | Emscripten `FS` is a browser/JS runtime surface, while the package-local helper owns deterministic API ergonomics before native init. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html] [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Schema/dictionary asset preload | TypeScript runtime package | Browser / Client host runtime | The helper should validate logical IDs and write explicit caller-provided asset content into Emscripten paths; the host remains responsible for obtaining assets. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Native RIME service initialization | Native adapter through Emscripten exports | TypeScript runtime package | `TypeDuckRuntime.init` binds and calls `yune_typeduck_init`; Phase 9 prepares files before that call but must not alter ownership/freeing. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt] |
| Persistent storage synchronization | Browser / Client host runtime | TypeScript runtime package | IDBFS persistence is an Emscripten browser FS concern invoked through `FS.syncfs`; the helper should expose narrow promise wrappers and fakeable tests. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] |
| Deployed config freshness/recovery | Native adapter/runtime files | TypeScript docs/helper orchestration | Native deploy/config code determines build freshness through deployed files and build metadata; Phase 9 documents and wraps the browser sync/preload/deploy order that keeps those files current. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/deployment.rs] |
| Userdb mutation persistence | Native runtime/session | Browser persistence sync wrapper/docs | Native key/candidate flows can write userdb data internally; current `yune_typeduck_*` exports do not expose mutation notifications, so Phase 9 should document host sync boundaries instead of adding exports. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/session.rs] [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt] |

## Standard Stack

### Core

| Library / API | Version | Purpose | Why Standard |
|---------------|---------|---------|--------------|
| TypeScript | 6.0.3; npm registry modified 2026-04-16T23:38:57.055Z [VERIFIED: npm registry] | Strict package-local TypeScript source for helper interfaces and errors. | Existing package already uses TypeScript `^6.0.3`, `moduleResolution: NodeNext`, `strict: true`, and declaration output. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json] [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/tsconfig.json] |
| Vitest | 4.1.5; npm registry modified 2026-04-23T10:30:30.524Z [VERIFIED: npm registry] | Deterministic package-local tests for fake Emscripten filesystem/module behavior. | Existing Phase 8 tests use Vitest with `FakeTypeDuckModule`, and `npm --prefix ... test` passed 38 tests. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json] [VERIFIED: Bash npm test 2026-05-05] |
| Emscripten `FS` API | Host-provided runtime API; local `emcc` unavailable [VERIFIED: Bash command -v emcc] | Browser virtual filesystem operations: create directories, mount IDBFS, write/read files, analyze paths, and sync persistence. | Official Emscripten docs define `FS.mkdirTree`, `FS.writeFile`, `FS.readFile`, `FS.analyzePath`, `FS.mount`, and `FS.syncfs`; Phase 9 should type only the minimal subset needed for fake tests. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html] |
| Emscripten IDBFS | Host-provided runtime filesystem type [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#filesystem-api-idbfs] | Browser IndexedDB-backed persistence for virtual filesystem state. | Emscripten documents IDBFS as browser persistence synchronized with `FS.syncfs`; Phase 9 context explicitly selects IDBFS as the primary documented target while allowing equivalent fake backends. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#filesystem-api-idbfs] [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Existing `TypeDuckRuntime` | Package-local source, no npm release [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] | Native adapter lifecycle wrapper for init, key processing, deploy, customize, and cleanup. | It already owns binding, response ownership, lifecycle errors, and process-global native state pointer handling, so filesystem helpers should compose around it. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] |

### Supporting

| Library / API | Version | Purpose | When to Use |
|---------------|---------|---------|-------------|
| Existing `EmscriptenTypeDuckModule` interface | Package-local source [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/module.ts] | Current narrow `cwrap`/`UTF8ToString` binding surface for `yune_typeduck_*` exports. | Keep it unchanged for native symbol binding; define a separate filesystem interface instead of bloating this adapter contract. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/08-typescript-bridge-and-runtime-package/08-CONTEXT.md] |
| Fake Emscripten filesystem test double | New Phase 9 test utility [ASSUMED] | Deterministic in-memory directory/file/sync simulation under Node/Vitest. | Use for layout, preload, mount, sync, error propagation, and wrapper-order tests without browser E2E. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/test/fake-module.ts] |
| `docs/typeduck-web-adapter.md` | Project documentation [VERIFIED: /Users/trenton/Projects/yune/docs/typeduck-web-adapter.md] | Browser filesystem contract and recovery procedure documentation. | Update after helper API is added so Phase 10 has a TypeDuck-Web handoff contract. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Package-local TypeScript helper | Rust core changes | Rejected by locked decision D-01; Rust core should not learn about Emscripten browser host policy. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Fake `FS`/sync tests | Real browser automation | Real browser E2E is explicitly deferred to Phase 10; fake tests keep Phase 9 deterministic and local. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Explicit caller-provided assets | Network fetch/CDN/service worker asset discovery | Network and app-specific cache policy are out of scope; implicit fetch would hide missing asset failures. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Current `yune_typeduck_*` exports | Add native userdb sync notification exports | Native export expansion is deferred unless a focused blocker proves necessary; current phase can document userdb sync boundaries. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt] |
| IDBFS or equivalent narrow backend | Root application storage framework | Phase 9 owns a reusable runtime helper, not app state, bundler, or frontend product policy. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |

**Installation:** No new npm packages are required for Phase 9 because TypeScript and Vitest already exist in `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json`. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json]

```bash
npm --prefix /Users/trenton/Projects/yune/packages/yune-typeduck-runtime install
```

**Version verification:** `npm view typescript version time.modified --json` returned 6.0.3 modified 2026-04-16, and `npm view vitest version time.modified --json` returned 4.1.5 modified 2026-04-23. [VERIFIED: npm registry]

## Architecture Patterns

### System Architecture Diagram

```text
Caller-provided browser assets and paths
  |
  v
Phase 9 TypeScript filesystem helper
  |-- validate logical IDs (schemaId, dictionaryId)
  |-- create sharedDataDir, userDataDir, userDataDir/build
  |-- optionally mount IDBFS/equivalent at configured mountpoint
  |-- syncfs(populate=true) before init
  |-- write default/schema/dict assets into sharedDataDir and build assets into userDataDir/build
  |-- verify required files exist
  |
  v
TypeDuckRuntime.init(module, { sharedDataDir, userDataDir, schemaId })
  |
  v
Emscripten cwrap -> yune_typeduck_init
  |
  v
Native adapter preloaded-asset guard
  |-- missing required file? -> init returns null -> TypeDuckLifecycleError
  |-- invalid schema/dict ID? -> init returns null -> TypeDuckLifecycleError
  |-- all assets valid? -> process-global Yune/RIME service starts
  |
  v
Runtime operations
  |-- deploy() -> native deployed files may change -> helper syncfs(populate=false)
  |-- customize(...) -> native config files may change -> helper syncfs(populate=false)
  |-- process/select/delete/flip -> possible userdb writes on commit boundaries -> documented host syncfs(populate=false)
  |-- cleanup() -> optional final sync according to host policy
```

### Recommended Project Structure

```text
packages/yune-typeduck-runtime/
├── src/
│   ├── filesystem.ts        # New Phase 9 DOM-free FS, asset preload, sync helpers, typed errors
│   ├── index.ts             # Export filesystem helper API alongside existing runtime exports
│   ├── typeduck.ts          # Existing lifecycle wrapper; compose with it, do not replace
│   └── module.ts            # Existing native symbol interface; keep narrow
├── test/
│   ├── filesystem.test.ts   # New Vitest coverage for layout/preload/sync/failure behavior
│   ├── fake-filesystem.ts   # New fake Emscripten FS/IDBFS test double
│   └── fake-module.ts       # Existing fake native module pattern to reuse
└── package.json             # Existing build/test scripts reused

docs/
└── typeduck-web-adapter.md  # Update browser filesystem usage and recovery order
```

### Pattern 1: Narrow fakeable filesystem interface

**What:** Define a small TypeScript interface for only the Emscripten FS pieces Phase 9 uses: `mkdirTree` or `mkdir`, `writeFile`, `readFile`, `analyzePath`, `mount`, `syncfs`, and an optional `IDBFS` filesystem type token. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html]  
**When to use:** Use this interface for browser helper inputs and fake Vitest tests; do not require DOM, `window`, IndexedDB globals, or a real Emscripten build in tests. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]  
**Example:**

```typescript
// Source: https://emscripten.org/docs/api_reference/Filesystem-API.html
export interface TypeDuckFilesystem {
  mkdirTree?(path: string, mode?: number): void;
  mkdir?(path: string, mode?: number): void;
  writeFile(path: string, data: string | Uint8Array, opts?: { flags?: string }): void;
  readFile(path: string, opts?: { encoding?: "utf8" | "binary" }): string | Uint8Array;
  analyzePath(path: string, dontResolveLastLink?: boolean): { exists: boolean; error?: unknown };
  mount?(type: unknown, opts: Record<string, unknown>, mountpoint: string): void;
  syncfs?(populate: boolean, callback: (error?: unknown) => void): void;
}
```

### Pattern 2: Promise wrapper for `FS.syncfs`

**What:** Convert Emscripten's callback-style `FS.syncfs(populate, callback)` into an awaited helper that rejects with a deterministic Phase 9 error type. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs]  
**When to use:** Use before init with `populate: true` and after deploy/customize/userdb-changing flows with `populate: false`. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]

```typescript
// Source: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs
export async function syncTypeDuckFilesystem(
  fs: Pick<TypeDuckFilesystem, "syncfs">,
  direction: "fromPersistence" | "toPersistence",
): Promise<void> {
  if (!fs.syncfs) {
    throw new TypeDuckFilesystemError("Emscripten FS.syncfs is unavailable");
  }
  const populate = direction === "fromPersistence";
  await new Promise<void>((resolve, reject) => {
    fs.syncfs!(populate, (error?: unknown) => {
      if (error) reject(new TypeDuckFilesystemError("TypeDuck filesystem sync failed", { cause: error }));
      else resolve();
    });
  });
}
```

### Pattern 3: Mirror native asset guards before init

**What:** TypeScript should validate the same required file set the native adapter checks: shared `default.yaml`, shared `<schema>.schema.yaml`, shared `<dict>.dict.yaml`, build `default.yaml`, and build `<schema>.schema.yaml`. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]  
**When to use:** Run after directory creation and asset writes, before calling `TypeDuckRuntime.init`, so failures are actionable setup errors instead of opaque null-init failures. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs]

```typescript
// Source: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs
export function isTypeDuckLogicalId(id: string): boolean {
  return /^[A-Za-z0-9_-]+$/.test(id);
}

export function requiredTypeDuckAssetPaths(options: {
  sharedDataDir: string;
  userDataDir: string;
  schemaId: string;
  dictionaryId: string;
}): string[] {
  const buildDir = joinVirtualPath(options.userDataDir, "build");
  return [
    joinVirtualPath(options.sharedDataDir, "default.yaml"),
    joinVirtualPath(options.sharedDataDir, `${options.schemaId}.schema.yaml`),
    joinVirtualPath(options.sharedDataDir, `${options.dictionaryId}.dict.yaml`),
    joinVirtualPath(buildDir, "default.yaml"),
    joinVirtualPath(buildDir, `${options.schemaId}.schema.yaml`),
  ];
}
```

### Pattern 4: Compose sync wrappers around runtime methods

**What:** Provide convenience functions that call `runtime.deploy()` or `runtime.customize(...)`, then persist the virtual filesystem with `syncfs(false)`, returning the original boolean result or throwing a sync error. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs]  
**When to use:** Use for host flows that want one safe call after known file-mutating operations without changing `TypeDuckRuntime` lifecycle or response ownership. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]

```typescript
// Source: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts
export async function deployAndSync(runtime: TypeDuckRuntime, fs: TypeDuckFilesystem): Promise<boolean> {
  const deployed = runtime.deploy();
  await syncTypeDuckFilesystem(fs, "toPersistence");
  return deployed;
}

export async function customizeAndSync(
  runtime: TypeDuckRuntime,
  fs: TypeDuckFilesystem,
  configId: string,
  key: string,
  value: string,
): Promise<boolean> {
  const customized = runtime.customize(configId, key, value);
  await syncTypeDuckFilesystem(fs, "toPersistence");
  return customized;
}
```

### Anti-Patterns to Avoid

- **Broadening `EmscriptenTypeDuckModule` for FS concerns:** Keep native symbol binding separate from browser filesystem helpers; `module.ts` currently only requires `cwrap` and `UTF8ToString`. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/module.ts]
- **Fabricating placeholder YAML files:** Missing assets must be deterministic failures, not hidden by fake schemas or dictionaries. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]
- **Accepting path-like IDs:** Reject traversal, absolute path syntax, separators, and anything outside the native adapter's ASCII alphanumeric/underscore/hyphen rule for schema and dictionary IDs. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]
- **Auto-fetching assets over the network:** Asset discovery, CDN policy, cache policy, and service workers are deferred and app-specific. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]
- **Requiring real TypeDuck-Web/browser E2E in Phase 9:** Phase 9 should test a fake Emscripten FS/IDBFS contract in Vitest; real app E2E belongs to Phase 10. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Browser persistence backend | Custom IndexedDB serialization for RIME files | Emscripten IDBFS or equivalent mounted backend with `FS.syncfs` | Emscripten already defines browser persistence semantics and sync direction; Phase 9 only needs a narrow wrapper. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#filesystem-api-idbfs] |
| Native lifecycle wrapper | New runtime class that bypasses `TypeDuckRuntime` | Existing `TypeDuckRuntime` plus helper functions | Existing wrapper owns `cwrap`, response free, cleanup idempotence, and post-cleanup errors. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] |
| YAML placeholder generation | Synthetic schema/dictionary/default files | Explicit caller-provided asset content | Native adapter and context require missing assets to fail visibly. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Path sanitizer with platform filesystem behavior | Native/path-module path normalization | Simple virtual path joining plus strict logical ID validation | Browser virtual FS paths are POSIX-like strings; schema/dict IDs are logical IDs, not host filesystem paths. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs] [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] |
| Browser automation harness | Playwright/Cypress/WebDriver setup | Package-local Vitest fake FS tests | Real TypeDuck-Web/browser validation is deferred; current package already uses Vitest successfully. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json] [VERIFIED: Bash npm test 2026-05-05] |
| Native userdb notification system | New `yune_typeduck_*` export for every mutation | Documented sync boundaries plus optional explicit host `syncAfterUserDataChange` helper | Export expansion is deferred unless a blocker appears; current exports do not include userdb sync notifications. [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt] [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |

**Key insight:** The hard part is not creating directories; it is preserving deterministic ownership boundaries across browser FS, TypeScript helper, Emscripten persistence, native process-global runtime, deployed config freshness, and userdb writes without hiding failures or inventing app policy. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]

## Common Pitfalls

### Pitfall 1: Syncing in the wrong direction

**What goes wrong:** Calling `FS.syncfs(false)` before init overwrites persisted data with empty in-memory state, or calling `FS.syncfs(true)` after deploy discards in-memory changes. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs]  
**Why it happens:** Emscripten uses a boolean `populate` parameter rather than explicit direction names. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs]  
**How to avoid:** Wrap the boolean in named helpers: `syncFromPersistenceBeforeInit()` uses `true`; `syncToPersistenceAfterMutation()` uses `false`. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs]  
**Warning signs:** Tests assert only that `syncfs` was called, not the `populate` argument. [ASSUMED]

### Pitfall 2: Starting the native process-global service before preflight checks

**What goes wrong:** A failed asset setup becomes a native init failure or leaves unclear lifecycle state. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]  
**Why it happens:** Caller invokes `TypeDuckRuntime.init` directly without helper validation. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts]  
**How to avoid:** Phase 9 helper should create layout, preload assets, verify required paths, then call or instruct callers to call `TypeDuckRuntime.init`. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]  
**Warning signs:** Missing asset tests expect only `TypeDuckLifecycleError("TypeDuck adapter init failed")` instead of a specific filesystem setup error. [ASSUMED]

### Pitfall 3: Diverging from native asset requirements

**What goes wrong:** TypeScript preflight passes but native `has_preloaded_runtime_assets` rejects init because build files or dictionary assets differ. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]  
**Why it happens:** Helper validates only shared schema/default files and forgets `userDataDir/build` or the schema's dictionary. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs]  
**How to avoid:** Treat the Rust guard and native tests as the source of truth; mirror all five required files. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]  
**Warning signs:** Tests do not include “wrong dictionary” or missing build-copy cases. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs]

### Pitfall 4: Accepting path-like resource IDs

**What goes wrong:** A schema or dictionary ID containing `../`, `/`, `\\`, drive-prefix syntax, or an empty string can escape expected virtual paths or fail native validation. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs] [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]  
**Why it happens:** Treating resource IDs as filenames instead of logical identifiers. [VERIFIED: /Users/trenton/Projects/yune/.planning/PROJECT.md]  
**How to avoid:** For Phase 9 schema/dictionary IDs, use the native adapter's stricter rule: nonempty ASCII alphanumeric plus `_` and `-` only. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]  
**Warning signs:** Tests include `typeduck/luna` or `../typeduck_luna` as accepted input. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs]

### Pitfall 5: Assuming all userdb mutations are observable in TypeScript

**What goes wrong:** Browser persistence misses learning data because native key/candidate flows write userdb internally and the wrapper cannot see every mutation. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/session.rs]  
**Why it happens:** Current `yune_typeduck_*` exports do not include mutation notifications or explicit userdb sync calls. [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt]  
**How to avoid:** Provide documented host boundaries such as sync after commit-producing interaction batches, before page unload when allowed, and after explicit deploy/customize wrappers; do not add native exports unless a blocker is proven. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]  
**Warning signs:** Research/plan promises automatic persistence of every userdb change without native API support. [ASSUMED]

### Pitfall 6: Coupling helper to TypeDuck-Web app policy

**What goes wrong:** Runtime helper becomes tied to DOM events, fetch URLs, service workers, or upstream app state, making it hard to reuse and test. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]  
**Why it happens:** Confusing Phase 9 browser FS plumbing with Phase 10 TypeDuck-Web integration. [VERIFIED: /Users/trenton/Projects/yune/.planning/ROADMAP.md]  
**How to avoid:** Accept explicit asset data and filesystem/module interfaces; leave asset acquisition and app lifecycle to callers. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]  
**Warning signs:** New code imports DOM types, `fetch`, service worker APIs, bundler config, or upstream TypeDuck-Web source. [ASSUMED]

## Code Examples

Verified patterns from official and project sources.

### Create directory tree and write assets

```typescript
// Source: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.mkdirTree
// Source: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.writeFile
export function ensureDir(fs: TypeDuckFilesystem, path: string): void {
  if (fs.analyzePath(path).exists) return;
  if (fs.mkdirTree) fs.mkdirTree(path);
  else fs.mkdir?.(path);
}

export function preloadTextAsset(fs: TypeDuckFilesystem, path: string, text: string): void {
  fs.writeFile(path, text, { flags: "w" });
}
```

### Verify required virtual files without throwing from `analyzePath`

```typescript
// Source: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.analyzePath
export function fileExists(fs: TypeDuckFilesystem, path: string): boolean {
  return fs.analyzePath(path).exists === true;
}
```

### Mount IDBFS at a caller-provided mountpoint

```typescript
// Source: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.mount
// Source: https://emscripten.org/docs/api_reference/Filesystem-API.html#filesystem-api-idbfs
export function mountTypeDuckPersistence(
  fs: TypeDuckFilesystem,
  type: unknown,
  mountpoint: string,
  opts: Record<string, unknown> = {},
): void {
  ensureDir(fs, mountpoint);
  if (!fs.mount) throw new TypeDuckFilesystemError("Emscripten FS.mount is unavailable");
  fs.mount(type, opts, mountpoint);
}
```

### Init after sync and preflight

```typescript
// Source: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts
// Source: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs
export async function prepareAndInitTypeDuck(
  module: EmscriptenTypeDuckModule,
  fs: TypeDuckFilesystem,
  options: PrepareTypeDuckFilesystemOptions,
): Promise<TypeDuckRuntime> {
  await syncTypeDuckFilesystem(fs, "fromPersistence");
  prepareTypeDuckFilesystem(fs, options);
  assertTypeDuckAssetsReady(fs, options);
  return TypeDuckRuntime.init(module, {
    sharedDataDir: options.sharedDataDir,
    userDataDir: options.userDataDir,
    schemaId: options.schemaId,
  });
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Browser host manually creates undocumented MEMFS paths before native init | Phase 9 helper documents and tests layout/preload/sync expectations in the TypeScript runtime package | Phase 9 planning, 2026-05-05 [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] | Planner should create helper/docs/tests rather than only updating docs. |
| Native adapter reports generic null init on missing browser assets | TypeScript preflight should expose deterministic missing-asset/setup errors before native init | Phase 9 planning, 2026-05-05 [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] | Improves recovery docs and tests without changing native exports. |
| Implicit persistence expectations around browser reloads | Explicit `syncfs(true)` before init and `syncfs(false)` after mutations | Official Emscripten FS API current docs [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] | Planner should require tests that assert direction booleans and callback error propagation. |
| Real browser/TypeDuck-Web validation as prerequisite | Fake Emscripten FS/IDBFS contract tests in Vitest | Locked Phase 9 decision [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] | Keeps Phase 9 local and deterministic; Phase 10 owns real app E2E. |

**Deprecated/outdated:**
- Treating browser filesystem setup as documentation-only is outdated for Phase 9; the phase success criteria require helper behavior and tested/documented recovery paths. [VERIFIED: /Users/trenton/Projects/yune/.planning/ROADMAP.md]
- Treating resource IDs as paths conflicts with the project security model and native adapter validation. [VERIFIED: /Users/trenton/Projects/yune/.planning/PROJECT.md] [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]

## Proposed Helper Responsibilities And Boundaries

### Responsibilities

1. Define `TypeDuckFilesystem` / `TypeDuckPersistentFilesystem` interfaces for minimal fakeable Emscripten FS operations. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html]
2. Define deterministic `TypeDuckFilesystemError` for layout, preload, validation, mount, and sync failures. [ASSUMED]
3. Validate `schemaId` and `dictionaryId` as nonempty ASCII alphanumeric, `_`, or `-`, matching `typeduck_web.rs`. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]
4. Join virtual paths with forward slashes and reject caller path components for resource IDs. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs]
5. Create `sharedDataDir`, `userDataDir`, and `${userDataDir}/build` before init. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md]
6. Preload caller-provided `default.yaml`, schema YAML, and dictionary YAML into `sharedDataDir`, plus deployed copies of default/schema YAML into `userDataDir/build`. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]
7. Verify required files exist before `TypeDuckRuntime.init`. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]
8. Expose `syncFromPersistenceBeforeInit` and `syncToPersistenceAfterMutation` wrappers around `FS.syncfs`. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs]
9. Expose optional `deployAndSync` and `customizeAndSync` convenience wrappers that call existing runtime methods and persist afterward. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts]
10. Document userdb sync boundaries where TypeScript cannot observe internal native mutations. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/session.rs]

### Boundaries

- Do not clone, import, or patch upstream TypeDuck-Web in Phase 9. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]
- Do not add browser E2E tools, bundler scaffolding, DOM code, network fetch, CDN/cache policy, or service worker behavior. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]
- Do not change `crates/yune-rime-api/src/typeduck_web.rs` or `scripts/typeduck-exports.txt` unless a focused blocker is discovered during implementation. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]
- Do not promise multiple simultaneous process-global runtimes. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]

## Asset Preload Validation Rules

| Rule | Source | Planner Action |
|------|--------|----------------|
| `schemaId` must be nonempty ASCII alphanumeric, `_`, or `-`. | [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] | Add TypeScript validation and tests for empty, `../typeduck_luna`, `typeduck/luna`, `typeduck\\luna`, and valid `typeduck_luna`. |
| `dictionaryId` should use the same validation as `schemaId` because native `required_dictionary` filters the schema's translator dictionary with `is_valid_schema_id`. | [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] | Require explicit `dictionaryId` or parse/verify the dictionary from schema content before writing/verification. |
| Required shared files are `default.yaml`, `<schemaId>.schema.yaml`, and `<dictionaryId>.dict.yaml`. | [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] | Preload exactly these names under `sharedDataDir`. |
| Required build files are `default.yaml` and `<schemaId>.schema.yaml` under `${userDataDir}/build`. | [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] | Write or verify these deployed/preloaded copies before init. |
| Missing files should fail setup/init visibly; helper must not generate placeholders. | [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] | Throw `TypeDuckFilesystemError` listing missing virtual paths. |
| Paths are browser virtual filesystem paths, not arbitrary native filesystem paths. | [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] | Use simple virtual path joining and keep resource IDs separate from directories. |

## Sync Timing And Wrapper Strategy

| Moment | Direction | Required Behavior | Notes |
|--------|-----------|-------------------|-------|
| Before layout/preload/init | `syncfs(true)` / from persistence | Populate the in-memory Emscripten FS from persistent storage before writing/validating assets. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] | If sync fails, throw deterministic error and do not call native init. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| After asset preload but before init | Optional `syncfs(false)` if caller wants preloaded assets persisted immediately [ASSUMED] | Planner may include this only if API design wants persisted preload before native init; not required by locked decisions. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] | Avoid redundant sync unless docs define why it is needed. [ASSUMED] |
| After `runtime.deploy()` | `syncfs(false)` / to persistence | Persist generated/deployed files after native deploy. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] | Wrapper should return deploy boolean after successful sync or throw on sync failure. [ASSUMED] |
| After `runtime.customize(...)` | `syncfs(false)` / to persistence | Persist custom config changes after native customize. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] | Do not change underlying `customize` signature. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts] |
| After commit-producing key/candidate flows | `syncfs(false)` / to persistence | Document host policy to sync after userdb-changing interaction batches because native session code can persist learning internally. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/session.rs] | TypeScript cannot reliably detect every mutation with current exports. [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt] |
| Before cleanup/page lifecycle boundary | `syncfs(false)` / to persistence | Provide documented optional final sync before or after `runtime.cleanup()` according to host lifecycle. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] | Browser page unload async guarantees are host/app policy and should not be overpromised. [ASSUMED] |

## Failure And Recovery Test Strategy

| Failure Mode | Test Coverage | Expected Behavior | Recovery Documentation |
|--------------|---------------|-------------------|------------------------|
| Missing shared `default.yaml` | Fake FS test omits default asset. | Helper throws deterministic missing-asset error before init. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] | Add asset and rerun prepare/preload/init. |
| Missing shared schema | Fake FS test omits `<schema>.schema.yaml`. | Helper lists missing schema path. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] | Preload the correct schema-scoped asset. |
| Wrong/missing dictionary | Fake FS test uses schema dictionary that does not match preloaded dict. | Helper or documented native fallback fails deterministically. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs] | Preload `<dict>.dict.yaml` for the schema's `translator.dictionary`. |
| Missing build default/schema | Fake FS test omits `${userDataDir}/build/default.yaml` or build schema. | Helper fails before init. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] | Rerun preload/deploy/sync flow to repopulate build files. |
| Invalid logical ID | Tests pass `../typeduck_luna`, `typeduck/luna`, `typeduck\\luna`, empty string. | Helper rejects before writing or joining paths. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs] | Use logical IDs only. |
| Failed sync before init | Fake `syncfs(true)` callback receives error. | Helper rejects and does not call `TypeDuckRuntime.init`. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] | Retry sync or initialize only after caller accepts no/stale persisted data policy. |
| Failed sync after deploy/customize | Fake `syncfs(false)` callback receives error after runtime method. | Wrapper throws `TypeDuckFilesystemError`; docs state in-memory changes may not be persisted. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] | Retry `syncfs(false)` before reload or rerun deploy/customize later. |
| Stale deployed config | Documentation test/repro sequence, and optional fake timestamp metadata test if helper has enough information. [ASSUMED] | Recovery prefers sync-from-persistence, ensure/preload assets, deploy if needed, sync-to-persistence, then init/select/process. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] | Document exact order in `docs/typeduck-web-adapter.md`. |

## Concrete Files Likely To Create/Modify

| Path | Action | Purpose |
|------|--------|---------|
| `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/filesystem.ts` | Create | Define filesystem interfaces, path/id validation, layout creation, asset preload, required path verification, sync wrappers, and typed filesystem errors. [ASSUMED] |
| `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/index.ts` | Modify | Export the new filesystem helper API from the package barrel. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/index.ts] |
| `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/test/fake-filesystem.ts` | Create | Fake Emscripten FS/IDBFS contract for deterministic Vitest tests, mirroring the existing fake-module style. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/test/fake-module.ts] |
| `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/test/filesystem.test.ts` | Create | Cover layout, preload, logical ID rejection, required file verification, sync direction, sync errors, and deploy/customize wrapper ordering. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/test/typeduck.test.ts] |
| `/Users/trenton/Projects/yune/docs/typeduck-web-adapter.md` | Modify | Replace Phase 9 deferral notes with helper usage, IDBFS/equivalent sync contract, userdb sync boundaries, and recovery order. [VERIFIED: /Users/trenton/Projects/yune/docs/typeduck-web-adapter.md] |
| `/Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs` | Avoid unless blocker | Native guard already enforces required assets; Phase 9 should not broaden native API by default. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] |
| `/Users/trenton/Projects/yune/scripts/typeduck-exports.txt` | Avoid unless blocker | Canonical exports already match Phase 8 bridge and do not include filesystem/userdb notifications. [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt] |

## Verification Commands

```bash
npm --prefix /Users/trenton/Projects/yune/packages/yune-typeduck-runtime test
npm --prefix /Users/trenton/Projects/yune/packages/yune-typeduck-runtime run build
```

Both commands passed before Phase 9 implementation on 2026-05-05. [VERIFIED: Bash npm test/build 2026-05-05]

Optional native regression command if implementation touches Rust adapter or export list:

```bash
/Users/trenton/.cargo/bin/cargo test -p yune-rime-api typeduck_web
```

Rust tools are available under `/Users/trenton/.cargo/bin`; `emcc` is not available in the current PATH. [VERIFIED: Bash tool probe 2026-05-05]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A new fake Emscripten filesystem test double should be created as a supporting utility. | Standard Stack / Concrete Files | Planner might instead inline fake FS behavior inside `filesystem.test.ts`; low architectural risk. |
| A2 | Tests should flag missing assertions on the `syncfs` populate argument as a warning sign. | Common Pitfalls | If the implementation has another way to prove direction, this warning is overly prescriptive. |
| A3 | Deterministic filesystem setup errors should use a new `TypeDuckFilesystemError`. | Proposed Helper Responsibilities | Planner may choose a different error class/name while preserving deterministic error behavior. |
| A4 | Persisting assets immediately after preload is optional rather than required. | Sync Timing | If the intended host flow requires cached preloads to survive reload before init, planner should add a sync-after-preload step. |
| A5 | Browser page unload async sync guarantees should not be overpromised. | Sync Timing | If host-specific APIs are later selected, docs may give stronger guarantees for that app. |
| A6 | Stale deployed config can be covered primarily through documentation/repro sequence unless helper has enough metadata access for fake tests. | Failure And Recovery Test Strategy | Planner may need a focused test fixture if docs-only coverage is considered insufficient. |
| A7 | Suggested file names such as `filesystem.ts`, `fake-filesystem.ts`, and `filesystem.test.ts` are likely but discretionary. | Concrete Files | Planner may pick different names; import/export paths must remain coherent. |

## Open Questions

1. **Should the helper parse `translator.dictionary` from schema YAML or require explicit `dictionaryId`?**
   - What we know: Native `required_dictionary` parses schema YAML and validates `translator.dictionary` with the same ID rule. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs]
   - What's unclear: The package currently has no YAML parser dependency, and adding one would be unnecessary if callers provide `dictionaryId`. [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json]
   - Recommendation: Require explicit `dictionaryId` in Phase 9 helper options and document that it must match the schema's `translator.dictionary`; avoid adding a YAML parser unless planner finds mismatch tests impossible without it. [ASSUMED]

2. **Should `prepareAndInitTypeDuck` be included, or should helpers stop before `TypeDuckRuntime.init`?**
   - What we know: Context requires helper state before init and allows convenience wrappers, but exact API shape is Claude's discretion. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]
   - What's unclear: A single combined helper may be convenient but could obscure the explicit host-owned ordering. [ASSUMED]
   - Recommendation: Provide granular helpers first (`syncFromPersistence`, `prepareFilesystem`, `assertAssetsReady`) and optionally a small `prepareAndInitTypeDuck` wrapper if tests keep ordering explicit. [ASSUMED]

3. **How aggressive should userdb persistence wrappers be after input events?**
   - What we know: Native session code can persist pending userdb learning on commit, and current TypeScript exports do not expose mutation notifications. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/session.rs] [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt]
   - What's unclear: The exact browser host cadence for syncing after commits is app policy and may affect responsiveness. [ASSUMED]
   - Recommendation: Document explicit host sync boundaries and expose `syncAfterUserDataChange(fs)` as a named helper, without automatically syncing after every `processKey` in Phase 9. [ASSUMED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | Package-local TypeScript/Vitest build and tests | ✓ | v24.14.1 [VERIFIED: Bash tool probe 2026-05-05] | — |
| npm | Running existing package scripts and verifying registry versions | ✓ | 11.11.0 [VERIFIED: Bash tool probe 2026-05-05] | — |
| TypeScript | Package build | ✓ | 6.0.3 in npm registry and package devDependency [VERIFIED: npm registry] [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json] | — |
| Vitest | Package tests | ✓ | 4.1.5 in npm registry and package devDependency [VERIFIED: npm registry] [VERIFIED: /Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json] | — |
| Rust cargo | Optional native adapter regression tests if Rust files change | ✓ | cargo 1.95.0 at `/Users/trenton/.cargo/bin/cargo` [VERIFIED: Bash tool probe 2026-05-05] | Avoid Rust changes for Phase 9 unless blocker appears. |
| rustc | Optional native adapter regression tests if Rust files change | ✓ | rustc 1.95.0 at `/Users/trenton/.cargo/bin/rustc` [VERIFIED: Bash tool probe 2026-05-05] | Avoid Rust changes for Phase 9 unless blocker appears. |
| Emscripten `emcc` | Real WASM/browser build validation | ✗ | — [VERIFIED: Bash command -v emcc 2026-05-05] | Use fake Emscripten FS/IDBFS Vitest contract tests; real browser/WASM validation remains Phase 10 or environment setup. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |

**Missing dependencies with no fallback:** None for Phase 9's planned fake-tested TypeScript helper work. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]

**Missing dependencies with fallback:**
- `emcc` is missing; fallback is package-local fake Emscripten FS/IDBFS tests and documentation, because real browser E2E is deferred. [VERIFIED: Bash command -v emcc 2026-05-05] [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Phase 9 has no authentication boundary. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| V3 Session Management | no | Runtime lifecycle is process-global native service state, not user auth session management. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/08-typescript-bridge-and-runtime-package/08-CONTEXT.md] |
| V4 Access Control | no | Phase 9 does not add multi-user authorization policy. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| V5 Input Validation | yes | Strict schema/dictionary logical ID validation and deterministic missing-asset errors. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] |
| V6 Cryptography | no | Phase 9 does not introduce crypto, secrets, tokens, or encryption. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| V12 File and Resources | yes | Treat schema/dictionary IDs as logical IDs, reject path traversal/separators, and write only caller-approved virtual FS paths. [VERIFIED: /Users/trenton/Projects/yune/.planning/PROJECT.md] [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs] |

### Known Threat Patterns for TypeScript + Emscripten virtual filesystem

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal through schema/dictionary ID | Tampering | Reject anything outside nonempty `[A-Za-z0-9_-]+` before joining virtual paths. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs] |
| Silent stale persisted state after sync failure | Tampering / Repudiation | Convert `syncfs` callback errors into deterministic TypeScript errors and document stale/no-persisted-data state. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Placeholder asset masking missing or malicious config | Tampering | Require explicit caller-provided assets and fail missing files visibly. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |
| Over-broad native API expansion | Elevation of Privilege / Tampering | Keep `yune_typeduck_*` export list unchanged unless a focused blocker proves necessity. [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt] |
| Multiple active runtime confusion | Denial of Service / Tampering | Preserve one active process-global service constraint and avoid multi-instance path isolation promises. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md] |

## Sources

### Primary (HIGH confidence)

- `/Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md` — locked Phase 9 decisions, boundaries, non-goals, and canonical references.
- `/Users/trenton/Projects/yune/.planning/ROADMAP.md` — Phase 9 goal, success criteria, and planned slices.
- `/Users/trenton/Projects/yune/.planning/REQUIREMENTS.md` — `TYPEDUCK-FS-01` through `TYPEDUCK-FS-04` requirements.
- `/Users/trenton/Projects/yune/.planning/PROJECT.md` — compatibility, security, local-first, process-global, and AI deferral constraints.
- `/Users/trenton/Projects/yune/.planning/phases/08-typescript-bridge-and-runtime-package/08-CONTEXT.md` — Phase 8 handoff and explicit filesystem deferral.
- `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/typeduck.ts` — existing TypeDuck runtime lifecycle wrapper.
- `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src/module.ts` — existing Emscripten native binding interface and export list.
- `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/test/fake-module.ts` — fake module testing pattern.
- `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/package.json` — package-local TypeScript/Vitest scripts and dependencies.
- `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/tsconfig.json` — strict NodeNext build configuration.
- `/Users/trenton/Projects/yune/docs/typeduck-web-adapter.md` — existing browser filesystem and adapter contract docs.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs` — native browser asset guard, schema/dictionary validation, deploy/customize exports.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs` — native tests for missing assets, wrong dictionary, path-like schema ID, and successful init.
- `/Users/trenton/Projects/yune/scripts/typeduck-exports.txt` — canonical TypeDuck export list.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/deployment.rs` — deployed config freshness and sync-facing operations.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/session.rs` — userdb learning mutation boundary.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs` — logical ID/path traversal security model.
- [Emscripten File System API](https://emscripten.org/docs/api_reference/Filesystem-API.html) — `FS.mkdirTree`, `FS.writeFile`, `FS.readFile`, `FS.analyzePath`, `FS.mount`, `FS.syncfs`, IDBFS.
- [Emscripten File Systems Overview](https://emscripten.org/docs/porting/files/file_systems_overview.html) — virtual FS, MEMFS, IDBFS overview.
- npm registry — TypeScript 6.0.3 and Vitest 4.1.5 version verification.

### Secondary (MEDIUM confidence)

- None required; official docs and project source covered the implementation domain.

### Tertiary (LOW confidence)

- Assumptions listed in the Assumptions Log about exact helper names, optional combined init helper, and userdb sync cadence.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — package dependencies were verified from `package.json` and npm registry, and existing test/build commands passed. [VERIFIED: npm registry] [VERIFIED: Bash npm test/build 2026-05-05]
- Architecture: HIGH — phase boundaries and ownership are locked in Phase 9/Phase 8 context and match existing TypeScript/native adapter code. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/09-browser-filesystem-and-persistence/09-CONTEXT.md]
- Pitfalls: HIGH for sync direction, asset guard, logical ID validation, and native lifecycle; MEDIUM for exact userdb sync cadence because current exports do not expose mutation notifications. [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html#FS.syncfs] [VERIFIED: /Users/trenton/Projects/yune/scripts/typeduck-exports.txt]

**Research date:** 2026-05-05  
**Valid until:** 2026-06-04 for project-local architecture; 2026-05-12 for npm/package version currency.
