---
phase: 08-typescript-bridge-and-runtime-package
reviewed: 2026-05-04T00:00:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - docs/typeduck-web-adapter.md
  - packages/yune-typeduck-runtime/package.json
  - packages/yune-typeduck-runtime/src/index.ts
  - packages/yune-typeduck-runtime/src/keys.ts
  - packages/yune-typeduck-runtime/src/module.ts
  - packages/yune-typeduck-runtime/src/response.ts
  - packages/yune-typeduck-runtime/src/typeduck.ts
  - packages/yune-typeduck-runtime/test/fake-module.ts
  - packages/yune-typeduck-runtime/test/keys.test.ts
  - packages/yune-typeduck-runtime/test/response.test.ts
  - packages/yune-typeduck-runtime/test/typeduck.test.ts
  - packages/yune-typeduck-runtime/tsconfig.json
findings:
  critical: 1
  warning: 1
  info: 0
  total: 2
status: issues_found
---

# Phase 8: Code Review Report

**Reviewed:** 2026-05-04T00:00:00Z
**Depth:** standard
**Files Reviewed:** 12
**Status:** issues_found

## Summary

Reviewed the Phase 8 TypeScript runtime package, adapter documentation, and package-local tests. Lockfile and `.gitignore` were read for context but excluded from source review per review-scope filtering. The package has a release-blocking module-resolution defect in its emitted ESM output, plus an adapter-boundary key-mask mismatch that makes browser meta-key combinations fail against the native runtime.

## Critical Issues

### CR-01: Published ESM cannot be imported by Node-compatible consumers

**File:** `packages/yune-typeduck-runtime/src/index.ts:1-4` and `packages/yune-typeduck-runtime/src/typeduck.ts:1-2`

**Issue:** The package declares `"type": "module"` and `"main": "dist/index.js"`, but source imports/exports omit `.js` extensions while `tsconfig.json` emits plain ES modules. `tsc -p tsconfig.json` therefore produces `dist/index.js` with `export * from './module';` and `dist/typeduck.js` with `import ... from "./keys";`. Node ESM resolution does not add extensions, so importing the built package fails with `ERR_MODULE_NOT_FOUND` for `dist/module`. This makes the documented `import { TypeDuckRuntime, keyEventToRimeKey } from "@yune-ime/typeduck-runtime"` unusable in Node-compatible ESM package consumers and many strict bundler/test environments.

**Fix:** Use Node ESM-compatible TypeScript resolution and include `.js` in relative source specifiers, or introduce a bundler that rewrites specifiers before publishing. For the current no-bundler package, prefer:

```json
{
  "compilerOptions": {
    "module": "NodeNext",
    "moduleResolution": "NodeNext"
  }
}
```

and update source specifiers, for example:

```typescript
export * from "./module.js";
export * from "./response.js";
export * from "./typeduck.js";
export * from "./keys.js";
```

```typescript
import { keyEventToRimeKey, type TypeDuckKeyboardEventLike } from "./keys.js";
import { bindTypeDuckModule, type EmscriptenTypeDuckModule, type TypeDuckBindings } from "./module.js";
import { readTypeDuckResponse, type TypeDuckResponse } from "./response.js";
```

Apply the same `.js` suffix pattern to all relative imports in `src/response.ts` and tests as required by the chosen module mode.

## Warnings

### WR-01: Browser meta key maps to an unsupported native mask

**File:** `packages/yune-typeduck-runtime/src/keys.ts:109-110`

**Issue:** `event.metaKey` is mapped to `RIME_MASK.Meta` (`1 << 28`), but the native Yune/RIME adapter currently accepts only Shift, Lock, Control, Alt, Super, and Release in `key_modifiers_from_rime_mask`; masks containing bit 28 are rejected as unsupported before they can become `KeyModifiers`. As a result, common browser Command/Windows-key combinations produce a mask that `yune_typeduck_process_key` returns as unhandled instead of behaving like the native Super modifier path. This breaks documented browser key processing for meta-key events.

**Fix:** Map DOM `metaKey` to the supported native Super mask unless the native adapter is expanded to accept Meta/Hyper bits end-to-end:

```typescript
if (event.metaKey === true) {
  mask |= RIME_MASK.Super;
}
```

Also update the corresponding tests to expect `RIME_MASK.Super` for `metaKey` and add an integration-style assertion that `processKeyboardEvent({ key: "x", metaKey: true })` forwards the supported mask.

---

_Reviewed: 2026-05-04T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
