# M44 Native Performance Owner Reduction Plan

> **Status:** Complete with measured blockers - **Milestone:** M44 (native
> performance owner reduction) - **Created:** 2026-06-26 - **Closed:**
> 2026-06-26 - **Type:** native-engine plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

## Goal

Reduce the four post-M43 native performance blockers together without
regressing closed behavior or storage wins:

1. Track A `luna_pinyin` short-key latency for `hao` and `ni`.
2. Track A `luna_pinyin` abbreviation latency for `cszysmsrsd` and
   `zybfshmsru`.
3. Track A whole-process memory not explained by the M43
   `poet.entries_by_code` reduction.
4. Track B `jyut6ping3_mobile` native product-profile short-row lookup
   explosion.

M44 is not allowed to close as a performance success unless all four target
families meet their stated targets and every no-regression gate passes.

## Closeout Summary

M44 closes as a partial native/profile performance reduction, not as a full
performance success. Final code-state evidence:

- `hao`: `24.700us`, `2.123x`; target met.
- `ni`: `49.450us`, `3.434x`; target missed and recorded as the residual
  short-key blocker.
- `cszysmsrsd`: `545.020us`, `0.445x`; target met with M42 candidate-output
  parity preserved.
- `zybfshmsru`: `540.970us`, `0.634x`; target met with M42 candidate-output
  parity preserved.
- Track A peak memory: `127,619,072 B`; target missed, so no memory win is
  claimed.
- Track B short rows `h`, `ha`, `hai`, `hau`, `nei`, and `ngo`: `84.7-92.4%`
  faster, with selected exact lookups reduced from thousands to `1-3` per key.

M44 preserves the M40 full-pinyin long-row path, M42 abbreviation
candidate-output guard, M43 storage guards, source-fallback-free Track A and
Track B deployed storage, bounded first-page context export, and native/profile
scope. Browser, frontend, packaging, deployment, public-demo, and broad
product-delivery speed are not claimed.

## Architecture

M44 is a native-engine owner-first milestone with four workstreams. Phase 0
adds missing counters and fresh baselines for all four issues before any
optimization branch is accepted. The likely implementation shape is a sequence
of bounded fixes: a borrowed first-page fast path for Track A short keys, a
bounded abbreviation span/code cache or ranking fast path, a retained/RSS owner
pass for memory, and a bounded spelling-expansion or lookup-index fix for the
Track B native product-profile path.

Each workstream must keep behavior and storage invariants intact. The M42
abbreviation branch must stay separate from the M40 full-pinyin sentence path,
short-key output must keep upstream-observable candidate shape, selected
table/prism heap mirrors must stay zero, source fallback must stay disabled,
and Track B work must remain native/profile-scoped rather than becoming a
browser, frontend, packaging, or public-demo claim.

## Tech Stack

