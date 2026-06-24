# M33 Engine Native Lookup Performance Plan

> **Status:** Complete - **Milestone:** M33 (engine native lookup performance vs upstream librime) - **Opened:** 2026-06-22 at user request - **Closed:** 2026-06-23 - **Type:** archived execution record
>
> **For agentic workers:** REQUIRED SUB-SKILL: use the repo's executing-plans / subagent-driven-development workflow to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. **Each lever has a measured stop/accept gate — do not proceed to the next lever without recording before/after evidence.**

> **Decision gate / sequencing.** This milestone was opened deliberately by the user after the 2026-06-23 `luna_pinyin` benchmark. It starts **after P2-WIN-02 is complete** and should run before M31 public deployment so the public demo does not ship with avoidable native startup/session unfairness. It must not run concurrently with P2-WIN-02 or other Yune engine/schema-load rewrites. P2-WIN-01 Windows product work remains the product track, but any Yune engine edits must be serialized through this milestone. M33 also reopens a question M30 closed ("lazy expansion infeasible in the current lookup design"): **Lever 2 is explicitly spike-gated** — confirm feasibility with evidence before committing to the rewrite.

**Goal:** Close the native classic-path performance gap to upstream `rime/librime 1.17.0` on the shared `luna_pinyin` C-ABI surface — startup, schema/session selection, per-key latency, and resident memory — without changing any candidate behavior or widening the default ABI. Target is to **reach the same order of magnitude as librime**, not to claim a blowout win (see the root-cause analysis: librime is near the floor for a classic table IME).

**Architecture:** M33 is an engine-only performance milestone driven by two measurement loops: the cross-engine harness `scripts/benchmark-yune-vs-librime.ps1` (vs the pinned `1.17.0` oracle DLL) and the in-repo Criterion bench `crates/yune-rime-api/benches/frontend_baselines.rs`. The optimization targets are, in priority order: (1) a build-once schema cache so `RimeSelectSchema` stops rebuilding the translator; (2) lazy spelling-algebra lookup over the prism/double-array instead of eager in-RAM expansion; (3) zero-copy `mmap` of the compiled `.table.bin`/`.prism.bin` instead of `fs::read` + parse-to-heap. The TypeDuck `jyut6ping3` profile and TypeDuck-Web harness are measurement/regression surfaces, not optimization targets.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), the existing compiled-format parsers and `DartsDoubleArray` in `crates/yune-core/src/dictionary/`, Criterion `frontend_baselines`, the `yune-vs-librime` C# harness (`scripts/yune-vs-librime-benchmark.cs`), oracle fixtures for upstream `1.17.0` and TypeDuck `v1.1.2`, and the TypeDuck-Web patch workflow.

## Closeout - 2026-06-23

M33 closed as a bounded native fairness/cache win. It did not reopen P2-WIN-02,
M30, M31, or M32, and it did not widen the default `RimeApi`, change
`RimeCandidate`, or alter TypeDuck-profile semantics.

Implemented:

- Build-once sharing for immutable dictionary translators, keyed by schema and
  resolved source/compiled asset signatures, with invalidation coverage.
- Lazy `stroke` reverse-lookup dictionary loading, so no-reverse `luna_pinyin`
  startup/session rows no longer compare a luna-plus-stroke Yune load against a
  luna-only librime load.
- Focused regression tests for cache reuse/invalidation, first-use reverse
  lookup loading, and the lazy-prism spike boundary.

Measured result:

| Row | Yune before | Yune after | librime after |
| --- | ---: | ---: | ---: |
| Cold startup/runtime-ready | `3,141,449.8 us` | `909,375.4 us` | `80,260.8 us` |
| Warm startup/runtime-ready | `2,881,852.7 us` | `47,556.3 us` | `26,964.8 us` |
| Session create/select/destroy median | `2,985,364.0 us` | `47,813.7 us` | `25,765.9 us` |
| Startup peak working set | `261,500,928 bytes` | `182,775,808 bytes` | `22,519,808 bytes` |
| Key `ni` median | `5,579.8 us` | `6,064.5 us` | `28.5 us` |
| Key `hao` median | `11,043.8 us` | `12,463.4 us` | `34.5 us` |
| Key `zhongguo` median | `34,024.0 us` | `37,572.3 us` | `1,479.8 us` |

Deferred:

- Lazy table+prism spelling-algebra lookup: no-go for M33. The checked-in
  upstream prism fixture maps spellings to descriptors but, as in librime,
  candidate text/comment/order payloads live on the table side. A byte-identical
  storage rewrite needs a broader queryable table+prism design.
- mmap compiled artifacts: deferred. The low-risk slice made warm re-select and
  session select cheap, but cold startup, peak footprint, and per-key lookup
  remain behind. Later hot-path review split the next work into two levers:
  bounded/lazy candidate production for typing latency, and queryable
  table+prism storage plus possible mmap for cold-start/memory. Mmap should not
  be treated as a standalone typing-latency fix.

Evidence and reports:

- [`../../reports/evidence/m33-2026-06-23/`](../../reports/evidence/m33-2026-06-23/)
- [`../../reports/yune-vs-librime-performance.md`](../../reports/yune-vs-librime-performance.md)
- [`../../reports/yune-vs-librime-root-cause-analysis.md`](../../reports/yune-vs-librime-root-cause-analysis.md)

Public claim status: safe to show as an honest cold/warm startup and session
improvement with caveats; unsafe to claim typing-speed, memory-footprint,
browser-startup, browser-typing, or overall "faster than librime" wins.

---

## Background and root cause

Full diagnosis: [`docs/reports/yune-vs-librime-root-cause-analysis.md`](../../reports/yune-vs-librime-root-cause-analysis.md).
Measurement: [`docs/reports/yune-vs-librime-performance.md`](../../reports/yune-vs-librime-performance.md).

Current 2026-06-23 medians on `luna_pinyin` through the librime-shaped C ABI:

| Workload | Yune | librime 1.17.0 | Gap |
|---|---:|---:|---:|
| Startup, runtime-ready | 2,722,088 us | 28,763 us | 94.6× |
| Session create/select/destroy | 2,761,004 us | 23,997 us | 115.1× |
| Key `ni` (2) | 5,612 us | 29.1 us | 192.9× |
| Key `hao` (3) | 11,046 us | 35.3 us | 312.9× |
| Key `zhongguo` (8) | 31,642 us | 1,374 us | 23.0× |
| Startup resident delta | 208.6 MiB | 0.9 MiB | — |

Three verified causes (file evidence in the root-cause doc):

1. **No mmap.** `crates/yune-rime-api/src/schema_install.rs` `load_schema_compiled_dictionary` does `fs::read` of table/prism/reverse (≈ lines 891–908) then `parse_rime_table_bin_dictionary` (≈ line 941). Zero `mmap` usage in `crates/`.
2. **Eager spelling-algebra expansion into RAM.** `crates/yune-core/src/translator/mod.rs` stores `entries_by_code: BTreeMap<String, Vec<Candidate>>` (struct ≈ lines 106–138); `with_spelling_algebra` (≈ lines 427–448) materializes the expanded cross-product. Instrumented as `spelling_algebra_expand` / `translator_index_build` spans.
3. **Rebuild per select.** `crates/yune-rime-api/src/schema_selection.rs` ≈ line 136 calls `install_schema_translator_chain` unconditionally on every `RimeSelectSchema`; no schema/translator cache.

## Comparison fairness — control for asset parity (mandatory)

The 2026-06-23 run is same-schema (both engines `RimeSelectSchema("luna_pinyin")`, identical copied `rime-shared` + `rime-user\build`, modules `default`), and the test directory holds only `luna_pinyin` (+ variants) and `stroke` — **no `cangjie`/`jyut6ping3`**. But it is **not fully fair on reverse-lookup loading**, and this must be fixed before any number is presented as a clean like-for-like result:

- `luna_pinyin.schema.yaml` configures `reverse_lookup: { dictionary: stroke }` and lists `reverse_lookup_translator`.
- **Yune eager-loads `stroke` at schema-select** (`schema_install.rs:365–403`, `install_schema_reverse_lookup_translator_from_config`): `stroke.{table,prism,reverse}.bin` ≈ **9.5 MB** (table 4.55 MB + prism 3.50 MB + reverse 1.50 MB) read into the heap, on top of luna_pinyin (~13.3 MB).
- **librime lazy-loads the reverse dictionary on first reverse-lookup query** — local clone `src/rime/gear/reverse_lookup_translator.cc:147` (`if (!initialized_) Initialize(); // load reverse dict at first use`). The `ni`/`hao`/`zhongguo` workloads never trigger reverse lookup, so **librime loads zero `stroke` bytes in the timed window.**
- Net: ~9.5 MB of the ~22.8 MB compiled payload Yune processes at startup is `stroke`, which librime skips here. Yune's startup/session/resident rows are inflated versus a strict like-for-like load.

