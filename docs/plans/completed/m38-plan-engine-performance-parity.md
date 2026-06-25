# M38 Engine Performance Parity Plan

> **Status:** Complete - **Milestone:** M38 (engine performance parity) - **Created:** 2026-06-24 - **Completed:** 2026-06-25 - **Type:** engine-performance plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Make Yune's isolated engine performance converge toward upstream librime `1.17.0` across startup, schema lifecycle, mmap-backed `rsmarisa` table lookup latency, lazy/page-bounded candidate production, context export, memory, and allocation behavior.

**Architecture:** M38 treats performance as a pure engine problem. The milestone starts with isolated native engine measurements against librime, decomposes each gap into named owners, then optimizes the top engine owner without involving frontend, delivery, packaging, or application-specific pipelines. A real marisa-backed deployed table reader through `rsmarisa` over mmap/file-backed deployed bytes is a required engine data-path outcome, not a probe or optional storage experiment. Lazy/page-bounded candidate iteration is also a required outcome for ordinary first-page reads. Integration paths are used only as regression guards after isolated engine wins are proven.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), `rsmarisa`, native in-process benchmark harness, upstream `luna_pinyin` deployed table/prism/reverse assets, marisa-backed deployed table fixtures, `StaticTableTranslator`, `TableLookup`, `CompactTableStore`, `RimePrismBinPayload`, startup trace spans, per-key owner metrics, allocation/memory counters, upstream librime `1.17.0` oracle, and report visualizations under `docs/reports/evidence/`.

---

## Closeout Summary

M38 closed on `2026-06-25` with final same-run native evidence in
`docs/reports/evidence/m38-engine-performance-parity/phase-5-final-native/`.

Final Track A upstream `luna_pinyin` medians versus same-run librime:

| Row | Yune median | librime median | Ratio | Gate |
| --- | ---: | ---: | ---: | --- |
| startup/runtime-ready | `23,363.300us` | `24,351.000us` | `0.959x` | pass, under `1.25x` |
| session create/select/destroy | `24,243.500us` | `27,969.500us` | `0.867x` | pass, under `1.25x` |
| `hao` | `38.933us` | `11.400us` | `3.415x` | pass, under `5x` |
| `ni` | `56.750us` | `14.300us` | `3.969x` | pass, under `5x` |
| `zhongguo` | `64.263us` | `181.375us` | `0.354x` | pass, under `5x` |

Final selected Track A storage is `rsmarisa_byte_backed`, with
`table_mapping_mode=mmap`, `prism_mapping_mode=mmap`,
`source_fallback=false`, table/prism heap mirror bytes `0`, positive
`rsmarisa` exact/prefix lookup counters, bounded first-page reads, and no
ordinary no-marisa compact fallback on the target rows.

Whole-process memory remains above librime and is explicitly carried as
residual context: Yune Track A medians are about `108-112 MB` versus librime
about `10-13 MB`, with Yune max peak `163,151,872 B`. This does not block M38
because the selected hot path has no hidden table/prism heap mirror and the
remaining gap is reported honestly.

Final verification passed:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- focused compact-table, upstream `luna_pinyin`, schema-selection,
  context-status, and yune-web reverse-lookup tests
- `cargo test --workspace`
- `npm.cmd --prefix packages/yune-web-runtime test`
- `npm.cmd --prefix packages/yune-web-runtime run build`
- final native benchmark
- final `git diff --check` after closeout docs

No frontend, browser, product, packaging, deployment, or public-delivery speed
claim is made from M38.

---

## Strategic Reset

M38 is not an application-pipeline milestone. It is an engine-performance
milestone. The old mistake was letting end-to-end application behavior steer engine
optimization. That hides the real engine owners behind unrelated deployment,
UI, profile, and integration costs.

The new rule is simple:

1. Measure the isolated engine path.
2. Compare it shoulder to shoulder with librime.
3. Attribute the gap before changing code.
4. Optimize the largest engine owner.
5. Keep measuring until the benchmarked startup and typing rows reach
   librime-level performance.

