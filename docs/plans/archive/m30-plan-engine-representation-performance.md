# M30 Engine Representation Performance Plan

> **Status:** Complete - **Milestone:** M30 (engine representation performance) - **Closed:** 2026-06-22 - **Type:** archived execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Closeout:** M30 is complete. Lever A removed duplicate steady-state expanded-entry storage for spelling-algebra-backed translators, preserved TypeDuck row order with a builder-only source stream, and reduced single-startup after-ready bytes from `1,103,331,328` to `838,209,536` in the Lever A run and `839,217,152` in the final gate. Browser startup/typing stayed flat/noisy after a fresh release WASM rebuild, so no browser latency win is claimed. Evidence is under `third_party/typeduck-web/e2e/results/m30-engine-performance/`; B/C shared payloads, sentence-DP backpointers, and correction-stress indexing are deferred because post-Lever-A rows did not justify those broader rewrites in this slice.

**Goal:** Reduce Yune engine startup memory and startup time from the expanded TypeDuck dictionary representation, then address measured long-input typing hot paths without changing candidate behavior.

**Architecture:** M30 is an engine-only performance milestone. It treats TypeDuck-Web as the measurement and browser-evidence loop, not as the optimization target. The primary lever is reducing duplicated expanded table representation in `StaticTableTranslator`; secondary levers are sentence-lattice path allocation and correction-stress indexing only after fresh measurements prove they still matter.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), Criterion benchmark `crates/yune-rime-api/benches/frontend_baselines.rs`, TypeDuck-Web Playwright performance evidence, existing oracle fixtures for TypeDuck v1.1.2 and upstream `rime/librime 1.17.0`, and the TypeDuck-Web patch workflow.

---

## Status

Draft. Windows frontend/product work is intentionally deferred while this engine performance pass is active.

## Scope

In scope:

- Reduce duplicated expanded table memory in `StaticTableTranslator`.
- Preserve byte-identical candidate text, order, comments, quality, commit, learning, and ABI behavior.
- Measure native startup, single-startup working set, long-input key processing, and browser attribution before and after each accepted lever.
- Keep TypeDuck-Web short-input first-key latency honest: React first mount and cold worker/WASM warm-up are frontend costs, not engine wins.

Out of scope:

- Windows TSF/frontend/product implementation.
- WebView2, TypeDuck-Windows repo work, candidate window UI, or P2-WIN-01.
- P2-WIN-02 rich comment-byte compatibility, unless M30 uncovers a shared regression.
- Candidate ranking, comments, Space/default-confirm behavior, Jyutping composition semantics, `RimeApi`, `RimeCandidate`, or TypeDuck profile ABI changes.
- Claims that short-input browser p95 improved from engine work unless native/worker evidence proves the engine portion moved.

## Current Evidence

- M29 classified the old `1.79GB` peak as repeated-benchmark high-water and recorded real single-startup ready pressure around `1.10GB`.
- M29's startup optimization reduced `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` only modestly: median `5,253,577us` to `5,045,837us`.
- `third_party/typeduck-web/e2e/results/m29-performance/native-startup-after.md` is the baseline evidence for M30.
- The remaining startup owner is representation/materialization-heavy:
  - `crates/yune-core/src/translator/mod.rs:107` stores `entries: Vec<(String, Candidate)>`.
  - `crates/yune-core/src/translator/mod.rs:108` stores `entries_by_code: BTreeMap<String, Vec<Candidate>>`.
  - `crates/yune-core/src/translator/mod.rs:1143-1152` builds `entries_by_code` by cloning candidate payloads.
  - `crates/yune-core/src/translator/mod.rs:109` stores `spelling_abbreviation_entries` as cloned `(code, text, comment)` triples.
  - `crates/yune-core/src/spelling_algebra.rs:572-596` deduplicates with cloned `(code, text, comment)` tuple keys.
