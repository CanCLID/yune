# M39 Long-Input Engine Hardening Plan

> **Status:** Complete - **Milestone:** M39 (long-input engine hardening) -
> **Created:** 2026-06-25 - **Type:** engine-performance plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring uninterrupted long-input latency into the same native
Yune-versus-librime performance gate as the M38 short/medium rows, and prove
whether the Cantonese `jyut6ping3_mobile` profile shares the same long-input
owner, while preserving startup/session, short-input latency,
mmap/`rsmarisa` activation, bounded output, memory, and behavior.

**Architecture:** M39 treats the 37-character and 59-character Track A rows and
a 50+ character Cantonese profile row as primary engine requirements, not
stress curiosities. The milestone starts by splitting the unsplit translator
bucket into sentence/composition/profile owners, then replaces unbounded
long-composition fallback with a measured bounded or pruned path only after
proving which path each profile uses. Every change is checked against the whole
engine shape so a long-input win cannot regress startup, short keys, memory, or
the deployed-data hot path.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), `StaticTableTranslator`,
`TableStorage`, `CompactTableStore`, `rsmarisa`, mmap-backed deployed
table/prism bytes, native in-process benchmark harness, upstream librime
`1.17.0`, owner counters, startup/session traces, working-set/peak memory
sampling, heap profiling where available, and reports under
`docs/reports/evidence/`.

---

## Current Evidence

Current dashboard:
[`docs/reports/yune-vs-librime-performance.md`](../../reports/yune-vs-librime-performance.md).

Root-cause dashboard:
[`archived root-cause report`](../../reports/history/2026-07-14-yune-vs-librime-root-cause-analysis-pre-consolidation.md).

Post-M38 long-input evidence:

- Higher-sample baseline:
  [`docs/reports/evidence/post-m38-long-input-baseline/baseline-native/`](../../reports/evidence/post-m38-long-input-baseline/baseline-native)
- 59-character stress baseline:
  [`docs/reports/evidence/post-m38-long-input-baseline/stress-59-native/`](../../reports/evidence/post-m38-long-input-baseline/stress-59-native)

Key current rows:

| Row | Yune | librime | Ratio | Read |
| --- | ---: | ---: | ---: | --- |
| startup/runtime-ready | `23,478.800 us` | `32,805.100 us` | `0.716x` | preserve |
| session create/select/destroy | `24,202.100 us` | `32,302.200 us` | `0.749x` | preserve |
| `hao` | `38.967 us` | `11.733 us` | `3.321x` | preserve |
| `ni` | `56.200 us` | `14.600 us` | `3.849x` | preserve |
| `zhongguo` | `62.025 us` | `172.950 us` | `0.359x` | preserve |
| `ceshiyixiachangjushuruxingnengzenyang` | `412,192.727 us` | `294.151 us` | `1,401.296x` | fix |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `1,202,404.588 us` | `702.212 us` | `1,712.310x` | fix |

The Track A long rows have active `rsmarisa`, mmap-backed table/prism bytes,
tiny raw lookup/context export times, and translator time near all of
process-key time. The current Track A owner is therefore long-composition
translator internals, not raw table lookup, not context export, and not marisa
activation.

Blocking scope gap before implementation: the Cantonese `jyut6ping3_mobile`
profile has not yet been measured on a 50+ character uninterrupted row. M39
must add at least this profile row to Track B:

```text
neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung
```