The benchmark target remains upstream `luna_pinyin` because it is the cleanest
shared schema for Yune-versus-librime comparison. The optimization target is
broader than one schema: every accepted change should improve Yune's general
engine data path rather than a special application pipeline.

The marisa table requirement belongs in this engine milestone. librime's table
path is fast because deployed table data is compact, shared, and read through
indexed structures rather than rebuilt into large heap mirrors. M38 must prove
that Yune can use a real marisa-backed deployed table through `rsmarisa` in the
hot lookup path, backed by mmap/file-backed deployed bytes on native. A payload
mmap probe is not enough; ordinary benchmark rows must report that table lookup
was served by the `rsmarisa` backend, that the selected bytes were mapped or
borrowed rather than copied into a full heap mirror, and that candidate
production stayed lazy/page-bounded for page-sized reads.

## Parity Dimensions

M38 tracks parity in these dimensions:

| Dimension | Primary rows | Closeout expectation |
| --- | --- | --- |
| Warm startup | runtime-ready with shared upstream assets | Within `1.25x` of same-run librime. |
| Session lifecycle | create/select/destroy | Within `1.25x` of same-run librime. |
| Deployed table backend | selected table format and lookup backend | Real marisa table selected through `rsmarisa` over mmap/file-backed deployed bytes; probe-only evidence does not close. |
| Mapped storage | mapping mode, bytes copied, heap mirror bytes | Native hot path uses mmap/file-backed table/prism bytes without a full heap mirror. |
| Raw lookup | prism lookup and table lookup microbench rows | Owner named, then moved toward librime-shaped lookup cost. |
| Key processing | `hao`, `ni`, `zhongguo` | Each row within `5x` of same-run librime, with the remaining owner named. |
| Candidate production | iterator mode, lookup views, owned candidates, sort/top-K count | Lazy/page-bounded work for page-sized reads unless a measured full-list semantic requires more. |
| Context / ABI export | context clone count and C-string allocation | Page-sized export and bounded allocation. |
| Memory | working set and peak working set | No large hidden heap mirror; memory movement or owner explanation required. |
| Allocation | per-row allocation count/bytes where measurable | Top allocation owner named and reduced when it blocks latency or memory parity. |

## Starting Evidence

M37 final upstream comparison:

| Row | Yune median | librime median | Gap |
| --- | ---: | ---: | ---: |
| startup/runtime-ready | `50,415.700us` | `29,163.700us` | `1.73x` |
| session create/select/destroy | `48,233.200us` | `29,940.000us` | `1.61x` |
| `hao` key sequence | `4,145.500us` | `11.900us` | `348.36x` |
| `ni` key sequence | `3,171.050us` | `14.600us` | `217.20x` |
| `zhongguo` key sequence | `4,801.675us` | `185.300us` | `25.91x` |
| median working set | `159-161 MB` | `11-13 MB` | about `12-14x` |

The first interpretation is that lifecycle is plausibly closeable in one
focused pass, while per-key lookup/candidate production is still structurally
wrong. The short-key rows are too slow for the explanation to be normal language
or Rust overhead. M38 must isolate whether the top owner is prism lookup, table
lookup, completion breadth, candidate materialization, sorting, context export,
ABI allocation, memory layout, or repeated setup.

## Scope

In scope:

- Upstream `luna_pinyin` Yune-versus-librime native comparison.
- A real marisa-backed deployed table fixture for the benchmarked engine path.
- Native mmap/file-backed loading for the benchmarked deployed table/prism bytes,
  with no full heap mirror on the selected hot path.
- `rsmarisa` runtime table lookup, including exact and prefix/completion lookup
  counters proving hot-path use.
- Lazy/page-bounded translation iteration for ordinary first-page reads,
  including counters for page limit, surplus, owned candidates, and full-list
  fallback.
- Engine-only microbenchmarks that isolate raw prism lookup, raw table lookup,
  translator candidate production, context export, memory, and allocation.
- Startup/session attribution inside Yune's engine and ABI-shaped engine entry
  points.
