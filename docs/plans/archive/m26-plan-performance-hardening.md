# M26 Performance Hardening Implementation Plan

> **Status:** Complete - **Milestone:** M26 (engine/runtime performance hardening) - **Closed:** 2026-06-22 - **Type:** execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace anecdotal TypeDuck-Web latency discussion with native and browser performance evidence, then land the first measured optimization slice without changing oracle-visible behavior.

**Architecture:** M26 is measurement-first and oracle-invariant. Separate native engine cost from browser/WASM/worker/React/render cost, attribute the current ~10.5s `runtime:initialized` startup path, then optimize only the measured owner. Do not compare the browser dogfood surface directly to native librime; compare native Yune against native Yune baselines, and compare the browser surface against its own keydown-to-paint and startup budgets.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), existing `frontend_baselines` bench harness, TypeDuck real assets under `apps/yune-web/source/public/schema/`, TypeDuck-Web React/TypeScript, Web Worker diagnostics, Playwright browser evidence, optional Windows packaging smoke only if native ABI behavior changes.

---

## Why This Milestone Exists

M25 made the browser playground usable but did not close the performance story:

- Startup improved from the M25 repro's ~47s `runtime:initialized` marker to about 10.5s, but the remaining `runtime:initialized` time is not yet attributed below the coarse worker marker.
- The M25 focused typing smoke records about 61-62ms p95 on `hai`, but repeated local sampling showed that value is a cold first-key outlier: warm per-key events were mostly in the 10-30ms range.
- The current browser diagnostics measure worker/action and TypeDuck-Web process-key timing, not true keydown-to-paint latency.
- The existing native `frontend_baselines` bench uses a tiny synthetic 4-entry lookup dictionary, so it cannot answer performance for the real 127k-row TypeDuck assets.
- `StaticTableTranslator` is not literally scanning all dictionary rows for every exact lookup today: it builds `entries_by_code: BTreeMap<String, Vec<Candidate>>`, exact lookup uses `get()`, and completion uses a sorted prefix range. The real bounded O(n) scan is the TypeDuck dynamic-correction branch (`translator/mod.rs`, currently `for canonical_code in self.entries_by_code.keys()`), which is gated to correction or `m`-prefix near-lookup behavior and must be benchmarked separately. The broader debt is that candidates are stored twice (`entries` plus `entries_by_code`), the runtime does not use the parsed prism/double-array as the hot lookup index, index construction clones candidates, candidate materialization clones large data, and startup still parses/builds heap-heavy structures eagerly.

M26 exists to turn those observations into reproducible numbers and measured fixes.

## Scope

In scope:

- Add large-real-asset native benchmarks for `jyut6ping3_mobile`, `luna_pinyin`, representative long TypeDuck inputs, and the TypeDuck dynamic-correction path.
- Add browser keydown-to-paint diagnostics separate from worker roundtrip and React render/update work.
- Add nested startup phase markers under `runtime:initialized` so dictionary load, schema deploy, runtime init, table parsing, translator/index build, and persistence are attributable.
- Implement the first optimization proven by those measurements. Candidate owners include reducing candidate/index double-storage, materializing only the visible candidate page, caching reusable lookup material, or replacing selected dynamic-correction/prefix work with a trie/prism-backed or purpose-built prefix index.
- Preserve oracle-visible candidate text, order, comments, paging, commit behavior, and ABI layout.
- Record before/after evidence under `apps/yune-web/e2e/results/m26-performance/` and, for native-only data, under a checked-in JSON or Markdown summary in the same folder.

Out of scope:

- Widening `RimeApi`, `RimeCandidate`, or TypeDuck profile ABI slots.
- Replacing the whole dictionary storage format in one step without a separate design review.
- Making the browser playground beat native librime. The browser surface pays WASM, worker, serialization, React, layout, and paint costs that native IMEs do not pay.
- Reopening M24 or M25 dogfooding ledgers. If a new manual UI defect appears, start a new dogfood ledger.
- Starting Windows product UI work. P2-WIN-02 and P2-WIN-01 remain separate tracks.

## Current Baseline To Preserve In The Plan

Record these as the starting point before implementation:

- Browser dev server restart: Vite ready in about 0.6s.
- Browser startup: `startup:complete.marker.totalMs` about 10.5s, dominated by `runtime:initialized`.
- Focused M25 `hai` typing smoke: `p95` about 61-62ms, no global loading overlay.
- Repeated local `hai` sampling after warm-up: median around 11-14ms, p95 around 28-34ms, with occasional first-key outliers.
- Existing native `cargo bench -p yune-rime-api --bench frontend_baselines` on 2026-06-22:
  - `session_create_destroy`: about 16us/op.
  - `per_key_simple_ascii_rime_process_key`: about 36us/op.
  - `per_key_schema_loaded_lookup_rime_process_key`: about 400us/op, but only with a 4-entry synthetic dictionary.
  - `schema_deploy_dictionary_load`: about 3.9ms/op, but only with a 4-entry synthetic dictionary.
  - `userdb_learning_sync`: about 10.6ms/op.

