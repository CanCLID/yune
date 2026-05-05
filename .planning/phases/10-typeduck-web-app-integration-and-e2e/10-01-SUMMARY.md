---
phase: 10-typeduck-web-app-integration-and-e2e
plan: 01
subsystem: typeduck-web-integration
tags: [upstream-source, seam-inspection, integration-metadata, documentation]
dependencies:
  requires: []
  provides:
    - reproducible TypeDuck-Web checkout metadata
    - upstream seam file documentation
    - integration gap analysis
  affects:
    - Phase 10 Plan 02 (seam replacement)
    - Phase 10 Plan 03 (browser E2E)
tech_stack:
  added:
    - git clone/pin workflow
    - upstream source metadata tracking
    - seam inspection documentation pattern
  patterns:
    - exclude upstream source from repository commits
    - document seam before patching
    - separate facts from assumptions
key_files:
  created:
    - third_party/typeduck-web/.gitignore
    - third_party/typeduck-web/README.yune-source.md
    - third_party/typeduck-web/typeduck-web.lock.json
    - docs/typeduck-web-integration-findings.md
  modified: []
decisions:
  - Clone upstream TypeDuck-Web to third_party/typeduck-web/source instead of vendoring
  - Exclude upstream source directory from repository commits via .gitignore
  - Document seam files before any source patching per D-02/D-03
metrics:
  duration: "4m 55s"
  tasks_completed: 2
  files_created: 4
  commits: 2
---

# Phase 10 Plan 01: Upstream TypeDuck-Web Source Handling Summary

**Status**: COMPLETE
**Execution Time**: 4m 55s
**Commits**: 2

## One-Liner

Created reproducible upstream TypeDuck-Web checkout metadata and documented the librime/WASM seam structure before Yune adapter integration.

## Summary

Plan 10-01 established a reproducible local TypeDuck-Web source handling path and inspected the current librime/WASM seam before any replacement work. Task 1 cloned upstream TypeDuck-Web to third_party/typeduck-web/source, pinned to commit 03f9afd2cf6ca75653197f2193f24d1cd0adbd83, and documented clone/refresh/setup commands in README and machine-readable lock JSON. Task 2 inspected 6 key seam files (worker.ts, rime.ts, types.ts, api.cpp, build_wasm.ts, CandidatePanel.tsx), documented the exact call flow from UI keyboard events through worker Module.ccall to native librime exports, and identified integration contract gaps between upstream and Yune adapters.

## Tasks Completed

### Task 1: Clone or refresh a pinned upstream TypeDuck-Web checkout

**Commit**: 3cd5075

**Actions**:
- Created third_party/typeduck-web directory structure
- Cloned upstream repository https://github.com/TypeDuck-HK/TypeDuck-Web.git
- Pinned to commit 03f9afd2cf6ca75653197f2193f24d1cd0adbd83 (main branch)
- Documented clone/refresh/setup commands in README.yune-source.md
- Added machine-readable metadata to typeduck-web.lock.json
- Excluded upstream source directory from repository commits via .gitignore

**Files Created**:
- third_party/typeduck-web/.gitignore — excludes source/ checkout from commits
- third_party/typeduck-web/README.yune-source.md — clone instructions, revision, setup command
- third_party/typeduck-web/typeduck-web.lock.json — upstream metadata (url, revision, seamFiles)

**Verification**: source/.git exists, HEAD readable, URL/revision fields present in README and lock JSON.

### Task 2: Inspect and document the existing TypeDuck-Web librime/WASM seam

**Commit**: a521d0e

**Actions**:
- Searched upstream source for seam indicators (librime, wasm, Module, cwrap, ccall, Rime)
- Read 6 key seam files: worker.ts, rime.ts, types.ts, api.cpp, build_wasm.ts, CandidatePanel.tsx
- Documented exact Module initialization, FS/IDBFS persistence, Action calls, notification callbacks
- Identified call flow: UI keydown → Rime.processKey → Worker queue → Module.ccall → Native exports → Librime
- Analyzed contract gaps: string input vs keycode/mask, RimeResult vs TypeDuckResponse, missing setOption
- Recorded persistence timing: syncfs(true) before init, syncfs(false) after commit/deploy
- Added seam files list to typeduck-web.lock.json
- Confirmed upstream source remains unpatched