This does **not** flip the result (luna_pinyin alone — heap parse + spelling-algebra expansion vs mmap — keeps Yune far behind; librime mmap confirmed via clone `src/rime/dict/mapped_file.{cc,h}`), but it over-states the gap and must be controlled.

**Fairness rule for all M33 measurement:** both engines select the **same schema id**, resolve the **same dictionaries + the same single reverse-lookup target**, use the **same modules**, and reverse-dictionary loading is either excluded for both (no reverse lookup in the workload) or included for both (trigger one reverse lookup). If they cannot be equalized, report luna-only and luna+reverse as **separate rows**.

## Status

Complete. Fresh baselines and after-runs are committed under
[`../../reports/evidence/m33-2026-06-23/`](../../reports/evidence/m33-2026-06-23/).
The accepted slice is build-once dictionary translator sharing plus lazy reverse
lookup. Lazy prism lookup and mmap are deferred by the closeout gate above.

## Scope

In scope:

- Build-once schema/dictionary cache keyed by a stable identity (schema id + resolved asset checksums) so repeated `RimeSelectSchema` of an already-built schema does not re-run load/materialize/expand.
- Lazy-load reverse-lookup target dictionaries (defer to first reverse-lookup query, as librime does) — both a comparison-fairness fix and a real startup/memory win (`stroke` ≈ 9.5 MB off the luna_pinyin startup path).
- Lazy spelling-algebra lookup over the prism/double-array (remove eager in-RAM expansion), **only if** the Task 2 feasibility spike proves byte-identical candidate output.
- Zero-copy `mmap` of compiled table/prism artifacts where the OS and file layout allow it.
- Before/after measurement on both the `yune-vs-librime` harness and `frontend_baselines` for every accepted lever.

Out of scope (do not change as part of M33):