- General engine data-path changes that improve measured owners.
- Existing compatibility tests as regression guards after engine changes.
- Report updates that separate isolated engine evidence from application or
  delivery evidence.

Out of scope:

- Frontend, UI, browser, installer, deployment, application shell, or external
  project performance.
- Application-specific schemas or profile semantics as performance targets.
- Optimizing an end-to-end application pipeline before the isolated engine owner
  is named and moved.
- Closing with `rsmarisa` probe evidence only. The hot path must use the marisa
  table backend.
- Closing with mmap probe evidence only. The selected native hot path must use
  mmap/file-backed deployed bytes or a documented borrowed equivalent.
- Hiding eager full-list materialization behind a faster table backend.
- Claiming public delivery or frontend wins from native engine evidence.
- Widening public ABI structs for convenience.
- Cloning librime's C++ architecture instead of copying the data-path lessons
  that measurements prove useful.

## Non-Negotiable Closeout Gates

- `M38-ENGINE-01` (pure engine target): Every M38 performance claim must be
  based on isolated engine evidence. End-to-end application rows may appear only
  as regression guards.
- `M38-ENGINE-02` (fresh same-run baseline): Phase 0 must rerun Yune and
  librime `1.17.0` in the same native harness and record startup, session,
  `hao`, `ni`, `zhongguo`, working set, and peak working set before
  implementation.
- `M38-ENGINE-03` (owner attribution first): Before optimizing, Phase 0 must
  split Yune rows into startup/session owners and per-key owners: runtime init,
  schema config, deployed artifact open/read/map, mapping mode, deployed bytes
  copied, heap mirror bytes, freshness check, prism parse, table parse/index
  install, selected table backend, marisa lookup calls, prism lookup, table
  lookup, prefix/completion enumeration, iterator mode, page limit, surplus
  bound, candidate view creation, owned candidate materialization, sort/top-K,
  full-list fallback count, filters/rankers/userdb if active,
  context snapshot/export, ABI allocation/free, working set, and allocation
  count/bytes.
- `M38-ENGINE-04` (marisa hot-path gate): Final engine status must report a
  marisa-backed deployed table selected through `rsmarisa` for the benchmarked
  table path. Final per-key and microbench metrics must show positive
  `rsmarisa` exact and/or prefix lookup calls and zero ordinary fallback through
  the old no-marisa compact table for the target rows. `rsmarisa_status=ok`,
  mmap success, or extracted-payload probes are insufficient.
- `M38-ENGINE-05` (native mmap/file-backed gate): Final selected table/prism
  bytes for the native benchmark path must be mmap-backed or otherwise
  file-backed/borrowed from deployed bytes. Metrics must report mapping mode,
  table bytes source, bytes copied, and heap mirror bytes. Loading selected
  hot-path data through `fs::read` into an owned table buffer or keeping a full
  owned table mirror does not close M38.
- `M38-ENGINE-06` (lifecycle target): Final startup and session medians must be
  within `1.25x` of same-run librime.
- `M38-ENGINE-07` (lookup target): Final `hao`, `ni`, and `zhongguo` key rows
  must each be within `5x` of same-run librime. Rows that merely improve from
  M37 but remain outside this bound do not close M38.
- `M38-ENGINE-08` (raw lookup proof): M38 must add or preserve Yune-only
  microbench rows for raw prism lookup, raw table lookup, and translator
  candidate production. These rows must show whether the remaining gap is raw
  lookup or higher-level candidate/context work.
- `M38-ENGINE-09` (lazy/page-bounded iterator gate): Ordinary first-page reads
  must flow through a lazy/page-bounded translation iterator or equivalent
  bounded view. Metrics must show owned candidates, candidate clones, context
  export, and sort/top-K work scale with visible page plus bounded surplus, not
  the full candidate list. Any full-list fallback must be explicit in counters
  and justified by a measured semantic.
- `M38-ENGINE-10` (memory and allocation visibility): Final evidence must report
  working set, peak working set, and the best available allocation attribution.
  A latency win that adds a large hidden heap mirror does not close M38.
