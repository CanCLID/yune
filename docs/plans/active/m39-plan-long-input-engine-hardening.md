# M39 Long-Input Engine Hardening Plan

> **Status:** Draft - **Milestone:** M39 (long-input engine hardening) -
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
[`docs/reports/yune-vs-librime-root-cause-analysis.md`](../../reports/yune-vs-librime-root-cause-analysis.md).

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

- [ ] **Step 0.1: Confirm integration base**

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

- [ ] **Step 0.2: Run the required same-run baseline**

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

- [ ] **Step 0.3: Set the Cantonese profile closeout target**

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

## Task 1 - Split Long-Composition Translator Time

**Files:**

- Modify: `crates/yune-core/src/m37_metrics.rs`
- Modify: `crates/yune-core/src/lib.rs`
- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-rime-api/src/lib.rs`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Create: `docs/reports/evidence/m39-long-input-engine-hardening/phase-1-attribution/`

- [ ] **Step 1.1: Add sentence/composition counters**

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

- [ ] **Step 1.2: Instrument `StaticTableTranslator::sentence_candidate`**

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

- [ ] **Step 1.3: Confirm path sharing before fixing**

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

- [ ] **Step 1.4: Capture attribution evidence**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m39-long-input-engine-hardening\phase-1-attribution -Iterations 1 -SessionIterations 5 -KeyIterations 1 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong" -TrackBInputs "hai,ngohaig,jigaajiusihaa,loengjathau,caksijathaacoenggeoizi,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

Expected:

- `m37_metrics.csv` names the dominant inner sentence/composition owner.
- The plan is updated if evidence contradicts the sentence/path hypothesis.

## Task 2 - Bound Or Prune Sentence Composition

**Files:**

- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify: `crates/yune-core/src/tests/translator.rs`
- Modify: `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`

- [ ] **Step 2.1: Add focused regression tests for bounded sentence behavior**

Add tests that construct a `StaticTableTranslator` with sentence enabled and
verify:

- a normal two-piece sentence still returns the same top candidate;
- a long ambiguous input does not explore unbounded paths;
- a priority-floor sentence still beats completion only when it did before;
- single-letter sentence guard behavior remains unchanged.

Expected command:

```powershell
cargo test -p yune-core translator:: -- --nocapture
```

Expected result:

- New tests fail before implementation or record current excessive path counts
  where the existing API cannot fail on result bytes alone.

- [ ] **Step 2.2: Replace clone-heavy path state**

Change the sentence path state so candidate text pieces are not cloned on every
transition. Use backpointers or a compact path record:

```text
end_pos -> best predecessor position, candidate text reference/id, fuzzy count,
quality, raw quality
```

Expected:

- `sentence_path_clones` drops sharply on the long rows.
- Candidate text output remains byte-identical for existing sentence fixtures.

- [ ] **Step 2.3: Add a bounded beam per input position**

Add a small configurable internal beam for sentence composition. The first
implementation should keep the best path per position as today plus a bounded
number of alternatives only when evidence shows they are needed for byte
parity.

Expected:

- `sentence_max_live_paths` is bounded.
- `sentence_substrings_considered` and lookup counts no longer grow into a
  multi-second translator stall on 37-character and 59-character rows.

- [ ] **Step 2.4: Avoid full-list fallback when a bounded sentence result is enough**

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

- [ ] **Step 2.5: Capture latency checkpoint**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m39-long-input-engine-hardening\phase-2-bounded-sentence -Iterations 3 -SessionIterations 10 -KeyIterations 5 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong" -TrackBInputs "hai,ngohaig,jigaajiusihaa,loengjathau,caksijathaacoenggeoizi,neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

Expected:

- Both Track A long rows are within `5x` of same-run librime or the remaining
  owner is still inside named sentence/composition counters.
- The `jyut6ping3_mobile` long row moves according to the Task 0 native profile
  target or remains blocked by a named measured owner.
- Startup/session and short rows remain inside no-regression gates.

## Task 3 - Memory Owner Attribution And No-Regression

**Files:**

- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Create: `docs/reports/evidence/m39-long-input-engine-hardening/phase-3-memory/`

- [ ] **Step 3.1: Capture heap owners**

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

- [ ] **Step 3.2: Reduce safe top owner if M39 owns it**

If the top owner is sentence/composition transient allocation or another
M39-touched owner, reduce it in the same milestone. If the top owner belongs to
an unrelated subsystem, document it as the next memory slice and keep the
no-regression gate.

Expected:

- Final median working set and peak are no worse than post-M38 thresholds.
- Any memory improvement is tied to a named owner.

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

- [ ] **Step 4.1: Run final native benchmark**

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

- [ ] **Step 4.2: Run behavior and quality gates**

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

- [ ] **Step 4.3: Refresh docs and close requirements**

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