- The long-input typing hot path includes `path.clone()` in `crates/yune-core/src/translator/mod.rs:1052`.
- The correction stress path scans `self.entries_by_code.keys()` in `crates/yune-core/src/translator/mod.rs:487-519`; it is mostly outside normal TypeDuck typing because correction is off by default, but it remains useful stress coverage.

## Acceptance Gates

- `M30-PERF-01`: Fresh M30 native and browser baselines are captured before implementation, using the current post-M29 code.
- `M30-PERF-02`: M29 evidence markdown tables match committed JSON before M30 optimization claims are made.
- `M30-PERF-03`: Lever A either lands with before/after evidence proving reduced startup memory and no behavior changes, or is rejected with evidence.
- `M30-PERF-04`: Any internal string-sharing / compact abbreviation representation lands only after Lever A and preserves public `Candidate` and ABI shapes.
- `M30-PERF-05`: Long-input sentence-lattice optimization, if landed, reduces native long-input engine cost without changing accepted M28 follow-up ranking fixtures.
- `M30-PERF-06`: Correction-stress optimization, if landed, improves the correction-on stress row while proving correction-off normal typing stays flat.
- `M30-PERF-07`: Full compatibility gates remain green, including Rust fmt/clippy/tests, `frontend_baselines`, runtime package tests/build, TypeDuck-Web build, focused browser performance evidence, patch checks when source changes, and `git diff --check`.

## File Responsibilities

- `crates/yune-core/src/translator/mod.rs`: owns `StaticTableTranslator` representation, lookup index, sentence candidate DP, and correction scan.
- `crates/yune-core/src/spelling_algebra.rs`: owns expansion and deduplication of spelling-algebra variants.
- `crates/yune-rime-api/benches/frontend_baselines.rs`: owns startup, memory, and key-processing benchmarks.
- `crates/yune-core/tests/cantonese_parity.rs`: owns TypeDuck profile behavior parity.
- `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`: owns upstream core behavior parity.
- `crates/yune-rime-api/tests/typeduck_web.rs`: owns browser ABI/runtime adapter contract.
- `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`: owns real-browser evidence.
- `third_party/typeduck-web/e2e/results/m30-engine-performance/`: new evidence folder for M30.
- `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`: regenerate only if TypeDuck-Web source changes.

---

## Task 0 - Repair And Freeze The M29 Evidence Baseline

**Files:**

- Modify: `third_party/typeduck-web/e2e/results/m29-performance/native-startup-after.md`
- Read: `third_party/typeduck-web/e2e/results/m29-performance/browser-startup-after.json`
- Read: `third_party/typeduck-web/e2e/results/m29-performance/typing-attribution-after.json`

- [x] **Step 0.1: Verify browser table values against JSON**

Before relying on any property path, inspect the committed JSON keys and confirm whether the startup files expose both `freshStartup.marker.totalMs` / `reloadStartup.marker.totalMs` and the convenience `startupTotalsMs` object. Do not assume `.marker.totalMs` exists without this shape check.

Run:

```powershell
$beforeStartup = Get-Content third_party\typeduck-web\e2e\results\m29-performance\browser-startup-before.json -Raw | ConvertFrom-Json
$afterStartup = Get-Content third_party\typeduck-web\e2e\results\m29-performance\browser-startup-after.json -Raw | ConvertFrom-Json
$beforeTyping = Get-Content third_party\typeduck-web\e2e\results\m29-performance\typing-attribution-before.json -Raw | ConvertFrom-Json
$afterTyping = Get-Content third_party\typeduck-web\e2e\results\m29-performance\typing-attribution-after.json -Raw | ConvertFrom-Json
"startup before keys=$($beforeStartup.PSObject.Properties.Name -join ',')"
"startup after keys=$($afterStartup.PSObject.Properties.Name -join ',')"
"startup before fresh=$($beforeStartup.freshStartup.marker.totalMs) reload=$($beforeStartup.reloadStartup.marker.totalMs)"
"startup after fresh=$($afterStartup.freshStartup.marker.totalMs) reload=$($afterStartup.reloadStartup.marker.totalMs)"
"startupTotals before fresh=$($beforeStartup.startupTotalsMs.fresh) reload=$($beforeStartup.startupTotalsMs.reload)"
"startupTotals after fresh=$($afterStartup.startupTotalsMs.fresh) reload=$($afterStartup.startupTotalsMs.reload)"
foreach ($scenario in @("hai","longPhrase","longComposition","paging","reverseLookup")) {
  $b = $beforeTyping.scenarioSummaries.$scenario
  $a = $afterTyping.scenarioSummaries.$scenario
  "$scenario before=$($b.totalKeydownToPaintMs.p95) after=$($a.totalKeydownToPaintMs.p95) worker=$($b.ownerP95Ms.nativeOrWorkerProcess)->$($a.ownerP95Ms.nativeOrWorkerProcess)"
}
```