- `M38-ENGINE-11` (behavior): Upstream `luna_pinyin` oracle fixtures, paging,
  selection, deletion, context reads, and existing workspace compatibility tests
  remain green where shared code is touched.
- `M38-ENGINE-12` (honest claims): Native engine wins are not frontend,
  browser, application, deployment, or public delivery wins without separate
  evidence.
- `M38-ENGINE-13` (quality gates): At closeout run `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, focused engine and
  touched compatibility tests, `cargo test --workspace`, final native
  benchmark, report/chart checks if touched, and `git diff --check`.

## File Responsibilities

- `Cargo.toml`, `Cargo.lock`, and crate manifests: own the `rsmarisa`
  dependency, feature flags, local patch/fork decisions, and platform feature
  boundaries.
- `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`: owns same-run
  Yune/librime engine rows, ratio tables, raw lookup microbench rows, memory
  rows, selected-backend status, and metrics export.
- `crates/yune-rime-api/src/startup_trace.rs`: owns startup/session trace names
  and lifecycle owner spans.
- `crates/yune-rime-api/src/schema_install.rs`: owns deployed artifact
  selection, table/prism/reverse loading, freshness checks, and translator
  install caching.
- `crates/yune-rime-api/src/session.rs`: owns session creation and schema
  selection behavior. Any lifecycle optimization must preserve fresh-session
  semantics.
- `crates/yune-core/src/dictionary/compiled_table.rs`: owns compact table
  lookup, byte-backed storage, marisa table parsing, `rsmarisa` lookup,
  code-group/string-id lookup, and low-level table reader changes.
- `crates/yune-core/src/dictionary/compiled_prism.rs`: owns prism
  canonical-code lookup and any cache/index needed to avoid repeated spelling
  work.
- `crates/yune-core/src/dictionary/query_table.rs`: owns the internal lookup
  abstraction. Revise it if the current iterator shape prevents `rsmarisa`
  lookup or forces unnecessary materialization.
- `crates/yune-core/src/translator/mod.rs`: owns `StaticTableTranslator`,
  prefix/completion behavior, bounded/eager decisions, materialization, and
  source-entry retention.
- `crates/yune-core/src/engine.rs`: owns candidate refresh, stable top-K versus
  full sort, context state, and page/window retention.
- `crates/yune-rime-api/src/context_api.rs`: owns `RimeGetContext` C ABI export
  and page-sized allocation behavior.
- `docs/reports/evidence/m38-engine-performance-parity/`: owns all M38
  evidence.
- `docs/reports/yune-vs-librime-performance.md`,
  `docs/reports/yune-vs-librime-root-cause-analysis.md`, `docs/roadmap.md`, and
  `docs/requirements.md`: own public claims and closeout status.

---

## Task 0 - Fresh Baseline And Engine Attribution

**Files:**

- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Modify: `crates/yune-rime-api/src/startup_trace.rs`
- Modify: `crates/yune-core/src/m37_metrics.rs` or successor metrics module
- Create: `docs/reports/evidence/m38-engine-performance-parity/phase-0-baseline/`

- [x] **Step 0.1: Confirm repo state**

Run:

```powershell
git fetch origin --prune
git status --short --branch --untracked-files=all
git log --oneline -5 --decorate
```

Expected:

- Worktree dirt is known before editing.
- The branch contains this active M38 engine-performance plan.

- [x] **Step 0.2: Rerun the same-run engine baseline**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m38-engine-performance-parity\phase-0-baseline -Iterations 9 -SessionIterations 20 -KeyIterations 50
```

Expected:

- Summary includes Yune and librime rows for startup, session, `hao`, `ni`,
  `zhongguo`, working set, and peak working set.
- Baseline summary records same-run ratios.

- [x] **Step 0.3: Add lifecycle owner spans**

Add or verify spans for:

