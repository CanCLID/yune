# M50 Track A Launch-Readiness Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close, or re-close as measured blockers, the remaining native Track A `luna_pinyin` launch-readiness gaps left by M49: `n`, `ni`, the 37-character pinyin row, and full `luna_pinyin` memory attribution.

**Architecture:** This is a native-engine-only, attribution-first milestone. First restore broad clippy as a usable gate, then capture a fresh M50 baseline, then reduce the named latency owners only when the owner evidence points at a bounded implementation, and finally attribute the full `luna_pinyin` memory peak/private shape without conflating it with the M47 TypeDuck keyboard-profile memory work.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), native benchmark harness (`scripts/benchmark-native-rime-inprocess.ps1`, `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`), Windows memory counters, upstream librime `1.17.0` same-run peer evidence, Markdown evidence/report docs.

---

## Scope

In scope:

- Native Track A `luna_pinyin` only.
- Same-run Yune vs upstream librime `1.17.0` evidence.
- The previous broad-clippy blocker at
  `crates/yune-core/src/dictionary/compiled_table.rs:2158`
  (`clippy::too_many_arguments`) introduced by the M49 MARISA traversal helper.
  Task 0 closes this blocker before latency/memory work begins.
- Latency owners for `n`, `ni`, and the 37-character pinyin row.
- Full `luna_pinyin` Track A memory attribution for peak, steady/private proxy, and named owner rows.

Out of scope:

- Web harness, frontend, WASM, public-demo, package, deployment, and product-delivery claims.
- M47 TypeDuck keyboard-profile memory work, Apple `phys_footprint`, and platform validation.
- Learned `.gram` / octagram grammar, unless a fresh upstream `luna_pinyin` oracle fixture proves it is required for the named target.
- ABI changes. The default `rime_get_api()` and `RimeCandidate` layouts must remain unchanged.

## Current Starting Point

M49 closed as a measured partial:

| Row | M49 final | Gate |
| --- | ---: | --- |
| `n` | `62.400 us` / `3.074x` | blocker |
| `ni` | `46.250 us` / `3.269x` | blocker |
| `hao` | `26.300 us` / `2.248x` | pass |
| 37-char pinyin | `894.400 us` / `3.094x` | blocker |
| 59-char pinyin | `1,543.742 us` / `2.280x` | pass |
| Track A peak memory | Yune `188.3 MB` vs librime `17.6 MB` | blocker |

Evidence root to compare against:
`docs/reports/evidence/m49-track-a-short-key-latency/`.

## Files And Responsibilities

- Modify: `crates/yune-core/src/dictionary/compiled_table.rs`
  - Remove the current `clippy::too_many_arguments` blocker without changing runtime behavior.
- Modify if owner evidence requires it: `crates/yune-core/src/dictionary/compiled_table.rs`
  - Track A compact-table prefix traversal, MARISA-backed lookup, and bounded candidate iteration.
- Modify if owner evidence requires it: `crates/yune-core/src/translator/mod.rs`
  - Short-prefix candidate filtering/ranking and bounded first-page materialization.
- Modify if owner evidence requires it: `crates/yune-core/src/poet/mod.rs`
  - 37-character sentence graph rebuild / preset-vocabulary pruning.
- Modify or extend if the benchmark lacks required columns: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
  - Add only the owner counters required to prove M50 decisions.
- Create: `docs/reports/evidence/m50-track-a-launch-readiness/`
  - Baseline, intermediate, and final native benchmark evidence.
- Modify on closeout: `docs/reports/yune-vs-librime-performance.md`
  - Current dashboard values and verdict.
- Modify on closeout: `docs/reports/yune-vs-librime-root-cause-analysis.md`
  - Current owner/root-cause rows.
- Modify on closeout: `docs/roadmap.md`
  - M50 verdict and next sequence.
- Modify on closeout: `docs/ledgers/milestone-history.md`
  - Completed M50 row if M50 is closed.
- Move on closeout: `docs/plans/active/m50-plan-track-a-launch-readiness-completion.md`
  to `docs/plans/completed/m50-plan-track-a-launch-readiness-completion.md`.

## Task 0: Restore Broad Clippy As A Gate

**Files:**

- Modify: `crates/yune-core/src/dictionary/compiled_table.rs`

- [x] **Step 0.1: Reproduce the current clippy blocker**