- Rust native engine: `crates/yune-core`.
- Rime ABI and benchmark harness: `crates/yune-rime-api`.
- Hot-path files likely touched:
  - `crates/yune-core/src/translator/mod.rs`
  - `crates/yune-core/src/poet/mod.rs`
  - `crates/yune-core/src/poet/index.rs`
  - Track B spelling/profile modules identified by Phase 0 owner evidence.
  - `crates/yune-core/src/m37_metrics.rs`
  - `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Report/evidence files:
  - `docs/reports/yune-vs-librime-root-cause-analysis.md`
  - `docs/reports/yune-vs-librime-performance.md`
  - `docs/reports/evidence/m44-native-performance-owner-reduction/`
- Oracle target: upstream `rime/librime 1.17.0` at
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.

## Current Baseline

M44 starts from the post-M43 bottleneck pass on current `main`
`ad93ec787d2b6e4f952b05836e8f0ed46b5a79d2`:

| Row | Yune median | librime median | Ratio | Main current owner |
| --- | ---: | ---: | ---: | --- |
| `cszysmsrsd` | `4,126.180 us` | `1,227.830 us` | `3.361x` | `4,101.990 us/key` upstream sentence-model abbreviation path |
| `zybfshmsru` | `4,244.470 us` | `836.990 us` | `5.071x` | `4,212.075 us/key` upstream sentence-model abbreviation path |
| `hao` | `37.733 us` | `11.333 us` | `3.329x` | `33.817 us/key` translator production |
| `ni` | `55.750 us` | `14.100 us` | `3.954x` | `51.500 us/key` translator production |
| `zhongguo` | `60.387 us` | `156.125 us` | `0.387x` | preserved faster-than-librime row |
| `ceshiyixiachangjushuruxingnengzenyang` | `284.792 us` | `288.322 us` | `0.988x` | preserved M40 full-pinyin parity |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `481.905 us` | `658.466 us` | `0.732x` | preserved M40 full-pinyin win |

Track B deployed product-profile rows from the corrected post-M43 run:

| Row | Yune median | Main current owner |
| --- | ---: | --- |
| `h` | `21,888.600 us` | `21,873.450 us/key` translator; `7,627` exact lookups/key |
| `ha` | `11,639.250 us` | `11,605.425 us/key` translator; `3,814` exact lookups/key |
| `hai` | `7,580.300 us` | `7,556.967 us/key` translator; `2,544.667` exact lookups/key |
| Track B long guard | `185.531 us` | guard row remains stable |

Important owner detail:

- `cszysmsrsd`: graph rebuild is only `118.340 us/key`, while upstream
  sentence-model abbreviation time is `4,101.990 us/key`.
- `zybfshmsru`: graph rebuild is only `98.025 us/key`, while upstream
  sentence-model abbreviation time is `4,212.075 us/key`.
- Current counters do not yet isolate abbreviation span discovery,
  `model.has_code`, code-span graph build, graph-to-sentence ranking, preedit
  formatting, or candidate formatting. M44 must add those counters before
  choosing an implementation.
- Current short-key counters identify translator production as the main owner,
  but do not yet prove whether candidate cloning, filtering, ordering,
  quality/comment formatting, or first-page materialization is the reducible
  sub-owner.
- Current memory evidence reduces `poet.entries_by_code` by `19,513,879 B`,
  but Track A peak remains around `127.5 MB`; M44 must profile RSS/allocator
  and mapped/shared owners before another storage rewrite.
- Current Track B product-profile evidence shows thousands of exact lookups per
  short key. M44 must identify whether the reducible owner is spelling
  expansion, no-marisa compact lookup, product-profile indexing, or candidate
  materialization.

## Scope Boundaries

In scope:

- Track A `luna_pinyin` short-key rows `hao` and `ni`.
- Track A `luna_pinyin` abbreviation rows `cszysmsrsd` and `zybfshmsru`.
- Track A whole-process memory attribution and one bounded memory owner
  reduction if Phase 0 identifies a safe owner.
- Track B `jyut6ping3_mobile` native product-profile short rows `h`, `ha`,
  `hai`, `hau`, `nei`, and `ngo`, plus the existing 50+ row as a guard.
- Missing owner counters for abbreviation, short-key translator production,
  retained/RSS memory, and Track B spelling/lookup expansion.
- Native ABI candidate-output parity against upstream librime `1.17.0` for
  Track A and profile-scoped behavior preservation for Track B.
- No-regression gates for startup, session, `zhongguo`, both M40 full-pinyin
  long rows, M42 abbreviation output, M43 storage status, bounded
  output/context, and the Track B 50+ guard row.
- Evidence hygiene for the new M44 evidence root.

Out of scope:

- Browser, frontend, public-demo, product-delivery, packaging, deployment, or
  web-harness speed claims.
- Learned `.gram`/octagram, plugin ABI, broader schema breadth, or AI-native
  behavior.
- Deleting completed milestone evidence as part of implementation.

## Evidence Retention And Cleanup Policy

The current disk pressure is mostly generated build output, not checked-in
report evidence:

- `target/`: about `32,669.87 MiB`.
- `docs/reports/evidence/`: about `120.95 MiB`.
- `apps/yune-web/e2e/results/`: about `35.10 MiB`.

M44 should keep its final evidence compact:

- Keep final `commands.txt`, `environment.txt`, `summary.csv`,
  `summary-comparison.csv`, `raw_lookup_microbench.csv`,
  `memory-owner-profile.csv`, `product_path_status.csv`, final candidate-output
  comparison, and final gates.
- Keep full `samples.csv` and `m37_metrics.csv` only for the final baseline and
  final closeout runs.
- Do not keep failed reruns, non-deployed Track B diagnostics, duplicate
  transient logs, or copied benchmark DLL artifacts unless the final report
  explicitly cites them.
- If old evidence cleanup is done later, it must be a separate evidence-hygiene
  slice with a manifest of deleted/generated-only paths. Do not delete
  completed `final-gates.md`, report-linked CSVs, candidate-output artifacts,
  or visualizations.
- It is acceptable to clean generated `target/native-inprocess/*` and broader
  `target/` build artifacts when the user explicitly asks for disk cleanup,
  with the understanding that later Rust builds and benchmarks will rebuild.

## Phase 0: Fresh Baseline And Missing Counters

- [ ] Capture a fresh same-run native benchmark under
  `docs/reports/evidence/m44-native-performance-owner-reduction/phase-0-baseline/`.
  Required rows:
  - startup/runtime-ready;
  - session create/select/destroy;
  - `hao`, `ni`, `zhongguo`;
  - `cszysmsrsd`, `zybfshmsru`;
  - `ceshiyixiachangjushuruxingnengzenyang`;
  - `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`;
  - Track B short rows `h`, `ha`, `hai`, `hau`, `nei`, and `ngo` with
    `-DeployProductBeforeBenchmark`;
  - Track B guard
    `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`.

- [ ] Add M44 abbreviation counters to `crates/yune-core/src/m37_metrics.rs`.
  Minimum fields:
  - `abbreviation_span_discovery_calls`;
  - `abbreviation_span_discovery_ns`;
  - `abbreviation_span_candidates_considered`;
  - `abbreviation_span_codes_emitted`;
  - `abbreviation_model_has_code_calls`;
  - `abbreviation_model_has_code_ns`;
  - `abbreviation_code_span_graph_build_ns`;
  - `abbreviation_sentence_ranking_ns`;
  - `abbreviation_preedit_format_ns`;
  - `abbreviation_candidate_format_ns`.

- [ ] Add M44 short-key counters to `crates/yune-core/src/m37_metrics.rs`.
  Minimum fields:
  - `short_key_candidate_rows_scanned`;
  - `short_key_candidates_materialized`;
  - `short_key_candidates_cloned`;
  - `short_key_filter_ns`;
  - `short_key_sort_rank_ns`;
  - `short_key_comment_quality_ns`;
  - `short_key_first_page_materialize_ns`.

- [ ] Add M44 memory counters or evidence fields to the native benchmark
  bundle. Minimum evidence:
  - Track A working set and peak repeated run band;
  - retained owner profile;
  - allocator or heap bucket owner rows where available;
  - mmap/shared/private-byte classification;
  - explicit reconciliation from retained-owner estimates to peak/RSS.

- [ ] Add M44 Track B counters to `crates/yune-core/src/m37_metrics.rs`.
  Minimum fields:
  - `track_b_spelling_expansions_considered`;
  - `track_b_spelling_expansion_ns`;
  - `track_b_exact_lookup_calls`;
  - `track_b_exact_lookup_ns`;
  - `track_b_prefix_lookup_calls`;
  - `track_b_prefix_lookup_ns`;
  - `track_b_candidates_materialized`;
  - `track_b_first_page_materialize_ns`.

- [ ] Add all new M44 fields to `M37_METRIC_FIELDS` in
  `crates/yune-rime-api/benches/native_inprocess_benchmark.rs` before trusting
  the benchmark CSV bundle.

- [ ] Add a focused metric-export regression test proving every new M44 metric
  appears in the snapshot JSON and `m37_metrics.csv`. If the existing
  `m37_metrics_exports_snapshot_json_for_loaded_benchmarks` test owns this
  surface, extend it; otherwise add the smallest focused test beside it.

- [ ] Add or extend focused tests for abbreviation candidate-output parity
  before implementation. The tests must assert candidate count, text, comments,
  order, context preedit, commit preview, and first-page metadata for
  `cszysmsrsd` and `zybfshmsru` against captured upstream evidence.

- [ ] Add or extend focused tests for `hao` and `ni` candidate-output parity
  before implementation. The tests must assert first-page text, comments,
  order, preedit, and candidate count against captured upstream evidence.

- [ ] Add focused Track B guard tests for the product-profile rows selected by
  Phase 0. At minimum, the long Track B guard must preserve candidate count,
  first-page order, storage status, and `source_fallback=false`; short-row
  output must not regress if the benchmark path has stable expected output.

- [ ] Record a Phase 0 verdict in
  `docs/reports/evidence/m44-native-performance-owner-reduction/phase-0-baseline/phase-0-verdict.md`.
  The verdict must record one status for each workstream:
  - `abbreviation-span-index-or-cache`;
  - `abbreviation-ranking-fast-path`;
  - `short-key-borrowed-first-page`;
  - `memory-rss-owner-reduction`;
  - `track-b-bounded-spelling-lookup`;
  - `workstream-reporting-no-go`.

Branch selection rules:

- Choose `abbreviation-span-index-or-cache` if span discovery plus
  `model.has_code` checks account for at least `40%` of the abbreviation row
  time or at least `1,000 us/key` on either row.
- Choose `abbreviation-ranking-fast-path` if code-span graph build plus
  sentence ranking/candidate formatting account for at least `40%` of the row
  time or at least `1,000 us/key` on either row.
- Choose `short-key-borrowed-first-page` if candidate materialization,
  filtering, ranking, comments, or first-page export accounts for at least
  `40%` of `hao` or `ni` latency.
- Choose `memory-rss-owner-reduction` if Phase 0 identifies at least `10 MB` of
  private heap/RSS owner bytes that are not already explained by M43
  `poet.entries_by_code` and can be reduced without selected table/prism heap
  mirrors or source fallback.
- Choose `track-b-bounded-spelling-lookup` if spelling expansion, exact lookup,
  or candidate materialization accounts for at least `40%` of any Track B
  short-row latency or at least `2 ms/key`.
- Choose `workstream-reporting-no-go` for any workstream where counters do not
  isolate a safe bounded owner, the likely fix requires behavior drift, or the
  observed timing is dominated by noise or uninstrumented external cost that
  needs a smaller diagnostic milestone first.

## Workstream A: Track A Abbreviation Latency

Run if Phase 0 selects `abbreviation-span-index-or-cache` or
`abbreviation-ranking-fast-path`.

- [ ] Write failing or guard tests for the chosen span/index behavior before
  implementation. Required assertions:
  - the same `SentenceCodeSpan` set is accepted for `cszysmsrsd` and
    `zybfshmsru`;
  - invalid or non-ASCII inputs still bypass abbreviation sentence expansion;
  - full-pinyin rows do not invoke abbreviation span expansion;
  - candidate-output parity for both abbreviation rows remains unchanged.

- [ ] Implement the smallest bounded span/index change. Acceptable shapes:
  - cache `prism.lookup_canonical_codes` results per `(start, end)` span for
    the active input;
  - replace repeated `model.has_code(code)` calls with a range-index or code-id
    check backed by `UpstreamSentenceModel`;
  - precompute valid abbreviation `SentenceCodeSpan` candidates once per active
    input and reuse them during graph construction.

- [ ] Keep all retained state bounded. If any cache survives beyond one active
  input processing pass, report its retained bytes in the memory-owner profile
  and prove it is invalidated on input reset, session destruction, and schema
  switch.

- [ ] Re-run the focused abbreviation tests and the Phase 0 abbreviation rows.
  The selected owner counter must drop by at least `25%` and both rows must
  clear the Phase 0 noise band. Target medians:
  - `cszysmsrsd <= 3,094.635 us` and same-run ratio no worse than `2.5x`;
  - `zybfshmsru <= 3,183.353 us` and same-run ratio no worse than `4.0x`.

- [ ] Write failing or guard tests for the chosen graph/ranking behavior before
  implementation. Required assertions:
  - full-input sentence candidate remains first when upstream oracle has it
    first;
  - matched lexicon candidates keep the captured order after the sentence;
  - partial candidates keep the captured consumed span and preedit;
  - duplicate candidate text is still de-duplicated in the same order.

- [ ] Implement the smallest bounded graph/ranking change. Acceptable shapes:
  - rank from borrowed graph entries and materialize only the final first page;
  - avoid cloning path text for states that cannot enter the final beam;
  - compute synthesized abbreviation sentence and follow-up lexicon candidates
    from the same bounded state list instead of rebuilding equivalent lists.

- [ ] Re-run focused abbreviation tests and the Phase 0 abbreviation rows. The
  selected owner counter must drop by at least `25%` and both rows must clear
  the Phase 0 noise band.

## Workstream B: Track A Short-Key Latency

Run if Phase 0 selects `short-key-borrowed-first-page`.

- [ ] Write failing or guard tests for `hao` and `ni` first-page behavior
  before implementation. Required assertions:
  - first-page candidate text and comments match the captured upstream oracle;
  - candidate order remains stable;
  - preedit and commit preview remain unchanged;
  - `upstream_sentence_model_calls=0`;
  - full-pinyin and abbreviation rows do not use the short-key fast path.

- [ ] Implement the smallest bounded borrowed first-page path for exact+prefix
  candidates. Acceptable shapes:
  - borrow table candidate text/comment data until final first-page export;
  - avoid cloning candidates that cannot appear on the first page;
  - split filtering/ranking from materialization so rejected candidates are not
    fully allocated;
  - keep ordering identical to the captured oracle.

- [ ] Re-run focused short-key tests and the Phase 0 `hao`/`ni` rows. The
  selected short-key owner counter must drop by at least `25%`. Target medians:
  - `hao <= 28.300 us` and same-run ratio no worse than `2.5x`;
  - `ni <= 41.813 us` and same-run ratio no worse than `3.0x`.

## Workstream C: Track A Whole-Process Memory

Run if Phase 0 selects `memory-rss-owner-reduction`.

- [ ] Record a reconciled memory-owner profile before implementation. It must
  separate retained heap, allocator overhead, private RSS, mmap/shared bytes,
  and overlap estimates. The branch may not use overlap estimates as reducible
  bytes.

- [ ] Write focused accounting tests for the selected owner before
  implementation. Required assertions:
  - selected table/prism heap mirror bytes remain `0`;
  - source fallback remains disabled;
  - owner accounting does not double-count mmap/shared bytes;
  - retained bytes for any new cache are reported.

- [ ] Implement the smallest bounded memory owner reduction. Acceptable shapes:
  - remove duplicated lookup keys that can be represented by ids/ranges;
  - pack remaining retained short strings into a byte pool;
  - lazy-load userdb or profile-only state only if behavior and startup/session
    gates remain intact;
  - share immutable profile data only where schema/profile boundaries stay
    explicit.

- [ ] Re-run memory tests and repeated Track A memory benchmark. Target:
  - Track A peak `<=107,797,708 B`; or
  - if that target is not reached, the selected private/RSS owner must drop by
    at least `10 MB` and the closeout must mark memory as a measured blocker,
    not a memory win.

## Workstream D: Track B Native Product-Profile Short Rows

Run if Phase 0 selects `track-b-bounded-spelling-lookup`.

- [ ] Write focused Track B guard tests before implementation. Required
  assertions:
  - deployed product profile remains `compiled_ready=true`;
  - selected storage remains byte-backed or mmap-backed as recorded by the
    benchmark;
  - selected heap mirrors remain `0`;
  - `source_fallback=false`;
  - the Track B 50+ guard row preserves first-page behavior.

- [ ] Implement the smallest bounded spelling/lookup fix. Acceptable shapes:
  - cap repeated spelling expansion by caching canonical expansions per active
    input;
  - replace repeated no-marisa compact exact lookups with a profile-scoped
    range or id index;
  - materialize only the bounded first page once candidate order is known;
  - keep Track B profile behavior isolated from the default Track A path.

- [ ] Re-run Track B short rows and the long guard. Targets:
  - `h`, `ha`, `hai`, `hau`, `nei`, and `ngo` medians drop by at least `50%`;
  - exact lookup calls/key drop by at least `75%` on rows where lookup
    explosion was the selected owner;
  - long Track B guard stays within `10%` of the Phase 0 median/p95.

## Workstream E: Reporting No-Go For Any Missed Target

Run for any workstream that selects `workstream-reporting-no-go` or misses its
target after implementation.

- [ ] Record the measured blocker in
  `docs/reports/evidence/m44-native-performance-owner-reduction/phase-0-baseline/no-go.md`
  or the final benchmark `final-gates.md`. The no-go must name:
  - the measured top owner;
  - why a safe bounded implementation is not available in M44;
  - which target was missed;
  - what exact extra instrumentation or oracle evidence is required next.

- [ ] Update the root-cause report and roadmap so the missed workstream remains
  visible as a measured blocker. Do not let one successful workstream hide a
  remaining short-key, abbreviation, memory, or Track B blocker.

## No-Regression Gates

- [ ] Preserve M42 abbreviation output for `cszysmsrsd` and `zybfshmsru`.
  Required artifact:
  `docs/reports/evidence/m44-native-performance-owner-reduction/final-native-benchmark/oracle-vs-yune-candidate-output.md`
  or equivalent JSON/Markdown with candidate count, text, comments, order,
  preedit, commit preview, and first-page metadata.

- [ ] Preserve M40 full-pinyin path boundaries. Final metrics must show the two
  full-pinyin long rows do not invoke abbreviation span expansion or short-key
  fast paths, and their same-run ratios remain within `1.25x` of librime or
  within the M43 final no-regression band.

- [ ] Preserve `hao` and `ni` candidate behavior while optimizing them. If
  either row misses its target, M44 may close only as a partial result with a
  measured short-key blocker.

- [ ] Preserve startup/runtime-ready, session create/select/destroy, and
  `zhongguo` no-regression gates from M43.

- [ ] Preserve Track A storage and output gates:
  - `selected_storage=rsmarisa_byte_backed`;
  - table/prism mapping mode `mmap`;
  - selected table/prism heap mirror bytes `0`;
  - `source_fallback=false`;
  - positive runtime `rsmarisa` counters;
  - first-page output and `RimeGetContext` remain page-bounded.

- [ ] Preserve and improve memory gates. Track A peak must not exceed the M43
  final peak band by more than `5%` under any workstream. A memory win requires
  Track A peak `<=107,797,708 B`; otherwise memory remains a measured blocker.
  Any new cache must report retained bytes.

- [ ] Preserve Track B profile behavior while optimizing short rows. The 50+
  `jyut6ping3_mobile` guard row must remain within `10%` of the Phase 0
  median/p95 or record a measured blocker. Track B reporting must stay
  profile-scoped and must not imply upstream `luna_pinyin` behavior.

## Final Benchmark And Closeout

- [ ] Run the final native benchmark under
  `docs/reports/evidence/m44-native-performance-owner-reduction/final-native-benchmark/`
  with the required M44 row set.

- [ ] Produce a final comparison summary that includes:
  - startup/runtime-ready;
  - session create/select/destroy;
  - `hao`, `ni`, `zhongguo`;
  - `cszysmsrsd`, `zybfshmsru`;
  - both M40 long full-pinyin rows;
  - Track B short rows `h`, `ha`, `hai`, `hau`, `nei`, and `ngo`;
  - Track B 50+ guard row;
  - selected owner counters before/after for all four workstreams;
  - storage and memory status;
  - candidate-output parity artifact.

- [ ] Update:
  - `docs/reports/yune-vs-librime-root-cause-analysis.md`;
  - `docs/reports/yune-vs-librime-performance.md`;
  - `docs/roadmap.md`;
  - `docs/requirements.md`;
  - `docs/decisions.md`;
  - `docs/ledgers/milestone-history.md`.

- [ ] Keep the M44 evidence root compact. Delete only generated failed reruns
  or uncited diagnostic artifacts inside the M44 evidence root, and record any
  deletion in `final-gates.md`. Do not delete old milestone evidence as part of
  M44 closeout.

- [ ] Run final quality gates:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

- [ ] Move this plan to
  `docs/plans/completed/m44-plan-native-performance-owner-reduction.md`
  only after every final gate above is backed by evidence.

## Success Criteria

M44 can close as a performance success only if all four target families pass:

- `hao <=28.300 us` with same-run ratio no worse than `2.5x`, and
  `ni <=41.813 us` with same-run ratio no worse than `3.0x`;
- `cszysmsrsd <=3,094.635 us` with same-run ratio no worse than `2.5x`, and
  `zybfshmsru <=3,183.353 us` with same-run ratio no worse than `4.0x`;
- Track A peak memory `<=107,797,708 B`;
- Track B short rows `h`, `ha`, `hai`, `hau`, `nei`, and `ngo` improve by at
  least `50%`, with selected lookup-explosion counters down at least `75%`
  where those counters are the selected owner;
- startup/session, `hao`, `ni`, `zhongguo`, both full-pinyin long rows,
  abbreviation candidate-output parity, storage, bounded output/context, and
  Track B guard rows pass.

M44 can close as a partial result with measured blockers only if:

- Phase 0 or implementation evidence proves one or more targets cannot be
  safely changed without behavior/storage regressions; and
- the final reports clearly state which of the four target families passed,
  which remain blockers, and what evidence is needed next.

M44 must not close if:

- candidate text/comment/order/preedit parity drifts;
- full-pinyin rows invoke abbreviation expansion or short-key fast paths;
- selected table/prism heap mirrors or source fallback return;
- final evidence omits startup, session, `hao`, `ni`, `zhongguo`, both
  abbreviation rows, both M40 long rows, Track B short rows, or the Track B
  guard row;
- report wording claims browser, frontend, packaging, deployment, public-demo,
  or upstream-default TypeDuck-profile speed wins not backed by M44 evidence.