Expected values:

- Fresh startup: `5299ms` before, `5378ms` after.
- Reload startup: `5211ms` before, `5245ms` after.
- `hai`: `61ms` before, `62ms` after.
- Long phrase: `50ms` before, `59ms` after.
- Long composition: `39ms` before, `44ms` after.
- Paging: `13ms` before, `16ms` after.
- Reverse lookup: `16ms` before, `29ms` after.

- [x] **Step 0.2: Keep the M29 closeout wording conservative**

`native-startup-after.md` must say browser startup and typing were flat/mixed, not wins. It must not claim reverse lookup, paging, or long composition improved if the committed JSON says otherwise.

## Task 1 - Capture Fresh M30 Baselines

**Files:**

- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/m30-baseline.md`
- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/native-before.md`
- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/browser-before.json`

- [x] **Step 1.1: Capture native baseline**

Run:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-before.txt 2>&1"
```

Expected:

- Command exits successfully.
- Existing `yune_rime_api` output filename collision warnings may appear and are non-fatal.
- Baseline file includes rows for:
  - `m29_single_startup_memory_jyut6ping3_mobile`
  - `startup_real_jyut6ping3_mobile_runtime_ready`
  - `startup_trace_jyut6ping3_mobile_spelling_algebra_expand`
  - `per_key_real_jyut6ping3_mobile_hai_full_abi`
  - `per_key_real_jyut6ping3_mobile_jigaajiusihaa_full_abi`

- [x] **Step 1.2: Capture browser baseline**

Run the focused M29 performance browser test as the M30 before run:

```powershell
$env:TYPEDUCK_APP_URL = "http://localhost:5173/web/"
$env:M29_EVIDENCE_LABEL = "m30-before"
npm.cmd --prefix third_party\typeduck-web\e2e run test:e2e -- --grep "M29 PERF" --workers=1
```

Expected:

- The browser app is running from fresh built assets.
- The test writes startup and typing attribution JSON.
- Copy or rename the resulting files into `third_party/typeduck-web/e2e/results/m30-engine-performance/` with `before` in the filename.

- [x] **Step 1.3: Record baseline summary**

Create `m30-baseline.md` with:

- native startup median and p95 for `spelling_algebra_expand`
- single-startup ready bytes and peak bytes
- long phrase native engine/full-ABI p95
- browser long phrase p95 and owner split
- explicit note that `hai` first-key p95 includes frontend first-mount and cold worker/WASM costs

## Task 2 - Lever A: Remove Steady-State Duplicate `entries`

**Files:**

- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify if needed: `crates/yune-rime-api/benches/frontend_baselines.rs`
- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/lever-a.md`

- [x] **Step 2.1: Add a behavior guard before representation changes**

Before deleting or narrowing any `entries` storage, run a source audit:

```powershell
rg -n "self\.entries|\bentries\b|entries_by_code|spelling_abbreviation_entries" crates\yune-core\src\translator\mod.rs
```

Then read every hit in context and classify it as builder-only, steady-state query, debug/inspector/schema-switch, helper, or test-only. The audit must explicitly cover `with_spelling_algebra(...)`, `with_upstream_sentence_model(...)`, lookup/query paths, correction scan, and any debug/inspector/schema-switch path. Record the result in `lever-a.md` before deleting steady-state `self.entries`, and prove no steady-state query path still depends on it.

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_followup
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-rime-api --test typeduck_web
```

