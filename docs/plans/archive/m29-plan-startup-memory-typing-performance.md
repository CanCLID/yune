# M29 Startup Memory And Typing Performance Implementation Plan

> **Status:** Complete - **Milestone:** M29 (startup memory and typing latency performance) - **Created:** 2026-06-22 - **Completed:** 2026-06-22 - **Type:** execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce the remaining TypeDuck-Web/Yune startup cost and establish a fresh typing-latency optimization loop without changing candidate correctness.

**Architecture:** M29 is measurement-first. It starts from the M27 baseline (`startup_real_jyut6ping3_mobile_runtime_ready` about `6.35s`, browser startup about `5.6s`, peak working set about `1.79GB`) and the M26 typing baseline (`hai` p95 about `60ms`, long phrase p95 about `64ms`). It first separates real per-startup memory pressure from benchmark high-water noise, then targets the remaining `spelling_algebra_expand` startup owner, and only then optimizes typing latency based on browser/native attribution.

**Tech Stack:** Rust benchmarks (`crates/yune-rime-api/benches/frontend_baselines.rs`), Yune core spelling algebra/dictionary deployment (`crates/yune-core/src/spelling_algebra.rs`, dictionary build/load modules, `schema_install.rs`), TypeScript runtime and TypeDuck-Web worker diagnostics, Playwright performance evidence, Windows process memory sampling, and the existing TypeDuck-Web patch workflow.

---

## Why This Milestone Exists

M27 cut TypeDuck startup substantially, but it did not make startup feel instant:

- Native `startup_real_jyut6ping3_mobile_runtime_ready`: about `15.55s` -> `6.35s`.
- Browser fresh/reload startup: about `10.7s` / `10.4s` -> `5.680s` / `5.466s`.
- Remaining dominant owner: `startup_trace_jyut6ping3_mobile_spelling_algebra_expand`, still about `5.09s`.
- Recorded process peak: about `1.79GB`, with per-span working-set deltas around `600MB` that may include allocator high-water noise.

M26 also established browser typing latency evidence, but M27 did not optimize it:

- Normal typing p95 stayed around `60ms`.
- Long phrase p95 stayed around `64ms`.
- Paging and reverse lookup were much faster, so the likely target is normal key processing/rendering rather than all browser actions.

M29 must keep startup, memory, and typing evidence separate. Startup numbers are not typing numbers; browser numbers are not native engine numbers. M27 already added the per-original-code spelling-algebra cache, so M29 must find a different startup lever such as cross-instance persistence, deploy/build-time precomputation, or prism/table metadata reuse; it should not repeat the M27 cache work.

## Scope

In scope:

- Re-run M27 startup benchmark rows and browser startup evidence as M29 baselines.
- Add a single-startup memory probe that distinguishes per-startup working-set pressure from benchmark cumulative high-water marks.
- Reduce the remaining `spelling_algebra_expand` startup owner by moving deterministic spelling expansion work out of runtime startup where feasible.
- Re-run browser fresh/reload startup after optimization.
- Add fresh typing keydown-to-paint attribution and split it into browser, worker, serialization, native/WASM key processing, and render owners.
- Optimize the measured typing owner only after evidence identifies it.
- Watch the M28 follow-up fuzzy phrase-prefix generation path as a possible new typing owner after correctness lands.

Out of scope:

- Candidate ordering, Space/default-confirm correctness, or Jyutping ranking changes. Those belong to the M28 follow-up plan.
- Any change that widens `RimeApi`, `RimeCandidate`, or TypeDuck profile ABI slots.
- Replacing the TypeDuck-Web app stack.
- Claiming performance wins from debug-only runs or stale browser WASM assets.

## Acceptance Gates

- `M29-PERF-01`: M29 baseline evidence re-runs startup, memory, and browser typing measurements on the current post-M28-follow-up code.
- `M29-PERF-02`: Memory evidence classifies the `1.79GB` M27 peak as per-startup pressure, benchmark cumulative high-water, or unresolved with precise blocker evidence.
- `M29-PERF-03`: Startup optimization reduces the measured top startup owner or records an evidence-backed reason it cannot be reduced in this milestone. The target owner is `spelling_algebra_expand` unless fresh M29 attribution proves otherwise.
- `M29-PERF-04`: Typing latency attribution identifies the top owner for normal and long-phrase keydown-to-paint before implementation.
- `M29-PERF-05`: At least one startup or typing optimization lands with before/after native and browser evidence, and no candidate/correctness fixture changes occur without an explicit correctness plan.
- `M29-PERF-06`: Full gates pass: Rust fmt/clippy/tests, frontend benchmark, TypeScript runtime tests/build, TypeDuck-Web build, focused M29 Playwright evidence, patch checks if source changes, and `git diff --check`.