- runtime initialize;
- schema config load;
- table path resolve/open/read or mmap;
- mapping mode and deployed bytes copied into owned buffers;
- heap mirror bytes for table/prism/reverse data;
- source checksum/freshness check;
- prism load/parse;
- table parse/index install;
- spelling algebra expansion;
- translator cache lookup hit/miss;
- engine/session construction;
- explicit schema select.

Expected:

- `phase-0-baseline/startup-session-attribution.md` names the largest startup
  and session owners before implementation work.

- [x] **Step 0.4: Add per-key owner metrics**

For `hao`, `ni`, and `zhongguo`, record:

- process-key total;
- prism canonical-code lookup time and result count;
- exact table lookup count/time;
- prefix/completion lookup count/time;
- selected table backend;
- mapping mode and table bytes source;
- deployed bytes copied and heap mirror bytes;
- `rsmarisa` exact lookup calls;
- `rsmarisa` prefix/completion lookup calls;
- no-marisa compact fallback lookup calls;
- iterator mode, page limit, and surplus bound;
- lookup views visited;
- owned candidates materialized;
- candidates sorted or considered by top-K;
- full-list fallback count;
- candidates stored;
- context snapshot/export candidates cloned;
- ABI candidates exported and C-string allocation time.

Expected:

- `phase-0-baseline/key-attribution.md` explains why the short-key rows remain
  far slower than librime.

- [x] **Step 0.5: Add raw engine microbench rows**

Add Yune-only rows that isolate:

- raw prism lookup for `hao`, `ni`, and `zhongguo`;
- raw table exact lookup for canonical codes produced by the prism;
- raw prefix/completion lookup for `hao` and `ni`;
- raw `rsmarisa` exact and prefix lookup over a real marisa-backed deployed
  table loaded through the final mmap/file-backed path;
- translator candidate production without `RimeGetContext`, split by lazy
  iterator/window counters and owned materialization counters;
- page-sized context export from an already-built candidate window.

Expected:

- The microbench rows separate raw lookup from translator and ABI/context cost.
- They are marked Yune-only and are not presented as cross-engine comparison
  ratios.

## Task 1 - Startup And Session Parity

**Files:**

- Modify: `crates/yune-rime-api/src/schema_install.rs`
- Modify: `crates/yune-rime-api/src/session.rs`
- Modify: `crates/yune-rime-api/src/startup_trace.rs`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Test: `crates/yune-rime-api/src/tests/schema_selection/`

- [x] **Step 1.1: Optimize the measured lifecycle owner**

Use Task 0 evidence to choose the first lifecycle change:

- if deployed artifact loading dominates, cache immutable table/prism/reverse
  data by artifact fingerprint;
- if checksum/freshness dominates, cache freshness results until deploy or file
  identity changes;
- if translator install dominates, reuse compatible immutable translator state;
- if schema selection dominates, make same-schema select a cheap state switch
  while preserving fresh-session semantics.

Expected:

- The change targets the top measured lifecycle owner, not a speculative cache.