Run:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
```

Expected before the fix: failure naming `crates/yune-core/src/dictionary/compiled_table.rs:2158` and `clippy::too_many_arguments`.

- [x] **Step 0.2: Fix only the linted signature/site**

Refactor the linted helper so the MARISA traversal context is grouped into a
small private struct or equivalent local abstraction. Preserve traversal order,
prefix compatibility checks, and candidate output.

Do not change parser behavior, byte-backed prism lookup, or any public API.

- [x] **Step 0.3: Verify the focused and broad gates**

Run:

```powershell
cargo fmt --check
cargo test -p yune-core dictionary::
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: all pass. If broad clippy exposes a second unrelated lint, record it in `docs/reports/evidence/m50-track-a-launch-readiness/task0-clippy/README.md` and fix it only if the fix is mechanical and scoped.

- [x] **Step 0.4: Commit the clippy unblock**

```powershell
git add -- crates/yune-core/src/dictionary/compiled_table.rs docs/reports/evidence/m50-track-a-launch-readiness/task0-clippy
git commit -m "Fix Track A clippy gate blocker"
git push origin main
```

Closed by the M50 Task 0 evidence in
`docs/reports/evidence/m50-track-a-launch-readiness/task0-clippy/`.

## Task 1: Fresh M50 Baseline And Owner Attribution

**Files:**

- Create: `docs/reports/evidence/m50-track-a-launch-readiness/phase-0-baseline/`
- Read: `docs/reports/evidence/m49-track-a-short-key-latency/final-native-benchmark/`

- [ ] **Step 1.1: Pull current main and confirm the baseline commit**

Run:

```powershell
git fetch origin main
git status --short
git rev-parse HEAD
git merge-base --is-ancestor ceb435e3e4f01eb93f0e125bb4be152f51aa275d HEAD
```

Expected: worktree is either clean or only known unrelated files are dirty; `merge-base` exits `0`.

- [ ] **Step 1.2: Run the serialized native Track A benchmark**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m50-track-a-launch-readiness\phase-0-baseline `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru `
  -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung `
  -DeployProductBeforeBenchmark
```

Expected: `summary.csv`, `m37_metrics.csv`, `memory-owner-profile.csv`, and per-track subfolders are written. Do not run browser or native probes in parallel with this benchmark.

- [ ] **Step 1.3: Write the Phase 0 evidence README**

Create `docs/reports/evidence/m50-track-a-launch-readiness/phase-0-baseline/README.md` with:

- Heading: `# M50 Phase 0 Baseline`.
- Scope sentence: native Track A `luna_pinyin` only; no browser, frontend,
  package, deployment, public-demo, TypeDuck product, or iOS-device claim.
- Baseline commit: the exact output of `git rev-parse HEAD`.
- A tracked-blockers table copied from `summary.csv` for `n`, `ni`,
  `ceshiyixiachangjushuruxingnengzenyang`, and the maximum Track A peak memory
  row.
- Owner notes copied from `m37_metrics.csv` and `memory-owner-profile.csv` for
  the short-prefix row, the 37-character row, and memory owners.

Do not commit the README until every numeric cell is copied from generated
evidence.

- [ ] **Step 1.4: Decide the M50 reduction order**

Use the baseline owner rows to choose one of these orders:

1. short-prefix first, if `n`/`ni` remain above `3.0x` and the owner is bounded.
2. 37-char first, if it regresses above the short rows or has a clearer no-heap reduction.
3. memory attribution first, if latency already passes but memory remains unexplained.

Record the chosen order in `docs/reports/evidence/m50-track-a-launch-readiness/README.md`.

- [ ] **Step 1.5: Commit Phase 0**

```powershell
git add -- docs/reports/evidence/m50-track-a-launch-readiness
git commit -m "Capture M50 Track A baseline"
git push origin main
```

## Task 2: Reduce `n` / `ni` Short-Prefix Latency

**Files:**