## Acceptance Gates

M26 is complete only when all of these are true:

- `M26-PERF-01`: Native large-real-asset benchmarks exist and report median, p95, p99, max, cold-first-key versus warm steady-state, operation count, fixture name, asset set, schema ID, and memory/allocation notes. Each named per-key scenario must include a full-ABI row and an engine-only row so engine time is not confused with context/commit/free overhead. Required scenarios:
  - `jyut6ping3_mobile`: `hai`, `ngohaig`, `jigaajiusihaa`.
  - `luna_pinyin`: `ni`, `zhongguo`.
  - one long TypeDuck sentence/composition input from existing cantonese parity fixtures.
  - at least one TypeDuck correction-path input where `jigaajiusihaa` or a documented alternative exercises the dynamic-correction scan.
- `M26-PERF-02`: Browser diagnostics record keydown-to-paint or closest browser-supported proxy, plus worker queue wait, worker process-key time, worker roundtrip, response parse/mapping, React update, and candidate panel paint marker.
- `M26-PERF-03`: Startup diagnostics break the current coarse `runtime:initialized` time into nested owners. The closeout must identify the top owner by evidence, not speculation.
- `M26-PERF-04`: At least one measured optimization lands, with before/after native and browser evidence. The landed slice must target the evidenced top owner from Tasks 1-3. If that owner is too large for M26, the closeout must explicitly name it as deferred, create a named follow-up plan with the profiling evidence attached, and then land a low-risk measured improvement from the same evidence set.
- `M26-PERF-05`: Compatibility and integration gates remain green:
  - `cargo fmt --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test -p yune-core --test cantonese_parity`
  - `cargo test -p yune-core --test upstream_luna_pinyin_parity`
  - `cargo test -p yune-rime-api --test typeduck_web`
  - `cargo bench -p yune-rime-api --bench frontend_baselines`
  - `npm --prefix packages/yune-typeduck-runtime test`
  - `npm --prefix packages/yune-typeduck-runtime run build`
  - `npm --prefix apps/yune-web/source run build`
  - focused Playwright M26 browser evidence tests.
  - if any file under `apps/yune-web/source/` changed, regenerate `apps/yune-web/patches/yune-web-runtime.patch` and run the TypeDuck-Web patch reverse/forward checks.

## Implementation Tasks

### Task 0 - Prepare A Clean Performance Slice

**Files:**

- Read: `docs/roadmap.md`
- Read: `docs/CONVENTIONS.md`
- Read: `docs/requirements.md`
- Read: `docs/plans/m26-plan-performance-hardening.md`
- Inspect: `crates/yune-rime-api/benches/frontend_baselines.rs`
- Inspect: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [x] Step 0.1: Confirm branch and worktree state.

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

- [x] Step 0.2: Capture the current benchmark baselines.

Run:

```powershell
cargo bench -p yune-rime-api --bench frontend_baselines
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "M25 DOGFOOD-03" --workers=1
```

Expected:

- Native bench prints the current synthetic numbers.
- Focused browser typing smoke passes and records current M25-style worker timings.
- Do not commit changed M25 evidence JSON from this baseline run; copy numbers into new M26 evidence instead.

### Task 1 - Add Native Large-Real-Asset Benchmarks

**Files:**

- Modify: `crates/yune-rime-api/benches/frontend_baselines.rs`
- Reuse fixture patterns from: `crates/yune-rime-api/tests/typeduck_web.rs`
- Read assets from: `apps/yune-web/source/public/schema/`

- [x] Step 1.1: Add a real-asset fixture helper.

Implementation shape:

- Copy the same browser-real assets used by `typeduck_web.rs` into the benchmark fixture shared data dir.
- Include `default.yaml`, `default.custom.yaml`, `common.yaml`, `common.custom.yaml`, `include.yaml`, `template.yaml`, active schema files, dictionaries, `.table.bin`, `.reverse.bin`, `.prism.bin`, and required OpenCC files.
- Keep the helper benchmark-local; do not expose new production APIs just for benchmarks.

Acceptance check:

```powershell
cargo bench -p yune-rime-api --bench frontend_baselines
```

Expected:

- Existing rows still print.
- New real-asset rows print schema ID, asset count or dictionary size, operation count, total ms, us/op, cold-first-key versus warm steady-state where applicable, and RSS/allocation notes when available.

- [x] Step 1.2: Add native per-key scenarios.

Required benchmark rows:

- full-ABI and engine-only variants for `per_key_real_jyut6ping3_mobile_hai`
- full-ABI and engine-only variants for `per_key_real_jyut6ping3_mobile_ngohaig`
- full-ABI and engine-only variants for `per_key_real_jyut6ping3_mobile_jigaajiusihaa`
- full-ABI and engine-only variants for `per_key_real_luna_pinyin_ni`
- full-ABI and engine-only variants for `per_key_real_luna_pinyin_zhongguo`
- a named correction-path row, preferably `per_key_real_jyut6ping3_mobile_jigaajiusihaa_correction`, that records whether it exercised the `entries_by_code.keys()` dynamic-correction scan.

Measurement rule:

- Use enough iterations to make timing stable, but keep the bench practical for local execution.
- Report cold-first-key and warm steady-state separately. Do not average the cold outlier into the warm typing number.
- Include status/context/commit/free cycles in the full-ABI rows.
- Add engine-only rows using an existing internal/test path or a benchmark-only helper. Do not widen production ABI just for benchmarking.
- Report memory/allocation data if a stable local metric is available; otherwise record the limitation in the M26 evidence summary.

- [x] Step 1.3: Add startup/deploy real-asset rows.

Required benchmark rows:

- `startup_real_jyut6ping3_mobile_runtime_ready`
- `deploy_real_jyut6ping3_mobile_cache_hit`
- `startup_real_luna_pinyin_runtime_ready`

Acceptance:

- Rows clearly distinguish first deploy/cache-miss from cache-hit/runtime-ready where the current API allows that split.
- If the existing bench harness cannot express the split cleanly, record the limitation in the M26 evidence summary and keep the row names honest.

### Task 2 - Add Browser Keydown-To-Paint Diagnostics

**Files:**

- Modify: `apps/yune-web/source/src/CandidatePanel.tsx`
- Modify: `apps/yune-web/source/src/rime.ts`
- Modify: `apps/yune-web/source/src/worker.ts`
- Modify as needed: `apps/yune-web/source/src/yune-integration/adapter.ts`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [x] Step 2.1: Add a diagnostic event shape.

Required fields for each key event:

- `input`
- `key`
- `keydownAt`
- `workerQueuedAt`
- `workerStartedAt`
- `workerFinishedAt`
- `responseReceivedAt`
- `stateAppliedAt`
- `paintObservedAt`
- `candidateCount`
- `firstCandidateText`

Store only the last bounded set of events in `document.documentElement.dataset.yunePerfDiagnostics`, following the M25 diagnostic pattern. Keep it test-only/debug-friendly and free of hidden user text beyond the typed sample already used in tests.

- [x] Step 2.2: Measure paint with a browser-safe marker.

Preferred implementation:

- Mark keydown before dispatching to the worker.
- Mark response mapping after worker result returns.
- Mark state application when React state has been updated.
- Use `requestAnimationFrame` after the candidate panel update to mark the closest available paint proxy.

Acceptance:

```powershell
npm --prefix apps/yune-web/source run build
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "M26 PERF" --workers=1
```

Expected:

- The M26 browser test can parse `yunePerfDiagnostics`.
- Each event has nondecreasing timestamps.
- The test saves JSON evidence under `apps/yune-web/e2e/results/m26-performance/`.
- If any `apps/yune-web/source/...` file changed, regenerate `apps/yune-web/patches/yune-web-runtime.patch`, reverse-check it from the patched source checkout, and forward-check it on a clean source checkout reset to `apps/yune-web/yune-web.lock.json`. Ignored `source/` edits alone are not a valid deliverable.

### Task 3 - Attribute Startup Below `runtime:initialized`

**Files:**

- Modify: `apps/yune-web/source/src/worker.ts`
- Modify: `apps/yune-web/source/src/yune-integration/adapter.ts`
- Modify if needed: `packages/yune-typeduck-runtime/src/typeduck.ts`
- Modify if needed: `packages/yune-typeduck-runtime/src/module.ts`
- Test: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [x] Step 3.1: Add nested startup markers.

Required marker names:

- `runtime:init:start`
- `wasm:module:create:start`
- `wasm:module:create:finish`
- `filesystem:mount:start`
- `filesystem:mount:finish`
- `assets:load:start`
- `assets:load:finish`
- `rime:init:start`
- `rime:init:finish`
- `schema:deploy:start`
- `schema:deploy:finish`
- `schema:select:start`
- `schema:select:finish`
- `runtime:init:finish`