- Candidate text, order, comments, preedit, quality, ranking, commit, or learning behavior. All must stay byte-identical to current oracle fixtures.
- Default `RimeApi` table shape, `RimeCandidate` layout, or the TypeDuck profile ABI (`rime_get_typeduck_profile_api`). No ABI widening.
- TypeDuck `jyut6ping3` profile semantics, sentence penalty, correction/prediction constants, or profile isolation rules (roadmap "Concrete next steps" #3).
- Windows TSF/frontend (P2-WIN-01/02), TypeDuck-Web UI features, AI-native behavior.
- Deploy-time dictionary compilation cost (the benchmark is warm/no-deploy). Improving the deployer is a separate milestone if a target needs it.
- Any claim that a browser typing/startup win follows from this native work unless browser evidence independently proves it (carry M27/M30 honesty rule).

## Current Evidence to reuse

- Cross-engine harness: `scripts/benchmark-yune-vs-librime.ps1` + `scripts/yune-vs-librime-benchmark.cs`. Committed baseline evidence: `docs/reports/evidence/yune-vs-librime-2026-06-23/`.
- Native bench: `crates/yune-rime-api/benches/frontend_baselines.rs` (startup, single-startup memory, per-key rows). M30 baseline context under `apps/yune-web/e2e/results/m30-engine-performance/`.
- Behavior oracles that must stay green: `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`, `crates/yune-core/tests/cantonese_parity.rs`, `crates/yune-rime-api/tests/typeduck_web.rs`.

## Acceptance Gates

- `M33-PERF-01`: Fresh baselines re-captured under this plan (both harnesses), with the runner/source head recorded, before any optimization claim.
- `M33-PERF-02`: **Lever 1 (build-once cache)** lands with before/after evidence showing the `session_create_select_destroy` median drops by at least an order of magnitude on a re-select of an already-built schema, **or** is rejected with evidence explaining why. Candidate behavior byte-identical.
- `M33-PERF-03`: **Lever 2 feasibility spike** produces a written go/no-go: can prism/double-array lazy lookup reproduce current `luna_pinyin` and `jyut6ping3` candidate output (text/order/comment/quality) byte-for-byte? Record the answer with a focused test before any broad rewrite.
- `M33-PERF-04`: If Lever 2 proceeds, eager `with_spelling_algebra` materialization is removed/replaced and single-startup resident delta and startup median both improve materially (target: resident delta within ~5× of librime, startup within ~10× of librime), with all parity fixtures unchanged. If the spike is no-go, record the blocker and stop at Lever 1 (+ optional Lever 3).
- `M33-PERF-05`: **Lever 3 (mmap)** lands only after Lever 2 (or with explicit justification if standalone), reduces resident/load further, and keeps behavior byte-identical. Handle Windows file-locking/lifetime correctly (mmaps must not break deploy/rebuild that rewrites the same files).
- `M33-PERF-06`: Final cross-engine comparison re-run; the report and root-cause analysis numbers are refreshed, and any remaining gap is stated honestly (matched vs still-behind, per row).
- `M33-PERF-07`: Full gates green — `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `frontend_baselines` builds/runs, runtime package tests/build, TypeDuck-Web build, focused browser evidence if WASM rebuilt, TypeDuck-Web patch discipline if source changes, and `git diff --check`.
- `M33-PERF-08`: **Comparison fairness.** Every reported row controls for asset parity per the *Comparison fairness* section: same schema id, same dictionaries + same single reverse-lookup target, same modules, and reverse-dictionary loading either excluded for both engines or included for both. The `stroke` eager-vs-lazy asymmetry is eliminated (Task 1.5) or measured as an explicit separate row. No headline number mixes a luna+stroke Yune load against a luna-only librime load.
- `M33-PERF-09`: Public-facing performance copy stays honest. README/front-page charting is refreshed only after the M33 fair rerun; until then, README links to the caveated report instead of publishing the unfair baseline as a headline.

## File Responsibilities

- `crates/yune-rime-api/src/schema_install.rs`: dictionary load path (`load_schema_compiled_dictionary`, `load_schema_table_dictionary`), translator chain build, and the future cache insertion point.
- `crates/yune-rime-api/src/schema_selection.rs`: `RimeSelectSchema` → `install_schema_translator_chain`; the cache lookup/short-circuit goes here.
- `crates/yune-core/src/translator/mod.rs`: `StaticTableTranslator` representation, `with_spelling_algebra`, `entries_by_code`, lookup, sentence DP, correction scan.
- `crates/yune-core/src/spelling_algebra.rs`: expansion/dedup; the lazy-lookup transformation source.
- `crates/yune-core/src/dictionary/{double_array.rs,compiled_prism.rs,compiled_table.rs}`: the prism/table structures to walk lazily and/or mmap.
- `crates/yune-rime-api/benches/frontend_baselines.rs`: native measurement rows.
- `crates/yune-core/tests/{upstream_luna_pinyin_parity.rs,cantonese_parity.rs}` and `crates/yune-rime-api/tests/typeduck_web.rs`: behavior guards.

---

## Task 0 — Re-capture fresh baselines

- [ ] **0.1 Cross-engine baseline.** Re-run the vs-librime harness and record the runner head + DLL SHA-256:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-yune-vs-librime.ps1 -OutputRoot docs\reports\evidence\yune-vs-librime-m33-before -Iterations 9 -SessionIterations 9 -KeyIterations 25
```

- [ ] **0.2 Native bench baseline.**

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m33-frontend-baselines-before.txt 2>&1"
```

- [ ] **0.3 Confirm the dominant owner.** From the startup-trace spans, record the share of startup spent in `compiled_table_load` + `translator_index_build` + `spelling_algebra_expand`. This anchors every later before/after.
- [ ] **0.4 Verify and isolate asset parity (gate `M33-PERF-08`).** Confirm both engines select `luna_pinyin`, resolve the same dictionaries (luna_pinyin + the `stroke` reverse target), and use modules `default`. Measure the `stroke` eager-load share of Yune startup (run with the reverse-lookup install skipped vs included). Equalize librime's lazy reverse-load: either trigger one reverse lookup (a `` ` ``-prefixed input) in both engines, or confirm neither loads `stroke` in the timed window. Record the corrected like-for-like baseline; this — not the 2026-06-23 numbers — is the reference for M33 deltas.

## Task 1 — Lever 1: Build-once schema cache (highest value, lowest risk)

**Files:** `schema_selection.rs`, `schema_install.rs`, possibly a new `schema_cache` module; runtime state in `runtime.rs`.

