# yune-web Integration Layer

This directory contains the Yune-owned integration layer that bridges the upstream-derived Actions interface to the repository-owned `@yune-ime/typeduck-runtime` package.

## Purpose

Replaces the upstream TypeDuck-Web librime/WASM binding with Yune runtime helpers while preserving the app's UI, worker queue, and Actions interface.

## Components

### adapter.ts

Primary seam adapter that:

- Imports `TypeDuckRuntime`, `keyEventToRimeKey`, filesystem/persistence helpers from `@yune-ime/typeduck-runtime` per D-04
- Preserves one active runtime per Emscripten Module per D-05
- Translates `TypeDuckResponse` to upstream `RimeResult` shape
- Parses upstream key sequence strings to keyboard event-like objects
- Enforces explicit sync boundaries (before init, after commit/deploy/customize)
- Provides deterministic cleanup on worker teardown

### assets.ts

Explicit asset contract that:

- Requires yune-web-owned `default.yaml`, schema YAML, and dictionary YAML
- Validates no synthetic/fake/substitute YAML content per D-06
- Fails visibly when assets are absent
- Documents asset sources before runtime init

### package-alias.md

Documents local package resolution for `@yune-ime/typeduck-runtime` without publishing.

## Integration Strategy

### Patch Scope (Per D-03)

Minimal patch touches only documented seam/config files:

- `src/worker.ts` — Replace librime WASM import with Yune runtime adapter
- `src/rime.ts` — Main-thread facade plus debug-only persistence diagnostics
- Preserve `src/types.ts` — Actions interface unchanged
- Preserve UI components — No visual/behavioral changes

### Lifecycle (Per D-05)

1. Worker loads Yune Emscripten Module (provides `yune_typeduck_*` exports)
2. Adapter initializes one `TypeDuckRuntime` instance
3. All actions reuse same runtime instance
4. Cleanup called before worker teardown or re-initialization
5. One active runtime per Module enforced at adapter level

### Persistence Pattern (Per D-04, Phase 9)

- `syncFromPersistenceBeforeInit(fs)` before runtime init
- `syncToPersistenceAfterMutation(fs)` after commit/deploy/customize with a
  browser-visible reason marker
- `runtime.deploy()` followed by reason-tagged sync for deploy action
- `runtime.customize(...)` followed by reason-tagged sync for customize action
- `syncAfterUserDataChange(fs)` after candidate delete

### Asset Contract (Per D-06)

Required assets before init:

- `default.yaml` — RIME default configuration
- `${schemaId}.schema.yaml` — Schema definition
- `${dictionaryId}.dict.yaml` — Dictionary source

All assets must be explicit yune-web-owned YAML files. Missing assets fail visibly at filesystem preparation.

## Contract Mismatches

### String Input vs Keycode/Mask

**Upstream**: `processKey(input: string)` sends sequences like `{BackSpace}`, `a`, `{Release+Enter}`

**Yune**: `processKeyboardEvent(event)` or `processKey(keycode, mask)` uses integer codes

**Adapter**: Parses string sequences to `TypeDuckKeyboardEventLike`, delegates to `keyEventToRimeKey`

### RimeResult vs TypeDuckResponse

**Upstream**: `RimeResult` with `isComposing`, `inputBuffer`, `candidates`, `page`, `committed`

**Yune**: `TypeDuckResponse` with `handled`, `commits`, `context.preedit`, `context.candidates`

**Adapter**: Translates Yune response fields to upstream RimeResult shape

### setOption

**Upstream**: `Actions.setOption(option: string, value: boolean)` sets session options

**Yune**: `TypeDuckRuntime.setOption` delegates to the `yune_typeduck_set_option` export

**Adapter**: Forwards upstream option toggles directly to the active Yune runtime

## Patch Application Instructions

### Prerequisites

1. Clone upstream TypeDuck-Web to `apps/yune-web/source`
2. Build Yune WASM artifact (provides `yune_typeduck_*` exports and Module)
3. Build `@yune-ime/typeduck-runtime` package
4. Alias local package per package-alias.md

### Apply Patch

```bash
# From repository root
cd apps/yune-web/source

# Apply Yune seam patch
git apply ../patches/yune-web-runtime.patch

# Verify patch applied
git status
```

### Configure Assets

Provide explicit YAML assets before worker init:

1. Create or fetch `default.yaml`
2. Create or fetch schema YAML (e.g., `luna_pinyin.schema.yaml`)
3. Create or fetch dictionary YAML (e.g., `luna_pinyin.dict.yaml`)
4. Configure asset loader in patched worker

### Build and Run

```bash
# Install dependencies (upstream uses Bun)
bun install

# Build worker
bun run worker

# Start development server
bun run start
```

## Lifecycle Hooks

### Worker Initialization

Patch replaces upstream `loadRime()` with:

```typescript
import { initYuneRuntime } from "../yune-integration/adapter.js";
import { loadExplicitAssets, validateExplicitAssets } from "../yune-integration/assets.js";

// Before Module init
const assets = await loadExplicitAssets(explicitAssetsConfig);
validateExplicitAssets(assets);

await initYuneRuntime(Module, FS, initOptions, assets, dictionaryId);
```

### Action Handlers

Patch replaces `Module.ccall` calls with adapter exports:

```typescript
import {
  processKey,
  selectCandidate,
  deleteCandidate,
  flipPage,
  deploy,
  customize,
  setOption,
} from "../yune-integration/adapter.js";

// processKey(input) instead of Module.ccall("process_key", ...)
// selectCandidate(index) instead of Module.ccall("select_candidate", ...)
// deploy() instead of Module.ccall("deploy", ...)
```

### Worker Teardown

Patch adds cleanup before worker termination:

```typescript
import { cleanupYuneRuntime } from "../yune-integration/adapter.js";

// Before worker unload
cleanupYuneRuntime();
```

## Known Gaps

### Remaining Adapter Widening Needed

Keep future widening evidence-driven per D-07:

- What upstream flow calls the missing behavior
- What Yune API surface lacks
- What native change is the smallest possible

### Upstream Build Tooling

yune-web uses the upstream Bun tooling. If Bun unavailable, document:

- Missing executable
- Install hint from upstream docs
- Fallback evidence (npm build attempt)

### Asset Discovery

The upstream-derived shell may expect specific asset paths or CDN URLs. Document:

- Required asset locations
- Missing asset discovery mechanism
- Yune-side workaround (explicit loader)

## Deferred Items (Per D-14)

Explicitly NOT part of this integration:

- AI-native provider calls, candidate generation, ranking policy
- AI-native context capture, memory, privacy controls
- New first-party Yune graphical frontend
- Multi-instance Yune/RIME service isolation
- Browser CDN/cache/service worker/storage quota policy

## Next Steps

After patch application:

1. Run upstream build/typecheck to verify compilation
2. Test browser E2E flows (composition, candidates, deploy, customize, persistence)
3. Document any remaining blockers in findings
4. Recommend AI-native frontend readiness (go/no-go with conditions)

---

**Phase**: 10-yune-web-app-integration-and-e2e
**Plan**: 10-02 (Yune seam patch/configuration layer)
**Status**: Integration layer created, pending patch application and build gates