Expected:

- Tests pass on the baseline before the representation change.

- [x] **Step 2.2: Refactor index building to move candidates**

In `translator/mod.rs`, add a consuming index builder next to the existing helper:

```rust
fn entries_by_code_from_entries(
    entries: Vec<(String, Candidate)>,
) -> BTreeMap<String, Vec<Candidate>> {
    let mut indexed = BTreeMap::<String, Vec<Candidate>>::new();
    for (code, candidate) in entries {
        indexed.entry(code).or_default().push(candidate);
    }
    indexed
}
```

Use it in construction paths where the source `entries` vector is no longer needed after initialization. Keep the existing borrowed `entries_by_code(...)` helper only for paths that still require a borrowed view during the transition.

- [x] **Step 2.3: Remove steady-state `entries` only after builder-chain needs are handled**

`with_upstream_sentence_model(...)` currently reads `self.entries`. `with_spelling_algebra(...)` currently consumes `self.entries`. Preserve these builder-chain semantics by using one of these concrete approaches:

- keep a builder-only `source_entries: Option<Vec<(String, Candidate)>>` and `take()` it in `with_spelling_algebra(...)`, or
- compute the upstream sentence model before dropping source entries, with a test proving the current call order still works for `luna_pinyin`.

The final steady-state `StaticTableTranslator` must not store two full copies of expanded `Candidate` payloads. It should keep the query index as the runtime source of truth.

- [x] **Step 2.4: Measure Lever A**

Run:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-lever-a.txt 2>&1"
```

Create `lever-a.md` with before/after rows for:

- `startup_trace_jyut6ping3_mobile_spelling_algebra_expand`
- `m29_single_startup_memory_jyut6ping3_mobile`
- `startup_real_jyut6ping3_mobile_runtime_ready`

Expected:

- Candidate behavior tests remain byte-identical.
- If single-startup ready bytes do not improve by at least `100MB`, record the result and stop before larger representation changes.

## Task 3 - Lever B/C: Compact Shared Candidate Payloads

**Completion note:** Deferred from M30 after Lever A. The accepted Lever A slice produced the intended memory win, and the watched per-key rows stayed flat/noisy rather than proving that a broader value-keyed payload rewrite was the next justified owner.

**Files:**

- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-core/src/spelling_algebra.rs`
- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/lever-bc.md`

- [ ] **Step 3.1: Introduce an internal indexed candidate payload**

Keep public `Candidate` unchanged. Intern or `Arc`-share only inside the table translator representation. The conversion back to public `Candidate` happens when building output rows.

Interning/deduplication must be value-keyed by candidate payload values, not by unstable construction-time `Arc` identity. Acceptable value keys include `(text, comment, preedit, source, quality, spelling_abbreviation)` or an equivalent stable payload value key. Only after payloads are value-deduped may the internal representation use `Arc<str>` or payload ids for cheaper storage.

Minimum internal shape:

```rust
#[derive(Clone)]
struct IndexedTableCandidate {
    text: std::sync::Arc<str>,
    comment: std::sync::Arc<str>,
    preedit: Option<std::sync::Arc<str>>,
    source: CandidateSource,
    quality: f32,
    spelling_abbreviation: bool,
}
```

Provide a helper:

```rust
impl IndexedTableCandidate {
    fn to_candidate(&self) -> Candidate {
        Candidate {
            text: self.text.to_string(),
            comment: self.comment.to_string(),
            preedit: self.preedit.as_ref().map(|value| value.to_string()),
            source: self.source.clone(),
            quality: self.quality,
        }
    }
}
```

- [ ] **Step 3.2: Replace abbreviation triple membership**

Replace `HashSet<(String, String, String)>` abbreviation membership with a per-indexed-candidate flag or a compact key that does not clone text/comment strings.

The behavior of `is_spelling_abbreviation_entry(...)` must remain the same:

- abbreviation candidates are ranked/filtered exactly as before
- non-abbreviation candidates remain preferred where existing tests expect them

- [ ] **Step 3.3: Reduce dedupe string cloning**

In `spelling_algebra.rs`, change the dedupe key to avoid cloning `(code, text, comment)` for every intermediate entry after the internal payload is shared. Acceptable implementation:

- use a stable payload id plus code, where the payload id came from value-keyed dedupe over `(text, comment, preedit, source, quality, spelling_abbreviation)`, or
- assign a stable payload id before dedupe and key by `(code, payload_id)`.

Do not key only by text; comments are part of TypeDuck candidate behavior.
Do not use raw `Arc` pointer identity from construction order as the semantic dedupe key.

- [ ] **Step 3.4: Measure Lever B/C**

Run:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-lever-bc.txt 2>&1"
```