- [ ] **1.1 Audit select-time work.** Confirm what `RimeSelectSchema` rebuilds vs what is genuinely per-session. Classify translator chain, filter chain, processors, segment tags. The expensive, schema-deterministic part (dictionary load + materialize + expand) is the cache target; per-session mutable state (userdb handle, options) is not.
- [ ] **1.2 Add a process-wide built-schema cache** keyed by `(schema_id, resolved asset checksums)`. On select, if a compatible built artifact exists, clone/share it into the session instead of rebuilding. Prefer `Arc`-shared immutable translator data so multiple sessions share one expanded representation (also reduces per-session memory).
- [ ] **1.3 Invalidate correctly.** A deploy/rebuild that changes the source or compiled assets must invalidate the cache (checksum mismatch → rebuild). Re-selecting the same schema after deploy must pick up new data. Add a test for deploy → select → reselect.
- [ ] **1.4 Measure (stop/accept gate `M33-PERF-02`).** Re-run Task 0.1. Expect `session_create_select_destroy` to fall by ≥ an order of magnitude on re-select. First-ever startup may be unchanged (that is Lever 2/3). Record before/after; if no material session improvement, stop and explain.
- [ ] **1.5 Lazy-load reverse-lookup dictionaries (fairness + real win).** Match librime: do not load the `reverse_lookup` target dictionary at schema-select (`install_schema_reverse_lookup_translator_from_config`); defer it to first `` ` ``-prefixed reverse-lookup query. Removes the `stroke` (~9.5 MB) eager-load from the startup/session path and makes the comparison like-for-like. Preserve reverse-lookup output on first use (parity test for a `` ` ``-prefixed lookup). Measure startup/session/resident before vs after.
- [ ] **1.6 Bounded-slice decision checkpoint.** After Task 1.4 and 1.5, record whether the bounded low-risk slice (build-once cache + lazy reverse lookup) is enough to unblock M31 public deployment. If Lever 2 looks risky or broad, close M33 as a partial performance win with the spike deferred rather than blocking the product/demo track indefinitely.

## Task 2 — Lever 2: Lazy spelling-algebra lookup (architectural, spike-gated)

**Files:** `translator/mod.rs`, `spelling_algebra.rs`, `dictionary/{double_array.rs,compiled_prism.rs}`.

- [ ] **2.1 Feasibility spike (gate `M33-PERF-03`).** Before any broad change, prove on a *small* slice that a prism/double-array walk plus query-time algebra reproduces current candidate output byte-for-byte for representative `luna_pinyin` codes (incl. fuzzy + abbreviation) and at least one `jyut6ping3` case. Write the finding (go/no-go) into this plan. M30 recorded eager expansion as load-bearing for the current lookup design, so this spike is the decision point, not a formality.
- [ ] **2.2 If go: introduce a lazy lookup path.** Make `StaticTableTranslator` query the prism/double-array (generating algebra variants along the trie walk) instead of reading a pre-expanded `entries_by_code`. Keep the eager path behind a flag during transition so parity tests can A/B both. Preserve: candidate text, order, comments, preedit, quality, abbreviation ranking, sentence-DP and correction semantics.
- [ ] **2.3 Remove eager expansion from the hot startup path.** Once lazy lookup is parity-clean, stop materializing the expanded `entries_by_code` at load time. Keep only what correctness needs (e.g. normal-code membership) in compact form.
- [ ] **2.4 Address the per-key hot paths surfaced by M30** while here (re-locate before editing — line numbers drift): the sentence-lattice DP path-vector clone (M30 Task 4) and the correction scan over `entries_by_code.keys()` / `expanded_lookup_specs` in `translator/mod.rs`. Only optimize with the `jigaajiusihaa`/`zhongguo` rows as evidence.
- [ ] **2.5 Measure (gate `M33-PERF-04`).** Re-run both harnesses. Expect large drops in startup median and resident delta. All `upstream_luna_pinyin_parity`, `cantonese_parity`, `typeduck_web` fixtures unchanged. If no-go from 2.1, record the blocker and skip to Task 3 or close at Lever 1.

## Task 3 — Lever 3: mmap compiled artifacts (zero-copy load)

**Files:** `schema_install.rs` load path; possibly a small `mmap`-backed reader in `yune-core/src/dictionary/`.