Acceptance:

- Existing `startup:complete` remains backward compatible.
- New markers identify which nested phase owns the remaining ~10.5s startup cost.
- Browser evidence records both fresh load and reload/cache-hit paths.
- If any `apps/yune-web/source/...` file changed, regenerate `apps/yune-web/patches/yune-web-runtime.patch`, reverse-check it from the patched source checkout, and forward-check it on a clean source checkout reset to `apps/yune-web/yune-web.lock.json`.

- [x] Step 3.2: Add a startup evidence test.

Test name should include `M26 PERF startup attribution`.

The test must:

- Load `/web/`.
- Wait for startup complete.
- Assert release WASM is still used.
- Assert nested markers exist and are monotonic.
- Save `startup-attribution-before.json` or `startup-attribution-after.json` under the M26 evidence folder.
- Include the patch regeneration/check result in the evidence summary when this task touched TypeDuck-Web source files.

### Task 4 - Optimize The Measured Hot Owner

**Coordination rule:** Tasks 0-3 are safe to run in parallel with P2-WIN-02. Task 4 is not automatically parallel-safe. If the chosen optimization touches candidate/comment materialization, `RimeCandidate.comment`, `typeduck_web` response construction, or browser candidate rendering, sequence it after P2-WIN-02 lands or explicitly rebase it on the P2-WIN-02 comment-byte fix before implementation.

**Files are chosen by the Task 1-3 evidence. Likely owners:**

- `crates/yune-core/src/translator/mod.rs`
- `crates/yune-core/src/dictionary/source.rs`
- `crates/yune-core/src/dictionary/compiled_table.rs`
- `crates/yune-core/src/dictionary/compiled_prism.rs`
- `crates/yune-core/src/dictionary/double_array.rs`
- `crates/yune-rime-api/src/schema_install.rs`
- `crates/yune-rime-api/src/typeduck_web.rs`
- `apps/yune-web/source/src/CandidatePanel.tsx`

- [x] Step 4.1: Pick exactly one first optimization target.

Choose the target by evidence:

- If native per-key time is dominated by lookup/index traversal, optimize lookup/indexing.
- If startup is dominated by dictionary parse/index construction, optimize load/build.
- If browser p95 is dominated by serialization/rendering after worker completion, optimize the TypeDuck-Web response and candidate rendering path.
- If worker queue wait dominates, optimize action scheduling and avoid settings/deploy work on typing.

Record the evidenced top owner and chosen target in `apps/yune-web/e2e/results/m26-performance/optimization-choice.md`. If the chosen target is not the top owner, the file must name the top owner, explain why it is deferred, and link to the follow-up plan created for it.

- [x] Step 4.2: Preserve oracle-visible behavior before optimization.

Run the narrow behavior gates before changing the hot path:

```powershell
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-rime-api --test typeduck_web
```

Expected:

- All pass before optimization.
- If any fail before edits, stop and classify the failure separately.

- [x] Step 4.3: Implement the selected optimization.

Allowed first-slice examples:

- Avoid cloning full `Candidate` values while building `entries_by_code`; store indexes or shared candidate records instead.
- Materialize only the visible `page_size` candidate rows for browser responses when nonvisible rich details are not needed.
- Cache prefix-range results for repeated long inputs within the same composition.
- Use parsed prism/double-array only after tests prove it preserves spelling algebra, correction, completion, prefix fallback, and abbreviation behavior for the named target.
- Defer non-active schema/dictionary initialization when startup evidence proves it is safe.

Do not:

- Change candidate ordering without pinned oracle evidence.
- Remove dictionary comments or lookup records to gain speed.
- Add a new dependency for zero-copy storage without documenting why existing data structures cannot meet the target.
- Optimize candidate/comment emission in a way that conflicts with the active P2-WIN-02 TypeDuck boundary fix. If this path is selected, rebase on that fix first and rerun its relevant gates.

- [x] Step 4.4: Capture before/after evidence.

Required commands:

```powershell
cargo bench -p yune-rime-api --bench frontend_baselines
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "M26 PERF" --workers=1
```

Required evidence:

- `native-before.json` or `native-before.md`
- `native-after.json` or `native-after.md`
- `startup-attribution-before.json`
- `startup-attribution-after.json`
- `typing-keydown-to-paint-before.json`
- `typing-keydown-to-paint-after.json`
- `optimization-choice.md`
- regenerated `apps/yune-web/patches/yune-web-runtime.patch` plus reverse/forward patch-check notes, if TypeDuck-Web source changed.