**Files Created**:
- docs/typeduck-web-integration-findings.md — Plan 10-01 seam inspection section, call flow, gap analysis

**Files Modified**:
- third_party/typeduck-web/typeduck-web.lock.json — added seamFiles array

**Verification**: Plan 10-01 section exists, seam keywords present, seamFiles in lock JSON, no @yune-ime/typeduck-runtime in upstream source.

## Deviations from Plan

None — plan executed exactly as written.

## Key Findings

### Primary Seam: src/worker.ts

Worker implementation is the main replacement target. It:
- Loads Emscripten-generated rime.js via importScripts
- Initializes Module with FS/IDBFS mounted at /rime
- Calls Module.ccall for all actions (init, set_option, process_key, select_candidate, delete_candidate, flip_page, customize, deploy)
- Returns parsed JSON RimeResult to main thread

### Secondary Seam: src/rime.ts

Main-thread facade creates Worker and queues action calls. Preserve this layer; only patch worker internals.

### Contract Gaps

1. **String input vs keycode/mask**: Upstream processKey accepts string sequences like `{BackSpace}`; Yune uses keycode/mask mapping from KeyboardEvent.key
2. **RimeResult vs TypeDuckResponse**: Different response shapes require adapter translation layer in worker
3. **Missing setOption**: Upstream Actions.setOption not present in current Yune TypeDuck wrapper

### Persistence Pattern

Upstream uses explicit FS.syncfs boundaries:
- syncfs(true) before init (populate from IDBFS)
- syncfs(false) after commit/deploy (flush to IDBFS)

Yune Phase 9 helpers match this pattern: syncFromPersistenceBeforeInit, syncToPersistenceAfterMutation, deployAndSync.

### Unpatched Source

Confirmed: grep -R "@yune-ime/typeduck-runtime" found no Yune package references in upstream source.

## Threat Surface

No new threat surface introduced in this plan. Seam inspection is documentation-only; no source patching or build execution performed.

## Deferred Items (Per D-14)

Explicitly deferred per locked decisions:
- AI-native provider calls, candidate generation, ranking policy
- AI-native context capture, memory, privacy controls
- New first-party Yune graphical frontend
- Multi-instance Yune/RIME service isolation

## Next Steps

Plan 10-02 will:
- Implement minimal seam replacement using @yune-ime/typeduck-runtime
- Patch src/worker.ts to load Yune Emscripten artifact and call TypeDuckRuntime methods
- Add adapter layer translating TypeDuckResponse to RimeResult
- Preserve Actions interface and listener events
- Document remaining blockers if WASM artifact unavailable

## Self-Check

### Files Verified

```bash
[ -f "third_party/typeduck-web/.gitignore" ] && echo "FOUND: .gitignore" || echo "MISSING: .gitignore"
[ -f "third_party/typeduck-web/README.yune-source.md" ] && echo "FOUND: README" || echo "MISSING: README"
[ -f "third_party/typeduck-web/typeduck-web.lock.json" ] && echo "FOUND: lock JSON" || echo "MISSING: lock JSON"
[ -f "docs/typeduck-web-integration-findings.md" ] && echo "FOUND: findings" || echo "MISSING: findings"
```

Expected: All FOUND.

### Commits Verified

```bash
git log --oneline --all | grep -q "3cd5075" && echo "FOUND: Task 1 commit" || echo "MISSING: Task 1 commit"
git log --oneline --all | grep -q "a521d0e" && echo "FOUND: Task 2 commit" || echo "MISSING: Task 2 commit"
```

Expected: Both FOUND.

## Execution Complete

**Plan**: 10-01 (Upstream TypeDuck-Web source handling)
**Tasks**: 2/2 complete
**Commits**: 3cd5075 (Task 1), a521d0e (Task 2)
**Success Criteria**: All verification checks passed, seam documented, upstream unpatched.

---
**Completed**: 2026-05-05T15:08:50Z

## Self-Check: PASSED

All files verified FOUND. All commits verified FOUND.