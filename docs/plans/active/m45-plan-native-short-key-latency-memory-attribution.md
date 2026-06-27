# M45 Native Short-Key Latency And Memory Attribution Plan

> **Status:** Active - **Milestone:** M45 (native short-key latency and memory
> attribution) - **Created:** 2026-06-27 - **Type:** native-engine plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

## Goal

Close the Track A `luna_pinyin` short-key latency gap for `n`, `ni`, and
`hao` against same-run upstream librime, and resolve the Track A memory target
question by attribution before any storage rewrite.

M45 has two independent workstreams:

1. Implement a bounded, behavior-guarded short-prefix path only if Phase 0
   proves the current owner and captures upstream candidate-output oracle
   evidence for `n`, `ni`, and `hao`.
2. Attribute native Track A memory by splitting steady-state after-ready
   resident size from observed high-water peak, private heap from file-backed
   mapped pages, and transient deploy/startup high-water from steady runtime.

The memory workstream is attribution and verdict first. M45 must not start
another `compact_table` or `poet` storage rewrite just because peak memory is
above the old target. M43 already reduced a large retained owner without moving
peak; M45 only authorizes code if new evidence names a safe, bounded,
peak-moving owner.

## Architecture

M45 is native-engine only. It follows the post-M44 diagnostic split:
short-prefix latency is a Track A hot-path problem centered on the `n` prefix
family, while the memory blocker is a measurement and attribution problem
until private heap, mapped residency, allocator high-water, and steady-state
resident size are separated.

The likely short-key implementation is a borrowed first-page prefix path:
keep exact and prefix candidates as borrowed table references, prove first-page
ordering before early stop, and materialize only the requested output page. The
existing M44 under-fill fallback remains mandatory: if filters or uniquifier
behavior can drop the bounded page below the requested size, the path must
escalate to a complete refresh rather than returning an under-filled first
page.

The memory implementation path is deliberately gated. Phase 0 must distinguish
a benchmark-cumulative high-water artifact from a real per-cold-start deploy
or startup peak. Those outcomes are not interchangeable: a benchmark artifact
can be reframed away from the resident-memory target, while a real cold-start
peak remains a standing peak cost and possible constrained-machine/OOM risk
even if steady after-ready resident memory meets target. A storage rewrite is
not in scope unless profiling names a bounded owner whose reduction is expected
to move the measured target.

## Tech Stack