- Modify if evidence points there: `crates/yune-core/src/dictionary/compiled_table.rs`
- Modify if evidence points there: `crates/yune-core/src/translator/mod.rs`
- Modify tests as needed: `crates/yune-core/src/tests/translator.rs`, `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- Create: `docs/reports/evidence/m50-track-a-launch-readiness/short-prefix/`

- [ ] **Step 2.1: Add or tighten a focused guard before implementation**

Add a focused test that protects `n` and `ni` candidate output and bounded work. Prefer extending an existing short-key or upstream Luna parity test rather than adding a separate harness.

The guard must assert:

- `n` and `ni` first-page candidate text remains upstream-compatible.
- Candidate materialization remains page-bounded.
- Storage remains `rsmarisa_byte_backed` for Track A.

- [ ] **Step 2.2: Run the focused guard and confirm it passes before optimization**

Run the exact focused test command chosen in Step 2.1. Expected: pass before optimization, proving this is a regression guard and not a behavior change fixture.

- [ ] **Step 2.3: Implement only an owner-backed short-prefix change**

Acceptable examples:

- avoid repeated code-string construction in the prefix traversal;
- avoid materializing or sorting candidates past the first page for `n`/`ni`;
- skip filters whose predicates cannot affect the first page;
- remove redundant string clones or C-string export work measured in `m37_metrics`.

Rejected examples:

- a retained heap index larger than 1 MB without explicit owner proof;
- changing `n`/`ni` candidate order to win speed;
- treating bare `n` as non-comparable without upstream evidence.

- [ ] **Step 2.4: Run focused tests**

Run:

```powershell
cargo fmt --check
cargo test -p yune-core short_key
cargo test -p yune-core --test upstream_luna_pinyin_parity
```

Expected: all pass.

- [ ] **Step 2.5: Re-run the native benchmark**

Run the same command as Task 1 with:

```powershell
-OutputRoot docs\reports\evidence\m50-track-a-launch-readiness\short-prefix
```

Expected success target:

- `n <= 3.0x`;
- `ni <= 3.0x`;
- no regression above `3.0x` for `hao`, `zhongguo`, 59-char pinyin, or abbreviation rows;
- Track A peak memory does not grow by more than 5% from the Task 1 baseline unless the growth is named and accepted as a blocker.

- [ ] **Step 2.6: Commit or close as measured blocker**

If both rows pass and guards are green:

```powershell
git add -- crates/yune-core/src/dictionary/compiled_table.rs crates/yune-core/src/translator/mod.rs crates/yune-core/src/tests/translator.rs crates/yune-core/tests/upstream_luna_pinyin_parity.rs docs/reports/evidence/m50-track-a-launch-readiness/short-prefix
git commit -m "Reduce Track A short-prefix latency"
git push origin main
```

If either row misses, keep only changes that reduce measured owners without adding retained heap, commit them as partial, and write the blocker into the evidence README.

## Task 3: Reduce 37-Character Sentence-Row Latency

**Files:**

- Modify if evidence points there: `crates/yune-core/src/poet/mod.rs`
- Modify tests as needed: `crates/yune-core/src/tests/poet.rs`, `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- Create: `docs/reports/evidence/m50-track-a-launch-readiness/sentence-row/`

- [ ] **Step 3.1: Add a no-retained-heap guard**

Add or extend a poet test so the sentence path skips irrelevant preset-vocabulary entries before expensive phrase-code derivation. The test must fail if the implementation reintroduces a large retained vocabulary prefix index.

Minimum assertion shape:

```rust
assert!(
    metrics.upstream_sentence_model_vocabulary_entries_considered <= expected_bound,
    "sentence row should prune irrelevant preset vocabulary before graph rebuild: {metrics:?}"
);
```

Choose `expected_bound` from a small synthetic fixture, not from the production benchmark.

- [ ] **Step 3.2: Run the focused poet guard**

Run:

```powershell
cargo test -p yune-core poet
```

Expected: pass before deeper optimization if it extends an existing invariant, or fail first if it captures a newly proven pruning gap.

- [ ] **Step 3.3: Implement only transient or byte-backed pruning**

Acceptable examples:

- better transient prefix pruning using existing character-code data;
- bounded phrase-code derivation for impossible suffixes;
- byte-backed compact vocabulary payload lookup if a persistent index is required.

Rejected examples:

- retained `poet.vocabulary_prefix_index` or equivalent large heap owner;
- dropping preset vocabulary needed by M48 `jianli`/`biancheng` parity;
- special-casing the benchmark input string.

- [ ] **Step 3.4: Verify correctness**

Run:

```powershell
cargo fmt --check
cargo test -p yune-core poet
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
```

Expected: all pass.

- [ ] **Step 3.5: Re-run the native benchmark**

Run the same command as Task 1 with:

```powershell
-OutputRoot docs\reports\evidence\m50-track-a-launch-readiness\sentence-row
```

Expected success target:

- 37-character pinyin row `<= 3.0x`;
- 59-character pinyin remains `<= 3.0x`;
- `jianli` / `biancheng` upstream parity remains green;
- no Track A memory regression above the accepted Task 1 baseline band.

- [ ] **Step 3.6: Commit or close as measured blocker**

Commit retained reductions with:

```powershell
git add -- crates/yune-core/src/poet/mod.rs crates/yune-core/src/tests/poet.rs crates/yune-core/tests/upstream_luna_pinyin_parity.rs docs/reports/evidence/m50-track-a-launch-readiness/sentence-row
git commit -m "Reduce Track A sentence-row latency"
git push origin main
```

If the target misses, preserve only measured-safe reductions and record the blocker.

## Task 4: Attribute Full `luna_pinyin` Track A Memory

**Files:**

- Modify only if needed: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Modify only if needed: memory-owner row providers under `crates/yune-core/src/`
- Create: `docs/reports/evidence/m50-track-a-launch-readiness/memory-attribution/`

- [ ] **Step 4.1: Confirm whether existing owner rows explain the peak**

Compare:

- `summary.csv` peak working set;
- row-level private proxy if available;
- `memory-owner-profile.csv`;
- deploy/product status rows;
- `m37_metrics.csv`.

Write `docs/reports/evidence/m50-track-a-launch-readiness/memory-attribution/README.md`
with a table covering these exact classes: working-set peak, private proxy,
named heap owners, clean/file-backed payload, and unexplained gap. Each row must
name the evidence file used (`summary.csv`, `memory-owner-profile.csv`, native
probe output, or a derived calculation) and classify the row as explained,
reducible, or a measured blocker.

- [ ] **Step 4.2: Add instrumentation only for unexplained owners**

If the unexplained gap is larger than 10 MB, add owner rows at the owning data structure, not in the benchmark script. Keep labels stable and specific, for example:

- `poet.preset_vocabulary`
- `compact_table.normal_code_index`
- `compact_table.rsmarisa_string_table`
- `schema.workspace_payload`

- [ ] **Step 4.3: Verify instrumentation**

Run:

```powershell
cargo fmt --check
cargo test -p yune-core memory_owner
```

If no `memory_owner` test selector exists for the touched module, run the narrow module selector that owns the added row, then record the exact command in the evidence README.

- [ ] **Step 4.4: Re-run the benchmark or native probe**

If the in-process benchmark has enough memory data, run the Task 1 benchmark with:

```powershell
-OutputRoot docs\reports\evidence\m50-track-a-launch-readiness\memory-attribution
```

If private/dirty split requires the native memory probe, run only the focused native memory probe and record the exact command in the README. Do not claim iOS `phys_footprint`.

- [ ] **Step 4.5: Decide whether to reduce or carry forward**

Reduce in M50 only if the owner is clearly in-scope, dirty/private, and reducible without risking latency/correctness. Otherwise close memory as an attributed measured blocker.

## Task 5: Final Gate And Closeout

**Files:**

- Modify: `docs/reports/yune-vs-librime-performance.md`
- Modify: `docs/reports/yune-vs-librime-root-cause-analysis.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/ledgers/milestone-history.md`
- Move: `docs/plans/active/m50-plan-track-a-launch-readiness-completion.md`
  to `docs/plans/completed/m50-plan-track-a-launch-readiness-completion.md`

- [ ] **Step 5.1: Run final native benchmark**

Run the Task 1 benchmark command with:

```powershell
-OutputRoot docs\reports\evidence\m50-track-a-launch-readiness\final-native-benchmark
```

- [ ] **Step 5.2: Run final gates**

Required:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core short_key
cargo test -p yune-core poet
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
```

Run broader `cargo test --workspace` only if shared ABI/runtime files changed.

- [ ] **Step 5.3: Close with the correct verdict**

Success requires:

- `n <= 3.0x`;
- `ni <= 3.0x`;
- 37-character pinyin `<= 3.0x`;
- no tracked passing Track A row regresses above `3.0x`;
- broad clippy is green;
- memory is either reduced or attributed with a named measured blocker;
- reports and roadmap show current values only in dashboard sections.

Close as partial if any target misses. Do not reframe a real memory peak away.

- [ ] **Step 5.4: Commit and push closeout**

```powershell
git add -- docs/reports/yune-vs-librime-performance.md docs/reports/yune-vs-librime-root-cause-analysis.md docs/roadmap.md docs/ledgers/milestone-history.md docs/plans/completed/m50-plan-track-a-launch-readiness-completion.md docs/reports/evidence/m50-track-a-launch-readiness
git commit -m "Close M50 Track A launch-readiness"
git push origin main
```

If the plan remains partial and should not move to completed yet, keep it under `docs/plans/active/` and commit the partial evidence with a message that says `partial`.
