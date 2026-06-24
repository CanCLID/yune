# M27 TypeDuck-Web Startup Runtime Init Implementation Plan

> **Status:** Complete - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** archived execution plan
>
> **Closeout:** M27 is complete. Evidence is recorded under `apps/yune-web/e2e/results/m27-startup-runtime/`, including native startup owner spans, Windows working-set metrics, browser fresh/reload evidence, control classification evidence, TypeDuck-Web patch checks, and `task-5-gates.md`.
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the measured TypeDuck-Web startup owner that M26 identified after browser assets load, and make TypeDuck-Web engine-control toggles honest about live versus deploy-time behavior.

**Architecture:** M27 is attribution-first but not measurement-only. First reconcile which native path the browser actually pays: cold deploy/build versus deploy-cache-hit/load-precompiled plus in-memory index build. Then split that browser-paid native path into named owners, add a real process-memory metric, map the browser startup wait back to those owners, and fix the evidenced startup bottleneck before closeout. Browser evidence remains the user-visible check; native startup rows are the source of truth for engine/runtime ownership.

**Tech Stack:** Rust (`yune-rime-api` benchmark harness and startup path), TypeScript runtime (`packages/yune-typeduck-runtime`), TypeDuck-Web worker diagnostics, Playwright startup evidence, existing TypeDuck-Web patch workflow, and Windows process working-set sampling in the benchmark harness.

---

## Why This Milestone Exists

M26 made performance measurable but deliberately deferred the biggest owner:

- Browser startup after M26 still takes about `10.3-10.7s`.
- M26 browser markers show assets finish in about `80-125ms`, then the app waits until `schema:select` / `runtime:init` finishes.
- Native `startup_real_jyut6ping3_mobile_runtime_ready` after M26 is about `15.1s` median, while `startup_real_luna_pinyin_runtime_ready` is about `0.79s` median.
- M26 could not measure memory/RSS. That was acceptable for the dynamic-correction prune, but it is not acceptable for a startup milestone because the suspected owners are clone-heavy dictionary parse, translator index build, compiled-data fallback, and eager resource initialization.

The raw M26 numbers are not directly comparable until M27 proves which startup path the browser pays. The native `startup_real_jyut6ping3_mobile_runtime_ready` row appears to include cold asset write/deploy/build work inside the benchmark loop, while the browser fresh/reload timings are both around `10.5s`, which suggests the browser is paying the deploy-cache-hit/load-precompiled path plus in-memory table/prism/index setup. M27 must not optimize a cold-build path unless Task 1 proves that path is actually part of the browser wait.

One post-M26 TypeDuck-Web interaction issue stays in M27 because it is the same startup/deploy cost seen from a control surface:

- Toggling engine controls such as auto-correction, AI candidates, or auto-completion shows `載入中 Loading...` today. M27 must classify each control as live, deploy-time, or browser-only before changing behavior. AI candidates should be live/browser-only. Options such as auto-correction, auto-completion, and sentence composition may legitimately be deploy-time customize options, so M27 should reduce their deploy/reload cost and avoid misleading full-engine-loading UI rather than pretending they are live `setOption` controls unless the engine actually supports that.

The long-sentence partial-selection issue is not M27 scope. Current engine code commits the whole input span in `Engine::commit_candidate`, so typing `caksijathaacoenggeoizi` and selecting `測` can produce `測sijathaacoenggeoizi`. That is an engine-side segment-aware partial-commit gap, not a browser insertion bug, and it needs TypeDuck v1.1.2 oracle capture plus native tests before implementation. It is tracked separately by M28: [`m28-plan-typeduck-partial-selection.md`](../m28-plan-typeduck-partial-selection.md).

M27 exists to close these concrete points:

1. Reconcile the browser-paid startup path against native benchmark rows.
2. Identify which browser-paid native startup sub-owner actually costs the time.
3. Measure its memory cost with real Windows process metrics.
4. Map the browser startup wait back to native owners without mixing browser and native claims.
5. Fix the evidenced startup bottleneck with behavior-preserving changes and before/after evidence.
6. Classify engine controls as live, deploy-time, or browser-only and fix the misleading or excessive reload behavior accordingly.

## Scope

In scope:

- Native startup-path reconciliation for `jyut6ping3_mobile`: cold deploy/build versus deploy-cache-hit/load-precompiled plus in-memory index setup.
- Native startup sub-attribution for the browser-paid `jyut6ping3_mobile` path and a comparison `luna_pinyin` path.
- A hard memory metric: Windows working set / peak working set in the native benchmark harness. If a non-Windows platform runs the benchmark, it may report `unavailable`, but M27 cannot close without checked-in Windows memory evidence.
- Browser startup markers and Playwright evidence before and after optimization.
- Browser evidence for live-vs-deploy engine controls, including AI candidates, auto-correction, and auto-completion.
- Measured startup optimization against the evidenced bottleneck. M27 cannot close by deferring the top owner and landing only a smaller unrelated slice; any split into a successor milestone requires an explicit user decision after the Task 3 evidence review.
- TypeDuck-Web patch regeneration and reverse/forward checks if `apps/yune-web/source/` changes.

