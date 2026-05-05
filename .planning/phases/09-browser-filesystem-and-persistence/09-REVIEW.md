---
status: issues_found
phase: 09-browser-filesystem-and-persistence
review_depth: standard
finding_count: 1
reviewed: 2026-05-05T00:00:00Z
files_reviewed: 5
files_reviewed_list:
  - packages/yune-typeduck-runtime/src/filesystem.ts
  - packages/yune-typeduck-runtime/src/index.ts
  - packages/yune-typeduck-runtime/test/fake-filesystem.ts
  - packages/yune-typeduck-runtime/test/filesystem.test.ts
  - docs/typeduck-web-adapter.md
findings:
  critical: 0
  high: 0
  medium: 0
  low_warning: 1
---

# Phase 09: Code Review Report

**Reviewed:** 2026-05-05T00:00:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Reviewed the Phase 09 TypeDuck runtime browser filesystem/persistence helpers, their package export, fake filesystem test support, filesystem tests, and adapter documentation. The recent fixes for mkdir-only absolute path creation, synchronous `FS.syncfs` throw wrapping, and dictionary/schema documentation were present. I found one real robustness defect in the persistence mount helper: mount failures from Emscripten are allowed to escape as arbitrary backend errors instead of the package's deterministic filesystem error type.

## Critical Issues

No critical issues found.

## High Issues

No high issues found.

## Medium Issues

No medium issues found.

## Low/Warning Issues

### LW-01: Persistence mount backend failures escape as non-deterministic errors

**File:** `packages/yune-typeduck-runtime/src/filesystem.ts:147-158`

**Impact:** `mountTypeDuckPersistence` creates the mountpoint and checks that `FS.mount` exists, but it calls `fs.mount(...)` without wrapping synchronous backend failures. In Emscripten, `FS.mount(IDBFS, ...)` can throw for an invalid backend, duplicate mount, or bad mountpoint state. Those errors currently escape as raw backend-specific values, unlike `syncTypeDuckFilesystem`, which normalizes sync failures into `TypeDuckFilesystemError` with stable package-level semantics. This degrades caller error handling and contradicts the helper contract documented in `docs/typeduck-web-adapter.md` that filesystem setup errors surface as deterministic wrapper/helper errors.

**Fix recommendation:** Wrap `fs.mount` in `TypeDuckFilesystemError` while preserving the original error as `cause`, for example:

```typescript
export function mountTypeDuckPersistence(
  fs: TypeDuckFilesystem,
  type: unknown,
  opts: Record<string, unknown>,
  mountpoint: string,
): void {
  ensureTypeDuckDirectory(fs, mountpoint);
  if (fs.mount === undefined) {
    throw new TypeDuckFilesystemError("Emscripten FS.mount is unavailable");
  }
  try {
    fs.mount(type, opts, mountpoint);
  } catch (error) {
    throw new TypeDuckFilesystemError("TypeDuck persistence mount failed", { cause: error });
  }
}
```

Add a test using `FakeTypeDuckFilesystem.mountError` to assert the thrown error is a `TypeDuckFilesystemError` with the stable message.

---

_Reviewed: 2026-05-05T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