This row is a Cantonese/Jyutping-style counterpart to the current 59-character
Mandarin stress sentence ("this engine should support very long sentence input
before it is usable"). It is a native engine profile row, not a browser,
frontend, packaging, or delivery claim.

**Task 0 update (2026-06-25):** Phase 0 baseline evidence is recorded under
[`docs/reports/evidence/m39-long-input-engine-hardening/phase-0-baseline/`](../../reports/evidence/m39-long-input-engine-hardening/phase-0-baseline)
with the required `jyut6ping3_mobile` row. The Track B row
`neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` measured
`189.207 us/op` median and `202.084 us/op` p95, versus Track A long rows at
`452,200.116 us/op` and `1,240,080.937 us/op`. Current broad counters therefore
do **not** support treating Track B as the same severe Track A
long-composition latency owner. Track B uses the product compiled
`byte_backed` no-marisa path with mmap-backed table/prism bytes and zero
selected table/prism heap mirror bytes. Task 1 must still add inner counters to
separate sentence composition, upstream sentence model, prefix fallback, and
dynamic correction before code optimization. Until a same-run TypeDuck-HK
librime `v1.1.2` oracle row is added, the Track B native profile target is a
no-regression gate from the Phase 0 median/p95 plus required owner attribution,
not a Yune-versus-librime ratio claim.

## Non-Negotiable Closeout Gates

- `M39-ENGINE-01` (same-run benchmark): final evidence includes startup,
  session, `hao`, `ni`, `zhongguo`,
  `ceshiyixiachangjushuruxingnengzenyang`, and
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` in the same
  native Yune/librime run, plus the `jyut6ping3_mobile` Track B row
  `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`.
- `M39-ENGINE-02` (startup/session no regression): startup and session remain
  within `1.25x` of same-run librime and do not regress by more than `10%` from
  the post-M38 baseline unless a measured librime-side shift explains the ratio.
- `M39-ENGINE-03` (short/medium no regression): `hao`, `ni`, and `zhongguo`
  remain within `5x` of same-run librime and do not regress by more than `10%`
  from the post-M38 baseline.
- `M39-ENGINE-04` (long-input parity): both required Track A long rows finish
  within `5x` of same-run librime. The required `jyut6ping3_mobile` Track B
  long row must be measured, attributed, and either brought inside the
  Task 0-agreed native profile target or closed by an explicit measured no-go
  before M39 can close.
- `M39-ENGINE-05` (storage hot path): final Track A status preserves
  `selected_storage=rsmarisa_byte_backed`, table/prism `mmap`, positive
  `rsmarisa` exact/prefix counters, zero ordinary no-marisa fallback for target
  rows, and zero selected table/prism heap mirror bytes.
- `M39-ENGINE-06` (bounded output): final target rows use bounded first-page
  candidate requests; any full-list fallback is named and justified by inner
  sentence/composition/profile metrics.
- `M39-ENGINE-07` (memory no regression and attribution): final median working
  set and max peak do not exceed the post-M38 baseline by more than `5%`, and
  final evidence includes heap-owner attribution. If a top heap owner is safe to
  reduce inside M39, reduce it; otherwise document the measured owner and the
  next memory slice.
- `M39-ENGINE-08` (behavior): upstream `luna_pinyin` behavior, paging,
  selection, deletion, context reads, and touched compatibility paths remain
  green.
- `M39-ENGINE-09` (honest claims): final reports separate native engine
  evidence from browser, frontend, application, packaging, deployment, and
  public-delivery claims.

## File Responsibilities

- `crates/yune-core/src/m37_metrics.rs`: owns performance counters. M39 should
  either extend this module with sentence/composition counters or rename it only
  in a mechanical follow-up after M39.
- `crates/yune-core/src/translator/mod.rs`: owns `StaticTableTranslator`,
  `translated_candidates_for_segment_with_request`, full-list fallback,
  `sentence_candidate`, substring lookup loops, path selection, and sentence
  candidate assembly.
- `crates/yune-core/src/engine.rs`: owns bounded refresh requests, candidate
  sorting/storage, context candidate retention, and no-regression checks for
  page-sized output.
- `crates/yune-core/src/dictionary/compiled_table.rs`: owns mapped compact
  table storage, `rsmarisa` table lookup, and heap mirror status.
- `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`: owns input
  rows, owner CSV fields, raw lookup rows, length-curve output, working set, and
  final same-run comparison evidence.
- `scripts/benchmark-native-rime-inprocess.ps1`: owns Track A and Track B
  benchmark input parameterization and evidence root orchestration.
- `docs/reports/evidence/m39-long-input-engine-hardening/`: owns M39 evidence.
- `docs/reports/yune-vs-librime-performance.md`,
  `docs/reports/yune-vs-librime-root-cause-analysis.md`, `docs/roadmap.md`, and
  `docs/requirements.md`: own user-facing claims, closeout state, and
  requirement traceability.

---

## Task 0 - Fresh Baseline And Length Curve

**Files:**

- Modify: `scripts/benchmark-native-rime-inprocess.ps1`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Create: `docs/reports/evidence/m39-long-input-engine-hardening/phase-0-baseline/`

- [x] **Step 0.1: Confirm integration base**

Run:

```powershell
git fetch origin --prune
git status --short --branch --untracked-files=all
git log --oneline -5 --decorate
```

Expected:

- The branch is current with `origin/main` or the worker has explicitly
  rebased/merged before implementation.
- Any unrelated dirt is listed before editing.

- [x] **Step 0.2: Run the required same-run baseline**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m39-long-input-engine-hardening\phase-0-baseline -Iterations 5 -SessionIterations 20 -KeyIterations 20 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong" -TrackBInputs "hai,ngohaig,jigaajiusihaa,loengjathau,caksijathaacoenggeoizi,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

Expected:

- `summary.csv`, `samples.csv`, `m37_metrics.csv`,
  `raw_lookup_microbench.csv`, `startup_session_trace.csv`, and
  `product_path_status.csv` are present.
- The run may be slow before fixes, but it must produce a complete baseline.
- The Track B `jyut6ping3_mobile` row is present in `summary.csv`,
  `samples.csv`, and `m37_metrics.csv`; M39 cannot proceed to Task 2 if this
  profile row is absent.

- [x] **Step 0.3: Set the Cantonese profile closeout target**

After Step 0.2, record a short `phase-0-baseline/cantonese-profile-gate.md`
summary with:

- the 50+ character `jyut6ping3_mobile` row median, p95, full-input sample cost,
  working set, peak working set, and top owner counters;
- whether the profile row appears to share the Track A long-composition owner;
- the native profile target for M39, or an explicit statement that a comparable
  TypeDuck-HK/librime oracle row must be added before a numeric ratio can be
  claimed.

Expected:

- The product/profile row is a hard closeout gate before Task 2 begins.
- The plan is updated if the profile row's owner is not the same as the Track A
  long-composition owner.

- [ ] **Step 0.4: Add a controlled length-curve mode if needed**

If Step 0.2 is too slow for repeated iteration, add benchmark options that
accept separate low-sample Track A and Track B length-curve input lists while
preserving the final same-run run above.

Required Track A length-curve rows:

```text
ni
zhongguo
ceshiyixiachangjushuruxingnengzenyang
zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong
```

Required Track B `jyut6ping3_mobile` length-curve rows:

```text
hai
ngohaig
jigaajiusihaa
caksijathaacoenggeoizi
neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung
```

Expected:

- Evidence records per-key medians and full-input sample cost for each length.
- Reports do not infer a final complexity class until inner counters exist.

Task 0 note: Step 0.4 remains optional. The required high-sample baseline was
slow but completed, and the existing script parameterization is sufficient for
the low-sample attribution command in Task 1. Add a dedicated length-curve mode
only if Task 2 iteration needs it after inner counters exist.

## Task 1 - Split Long-Composition Translator Time

**Files:**

- Modify: `crates/yune-core/src/m37_metrics.rs`
- Modify: `crates/yune-core/src/lib.rs`
- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-rime-api/src/lib.rs`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Create: `docs/reports/evidence/m39-long-input-engine-hardening/phase-1-attribution/`

- [x] **Step 1.1: Add sentence/composition counters**

Add counters with these exact exported field names:

```text
sentence_candidate_calls
sentence_candidate_ns
sentence_substrings_considered
sentence_exact_lookup_calls
sentence_exact_lookup_ns
sentence_exact_lookup_candidates
sentence_prefix_lookup_calls
sentence_prefix_lookup_ns
sentence_prefix_lookup_candidates
sentence_entry_matches_collected
sentence_path_clones
sentence_path_replacements
sentence_paths_pruned
sentence_max_live_paths
sentence_result_candidates
upstream_sentence_model_calls
upstream_sentence_model_ns
upstream_sentence_model_candidates
prefix_fallback_calls
prefix_fallback_ns
prefix_fallback_views_visited
prefix_fallback_candidates
```

Expected:

- `yune_m37_metrics_snapshot_json` exposes the new fields.
- `native_inprocess_benchmark.rs` writes them to `m37_metrics.csv`.

- [x] **Step 1.2: Instrument `StaticTableTranslator::sentence_candidate`**

In `crates/yune-core/src/translator/mod.rs`, record:

- total `sentence_candidate` elapsed time;
- every `(pos, end)` substring considered;
- exact lookup elapsed time and candidate count for `entry_code`;
- final-segment prefix lookup elapsed time and candidate count;
- entry matches collected before filtering;
- path clone count;
- path replacement count;
- paths pruned or skipped by a bound once Task 2 lands;
- maximum live path count.

Expected:

- The 37-character and 59-character Track A rows identify the inner owner before
  any optimization is attempted.
- The `jyut6ping3_mobile` Track B long row identifies whether it uses the same
  `sentence_candidate` owner, the upstream sentence model owner, prefix
  fallback, dynamic correction, or another profile-specific owner.

- [x] **Step 1.3: Confirm path sharing before fixing**

Use the phase-1 counters to compare:

- Track A `luna_pinyin` long rows;
- Track B `jyut6ping3_mobile`
  `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`;
- the current schema-install flags in
  `crates/yune-rime-api/src/schema_install.rs`.

Current code expectation before implementation:

- upstream `luna_pinyin` installs `with_upstream_sentence_model(100)`;
- `jyut6ping3_mobile` installs the TypeDuck sentence word penalty but does not
  automatically prove it shares Track A's long-row owner;
- the counters, not code inspection alone, decide whether Task 2 fixes one
  shared owner or needs a profile-specific path.

Expected:

- If the `jyut6ping3_mobile` row is not dominated by the same owner as the
  Track A rows, update Task 2 before coding.
- M39 does not optimize the `luna_pinyin` row first and assume transfer to the
  Cantonese profile.

- [x] **Step 1.4: Capture attribution evidence**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m39-long-input-engine-hardening\phase-1-attribution -Iterations 1 -SessionIterations 5 -KeyIterations 1 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong" -TrackBInputs "hai,ngohaig,jigaajiusihaa,loengjathau,caksijathaacoenggeoizi,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

Expected:

- `m37_metrics.csv` names the dominant inner sentence/composition owner.
- The plan is updated if evidence contradicts the sentence/path hypothesis.

Task 1 update (2026-06-25): attribution evidence is recorded in
[`phase-1-attribution/owner-attribution.md`](../../reports/evidence/m39-long-input-engine-hardening/phase-1-attribution/owner-attribution.md).
The Track A long rows are dominated by `upstream_sentence_model_ns`, not
`StaticTableTranslator::sentence_candidate`: `436,917.530 us/op` for
`ceshiyixiachangjushuruxingnengzenyang` and `1,228,565.656 us/op` for
`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`. Track B
`neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` does not share
that owner: `upstream_sentence_model_calls_per_op=0`, `sentence_candidate_ns`
averages `7.414 us` per call, and total median latency is `225.259 us/op`.
Task 2 is therefore retargeted to `UpstreamSentenceModel` word-graph
construction for Track A, with Track B kept as a native no-regression guard.

## Task 2 - Bound Or Prune Sentence Composition

**Files:**

- Modify: `crates/yune-core/src/poet/mod.rs`
- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-core/src/tests/poet.rs`
- Modify: `crates/yune-core/src/tests/translator.rs`
- Modify: `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`

- [x] **Step 2.1: Add focused regression tests for bounded sentence behavior**

Add tests that construct an `UpstreamSentenceModel` or `StaticTableTranslator`
with the upstream sentence model enabled and
verify:

- a normal two-piece sentence still returns the same top candidate;
- a long unmatched or sparsely matched input does not scan the full model entry
  list per suffix;
- a priority-floor sentence still beats completion only when it did before;
- single-letter sentence guard behavior remains unchanged.

Expected command:

```powershell
cargo test -p yune-core translator:: -- --nocapture
```

Expected result:

- New tests fail before implementation or record current excessive path counts
  where the existing API cannot fail on result bytes alone.

- [x] **Step 2.2: Replace clone-heavy path state**

Change the upstream sentence model word-graph construction so it does not scan
every model entry for every input suffix. Use a compact code index equivalent to:

```text
code -> table entries, then enumerate input prefixes for each suffix
```

Expected:

- `upstream_sentence_model_ns` drops sharply on the long rows.
- Candidate text output remains byte-identical for existing sentence fixtures.

- [x] **Step 2.3: Add a bounded beam per input position**

Keep the existing bounded sentence beam, but avoid feeding it an unbounded graph
build. If follow-up evidence still shows sentence graph construction or path
state growth as the owner, add a small configurable internal bound at that
measured point only.

Expected:

- `sentence_max_live_paths` is bounded.
- `sentence_substrings_considered` and lookup counts no longer grow into a
  multi-second translator stall on 37-character and 59-character rows if that
  path becomes active; otherwise `upstream_sentence_model_ns` is the named owner
  gate.

- [x] **Step 2.4: Avoid full-list fallback when a bounded sentence result is enough**

In `translated_candidates_for_segment_with_request`, stop treating sentence
fallback as an unconditional eager full-list path for bounded first-page
requests. Return a bounded sentence candidate when:

- the request has a positive limit;
- lookup/output candidates are empty or sentence-over-completion applies;
- the sentence candidate can be produced through the bounded/pruned sentence
  path;
- existing byte-parity tests still pass.

Expected:

- `full_list_fallback_count` falls on long rows.
- `candidate_request_bounded_calls` remains positive and
  `candidate_request_unbounded_calls` remains zero for target rows.

- [x] **Step 2.5: Capture latency checkpoint**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m39-long-input-engine-hardening\phase-2-bounded-sentence -Iterations 3 -SessionIterations 10 -KeyIterations 5 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong" -TrackBInputs "hai,ngohaig,jigaajiusihaa,loengjathau,caksijathaacoenggeoizi,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

Expected:

- Both Track A long rows are within `5x` of same-run librime or the remaining
  owner is still inside named sentence/composition counters.
- The `jyut6ping3_mobile` long row remains inside the Task 0 no-regression
  native profile target, or any regression is attributed to a named measured
  owner. Phase 0 does not show this row sharing the same severe Track A
  long-composition latency owner, so Task 2 must treat Track B as a protected
  regression row unless Task 1 inner counters prove a shared fix is actually
  needed.
- Startup/session and short rows remain inside no-regression gates.

Task 2 update (2026-06-25): the final Task 2 checkpoint is recorded under
[`phase-2-bounded-sentence-streamed/`](../../reports/evidence/m39-long-input-engine-hardening/phase-2-bounded-sentence-streamed)
because the streamed upstream sentence-model builder also fixed the transient
memory peak found during Task 3. Track A long-row medians are `506.227 us/op`
(`1.715x` same-run librime) and `916.183 us/op` (`1.329x` same-run librime),
with `full_list_fallback_count=0`,
`candidate_request_unbounded_calls=0`, bounded first-page requests, and
positive `rsmarisa` counters. Track B
`neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` remains a
separate product-profile owner: no upstream sentence-model calls, no-marisa
exact/prefix lookup plus profile fallback, and `202.693 us/op` median /
`204.539 us/op` p95 in the low-sample checkpoint. The final Task 4 benchmark
must still rerun with the exact closeout command and input set.

## Task 3 - Memory Owner Attribution And No-Regression

**Files:**

- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Create: `docs/reports/evidence/m39-long-input-engine-hardening/phase-3-memory/`

- [x] **Step 3.1: Capture heap owners**

Use the best available Windows-compatible heap attribution method for this
workspace. Acceptable evidence includes a checked-in heap profiler summary, a
repeatable allocation-owner CSV from the benchmark, or a documented profiler
blocker plus the deepest available owner table.

Required owner groups:

```text
selected table/prism/reverse bytes
translator install state
sentence/composition transient allocations
schema/runtime config
reverse/userdb/filter state
benchmark harness overhead
Rust/runtime/library baseline
```

Expected:

- Evidence names the top memory owner instead of inferring it from working set.

- [x] **Step 3.2: Reduce safe top owner if M39 owns it**

If the top owner is sentence/composition transient allocation or another
M39-touched owner, reduce it in the same milestone. If the top owner belongs to
an unrelated subsystem, document it as the next memory slice and keep the
no-regression gate.

Expected:

- Final median working set and peak are no worse than post-M38 thresholds.
- Any memory improvement is tied to a named owner.

Task 3 update (2026-06-25): memory evidence is recorded under
[`phase-3-memory/`](../../reports/evidence/m39-long-input-engine-hardening/phase-3-memory).
UMDH/GFlags/XPerf were unavailable in this workspace, so the checked-in
attribution uses the deepest repeatable benchmark evidence: working-set/peak
rows, product-path storage status, selected heap-mirror counters, and M39 owner
counters. The M39-owned transient peak was upstream sentence-model construction
holding a full temporary table-entry list alongside model entries; streaming
entries into `UpstreamSentenceModel::from_table_entries` reduced Track A max
peak from `163,598,336` bytes to `123,891,712` bytes in the Task 2 checkpoint.
Track B peak remained below Phase 0 (`504,057,856` versus `504,557,568` bytes).

## Task 4 - Full Final Benchmark And Report Closeout

**Files:**

- Modify: `docs/reports/yune-vs-librime-performance.md`
- Modify: `docs/reports/yune-vs-librime-root-cause-analysis.md`
- Modify: `docs/requirements.md`
- Modify: `docs/roadmap.md`
- Move on closeout:
  `docs/plans/active/m39-plan-long-input-engine-hardening.md` to
  `docs/plans/completed/m39-plan-long-input-engine-hardening.md`
- Create: `docs/reports/evidence/m39-long-input-engine-hardening/final-gates.md`

- [x] **Step 4.1: Run final native benchmark**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m39-long-input-engine-hardening\phase-4-final-native -Iterations 9 -SessionIterations 20 -KeyIterations 20 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong" -TrackBInputs "hai,ngohaig,jigaajiusihaa,loengjathau,caksijathaacoenggeoizi,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

Expected:

- Same-run Yune/librime ratios are recorded for all Track A target rows, and
  the Track B profile row records the matching Yune owner/status/memory fields.
- Owner counters prove the long-input owner moved.
- The `jyut6ping3_mobile` 50+ character profile row is present, attributed, and
  either inside the Task 0 native profile target or explicitly closed by
  measured no-go.
- Storage, memory, and no-regression gates are visible in CSVs and markdown.

- [x] **Step 4.2: Run behavior and quality gates**

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

Also run focused tests touched by Task 2:

```powershell
cargo test -p yune-core translator:: -- --nocapture
cargo test -p yune-core upstream_luna_pinyin -- --nocapture
```

Expected:

- All gates pass before closeout.
- Any unavailable profiler or platform-specific gate is documented with the
  exact command and blocker.

- [x] **Step 4.3: Refresh docs and close requirements**

Update final docs with:

- startup/session before and after;
- short-row before and after;
- 37-character and 59-character before and after;
- `jyut6ping3_mobile` 50+ character profile row before and after;
- path-sharing verdict: whether the Track A and Track B long rows used the same
  owner or required separate fixes/no-goes;
- long-row inner owner table;
- mmap/`rsmarisa` status;
- bounded-output counters;
- memory owner table and final working-set/peak rows;
- quality gate results.

Expected:

- The reports cannot be read as browser/frontend/application claims.
- The plan stays active if any non-negotiable gate remains open.

Task 4 update (2026-06-25): final native evidence is recorded under
[`phase-4-final-native/`](../../reports/evidence/m39-long-input-engine-hardening/phase-4-final-native)
with final gate summary in
[`final-gates.md`](../../reports/evidence/m39-long-input-engine-hardening/final-gates.md).
The closeout run includes `-TrackBInputs`, startup/session, `hao`, `ni`,
`zhongguo`, both Track A long rows, and the required Track B
`jyut6ping3_mobile` row.

Final Track A results are inside the agreed gates: startup `0.917x`, session
`0.938x`, `hao` `3.281x`, `ni` `3.863x`, `zhongguo` `0.329x`,
`ceshiyixiachangjushuruxingnengzenyang` `1.765x`, and
`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` `1.320x` versus
same-run librime. The Track B 50+ row
`neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung` closes at
`188.857 us/op` median and `194.910 us/op` p95, below Phase 0. Track B remains
a separate TypeDuck-profile owner, not the Track A upstream sentence-model
owner.

Final storage and memory gates pass: Track A remains
`selected_storage=rsmarisa_byte_backed`, table/prism bytes are `mmap`, selected
heap mirrors are `0`, `source_fallback=false`, runtime `rsmarisa` counters are
positive, and target rows have bounded first-page output with no full-list
fallback. Track A max peak moved from `163,598,336` to `123,985,920` bytes.
Track B remains byte-backed/mmap-backed with selected heap mirrors `0`; its
peak moved from `504,557,568` to `504,041,472` bytes.

Final gates passed: `cargo fmt --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo test -p yune-core translator:: -- --nocapture`,
`cargo test -p yune-core upstream_luna_pinyin -- --nocapture`, the focused
TypeDuck boundary test, final native benchmark, docs closeout, and
`git diff --check`. Reports remain native-engine claims only.

## Implementation Notes

- Do not optimize by disabling sentence behavior unless upstream behavior
  evidence proves that is correct for the target row.
- Do not hide the problem by dropping long rows from the benchmark.
- Do not hide the Cantonese profile problem by benchmarking only
  `luna_pinyin`; the `jyut6ping3_mobile` 50+ character row is a closeout gate.
- Do not assume a `luna_pinyin` long-input fix transfers to
  `jyut6ping3_mobile`; Task 1 must prove or disprove path sharing first.
- Do not trade a long-input win for startup/session, short-input, memory, or
  storage-backend regression.
- Do not close with only a broad `translator_ns` improvement. Final evidence
  must show which sentence/composition owner moved.
- Keep `rsmarisa` and mmap status in every final run; a faster row served by the
  wrong backend is not a clean M39 result.