- [x] **Step 1.2: Capture lifecycle checkpoint**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m38-engine-performance-parity\phase-1-lifecycle -Iterations 9 -SessionIterations 20 -KeyIterations 50
```

Expected:

- Startup and session are within `1.25x` of same-run librime, or the remaining
  owner is named before moving to lookup work.

## Task 2 - Marisa Table, Raw Lookup, And Candidate Production

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/yune-core/src/dictionary/compiled_prism.rs`
- Modify: `crates/yune-core/src/dictionary/compiled_table.rs`
- Modify: `crates/yune-core/src/dictionary/query_table.rs`
- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-core/src/engine.rs`
- Test: `crates/yune-core/src/tests/`
- Test: `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`

- [x] **Step 2.1: Prove the marisa table fixture**

Add or identify a real upstream-compatible deployed table fixture whose table
payload includes a marisa string table. Record its table format, marisa payload
offset/length, key count, sample keys, and checksum in:

```text
docs/reports/evidence/m38-engine-performance-parity/phase-2-lookup/marisa-table-fixture.md
```

Expected:

- The benchmarked engine path has a real marisa-backed table artifact.
- The evidence distinguishes a real runtime lookup fixture from an extracted
  probe payload.

- [x] **Step 2.2: Implement the `rsmarisa` table backend**

Implement an internal marisa-backed table store that can:

- mmap or otherwise borrow the deployed table bytes safely on native;
- load the embedded marisa table through `rsmarisa`;
- answer exact lookup;
- answer prefix/completion lookup;
- return borrowed or compact candidate views without building a full heap table;
- report backend status, mapping mode, table bytes source, heap mirror bytes,
  and per-key lookup counters.

Expected:

- The backend implements Yune's internal table lookup surface or a revised
  lookup surface that avoids unnecessary owned strings.
- The implementation does not build a parallel full heap mirror for normal
  lookup.
- Native benchmark rows report `mapping_mode=mmap` or an equivalent
  file-backed/borrowed selected-byte path.

- [x] **Step 2.3: Select `rsmarisa` in the hot path**

Route the benchmarked engine table path to the marisa backend when the deployed
table supports it. Keep fallback explicit and counted.

Expected:

- Final status reports `table_backend=rsmarisa` or equivalent for the target
  benchmark rows.
- Final status reports `mapping_mode=mmap` or equivalent and
  `table_bytes_source=mapped` or equivalent for the same rows.
- Per-key metrics show `rsmarisa` lookup calls and zero ordinary fallback to the
  no-marisa compact table.

- [x] **Step 2.4: Optimize the measured lookup owner**

Use Task 0 metrics to choose the first lookup change:

- if prism lookup dominates, cache canonical-code lookup or revise prism access;
- if table lookup dominates, reduce code-group/string lookup overhead;
- if prefix breadth dominates, jump directly to the matching compact range and
  stop at the page/surplus bound;
- if materialization dominates, keep lookup candidates borrowed until the page
  boundary;
- if sorting dominates, replace full sort with stable top-K where byte identity
  is proven.

Expected:

- No unrelated storage rewrite lands before the top lookup owner is named.

- [x] **Step 2.5: Implement lazy/page-sized engine iteration**

For ordinary first-page reads, generate candidates through a lazy/page-bounded
iterator or equivalent bounded view: only the visible page plus bounded surplus
may be materialized unless a measured full-list semantic explicitly requires
more.

Expected:

- `hao` and `ni` no longer materialize/sort/export a large completion set before
  returning one page.
- Metrics report iterator/window mode, page limit, surplus, owned candidates,
  sorted/considered candidates, and full-list fallback count.
- Candidate order stays byte-identical.

- [x] **Step 2.6: Keep exact multi-syllable lookup direct**

Ensure multi-syllable exact/canonical-code lookup avoids unnecessary prefix
fallback and does not pay short-key completion breadth.

Expected:

- `zhongguo` remains the proof row for direct table/prism lookup.

- [x] **Step 2.7: Capture lookup checkpoint**

Run the benchmark into:

```powershell
docs\reports\evidence\m38-engine-performance-parity\phase-2-lookup
```

Expected:

- The marisa backend is active in the hot path.
- `hao`, `ni`, and `zhongguo` are each within `5x` of same-run librime, or M38
  remains open with a measured owner table explaining the blocker.

## Task 3 - Context Export, ABI Allocation, And Memory

**Files:**

- Modify: `crates/yune-core/src/engine.rs`
- Modify: `crates/yune-core/src/state.rs`
- Modify: `crates/yune-rime-api/src/context_api.rs`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Test: `crates/yune-rime-api/src/tests/`

- [x] **Step 3.1: Measure export and allocation after lookup changes**

After Task 2, check whether `RimeGetContext`, C-string export, allocation, or
working-set growth is now the top owner.

Expected:

- If export/allocation is small, do not rewrite it.
- If export/allocation dominates, continue with Step 3.2.

- [x] **Step 3.2: Export only the visible page**

Ensure `RimeGetContext` exports page-sized candidates without cloning a full
list.

Expected:

- Context candidate clones equal page size for ordinary first-page reads.
- `RimeCandidate` ABI layout remains unchanged.

- [x] **Step 3.3: Reduce transient allocation**

If allocation is measured as a top owner, reuse bounded page buffers or avoid
duplicate string formatting before ABI export.

Expected:

- Allocation changes are internal and covered by free-context tests.

## Task 4 - Final Evidence And Report Closeout

**Files:**

- Modify: `docs/reports/yune-vs-librime-performance.md`
- Modify: `docs/reports/yune-vs-librime-root-cause-analysis.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Moved on closeout to `docs/plans/completed/m38-plan-engine-performance-parity.md`
- Create: `docs/reports/evidence/m38-engine-performance-parity/final-gates.md`