Create `lever-bc.md` with:

- memory delta versus baseline and Lever A
- startup span delta versus baseline and Lever A
- per-key row deltas for at least `per_key_real_jyut6ping3_mobile_hai_*` and `per_key_real_jyut6ping3_mobile_jigaajiusihaa_*`
- behavior gate results

Expected:

- Public candidate behavior remains byte-identical.
- Single-startup ready bytes improves materially beyond Lever A.
- If CPU time regresses by more than `10%`, record why the memory win is or is not worth accepting.

## Task 4 - Long-Input Sentence-Lattice DP Backpointers

**Completion note:** Deferred from M30. Existing `jigaajiusihaa` rows were sufficient to watch long-input cost, and post-Lever-A measurements did not justify a sentence-DP rewrite in this slice.

**Files:**

- Modify: `crates/yune-core/src/translator/mod.rs`
- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/sentence-dp.md`

- [ ] **Step 4.1: Add a focused long-input benchmark row if missing**

Ensure `frontend_baselines.rs` includes a row that exercises long TypeDuck input through engine-only and full-ABI paths. The existing `per_key_real_jyut6ping3_mobile_jigaajiusihaa_*` rows are sufficient if they still exist.

- [ ] **Step 4.2: Replace cloned path vectors with backpointers**

Replace `path.clone()` in sentence DP with a backpointer representation:

```rust
struct SentencePathNode {
    previous: Option<usize>,
    piece: String,
}

#[derive(Clone)]
struct SentencePathState {
    node: usize,
    piece_count: usize,
    fuzzy_pieces: usize,
    quality: f32,
    raw_quality: f32,
}
```

Keep the current tie-break order:

1. fewer fuzzy pieces
2. higher `quality`
3. higher `raw_quality`

At the end, reconstruct pieces by walking backpointers and reversing the collected text.

- [ ] **Step 4.3: Measure long-input impact**

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_followup
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-sentence-dp.txt 2>&1"
```

Expected:

- M28 follow-up ranking fixtures remain unchanged.
- Long-input native engine/full-ABI rows improve or remain flat.
- `hai` stays flat.

## Task 5 - Correction Stress Indexing

**Completion note:** Deferred from M30. M26 already reduced the correction-stress path, and M30's correction-on stress row stayed flat after Lever A.

**Files:**