- [ ] **3.1 Replace `fs::read` of table/prism with a memory-mapped, borrowed view** for the compiled path, parsing/validating headers without copying the bulk payload into owned `Vec`s. Add the `memmap2` crate (or equivalent) as a dependency; keep `yune-core` unsafe-discipline intact (FFI lint exceptions already exist for `yune-rime-api`).
- [ ] **3.2 Lifetime + Windows correctness.** A held mmap locks the file on Windows; ensure deploy/rebuild that rewrites `*.bin` either drops mmaps first or writes new files atomically. Add a deploy-after-load test.
- [ ] **3.3 Measure (gate `M33-PERF-05`).** Re-run both harnesses; record resident/load deltas vs Lever 2. Behavior byte-identical.

## Task 4 — Final comparison, docs, closeout

- [ ] **4.1 Final cross-engine run** into `docs/reports/evidence/yune-vs-librime-m33-after/`.
- [ ] **4.2 Refresh the numbers** in `docs/reports/yune-vs-librime-performance.md`, `docs/reports/yune-vs-librime-root-cause-analysis.md`, and the README performance section. Regenerate `docs/reports/assets/yune-vs-librime-performance.svg` only after the fairness-controlled final run. State per-row whether Yune now matches, approaches, or still trails librime — no overclaiming.
- [ ] **4.3 Full gates** (`M33-PERF-07`). If TypeDuck-Web WASM is rebuilt, capture focused browser evidence and keep native vs browser claims separate; regenerate the TypeDuck-Web patch if source changed.
- [ ] **4.4 Roadmap + requirements + decisions.** Register M33 in `docs/roadmap.md` (document map, Planned/Next up, per-milestone detail), add `M33-PERF-*` rows to `docs/requirements.md`, note any architectural decision (e.g. "adopt prism-walk lazy lookup") in `docs/decisions.md`, and archive this plan under `docs/plans/archive/` when complete.

## Risks and honest expectations

- **Lever 2 may be a no-go.** M30 deemed lazy expansion infeasible in the current lookup design. If the spike confirms byte-parity cannot be preserved without unacceptable complexity, the defensible outcome is Lever 1 + Lever 3 only, which still fixes the worst rows (session re-select, resident memory, load time) even if first-startup compute stays higher than librime.
- **Matching, not beating.** librime is near the classic-IME floor. Success is "same order of magnitude," and the durable differentiation remains the AI-native layer, not raw pinyin latency.
- **Behavior is sacred.** Any candidate-output drift fails the milestone regardless of speed. The oracle fixtures are the gate.

## Execution Handoff

Paste into a new session:

```text
/goal Implement M33 engine native lookup performance in C:\Users\laubonghaudoi\Documents\GitHub\yune.

Read AGENTS.md, docs/CONVENTIONS.md, docs/roadmap.md, docs/requirements.md,
docs/reports/yune-vs-librime-performance.md,
docs/reports/yune-vs-librime-root-cause-analysis.md,
docs/plans/archive/m33-plan-engine-native-lookup-performance.md, and the archived M27/M29/M30 plans first.

This is engine-only performance work measured by scripts/benchmark-yune-vs-librime.ps1 and
crates/yune-rime-api/benches/frontend_baselines.rs. Start only after P2-WIN-02 is complete.
Do not run concurrently with other Yune engine/schema-load edits. M31 public deployment should
wait for the bounded M33 fairness/cache slice; M32 AI work is out of scope.

Start with Task 0 baselines. Then Lever 1 (build-once schema cache) plus lazy reverse lookup and measure before continuing.
Lever 2 (lazy spelling-algebra prism lookup) is SPIKE-GATED: prove byte-identical candidate output
on a small slice before any broad rewrite; M30 recorded eager expansion as load-bearing. Lever 3
(mmap) after Lever 2. After Lever 1 + lazy reverse lookup, make a bounded-slice decision: if the
larger lazy-prism rewrite is too risky or broad, close the measured slice and defer the rest rather
than blocking M31 indefinitely.

Guardrails: no candidate text/order/comment/ranking/commit/learning change; do not widen RimeApi,
RimeCandidate, or the TypeDuck profile ABI; keep TypeDuck profile constants isolated; preserve
TypeDuck-Web patch discipline if source changes. Every lever needs before/after evidence on both
harnesses and green oracle fixtures (upstream_luna_pinyin_parity, cantonese_parity, typeduck_web).
Run full gates, refresh the perf report + root-cause doc + README performance section
(and chart only if the final fairness-controlled run supports it), register M33 in
roadmap/requirements/decisions, archive the plan, and push to origin/main when green.
```