### Task 5 - Close With Compatibility Gates And Updated Docs

**Files:**

- Modify if needed: `docs/roadmap.md`
- Modify if needed: `docs/requirements.md`
- Modify if needed: `docs/CONVENTIONS.md`
- Archive when complete: `docs/plans/archive/m26-plan-performance-hardening.md`

- [x] Step 5.1: Run full verification.

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
cargo bench -p yune-rime-api --bench frontend_baselines
npm --prefix packages/yune-typeduck-runtime test
npm --prefix packages/yune-typeduck-runtime run build
npm --prefix apps/yune-web/source run build
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "M26 PERF" --workers=1
git diff --check
```

If any file under `apps/yune-web/source/` changed, also run the M24/M25 patch workflow before `git diff --check`:

```powershell
# First regenerate apps/yune-web/patches/yune-web-runtime.patch from the patched source checkout.

# Reverse-check from the patched source checkout.
Push-Location apps/yune-web/source
git apply --reverse --check ..\patches\yune-web-runtime.patch
Pop-Location

# Then forward-check the regenerated patch on a separate clean source checkout reset to
# apps/yune-web/yune-web.lock.json.
Push-Location <clean-typeduck-web-source-checkout>
git apply --check <path-to-yune>\apps\yune-web\patches\yune-web-runtime.patch
Pop-Location
```

Expected:

- All pass.
- If `cargo bench` prints known output filename collision warnings, record them as warnings only if the benchmark still exits 0.
- If TypeDuck-Web source changed, the staged/tracked deliverable includes the regenerated patch; ignored `source/` edits alone are not complete.

- [x] Step 5.2: Update closeout docs.

Required updates:

- `docs/roadmap.md`: mark M26 complete, summarize measured before/after numbers, and preserve the distinction between native engine speed and browser dogfood latency.
- `docs/requirements.md`: mark M26 requirements complete with evidence paths.
- `docs/CONVENTIONS.md`: update the performance-risk sentence to describe the remaining measured risk accurately.
- `apps/yune-web/patches/yune-web-runtime.patch`: regenerated and checked if any TypeDuck-Web source files changed.
- Move this plan to `docs/plans/archive/m26-plan-performance-hardening.md` only after evidence and gates pass.

## Review Checkpoint For Claude

Ask for review before implementation if possible, and again after Task 3 before Task 4. The first review should focus on:

- Whether the plan correctly avoids the false claim that the hot path is an unconditional full-table scan.
- Whether the native large-real-asset benchmark scenarios are enough to prove or disprove the dictionary/index hypothesis.
- Whether the browser keydown-to-paint instrumentation is honest enough to support future performance budgets.
- Whether the first optimization slice is closeable without turning into a full storage-format rewrite.
- Whether Task 4 correctly sequences around P2-WIN-02 if it touches candidate/comment materialization.
- Whether the TypeDuck-Web patch regeneration and reverse/forward checks are sufficient for any browser-source changes.

## Handoff Message

Use this message to start an execution session:

```text
Please execute M26 performance hardening in `C:\Users\laubonghaudoi\Documents\GitHub\yune`.

Read `docs/plans/m26-plan-performance-hardening.md`, `docs/roadmap.md`, `docs/requirements.md`, and `docs/CONVENTIONS.md` first. Use the plan task-by-task. Keep M26 measurement-first: add native large-real-asset benchmarks, add browser keydown-to-paint diagnostics, attribute the current ~10.5s `runtime:initialized` startup path, then implement only the first optimization target proven by evidence.

Important constraints:
- Do not reopen M24 or M25 dogfood ledgers.
- Do not widen `RimeApi`, `RimeCandidate`, or TypeDuck profile ABI slots.
- Do not claim the hot path is an unconditional full-table scan; `StaticTableTranslator` already has `entries_by_code`, but prism/double-array is not used as the runtime lookup index and clone/startup debt remains.
- Benchmark the bounded dynamic-correction scan separately; `jigaajiusihaa` or a documented equivalent should exercise the `entries_by_code.keys()` correction branch.
- Preserve oracle-visible candidate text, order, comments, paging, and commit behavior.
- If Task 4 touches candidate/comment materialization, sequence it after or explicitly rebase it on P2-WIN-02.
- If TypeDuck-Web source files change, regenerate `apps/yune-web/patches/yune-web-runtime.patch` and run reverse/forward patch checks before closeout.
- Save M26 evidence under `apps/yune-web/e2e/results/m26-performance/`.
- Ask for review after startup/key typing attribution and before choosing the optimization target.

Verification target: the M26 plan's Task 5 command list.
```