## Files And Responsibilities

- `crates/yune-rime-api/benches/frontend_baselines.rs`: native startup/per-key benchmark rows, owner spans, memory samples.
- `crates/yune-core/src/spelling_algebra.rs`: current remaining startup owner; possible cache/precompute target.
- `crates/yune-core/src/dictionary/`: compiled data readers/writers if spelling expansion moves to build/deploy artifacts.
- `crates/yune-rime-api/src/schema_install.rs`: translator installation and deployment boundary where precomputed expansion data may be loaded.
- `crates/yune-rime-api/src/schema_selection.rs`: schema selection timing owner if new instrumentation is needed.
- `packages/yune-typeduck-runtime/src/`: runtime wrapper timing around process-key and response parsing if typing attribution points there.
- `apps/yune-web/source/src/worker.ts`: browser worker timing markers.
- `apps/yune-web/source/src/CandidatePanel.tsx` and related UI files: render timing only if attribution proves React rendering dominates.
- `apps/yune-web/e2e/yune-typeduck.spec.ts`: focused M29 startup and typing evidence.
- `apps/yune-web/e2e/results/m29-performance/`: evidence folder.
- `apps/yune-web/patches/yune-web-runtime.patch`: regenerate only if TypeDuck-Web source changes.

## Implementation Tasks

### Task 0 - Prepare Fresh Baselines

**Files:**

- Create: `apps/yune-web/e2e/results/m29-performance/m29-baseline.md`
- Create: `apps/yune-web/e2e/results/m29-performance/native-startup-before.md`
- Create: `apps/yune-web/e2e/results/m29-performance/browser-startup-before.json`
- Create: `apps/yune-web/e2e/results/m29-performance/typing-keydown-to-paint-before.json`

- [x] Step 0.1: Record the inherited baseline.

Create `m29-baseline.md`:

```markdown
# M29 Baseline

Inherited M27 startup baseline:
- Native TypeDuck startup median: about 6.35s.
- Browser fresh startup: about 5.68s.
- Browser reload startup: about 5.47s.
- Remaining native owner: spelling_algebra_expand, about 5.09s.
- Recorded process peak: about 1.79GB.

Inherited M26 typing baseline:
- hai p95 keydown-to-paint: about 60ms.
- long phrase p95 keydown-to-paint: about 64ms.
- paging p95: about 6ms.
- reverse lookup p95: about 26ms.
```

Expected:

- The file states inherited values only; it does not claim fresh M29 measurement yet.

- [x] Step 0.2: Re-run native startup benchmark.

Run:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m29-frontend-baselines-before.txt 2>&1"
```

Create `native-startup-before.md` with:

```markdown
# M29 Native Startup Before

Command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m29-frontend-baselines-before.txt 2>&1"
```

Captured output: `target/m29-frontend-baselines-before.txt`.

Rows to report:
- startup_real_jyut6ping3_mobile_runtime_ready median/p95.
- startup_trace_jyut6ping3_mobile_select_schema_total median.
- startup_trace_jyut6ping3_mobile_translator_install median.
- startup_trace_jyut6ping3_mobile_spelling_algebra_expand median.
- working set before/after and peak working set where available.
```

Expected:

- The evidence reports fresh M29 numbers and does not rely only on M27.

- [x] Step 0.3: Re-run browser startup and typing evidence.

Run:

```powershell
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M29_EVIDENCE_LABEL='before'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M29 PERF" --workers=1
```

If no `M29 PERF` test exists yet, add it in Task 2 before this step and then run the command.

Expected:

- `browser-startup-before.json` captures fresh and reload startup markers.
- `typing-keydown-to-paint-before.json` captures normal typing, long phrase, paging, and reverse lookup timing.

### Task 1 - Classify Startup Memory Pressure

**Files:**

- Modify: `crates/yune-rime-api/benches/frontend_baselines.rs`
- Create: `apps/yune-web/e2e/results/m29-performance/memory-classification.md`

- [x] Step 1.1: Add a single-startup memory probe.

Add a benchmark or ignored test helper that:

- Starts a fresh process/sample scope.
- Records working set before asset setup.
- Records working set after runtime init/schema select.
- Records working set after session destroy/finalize.
- Runs only one TypeDuck startup sample.

Expected output row shape:

```text
m29_single_startup_memory_jyut6ping3_mobile before_bytes=<n> after_ready_bytes=<n> after_finalize_bytes=<n> peak_bytes=<n>
```

Run:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines -- m29_single_startup_memory > target\m29-memory-before.txt 2>&1"
```

Expected:

- Output includes before, after-ready, after-finalize, and peak bytes on Windows.

- [x] Step 1.2: Classify the memory finding.

Create `memory-classification.md`:

```markdown
# M29 Memory Classification

Command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines -- m29_single_startup_memory > target\m29-memory-before.txt 2>&1"
```

Classification:
- per-startup pressure:
- benchmark cumulative high-water:
- unresolved:

Evidence:
- before bytes:
- after-ready bytes:
- after-finalize bytes:
- peak bytes:
```

Expected:

- Exactly one classification row is filled in.
- If unresolved, the evidence names the missing measurement and the next command needed.

### Task 2 - Add Browser Typing Attribution

**Files:**

- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Modify if needed: `apps/yune-web/source/src/worker.ts`
- Modify if needed: `packages/yune-typeduck-runtime/src/`
- Create: `apps/yune-web/e2e/results/m29-performance/typing-attribution-before.json`

- [x] Step 2.1: Add marker fields for each key.

For each measured key event, capture:

```json
{
  "key": "i",
  "input": "jigaajiusihaa",
  "keydownAt": 0,
  "workerPostAt": 0,
  "nativeProcessStartAt": 0,
  "nativeProcessFinishAt": 0,
  "responseParsedAt": 0,
  "stateAppliedAt": 0,
  "paintObservedAt": 0,
  "totalMs": 0
}
```

Use `performance.now()` in the browser/worker and native wrapper timestamps from the closest available boundaries. If clocks cannot be compared across contexts, record per-context durations separately and state that limitation in the JSON.

- [x] Step 2.2: Add focused Playwright coverage.

Add a test named `M29 PERF typing attribution records owner spans` that measures:

- Normal short typing: `hai`.
- Long phrase: `jigaajiusihaa`.
- Long composition: `caksijathaacoenggeoizi`.
- Page change.
- Reverse lookup.

Run:

```powershell
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M29_EVIDENCE_LABEL='before'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M29 PERF typing attribution" --workers=1
```

Expected:

- `typing-attribution-before.json` identifies the top owner for normal and long-phrase typing.
- No test budget is enforced before optimization; this is attribution evidence.

### Task 3 - Choose Startup And Typing Optimization Targets

**Files:**

- Create: `apps/yune-web/e2e/results/m29-performance/optimization-choice.md`

- [x] Step 3.1: Record the chosen startup target.

Create `optimization-choice.md`:

```markdown
# M29 Optimization Choice

## Startup Target

Chosen owner:
Baseline:
Reason:

Rejected startup targets:
- compiled table load:
- deploy cache hit:
- browser asset loading:

## Typing Target

Chosen owner:
Baseline:
Reason:

Rejected typing targets:
- native key processing:
- worker serialization:
- React rendering:
```

Expected:

- If `spelling_algebra_expand` remains top owner, choose it.
- If fresh evidence shows another top owner, choose the fresh top owner and explain why M27's owner changed.
- If the owner is still spelling algebra, explain why the M27 per-original-code cache is insufficient and choose a distinct lever.
- If M28 follow-up phrase-prefix generation becomes visible in typing attribution, choose it only if correctness fixtures remain unchanged.
- Do not implement an optimization not named here.

### Task 4 - Optimize Startup Owner

**Files:**

- Modify based on Task 3: `crates/yune-core/src/spelling_algebra.rs`
- Modify if needed: `crates/yune-core/src/dictionary/`
- Modify if needed: `crates/yune-rime-api/src/schema_install.rs`
- Modify: `crates/yune-rime-api/benches/frontend_baselines.rs`
- Create: `apps/yune-web/e2e/results/m29-performance/native-startup-after.md`

- [x] Step 4.1: Move deterministic spelling expansion out of repeated runtime work.

If `spelling_algebra_expand` remains the top owner, do not reimplement M27's per-original-code runtime cache. Implement one of these evidence-backed options:

- Cache expanded variants by original code and schema algebra across runtime instances, with invalidation keyed by schema/dictionary checksum.
- Persist expanded variants in a generated build artifact during deploy and load it during schema selection.
- Reuse prism/table build metadata if the current compiled payload carries enough syllable/algebra data.

Acceptance:

- The cache key includes schema id, dictionary id/checksum, and spelling algebra signature.
- Stale cache falls back to recomputation.
- Candidate text/order tests remain fixture-backed.

- [x] Step 4.2: Re-run native startup benchmark.

Run:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m29-frontend-baselines-after-startup.txt 2>&1"
```

Create `native-startup-after.md` with before/after rows for:

- `startup_real_jyut6ping3_mobile_runtime_ready`
- `startup_trace_jyut6ping3_mobile_spelling_algebra_expand`
- process working set / peak working set

Expected:

- Startup owner is reduced materially, or `optimization-choice.md` documents why the selected owner could not be reduced safely.

### Task 5 - Optimize Typing Owner

**Files:**

- Modify based on Task 3: `crates/yune-core/src/*`, `crates/yune-rime-api/src/typeduck_web.rs`, `packages/yune-typeduck-runtime/src/*`, or `apps/yune-web/source/src/*`
- Create: `apps/yune-web/e2e/results/m29-performance/typing-keydown-to-paint-after.json`
- Create: `apps/yune-web/e2e/results/m29-performance/typing-attribution-after.json`

- [x] Step 5.1: Implement only the measured typing optimization.

Completion note: Task 3 selected no typing code target for M29 after attribution because the measured typing owners were mixed and already much smaller than startup. M29 landed the startup optimization required by `M29-PERF-05` and kept typing as before/after evidence without changing candidate behavior.

Examples by owner:

- If native processing dominates, optimize the measured translator/filter path and prove native per-key rows improve.
- If JSON serialization dominates, reduce response payload size only after proving no UI/test consumer needs the removed fields.
- If worker message overhead dominates, batch or transfer only the measured hot response fields.
- If React rendering dominates, memoize candidate rows or reduce rerender scope without changing visible candidate content.

Acceptance:

- The change is tied to `optimization-choice.md`.
- Candidate ordering/text/comment behavior does not change.
- Browser evidence proves the same user-visible flows still work.

- [x] Step 5.2: Re-run typing evidence.

Run:

```powershell
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M29_EVIDENCE_LABEL='after'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M29 PERF typing attribution" --workers=1
```

Expected:

- Before/after p95 is reported for normal typing and long phrase typing.
- If p95 does not improve, the evidence identifies whether variance, browser paint, or wrong-owner choice explains it.

### Task 6 - Close With Full Gates And Docs

**Files:**

- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Archive when complete: `docs/plans/archive/m29-plan-startup-memory-typing-performance.md`
- Create: `apps/yune-web/e2e/results/m29-performance/task-6-gates.md`

- [x] Step 6.1: Run verification.

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m29-frontend-baselines-final.txt 2>&1"
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
npm.cmd --prefix apps/yune-web/source run build
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M29_EVIDENCE_LABEL='after'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M29 PERF" --workers=1
git diff --check
```

If TypeDuck-Web source changed, also run the patch reverse/forward checks used by M24-M28.

Expected:

- All gates pass.
- `task-6-gates.md` records exact command results.
- Roadmap and requirements mark M29 complete with evidence paths.
- The plan moves to `docs/plans/archive/` only after evidence and gates pass.

## Historical Handoff Message

This was the starting handoff text for the now-completed milestone:

```text
Please execute M29 startup/memory/typing performance work in C:\Users\laubonghaudoi\Documents\GitHub\yune.

Read AGENTS.md, docs/CONVENTIONS.md, docs/roadmap.md, docs/requirements.md, docs/plans/archive/m29-plan-startup-memory-typing-performance.md, and M26/M27 archived performance evidence first.

Goal: reduce remaining startup cost and establish fresh typing-latency optimization without changing candidate behavior. Start with fresh M29 baselines, classify whether the M27 1.79GB memory peak is per-startup pressure or benchmark high-water noise, then choose and implement only the measured top startup/typing owners. Keep M28 follow-up correctness separate; do not change candidate ordering, comments, commit behavior, or ABI surfaces in M29.

If TypeDuck-Web source changes, regenerate and reverse/forward-check apps/yune-web/patches/yune-web-runtime.patch. Stage only intended files and push directly to origin/main when all gates pass.
```