- Modify: `crates/yune-core/src/translator/mod.rs`
- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/correction-stress.md`

- [ ] **Step 5.1: Confirm correction-on stress remains isolated**

Before optimizing, record whether the high-cost correction row requires `enable_correction=true`. The normal TypeDuck profile must remain correction-off unless the schema explicitly enables it.

- [ ] **Step 5.2: Add a length-bucket index**

Build a compact length bucket over canonical codes:

```rust
correction_codes_by_len: BTreeMap<usize, Vec<String>>
```

When scanning dynamic near lookups, only inspect code lengths where `typeduck_length_distance_lower_bound(canonical_code, lookup_code) <= TYPEDUCK_CORRECTION_MAX_DISTANCE` can be true.

- [ ] **Step 5.3: Reuse edit-distance scratch**

Refactor `typeduck_restricted_distance(...)` to accept an optional scratch buffer owned by the caller, or add a sibling helper used only by the scan:

```rust
fn typeduck_restricted_distance_with_scratch(
    left: &str,
    right: &str,
    threshold: usize,
    scratch: &mut Vec<usize>,
) -> usize
```

The result must match `typeduck_restricted_distance(...)` for existing tests. Do not replace the TypeDuck restricted metric with plain Levenshtein.

- [ ] **Step 5.4: Measure correction stress**

Run:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-correction.txt 2>&1"
```

Expected:

- Correction-on stress row improves.
- Correction-off normal rows stay flat.
- Candidate behavior tests remain unchanged.

## Task 6 - Browser Evidence, Docs, And Closeout

**Files:**

- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/browser-after.json`
- Create: `third_party/typeduck-web/e2e/results/m30-engine-performance/task-6-gates.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Archive when complete: `docs/plans/archive/m30-plan-engine-representation-performance.md`

- [x] **Step 6.1: Capture browser after evidence**

Run:

```powershell
$env:TYPEDUCK_APP_URL = "http://localhost:5173/web/"
$env:M29_EVIDENCE_LABEL = "m30-after"
npm.cmd --prefix third_party\typeduck-web\e2e run test:e2e -- --grep "M29 PERF" --workers=1
```

Expected:

- Browser evidence is captured as attribution.
- Short-input first-key p95 is not claimed as an engine win unless worker/native process time also moved.
- Long-input browser scenarios are compared against native engine rows.

- [x] **Step 6.2: Run full gates**

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-final.txt 2>&1"
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
npm.cmd --prefix third_party\typeduck-web\source run build
git diff --check
```

If TypeDuck-Web source changes:

```powershell
git -C third_party\typeduck-web\source diff --binary --submodule=diff > third_party\typeduck-web\patches\yune-typeduck-runtime.patch
git -C third_party\typeduck-web\source apply --reverse --check ..\patches\yune-typeduck-runtime.patch
```

Also forward-check the patch in a clean detached worktree at `third_party/typeduck-web/typeduck-web.lock.json`.

- [x] **Step 6.3: Close docs**

Update:

- `docs/roadmap.md`: M30 complete summary, evidence paths, next sequencing.
- `docs/requirements.md`: mark M30 requirements complete.
- archive this plan under `docs/plans/archive/`.

Do not change P2-WIN-01/P2-WIN-02 status except to say Windows product/frontend work remains deferred until M30 closes.

## Execution Handoff

Paste into a new session:

```text
/goal Complete M30 engine representation performance in C:\Users\laubonghaudoi\Documents\GitHub\yune.

Read AGENTS.md, docs/CONVENTIONS.md, docs/roadmap.md, docs/requirements.md, docs/plans/m30-plan-engine-representation-performance.md, and the archived M26/M27/M29 plans/evidence first.

Windows/P2-WIN work is deferred. This is engine-only performance work using the TypeDuck-Web harness as measurement evidence.

Start by confirming M29 evidence markdown matches committed JSON, then capture fresh M30 baselines. Implement Lever A first: remove steady-state duplicate expanded entries and build entries_by_code by moving candidate payloads. Measure before proceeding. Only continue to internal Arc/shared-payload compaction, sentence-DP backpointers, or correction-stress indexing if the measured data justifies it.

Guardrails: no candidate text/order/comment/ranking/commit behavior changes; do not widen RimeApi, RimeCandidate, or TypeDuck profile ABI; do not claim short-input browser first-key p95 as an engine win unless worker/native time moved. Preserve TypeDuck-Web patch discipline if source changes. Run full gates, archive the plan, update roadmap/requirements, and push directly to origin/main when green.
```