- [x] **Step 4.1: Run final engine benchmark**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m38-engine-performance-parity\phase-5-final-native -Iterations 9 -SessionIterations 20 -KeyIterations 50
```

Expected:

- Final evidence includes same-run Yune and librime summaries, samples, owner
  metrics, startup/session traces, memory rows, and allocation notes.

- [x] **Step 4.2: Refresh reports**

Update reports with:

- M37 final versus M38 final startup/session bars;
- M37 final versus M38 final `hao`/`ni`/`zhongguo` bars;
- same-run Yune/librime ratio tables;
- raw lookup and translator microbench owner tables;
- memory and allocation owner notes;
- explicit wording that M38 is isolated native engine evidence.

Expected:

- The report cannot be read as a frontend, browser, application, packaging, or
  public-delivery win.

- [x] **Step 4.3: Run quality gates**

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

Also run focused upstream parity tests and any touched compatibility tests.

Expected:

- All gates pass before closeout.

- [x] **Step 4.4: Close docs**

Record in closeout docs:

- final startup/session medians and ratios;
- final table backend status and `rsmarisa` hot-path counters;
- final mapping mode, table bytes source, copied bytes, and heap mirror bytes;
- final lazy/page-bounded iterator counters and full-list fallback count;
- final `hao`/`ni`/`zhongguo` medians and ratios;
- final raw lookup and translator owner attribution;
- final working-set and allocation rows;
- quality gate results;
- explicit statement that application/frontend delivery was not measured.

Expected:

- M38 plan moves to `docs/plans/completed/`.
- Roadmap, requirements, reports, and milestone ledger agree.

## Evidence Layout

Use this directory:

```text
docs/reports/evidence/m38-engine-performance-parity/
  phase-0-baseline/
  phase-1-lifecycle/
  phase-2-lookup/
  phase-5-final-native/
  final-gates.md
```

Required final files:

- `phase-5-final-native/summary.csv`
- `phase-5-final-native/samples.csv`
- `phase-5-final-native/m37_metrics.csv`
- `phase-5-final-native/product_path_status.csv`
- `phase-5-final-native/raw_lookup_microbench.csv`
- `phase-5-final-native/startup_session_trace.csv`
- `final-gates.md`

## Implementation Notes

- Do not optimize an application pipeline while working this plan.
- Do not close with `rsmarisa` probe-only evidence. The target rows must use the
  marisa-backed table through `rsmarisa` in the hot path.
- Do not close with mmap probe-only evidence. The selected native hot path must
  use mmap/file-backed deployed bytes or a documented borrowed equivalent.
- Do not close with a hidden eager `Vec<Candidate>` full-list pipeline. Ordinary
  first-page reads must use lazy/page-bounded iteration or an equivalent bounded
  view with counters proving the bound.
- Use same-run ratios. Do not compare M38 Yune numbers to stale librime rows if
  the final benchmark captured fresh librime rows.
- Keep microbench rows separate from same-run comparison rows. Yune-only lookup
  microbench rows are diagnostic evidence, not public comparison ratios.
- If the first optimization only improves startup/session but lookup remains
  hundreds of times slower, M38 is not done.
- If startup/session are above `1.25x` or any benchmarked typing row is above
  `5x` of same-run librime, M38 is not done even if the cause is understood.