Out of scope:

- Widening `RimeApi`, `RimeCandidate`, TypeDuck profile ABI slots, or public `yune_typeduck_*` exports only for diagnostics.
- Replacing the dictionary storage format without a separate design review.
- Treating browser startup numbers as native engine numbers.
- Segment-aware partial candidate commit and remaining-composition repartition. That is M28 engine work with TypeDuck v1.1.2 oracle fixtures, not M27 performance work.
- Reopening M24/M25 dogfood ledgers or M26 performance closeout.
- Windows product/frontend work. P2-WIN-02 and P2-WIN-01 remain separate tracks.

## Evidence From M26

Use these M26 files as the baseline:

- `apps/yune-web/e2e/results/m26-performance/native-after.md`
- `apps/yune-web/e2e/results/m26-performance/startup-attribution-after.json`
- `apps/yune-web/e2e/results/m26-performance/optimization-choice.md`
- `apps/yune-web/e2e/results/m26-performance/task-5-gates.md`

Important M26 numbers:

- Browser fresh startup after M26: `startup:complete.totalMs` about `10.7s`; assets finish around `125ms`.
- Browser reload startup after M26: about `10.4s`.
- Native `startup_real_jyut6ping3_mobile_runtime_ready` after M26: median about `15.1s`, p95 about `15.6s`.
- Native `startup_real_luna_pinyin_runtime_ready` after M26: median about `0.79s`, p95 about `0.80s`.
- Treat the `15.1s` row as a suspicious cold-build benchmark until Task 1 proves whether it matches the browser's `schema:select`/`runtime:init` path.

## Acceptance Gates

M27 is complete only when all of these are true:

- `M27-STARTUP-01`: Native startup benchmarks first reconcile whether the browser-paid path is cold deploy/build or deploy-cache-hit/load-precompiled. The benchmark then splits the browser-paid `jyut6ping3_mobile` startup path into at least `setup`, `initialize`, `create_session`, `select_schema_total`, `schema_config_load`, `processor_install`, `translator_install`, `compiled_table_load`, `compiled_prism_load`, `source_dictionary_parse_if_any`, `translator_index_build`, `filter_install`, `userdb_open_or_sync`, and `teardown_or_finalize` where the current code can observe those spans without ABI/export widening.
- `M27-STARTUP-02`: Native startup evidence includes a real Windows memory metric for each startup sample: working set bytes and peak working set bytes, or a documented equivalent process memory metric. This is a hard gate; M27 cannot close with only "RSS unavailable".
- `M27-STARTUP-03`: Browser startup evidence still records fresh and reload paths, preserves the existing M26 markers, and adds any M27 markers needed to map browser `schema:select` to native owners.
- `M27-STARTUP-04`: The evidenced top startup owner is fixed or materially reduced by behavior-preserving change(s), with native before/after timing, memory before/after, and browser fresh/reload before/after evidence. M27 cannot close by deferring the top owner and landing only a lower-risk unrelated slice; if the evidence proves the top owner requires a separate milestone, pause after Task 3 for explicit user approval before splitting scope.
- `M27-STARTUP-05`: Compatibility and integration gates remain green:
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p yune-core --test cantonese_parity`
  - `cargo test -p yune-core --test upstream_luna_pinyin_parity`
  - `cargo test -p yune-rime-api --test typeduck_web`
  - `cargo test --workspace`
  - `cargo bench -p yune-rime-api --bench frontend_baselines`
  - `npm.cmd --prefix packages/yune-typeduck-runtime test`
  - `npm.cmd --prefix packages/yune-typeduck-runtime run build`
  - `npm.cmd --prefix apps/yune-web/source run build`
  - focused Playwright startup evidence
  - focused Playwright control-classification evidence
  - TypeDuck-Web patch reverse/forward checks if source changed
  - `git diff --check`
- `M27-STARTUP-06`: Engine controls are classified and evidenced by control type. Live/browser-only controls such as AI candidates must not trigger `TypeDuckRuntime.init`, `selectSchema`, deploy, or visible `載入中 Loading...`. Deploy-time controls such as auto-correction and auto-completion must either benefit from the M27 deploy/startup speedup or show a scoped deploy/reload state rather than a misleading full-engine-loading loop. Browser evidence must record the marker sequence for each control class.

## Files And Responsibilities

- `crates/yune-rime-api/benches/frontend_baselines.rs`: native startup sub-attribution, Windows memory sampling, before/after benchmark rows, evidence-friendly output.
- `crates/yune-rime-api/src/schema_selection.rs`: expected owner for zero-cost internal timing hooks if the benchmark harness cannot split owners sufficiently from call boundaries.
- `crates/yune-rime-api/src/schema_install.rs`: expected owner for zero-cost internal timing hooks and possible optimization owner for translator/filter/dictionary install, only after attribution.
- `crates/yune-core/src/dictionary/*`: possible optimization owner for dictionary parse/load or compiled-data use, only after attribution.
- `crates/yune-core/src/translator/mod.rs`: possible optimization owner for `entries` + `entries_by_code` storage/index build, only after attribution.
- `apps/yune-web/source/src/worker.ts`: browser startup markers if needed.
- `apps/yune-web/source/src/yune-integration/adapter.ts`: TypeDuckRuntime startup markers if needed.
- `apps/yune-web/source/src/Preferences.tsx`: likely owner for live engine-control toggle behavior.
- `apps/yune-web/source/src/hooks.ts`: likely owner for removing full-page/global loading from live option toggles.
- `apps/yune-web/e2e/yune-typeduck.spec.ts`: focused M27 Playwright startup evidence.
- `apps/yune-web/patches/yune-web-runtime.patch`: regenerated if TypeDuck-Web source changes.
- `apps/yune-web/e2e/results/m27-startup-runtime/`: new M27 evidence folder.

## Implementation Tasks

### Task 0 - Prepare The M27 Baseline

**Files:**

- Read: `docs/plans/m27-plan-typeduck-web-startup-runtime-init.md`
- Read: `docs/roadmap.md`
- Read: `docs/requirements.md`
- Read: `docs/CONVENTIONS.md`
- Read: `apps/yune-web/e2e/results/m26-performance/native-after.md`
- Read: `apps/yune-web/e2e/results/m26-performance/startup-attribution-after.json`

- [ ] Step 0.1: Confirm the branch and worktree.

Run:

```powershell
git status --short
git rev-parse --abbrev-ref HEAD
git fetch origin
git rev-parse HEAD
git rev-parse origin/main
```

Expected:

- Work is on the intended branch/worktree.
- Any unrelated dirty files are listed and left untouched.
- If `HEAD` differs from `origin/main`, record why before editing.

- [ ] Step 0.2: Copy M26 baseline numbers into M27 evidence.

Create `apps/yune-web/e2e/results/m27-startup-runtime/m27-baseline.md` with:

```markdown
# M27 Baseline

> **Status:** Captured from M26 - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

## M26 Startup Numbers

- Browser fresh startup after M26: `startup:complete.totalMs` about `10.7s`.
- Browser reload startup after M26: about `10.4s`.
- Native `startup_real_jyut6ping3_mobile_runtime_ready`: median about `15.1s`, p95 about `15.6s`.
- Native `startup_real_luna_pinyin_runtime_ready`: median about `0.79s`, p95 about `0.80s`.

## Rule

M27 must not close until native startup rows include a real Windows process memory metric.
```

Acceptance:

- The evidence folder exists.
- The file records M26 as baseline, not as a new measurement.

### Task 1 - Add Native Startup Sub-Attribution And Memory Sampling

**Files:**

- Modify: `crates/yune-rime-api/benches/frontend_baselines.rs`
- Modify if needed: `crates/yune-rime-api/src/schema_selection.rs`
- Modify if needed: `crates/yune-rime-api/src/schema_install.rs`
- Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/startup-path-reconciliation.md`
- Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/native-startup-before.md`

- [ ] Step 1.0: Reconcile cold-build vs browser-paid startup path.

Create `startup-path-reconciliation.md` before adding optimizations. It must compare:

- `run_real_startup_runtime_ready`: whether it writes schema assets and deploys inside the benchmark loop.
- `run_real_deploy_cache_hit`: whether it writes once and measures the deploy-cache-hit/load-precompiled path.
- Browser startup evidence: whether fresh and reload both record `deploy:cache-hit`, `deployCacheFresh`, `preserveDeployedAssets`, or equivalent markers.
- The browser `schema:select` interval: whether it maps to cold deploy/build, compiled `.bin` load, table/prism parse, translator index build, userdb open/sync, or browser-only worker/runtime work.

Required conclusion fields:

```markdown
## Reconciliation Result

- Browser-paid path: cold-deploy-build | deploy-cache-hit-load-precompiled | mixed | unknown
- Native benchmark row that matches browser path: exact row name
- Native benchmark row that must not drive browser optimization: exact row name or `none`
- Evidence for fresh browser path:
- Evidence for reload browser path:
- Startup owners to split next: exact owner list
```

Acceptance:

- M27 does not treat the `15.1s` `startup_real_jyut6ping3_mobile_runtime_ready` row as browser-comparable unless the reconciliation proves the browser pays that same path.
- If the browser pays the deploy-cache-hit/load-precompiled path, Task 1 attribution centers on that path, not cold deploy/build.
- `dictionary_parse_or_load` is split into at least `compiled_table_load`, `compiled_prism_load`, `source_dictionary_parse_if_any`, and `translator_index_build`.

- [ ] Step 1.1: Add benchmark-local startup span structs.

Implementation shape in `frontend_baselines.rs`:

```rust
#[derive(Clone, Debug)]
struct StartupSpan {
    name: &'static str,
    micros: u128,
    working_set_before: Option<u64>,
    working_set_after: Option<u64>,
    peak_working_set_after: Option<u64>,
}

#[derive(Clone, Debug)]
struct MemorySample {
    working_set_bytes: Option<u64>,
    peak_working_set_bytes: Option<u64>,
}
```

Expected:

- These structs are benchmark-local.
- No production `RimeApi`, `RimeCandidate`, or `yune_typeduck_*` export changes.

- [ ] Step 1.2: Add a Windows working-set sampler.

Implementation shape:

```rust
#[cfg(windows)]
fn current_memory_sample() -> MemorySample {
    use std::ffi::c_void;
    use std::mem;

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }

    #[link(name = "psapi")]
    extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    let mut counters = ProcessMemoryCounters {
        cb: mem::size_of::<ProcessMemoryCounters>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
    };
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            mem::size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    if ok == 0 {
        return MemorySample {
            working_set_bytes: None,
            peak_working_set_bytes: None,
        };
    }
    MemorySample {
        working_set_bytes: Some(counters.working_set_size as u64),
        peak_working_set_bytes: Some(counters.peak_working_set_size as u64),
    }
}

#[cfg(not(windows))]
fn current_memory_sample() -> MemorySample {
    MemorySample {
        working_set_bytes: None,
        peak_working_set_bytes: None,
    }
}
```

Acceptance:

- On Windows, M27 evidence must show numeric memory values.
- On non-Windows, the benchmark may print `unavailable`, but that cannot be used as final M27 closeout evidence.
- Per-span working-set deltas are treated as coarse evidence because allocator behavior can hide freed memory. The closeout must emphasize full-startup total delta and peak-working-set high-water mark.
- If `translator_index_build` or `entries_by_code` clone/index work is the selected owner, add a benchmark-local allocation counter or direct index-size/entry-count evidence so the clone/index cost is not inferred from working-set deltas alone.

- [ ] Step 1.3: Split the browser-paid native startup path.

Split the path identified by `startup-path-reconciliation.md`. Keep cold-build rows only as comparison rows unless the browser actually pays cold build. At minimum split around:

- `write_real_schema_assets`
- `setup_fixture`
- `create_session`
- `select_schema`
- `destroy_session`
- `reset_runtime`

If direct call boundaries do not expose enough detail, add a `#[cfg(any(test, feature = "bench-diagnostics"))]` or `#[doc(hidden)]` internal timing sink in `schema_selection.rs` / `schema_install.rs`. The mechanism must be zero-cost when disabled: no heap allocation, no timestamp reads, and no public ABI/export change in normal builds. Acceptable row names:

- `startup_trace_jyut6ping3_mobile_setup`
- `startup_trace_jyut6ping3_mobile_initialize`
- `startup_trace_jyut6ping3_mobile_create_session`
- `startup_trace_jyut6ping3_mobile_select_schema_total`
- `startup_trace_jyut6ping3_mobile_schema_config_load`
- `startup_trace_jyut6ping3_mobile_compiled_table_load`
- `startup_trace_jyut6ping3_mobile_compiled_prism_load`
- `startup_trace_jyut6ping3_mobile_source_dictionary_parse_if_any`
- `startup_trace_jyut6ping3_mobile_translator_index_build`
- `startup_trace_jyut6ping3_mobile_filter_install`
- `startup_trace_jyut6ping3_mobile_userdb_open_or_sync`
- `startup_trace_jyut6ping3_mobile_destroy_session`
- `startup_trace_jyut6ping3_mobile_finalize`
- matching `luna_pinyin` rows where practical

Acceptance:

- `cargo bench -p yune-rime-api --bench frontend_baselines` prints both existing M26 rows and new `startup_trace_*` rows.
- Each `startup_trace_*` row includes timing and memory notes.

- [ ] Step 1.4: Capture before evidence.

Run:

```powershell
cargo bench -p yune-rime-api --bench frontend_baselines
```

Save a summarized table to:

```text
apps/yune-web/e2e/results/m27-startup-runtime/native-startup-before.md
```

Required summary columns:

- row
- schema
- median_us
- p95_us
- working_set_before_bytes
- working_set_after_bytes
- peak_working_set_after_bytes
- memory_delta_bytes

Expected:

- The largest native startup owner is named by evidence.
- Memory values are numeric on Windows.

### Task 2 - Add Browser Startup M27 Evidence

**Files:**

- Modify if needed: `apps/yune-web/source/src/worker.ts`
- Modify if needed: `apps/yune-web/source/src/yune-integration/adapter.ts`
- Modify if needed: `apps/yune-web/source/src/Preferences.tsx`
- Modify if needed: `apps/yune-web/source/src/hooks.ts`
- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Patch if source changed: `apps/yune-web/patches/yune-web-runtime.patch`
- Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/browser-startup-before.json`
- Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/control-classification-before.json`

- [ ] Step 2.1: Preserve M26 markers and add M27 label.

The existing markers are:

- `runtime:init:start`
- `wasm-glue:loaded`
- `wasm:module:create:start`
- `wasm:module:create:finish`
- `filesystem:mount:start`
- `filesystem:mount:finish`
- `assets:load:start`
- `assets:load:finish`
- `schema:select:start`
- `schema:select:finish`
- `runtime:init:finish`
- `runtime:initialized`

Add `m27EvidenceVersion: "m27-startup-v1"` to the diagnostic payload rather than renaming existing marker phases.

Acceptance:

- Existing M24/M25/M26 startup tests still find the old marker names.
- New M27 test can distinguish M27 evidence files.

- [ ] Step 2.2: Add a focused M27 startup Playwright test.

Test name:

```text
M27 STARTUP attribution records startup owner evidence
```

The test must:

- Load `/web/`.
- Wait for `startup:complete`.
- Save fresh startup diagnostics.
- Reload once.
- Save reload startup diagnostics.
- Assert `wasmBuildProfile === "release"`.
- Assert markers are monotonic.
- Assert `schema:select:start` happens after `assets:load:finish`.

Evidence path:

```text
apps/yune-web/e2e/results/m27-startup-runtime/browser-startup-before.json
```

Run:

```powershell
$env:M27_EVIDENCE_LABEL='before'; npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M27 STARTUP" --workers=1
```

Expected:

- The focused test passes.
- Evidence contains both fresh and reload startup payloads.

- [ ] Step 2.3: Add focused control-classification browser evidence.

Add this Playwright test to `apps/yune-web/e2e/yune-typeduck.spec.ts`:

```text
M27 CONTROLS classify live versus deploy-time toggles
```

The test must:

- Load `/web/` and wait until `document.documentElement.dataset.yuneLoading === "false"`.
- Capture the current startup diagnostic count or startup marker sequence.
- Toggle at least these controls one at a time: AI candidates, auto-correction, and auto-completion.
- Save the observed marker sequence, loading indicator visibility, and elapsed control-update time for each control to `control-classification-before.json`.
- Classify each control in that evidence file as one of:
  - `live`: must not emit `runtime:init:start`, `schema:select:start`, deploy markers, or visible `載入中 Loading...`.
  - `browser-only`: must not call the worker/runtime initialization path at all.
  - `deploy-time`: may emit deploy/schema markers, but must be judged against M27 startup/deploy speedup and must not look like an uncontrolled full-engine-loading loop.

Expected initial classification:

| Control | Expected class | Reason |
| --- | --- | --- |
| AI candidates | browser-only/live | Default-off second-pass candidate staging flag; should not redeploy schema assets. |
| ASCII mode | live | Existing session option. |
| Full shape | live | Existing session option. |
| Simplification/traditionalization | live if implemented as session option; otherwise classify from evidence | Must be proven by markers. |
| Auto-correction | deploy-time unless evidence proves live support | M20/M22 treated correction/completion/sentence as customize/deploy options. |
| Auto-completion | deploy-time unless evidence proves live support | M20/M22 treated correction/completion/sentence as customize/deploy options. |
| Auto-composition/sentence | deploy-time unless evidence proves live support | Schema translator behavior, not assumed live. |

Acceptance:

- Live/browser-only controls have hard no-reload assertions.
- Deploy-time controls are measured and tied to the M27 startup/deploy optimization instead of being forced through `setOption`.
- The evidence file records browser URL, commit SHA, build mode, control class, marker deltas, and visible loading behavior.
- Do not weaken existing M24/M25/M26 browser assertions while adding these tests.

- [ ] Step 2.4: Regenerate patch if source changed.

If any file under `apps/yune-web/source/` changed:

1. Regenerate `apps/yune-web/patches/yune-web-runtime.patch` from the patched source checkout.
2. Reverse-check it from `apps/yune-web/source/`:

```powershell
Push-Location apps/yune-web/source
git apply --reverse --check ..\patches\yune-web-runtime.patch
Pop-Location
```

3. Forward-check it on a clean TypeDuck-Web source checkout at the revision in `apps/yune-web/yune-web.lock.json`.

Expected:

- Patch checks pass.
- Ignored `source/` edits alone are not considered landed.

### Task 3 - Choose The Startup Optimization

**Files:**

- Create: `apps/yune-web/e2e/results/m27-startup-runtime/optimization-choice.md`
- Read: `apps/yune-web/e2e/results/m27-startup-runtime/startup-path-reconciliation.md`
- Read: `apps/yune-web/e2e/results/m27-startup-runtime/native-startup-before.md`
- Read: `apps/yune-web/e2e/results/m27-startup-runtime/browser-startup-before.json`
- Read: `apps/yune-web/e2e/results/m27-startup-runtime/control-classification-before.json`

- [ ] Step 3.1: Name the top owner.

Create `optimization-choice.md` with this structure and replace each instruction with the measured value or a concrete statement:

```markdown
# M27 Optimization Choice

> **Status:** Chosen - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

## Startup Path Reconciliation

- Browser-paid path: cold-deploy-build | deploy-cache-hit-load-precompiled | mixed | unknown
- Native row chosen for optimization: exact row name
- Native row excluded from browser optimization: exact row name or `none`
- Reason: one evidence-backed sentence

## Top Native Owner

- Owner: exact span name from `native-startup-before.md`
- Median before: numeric duration and unit
- P95 before: numeric duration and unit
- Working-set delta: numeric bytes or MiB
- Peak working set: numeric bytes or MiB

## Browser Owner

- Fresh startup total: numeric duration and unit from `browser-startup-before.json`
- Reload startup total: numeric duration and unit from `browser-startup-before.json`
- Browser phase aligned with native owner: exact browser marker or `none`

## Required Control Fixes

- AI candidates class: browser-only | live | deploy-time | unknown
- Auto-correction class: browser-only | live | deploy-time | unknown
- Auto-completion class: browser-only | live | deploy-time | unknown
- Other controls that emitted startup/deploy markers: explicit list or `none`
- Fix strategy: live no-reload fix | deploy-speedup/UI-state fix | no change needed

## Chosen Slice

- Chosen change: one sentence naming the implementation slice
- Why this is the smallest behavior-preserving slice: one evidence-backed sentence
- Files to change: explicit paths
- Gates that protect behavior: explicit test/benchmark names

## Deferred Owners

- Owner: exact span name or `none`
- Reason deferred: evidence-backed reason or `none`
- Follow-up doc if needed: explicit path or `none`
```

No field may be left blank in the committed file. Use `none` only when the named field is genuinely not applicable.

- [ ] Step 3.2: Pick exactly one optimization.

Choose by evidence:

- If cold deploy/build dominates only the excluded native row and not the browser-paid row, do not choose cold deploy/build as the M27 browser startup optimization.
- If `compiled_table_load` or `compiled_prism_load` dominates, optimize compiled-data consumption first.
- If `source_dictionary_parse_if_any` dominates the browser-paid path, fix the compiled-data fallback first because the browser should not parse source dictionaries on ordinary deploy-cache-hit startup.
- If `translator_index_build` dominates and memory grows sharply, reduce `entries` + `entries_by_code` double storage or defer building inactive lookup indexes.
- If `schema_config_load` dominates, cache parsed deployed config only with freshness/signature evidence.
- If `userdb_open_or_sync` dominates, avoid sync/open work on startup unless schema options require it.
- If browser-only work dominates after native improvement, optimize worker/runtime scheduling without changing engine behavior.

Acceptance:

- The chosen optimization targets the measured top owner.
- If it does not, execution pauses after Task 3 and asks the user whether to split scope; M27 is not considered closeable by landing only a lower-risk unrelated slice.
- Required controls are classified before Task 4 starts.
- Segment-aware partial selection is not a Task 3 or Task 4 M27 gate; if observed during testing, record it as M28 scope and do not patch it in M27.

### Task 4 - Implement The Required Control Fixes And Measured Startup Slice

**Files:**

Chosen by Task 3. Likely owners:

- `crates/yune-rime-api/src/schema_selection.rs`
- `crates/yune-rime-api/src/schema_install.rs`
- `crates/yune-rime-api/src/deployment.rs`
- `crates/yune-core/src/dictionary/source.rs`
- `crates/yune-core/src/dictionary/compiled_table.rs`
- `crates/yune-core/src/dictionary/compiled_prism.rs`
- `crates/yune-core/src/translator/mod.rs`
- `packages/yune-typeduck-runtime/src/typeduck.ts`
- `apps/yune-web/source/src/worker.ts`
- `apps/yune-web/source/src/yune-integration/adapter.ts`
- `apps/yune-web/source/src/hooks.ts`
- `apps/yune-web/source/src/Preferences.tsx`
- `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [ ] Step 4.1: Run behavior gates before editing the owner.

Run:

```powershell
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-rime-api --test typeduck_web
$env:M27_EVIDENCE_LABEL='before'; npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M27 CONTROLS" --workers=1
```

Expected:

- Existing Rust gates pass before optimization.
- M27 control classification evidence is captured. Record any failure output and evidence path; do not proceed if unrelated browser tests fail.

- [ ] Step 4.2: Fix control behavior according to the live-vs-deploy classification.

Expected implementation direction:

- AI candidates should toggle the default-off second-pass browser flow without `TypeDuckRuntime.init` or `selectSchema`.
- Live session options should use the existing live `setOption` path without remounting the runtime, reselecting schema, deploying, or triggering the global startup/loading indicator.
- Deploy-time controls such as auto-correction, auto-completion, and auto-composition/sentence should not be forced through `setOption` unless the engine/runtime already supports that. Their fix is either the M27 startup/deploy speedup itself, a narrower deploy progress state, or both.
- If a control truly requires schema redeploy or runtime restart, it must be labeled/evidenced as deploy-time rather than grouped with live controls.

Likely files:

- `apps/yune-web/source/src/hooks.ts`
- `apps/yune-web/source/src/Preferences.tsx`
- `apps/yune-web/source/src/worker.ts`
- `apps/yune-web/source/src/yune-integration/adapter.ts`

Acceptance:

- The `M27 CONTROLS classify live versus deploy-time toggles` test passes.
- Browser evidence proves AI candidates do not emit runtime init, schema select, deploy markers, or visible `載入中 Loading...`.
- Browser evidence records whether auto-correction and auto-completion are deploy-time controls, and shows their post-M27 deploy/update elapsed time and UI state.

- [ ] Step 4.3: Implement only the selected startup optimization slice.

Allowed examples:

- Avoid source dictionary parsing when a supported compiled table path is already available for the active schema.
- Avoid constructing inactive side-lookup dictionaries on initial `jyut6ping3_mobile` startup.
- Reduce `entries` + `entries_by_code` double storage during translator index build if Task 1 proves it is the top owner.
- Cache parsed deployed config data only when the deploy signature proves it is fresh.

Do not:

- Change candidate text, order, comments, paging, or commit behavior without oracle evidence.
- Widen `RimeApi`, `RimeCandidate`, TypeDuck profile ABI slots, or public `yune_typeduck_*` exports.
- Remove dictionary comments or reverse lookup data to gain speed.
- Treat a warm browser reload improvement as a native engine improvement.

- [ ] Step 4.4: Capture after evidence.

Run:

```powershell
cargo bench -p yune-rime-api --bench frontend_baselines
$env:M27_EVIDENCE_LABEL='after'; npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M27 STARTUP" --workers=1
$env:M27_EVIDENCE_LABEL='after'; npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M27 CONTROLS" --workers=1
```

Save:

- `apps/yune-web/e2e/results/m27-startup-runtime/native-startup-after.md`
- `apps/yune-web/e2e/results/m27-startup-runtime/browser-startup-after.json`
- `apps/yune-web/e2e/results/m27-startup-runtime/control-classification-after.json`

Required comparison:

- Native before/after median and p95 for the chosen owner.
- Native before/after working-set delta and peak working set.
- Browser fresh/reload before/after startup total.
- Statement that browser numbers include browser/WASM/worker/React overhead.
- Statement that live/browser-only controls avoid startup/deploy markers and deploy-time controls have measured update cost/UI state evidence.

### Task 5 - Close With Full Gates And Docs

**Files:**

- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Modify if needed: `docs/CONVENTIONS.md`
- Archive when complete: `docs/plans/archive/m27-plan-typeduck-web-startup-runtime-init.md`
- Evidence: `apps/yune-web/e2e/results/m27-startup-runtime/task-5-gates.md`

- [ ] Step 5.1: Run full verification.

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
cargo bench -p yune-rime-api --bench frontend_baselines
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
npm.cmd --prefix apps/yune-web/source run build
$env:M27_EVIDENCE_LABEL='after'; npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M27 STARTUP" --workers=1
$env:M27_EVIDENCE_LABEL='after'; npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M27 CONTROLS" --workers=1
git diff --check
```

If TypeDuck-Web source changed, also run the patch workflow from Task 2.

Expected:

- All gates pass.
- `task-5-gates.md` lists every command, result, and evidence path.
- Known `cargo bench` output filename collision warnings may be recorded as warnings only if the benchmark exits 0.

- [ ] Step 5.2: Update closeout docs.

Required updates:

- `docs/roadmap.md`: mark M27 complete, summarize native startup before/after, memory before/after, and browser startup before/after without mixing native and browser claims.
- `docs/requirements.md`: mark M27 requirements complete with startup, memory, and control-classification evidence paths.
- `docs/CONVENTIONS.md`: update the performance-risk sentence to describe any remaining measured startup risk.
- Move this plan to `docs/plans/archive/m27-plan-typeduck-web-startup-runtime-init.md` only after evidence and gates pass.

## Coordination With P2-WIN-02

M27 Tasks 0-3 are measurement, browser evidence, and decision work, so they may run in parallel with P2-WIN-02. Task 4 is different: it may touch `schema_selection.rs`, `schema_install.rs`, TypeDuck-profile behavior, or TypeDuck-Web runtime conversion code. Before Task 4 starts:

- Fetch `origin/main`.
- Check whether P2-WIN-02 has landed or is actively editing the same files.
- If P2-WIN-02 landed, rebase or restart Task 4 from the new `origin/main`.
- If P2-WIN-02 is still active and touches the same files, pause and coordinate rather than landing concurrent engine edits.

## Review Checkpoint For Claude

Ask for review before execution and again after Task 3 before Task 4. The review should focus on:

- Whether `startup-path-reconciliation.md` correctly identifies the browser-paid path and excludes cold-build rows if the browser does not pay them.
- Whether the native sub-attribution owners are precise enough to choose a startup optimization, including compiled table/prism load versus source parse versus translator index build.
- Whether the Windows memory metric is hard enough and meaningful enough for clone/index-build work.
- Whether the plan avoids widening ABI or browser runtime exports just for diagnostics.
- Whether the selected Task 4 slice is behavior-preserving and small enough to close.
- Whether browser evidence remains separate from native engine claims.
- Whether Task 4 should wait for or rebase after P2-WIN-02 if both touch `schema_selection.rs`, `schema_install.rs`, or TypeDuck-profile behavior.

## Handoff Message

Use this message to start an execution session:

```text
Please execute M27 TypeDuck-Web startup/runtime-init performance work in `C:\Users\laubonghaudoi\Documents\GitHub\yune`.

Read first:
- `docs/plans/m27-plan-typeduck-web-startup-runtime-init.md`
- `docs/roadmap.md`
- `docs/requirements.md`
- `docs/CONVENTIONS.md`
- `apps/yune-web/e2e/results/m26-performance/native-after.md`
- `apps/yune-web/e2e/results/m26-performance/startup-attribution-after.json`

Goal:
Fix the TypeDuck-Web startup/schema-selection/runtime-init owner identified by M26, and classify/fix the TypeDuck-Web engine-control update behavior. AI candidates must not reload the runtime. Deploy-time controls such as auto-correction and auto-completion must be measured and improved through the M27 startup/deploy work rather than incorrectly forced through live `setOption`.

Start by reconciling the browser-paid startup path against native benchmark rows. Do not optimize cold deploy/build if the browser is actually paying deploy-cache-hit/load-precompiled plus in-memory index setup. Then split that browser-paid native path into named owners and add a real Windows process-memory metric.

Hard constraints:
- Do not widen `RimeApi`, `RimeCandidate`, TypeDuck profile ABI slots, or public `yune_typeduck_*` exports for diagnostics.
- M27 cannot close with only "RSS unavailable"; checked-in Windows memory evidence is required.
- Browser startup numbers are browser/WASM/worker/React latency, not native engine speed.
- M27 cannot close by improving an excluded cold-build benchmark row if the browser does not pay that path.
- M27 cannot close unless control behavior is classified as live, browser-only, or deploy-time with browser marker evidence.
- Do not implement segment-aware partial candidate commit in M27. The `caksijathaacoenggeoizi` partial-selection issue is M28 engine/oracle scope.
- Preserve oracle-visible candidate text, order, comments, paging, and commit behavior.
- If TypeDuck-Web source files change, regenerate `apps/yune-web/patches/yune-web-runtime.patch` and run reverse/forward patch checks.
- Save evidence under `apps/yune-web/e2e/results/m27-startup-runtime/`.

Execution:
Follow the M27 plan task-by-task. After Task 3, report the measured top owner and chosen optimization before starting Task 4.
Measurement and Task 3 review may run in parallel with P2-WIN-02. Coordinate or rebase before Task 4 if P2-WIN-02 touches `schema_selection.rs`, `schema_install.rs`, or TypeDuck-profile behavior.

Verification target:
Run the full Task 5 gate list from the M27 plan.

Publish:
Commit and push the completed M27 slice directly to `origin/main` unless the work becomes invasive enough to require a temporary branch. Stage only intentional M27 files and leave unrelated local edits untouched.
```