- Rust native engine: `crates/yune-core`.
- Rime ABI and native benchmark harness: `crates/yune-rime-api`.
- Likely hot-path files:
  - `crates/yune-core/src/translator/mod.rs`
  - `crates/yune-core/src/engine.rs`
  - `crates/yune-core/src/m37_metrics.rs`
  - `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Tests likely touched:
  - `crates/yune-core/src/tests/translator.rs`
  - `crates/yune-rime-api/src/tests/session_api.rs`
- Reports and evidence:
  - `docs/reports/evidence/m45-native-short-key-memory-attribution/`
  - `docs/reports/yune-vs-librime-performance.md`
  - `docs/reports/yune-vs-librime-root-cause-analysis.md`
- Oracle target: upstream `rime/librime 1.17.0` at
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.

## Baseline

M45 starts from the post-M44 native diagnostic evidence under
`docs/reports/evidence/post-m44-bottleneck-profiling/phase-0-native-diagnostic/`.
That run is diagnostic evidence, not an M45 closeout.

Short-prefix rows:

| Row | Yune median | librime median | Ratio | Current read |
| --- | ---: | ---: | ---: | --- |
| `n` | `79.400 us` | `21.900 us` | `3.626x` | Sharpest remaining Track A short-prefix owner. |
| `ni` | `53.750 us` | `15.050 us` | `3.571x` | Misses parity; includes the earlier `n` prefix step. |
| `hao` | `25.667 us` | `12.100 us` | `2.121x` | M44 target remains met, but constant-factor gap remains. |

Raw lookup evidence:

| Input | Prism completions | Table lookup codes | Raw candidates | Raw table median | Translator median |
| --- | ---: | ---: | ---: | ---: | ---: |
| `n` | `26` | `27` | `1,260` | `166.000 us` | `74.100 us` |
| `ni` | `1` | `1` | `182` | `19.600 us` | `50.200 us` |
| `hao` | `1` | `1` | `139` | `15.500 us` | `22.533 us` |

Memory diagnostic:

| Measurement | Value | Current read |
| --- | ---: | --- |
| Repeated Track A high-water peak | `127,430,656 B` | Same peak repeats across startup, session, short, long, and abbreviation rows. |
| Session after-ready median | `87,240,704 B` | Steady session sample is below the `107,797,708 B` target. |
| `n` after-ready median | `90,714,112 B` | Short-prefix steady footprint is below the target. |
| Longest diagnostic row after-ready median | `97,677,312 B` | Longest sampled row remains below the target. |
| Reducible retained owner still named | `18,694,662 B` | `poet.entries_by_code`; not enough to explain process peak. |
| Mapped table bytes | `13,013,460 B` | File-backed table storage; not a selected heap mirror. |

Phase 0 must classify the repeated `127,430,656 B` high-water value. It may be
a benchmark-cumulative artifact from one process carrying maximum working-set
history across rows, or it may be a real per-cold-start deploy/startup spike.
M45 closeout must keep both the peak value and the steady after-ready resident
value visible either way.

## Scope Boundaries

In scope:

- Native Track A `luna_pinyin` short-key rows `n`, `ni`, and `hao`.
- Upstream candidate-output oracle capture for `n`, `ni`, and `hao`: candidate
  count, text, comments, order, page metadata, context preedit, and commit
  preview.
- Short-key owner counters for prefix enumeration, raw table lookup, borrowed
  candidate iteration, candidate materialization, ranking/sort,
  comment/quality formatting, filters, context export, and ABI string export.
- Native Track A memory attribution for steady after-ready resident size,
  high-water peak, private heap, file-backed mapped pages, allocator behavior,
  deployment/startup transient memory, and retained owner estimates.
- No-regression evidence for M40, M42, M43, and M44 closed behavior and storage
  gates.

Out of scope:

- Browser harness, WASM linear memory, public-demo packaging, frontend startup,
  payload transfer, and yune-web/My RIME browser comparison. WEB-01 owns those.
- Future WASM engine-memory reduction for the `893 MiB` browser Jyutping row.
- New abbreviation optimization; M44 already closed the selected abbreviation
  latency target.
- Track B short-row re-optimization; M44 already closed the selected native
  Track B short-row targets.
- AI behavior, learned `.gram`/octagram, plugin ABI, broader schema breadth,
  product delivery, packaging, deployment, or public-demo speed claims.

## Phase 0: Fresh Native Baseline And Oracle Capture

- [ ] Capture a fresh same-run native benchmark under
  `docs/reports/evidence/m45-native-short-key-memory-attribution/phase-0-native-baseline/`.
  Required rows:
  - startup/runtime-ready;
  - session create/select/destroy;
  - `n`, `ni`, `hao`, and `zhongguo`;
  - `ceshiyixiachangjushuruxingnengzenyang`;
  - `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`;
  - `cszysmsrsd` and `zybfshmsru`;
  - Track B 50+ guard row
    `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`.

- [ ] Capture upstream librime `1.17.0` native candidate-output oracle evidence
  for `n`, `ni`, and `hao` before implementation. Store the artifact under the
  M45 evidence root as `phase-0-short-key-oracle/` and include:
  - schema id and oracle version;
  - input string;
  - candidate count;
  - first-page candidate text, comments, and order;
  - page number, page size, highlighted candidate index, and has-next state;
  - context preedit;
  - commit preview where available;
  - capture command and environment.

- [ ] Add a Yune-vs-librime candidate comparison artifact for current Yune
  `n`, `ni`, and `hao`. If Yune already matches output, record that as the
  behavior guard. If it differs, stop short-key implementation until the
  behavior target is clarified.

- [ ] Confirm short-key owner split for `n`, `ni`, and `hao` in
  `raw_lookup_microbench.csv` and `m37_metrics.csv`. Required fields must
  distinguish:
  - prism/prefix code enumeration count and time;
  - raw table lookup codes and candidates considered;
  - borrowed or owned candidate rows scanned;
  - candidates materialized and cloned;
  - sort/rank time;
  - comment/quality formatting time;
  - filter pipeline time;
  - first-page materialization time;
  - context export and ABI string allocation time.

- [ ] If any required counter is missing from the CSV bundle, add it to
  `crates/yune-core/src/m37_metrics.rs` and export it through
  `M37_METRIC_FIELDS` in
  `crates/yune-rime-api/benches/native_inprocess_benchmark.rs` before trusting
  the baseline.

- [ ] Add or extend a metric-export regression test proving any new M45 metric
  appears in the snapshot JSON and benchmark CSV export list.

- [ ] Produce a Phase 0 short-key verdict:
  - `short-key-borrowed-prefix`: proceed only if output is guardable and the
    owner is prefix enumeration/materialization/ranking/export work;
  - `short-key-measured-no-go`: record the owner if no bounded safe branch is
    available;
  - `short-key-reporting-only`: stop if current output does not match upstream
    and cannot be corrected inside M45 scope.

## Phase 0: Memory Attribution Gate

- [ ] Extend the native benchmark evidence, or add a focused helper, so M45 can
  separate these memory classes for Track A:
  - after-ready resident working set;
  - observed high-water peak;
  - benchmark-cumulative high-water carried across rows;
  - real per-cold-start deploy/startup peak;
  - after-finalize working set;
  - private bytes or closest Windows-supported proxy;
  - file-backed mapped bytes and resident mapped pages where available;
  - allocator high-water or retained heap proxy;
  - deployment/startup transient memory;
  - retained structural owner estimates from `memory-owner-profile.csv`.

- [ ] Capture repeated Track A memory bands for startup, session, `n`, `ni`,
  `hao`, `zhongguo`, both M40 long rows, and both M42 abbreviation rows.
  Include min/median/max after-ready working set and max high-water peak.

- [ ] Reconcile the memory-owner profile against measured process memory.
  The report must explicitly name which bytes are:
  - heap-owned reducible;
  - heap-owned guarded;
  - file-backed mmap;
  - shared or overlap estimates;
  - unclassified process memory.

- [ ] Produce a Phase 0 memory verdict before memory code changes:
  - `steady-state-meets-target-benchmark-artifact`: after-ready resident Track
    A rows are within `<=107,797,708 B`, the repeated high-water peak is proven
    to be a benchmark-cumulative artifact rather than a real cold-start cost,
    both peak and steady-state numbers remain visible in reports, and the
    remaining librime gap is reported honestly;
  - `steady-state-meets-target-standing-peak-cost`: after-ready resident Track
    A rows are within `<=107,797,708 B`, but the `127 MB`-class high-water is a
    real per-cold-start deploy/startup spike. M45 may report resident target
    success, but it must keep the peak as a standing cost/blocker and cannot
    declare full memory success;
  - `transient-peak-bound`: peak is caused by a bounded transient owner that
    can be safely reduced without changing storage representation;
  - `measured-no-go`: the remaining gap is mapped-page, allocator, benchmark,
    or unknown behavior that is not safe to change in M45;
  - `memory-owner-reduction`: only allowed if a bounded owner is named and the
    expected movement is tied to the measured target.

## Workstream A: Bounded Short-Key Prefix Path

This workstream starts only after Phase 0 records
`short-key-borrowed-prefix`.

- [ ] Add failing or guard tests for `n`, `ni`, and `hao` first-page output
  against the captured Phase 0 oracle. The tests must cover candidate text,
  comments, order, context preedit, page metadata, and commit preview where
  available.

- [ ] Implement the smallest target-scoped borrowed prefix path. Expected
  shape:
  - keep exact and prefix table rows borrowed while ranking first-page output;
  - avoid cloning or allocating full candidate lists for the broad `n` prefix;
  - early-stop only when first-page order is proven stable under the active
    filters, uniquifier, simplifier, and page-size settings;
  - materialize owned candidates only for the exported page.

- [ ] Preserve the M44 under-fill fallback. If a bounded path produces fewer
  than the requested first-page candidates after filters, rerun the complete
  refresh path for that target rather than returning an under-filled page.

- [ ] Keep `upstream_sentence_model_calls=0` on `n`, `ni`, and `hao`. The M45
  short-key path must not invoke M40 full-pinyin sentence lookup or M42
  abbreviation sentence routing.

- [ ] Keep the fast path target-scoped to upstream `luna_pinyin` short-key rows
  or to a provably equivalent generic short-key condition. Do not silently
  widen TypeDuck/profile behavior.

- [ ] Rerun focused short-key tests and candidate-output comparison for `n`,
  `ni`, and `hao`.

- [ ] Rerun the native short-key profile. Workstream A success requires:
  - `n`, `ni`, and `hao` final ratios are each `<=3.0x` same-run upstream
    librime;
  - `hao` does not regress beyond its M44 `2.5x` ratio guard unless the final
    candidate-output oracle requires more work;
  - first-page candidate output matches the captured Phase 0 oracle;
  - no closed M40/M42/M44 guard regresses.

If only one or two rows pass, M45 may not close as full short-key success. It
must record the missed row as a measured blocker with owner evidence. Bare
single-letter `n` is a degenerate row (`27` lookup codes and `1,260` raw
candidates in the post-M44 diagnostic), so a partial closeout where `n` remains
above `<=3.0x` is acceptable only if the final report names it as a measured
benchmark-parity blocker rather than a user-visible UX problem. These rows are
already tens of microseconds; M45 must not claim a perceptible typing UX win
from improving them.

## Workstream B: Memory Attribution And Verdict

This workstream starts after Phase 0 records a memory verdict.

- [ ] If Phase 0 selects `steady-state-meets-target-benchmark-artifact`, update
  the reports to distinguish steady after-ready resident memory from observed
  high-water peak. Record the old peak target as not comparable to the new
  steady-state metric only because the high-water value is proven to be a
  benchmark artifact. Keep both values visible. Do not claim parity with
  librime; Yune may still use multiple times the steady resident memory of
  librime.

- [ ] If Phase 0 selects `steady-state-meets-target-standing-peak-cost`, update
  the reports to say the resident target is met but the peak target is not.
  Keep the cold-start peak as a standing cost, including any constrained-memory
  or OOM risk, and do not close memory as a full success.

- [ ] If Phase 0 selects `transient-peak-bound`, implement only the bounded
  transient reduction named by evidence. Do not rewrite compact storage,
  sentence storage, or table/prism representation unless that owner is directly
  named by the attribution.

- [ ] If Phase 0 selects `measured-no-go`, do not change memory code. Record
  the blocker, the measured classes, and the next evidence needed.

- [ ] If Phase 0 selects `memory-owner-reduction`, add a focused test or
  metric guard before changing representation. The change must preserve:
  - selected storage `rsmarisa_byte_backed`;
  - selected table/prism heap mirrors at `0`;
  - table/prism mmap or selected byte-backed mapping;
  - `source_fallback=false`;
  - positive runtime `rsmarisa` counters;
  - upstream-observable candidate output.

- [ ] Rerun memory bands after any memory-code change. A memory success claim
  requires either:
  - steady after-ready Track A rows `<=107,797,708 B` with high-water peak
    classified as a benchmark artifact and reported separately; or
  - the retained/peak target explicitly chosen in Phase 0 and met by final
    evidence.

If steady resident memory meets target but a real per-cold-start peak remains
above target, M45 may close only as resident-memory success with a standing
peak-cost blocker.

## Non-Regression Gates

M45 inherits closed gates from M40, M42, M43, and M44:

- [ ] Startup/runtime-ready and session remain within `1.25x` same-run upstream
  librime and within the M44 no-regression band unless a measured environment
  shift is documented.
- [ ] `zhongguo` remains faster than same-run upstream librime and within `5%`
  of the M44 final Yune median.
- [ ] Both M40 full-pinyin long rows remain within `1.25x` same-run upstream
  librime. They must not invoke abbreviation expansion or the M45 short-key
  fast path.
- [ ] `cszysmsrsd` and `zybfshmsru` preserve M42/M44 candidate-output parity
  and do not regress beyond a `10%` latency band from M44 final medians unless
  the final same-run ratio remains faster than librime and the difference is
  explained.
- [ ] Track A storage remains `rsmarisa_byte_backed`, table/prism mapping stays
  mmap or selected byte-backed, selected table/prism heap mirrors remain `0`,
  `source_fallback=false`, and runtime `rsmarisa` counters remain positive.
- [ ] First-page output and `RimeGetContext` remain bounded and page-sized.
- [ ] Track B 50+ guard remains stable and source-fallback-free. No Track B
  speed claim is made from M45.
- [ ] WEB-01/browser/WASM/product claims remain out of the M45 reports unless
  referenced only as explicitly separate work.

## Final Evidence And Closeout

M45 closeout must include:

- [ ] Phase 0 native benchmark and owner evidence under
  `docs/reports/evidence/m45-native-short-key-memory-attribution/`.
- [ ] Upstream oracle candidate-output artifact for `n`, `ni`, and `hao`.
- [ ] Final Yune-vs-librime candidate-output comparison for `n`, `ni`, and
  `hao`.
- [ ] Final native benchmark with startup, session, `n`, `ni`, `hao`,
  `zhongguo`, both M40 long rows, `cszysmsrsd`, `zybfshmsru`, and the Track B
  50+ guard.
- [ ] Final memory attribution artifact naming the selected memory verdict.
- [ ] Updated performance report and root-cause report, including any visual
  refresh needed to keep the reports clear.
- [ ] Updated roadmap, requirements, decisions, and milestone history if M45
  closes.
- [ ] Required final quality gates:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

M45 moves to `docs/plans/completed/` only after the final evidence supports
both the short-key verdict and the memory verdict. If either workstream misses
its target, the plan closes only as a partial result with a measured blocker,
not as a full performance success.
