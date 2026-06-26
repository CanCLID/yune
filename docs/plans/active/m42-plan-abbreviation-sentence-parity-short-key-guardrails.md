# M42 Abbreviation Sentence Parity And Short-Key Guardrails Plan

> **Status:** Active - **Milestone:** M42 (abbreviation sentence parity and
> short-key guardrails) - **Created:** 2026-06-26 - **Type:**
> native-engine plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

## Goal

Verify upstream-observable `luna_pinyin` incomplete-pinyin abbreviation
sentence behavior for `cszysmsrsd` and `zybfshmsru`; restore it only if Phase
0 confirms a real oracle target; then profile and reduce the remaining
`ni`/`hao` short-key fixed overhead only when the owner is measured and the fix
does not regress the M40 native-engine wins.

M42 is a native-engine follow-up to M40. The current fast `cszysmsrsd` and
`zybfshmsru` rows are not wins because Yune exports `0` candidates. The public
My RIME reference and user observation suggest these inputs should produce
abbreviation-driven sentence and lexicon candidates, but Phase 0 must confirm
the exact upstream librime `1.17.0` candidate shape, comments, order, preedit,
and count before implementation hard-codes expected output. M42 must make those
rows behavior-comparable before using their latency as performance evidence.

## Architecture

M42 keeps the M40 data path intact and adds only the minimum spelling graph
needed for `luna_pinyin` abbreviation sentence lookup:

- upstream `rime/librime 1.17.0` remains the behavior oracle;
- My RIME can be cited as a qualitative reproducer, not as the oracle or as the
  source of final candidate ordering;
- the prism remains the spelling/canonical-code source of truth;
- the table and sentence model remain mmap or byte-backed where they are
  already selected;
- the M40 `SentenceLookupIndex` remains the full-pinyin long-row hot path;
- abbreviation expansion must be bounded by input length, schema spelling
  rules, and real reachable vertices;
- candidate export remains first-page/context bounded;
- no source-YAML fallback, browser harness claim, product delivery claim, or
  Track B optimization claim is allowed in this milestone.

The expected implementation shape is a small abbreviation-aware span graph:
each raw input byte span maps to one or more canonical codes accepted by the
compiled prism, and the sentence model walks that graph instead of treating the
raw input as one literal code. This should let `c s z y s m s r s d` and
`z y b f s h m s r u` behave like librime's abbreviation path without
reintroducing all-substrings table scans or cloned-string mirrors.

## Tech Stack

- Rust native engine: `crates/yune-core` and `crates/yune-rime-api`.
- Key modules: `dictionary/compiled_prism.rs`, `translator/mod.rs`,
  `poet/mod.rs`, and `poet/index.rs`.
- Benchmark and evidence path: `scripts/benchmark-native-rime-inprocess.ps1`
  plus `docs/reports/evidence/m42-abbreviation-sentence-parity/`.
- Oracle target: upstream `rime/librime 1.17.0` at
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- Guard rows inherited from M40: startup, session, `hao`, `ni`, `zhongguo`,
  both long Track A rows, both incomplete-pinyin rows, and the Track B 50+
  guard row.

## Current Evidence

M40 final native evidence proves the long-row index work is valuable and must
be protected:

| Row | M40 Yune median | M40 librime median | Status |
| --- | ---: | ---: | --- |
| startup/runtime-ready | `23,934.200us` | `26,218.400us` | protect |
| session create/select/destroy | `23,994.000us` | `25,700.000us` | protect |
| `hao` | `38.200us` | `11.800us` | still `3.237x`; profile carefully |
| `ni` | `56.850us` | `14.700us` | still `3.867x`; profile carefully |
| `zhongguo` | `60.275us` | `186.400us` | protect |
| `ceshiyixiachangjushuruxingnengzenyang` | `289.914us` | `295.800us` | protect |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `494.017us` | `694.175us` | protect |
| `cszysmsrsd` | `24.820us` | `1,237.820us` | wrong behavior: `0` candidates |
| `zybfshmsru` | `26.350us` | `866.720us` | wrong behavior: `0` candidates |

The current short-key metrics do not implicate the sentence model:

- `ni` has `upstream_sentence_model_calls=0`, but still visits about `222`
  lookup views and materializes about `40` owned candidates per benchmark
  operation.
- `hao` has `upstream_sentence_model_calls=0`, but still visits about `217`
  lookup views and materializes about `60` owned candidates per benchmark
  operation.

Therefore M42 must not conflate two issues:

- incomplete-pinyin rows are a correctness/parity gap in abbreviation-aware
  sentence lookup;
- `ni`/`hao` are a fixed-overhead short-key latency problem in translator,
  lookup, materialization, filter/ranker, or ABI export shape.

## Scope Boundaries

In scope:

- upstream `luna_pinyin` native engine behavior for `cszysmsrsd` and
  `zybfshmsru`;
- focused core and ABI tests for abbreviation sentence candidate output;
- bounded native-engine profiling and possible optimization for `ni`/`hao`;
- preservation of M40 storage, memory, long-row, startup/session, and Track B
  guard evidence;
- performance and root-cause report updates after final evidence exists.

Out of scope:

- web harness, React shell, browser startup, browser typing, public demo,
  packaging, deployment, or delivery speed claims;
- TypeDuck/Jyutping feature work beyond the existing Track B guard row;
- broad memory redesign unless needed to prevent an M42 regression;
- plugin, octagram, grammar, or full librime feature parity beyond the named
  rows;
- moving the plan to completed before every closeout gate has evidence.

## Non-Regression Gates

M42 must protect the M40 wins while resolving the incomplete rows. The
abbreviation implementation gates apply only if Phase 0 proves upstream librime
has meaningful candidate output for the rows.

| Gate | Required closeout evidence |
| --- | --- |
| Abbreviation behavior | Implementation branch: `cszysmsrsd` and `zybfshmsru` export candidates through the native ABI; candidate count, text, comments, order, and preedit are compared against the captured upstream librime `1.17.0` oracle. Reporting/no-go branch: upstream exports no meaningful candidates, no abbreviation span graph is added, and reports reclassify the rows as non-comparable zero-candidate evidence rather than a speed win. |
| Abbreviation latency | Implementation branch: once behavior is comparable, both incomplete-pinyin rows must be benchmarked against same-run librime. If either row misses the `1.25x` native-engine target, the plan stays active unless a measured blocker is explicitly accepted. Reporting/no-go branch: latency is reported only as non-comparable evidence. |
| Startup/session | Startup and session medians remain within same-run librime and within `5%` of M40 medians unless the delta is measured and accepted before closeout. |
| Short rows | `hao`, `ni`, and `zhongguo` do not regress by more than `5%` from M40 medians. Any `ni`/`hao` improvement must name the measured owner. |
| Long rows | Both Track A long rows stay within `1.25x` same-run librime and do not regress by more than `5%` from M40 medians. |
| Storage | Selected Track A storage stays `rsmarisa_byte_backed`, table/prism stays mmap or byte-backed, selected table/prism heap mirrors stay `0`, source fallback stays false, and positive runtime `rsmarisa` counters remain present. |
| Memory | Track A peak working set does not exceed the M40 peak baseline `123,957,248 B` by more than `5%` (`130,155,110 B` maximum); any row-level working-set comparison must quote the exact M40 row baseline beside the new value; any new abbreviation graph or index has owner attribution. |
| Bounded output/context | First-page candidate export and `RimeGetContext` stay page-bounded. |
| Track B guard | The 50+ `jyut6ping3_mobile` row remains a guard only; no Track B speed claim is made. |
| Native-only claim | Reports and roadmap make no web, frontend, product, packaging, delivery, or browser speed claim. |

## Implementation Tasks

### 1. Capture The Oracle And Lock The Failure

- [ ] Capture upstream librime `1.17.0` candidate output for `cszysmsrsd` and
  `zybfshmsru`, including first-page candidate text, comments, order,
  composition/preedit, and selected schema metadata.
- [ ] Record the current Yune failure through the same native ABI path, proving
  the current output is `0` candidates for both rows.
- [ ] Store the oracle evidence under
  `docs/reports/evidence/m42-abbreviation-sentence-parity/phase-0-oracle/`.
- [ ] Add or update golden fixtures so expected output is captured from the
  oracle, not derived from Yune.
- [ ] Keep the My RIME observation in the notes only as a reproducer/reference;
  do not make it the oracle of record.

### Phase 0 Decision Gate

- [ ] If upstream librime `1.17.0` exports meaningful candidates for
  `cszysmsrsd` and `zybfshmsru`, continue with the abbreviation span-graph
  implementation and use the captured oracle output as the behavior target.
- [ ] If upstream librime `1.17.0` exports no meaningful candidates for either
  row, stop the abbreviation implementation path. In that case M42 must become
  a reporting/no-go correction for the misleading fast zero-candidate rows,
  with zero abbreviation engine change unless a separate measured owner remains
  in scope.
- [ ] Record the Phase 0 verdict before editing `compiled_prism.rs`,
  `translator/mod.rs`, or `poet/mod.rs`.

### 2. Add Focused Failing Tests

Implementation branch only. If Phase 0 records the reporting/no-go branch, skip
the abbreviation-output tests and record that the oracle did not prove a
behavior target.

- [ ] Add core translator or poet tests that assert the captured upstream
  abbreviation output for both incomplete-pinyin rows.
- [ ] Add an ABI-facing regression test that proves the Rime session path
  exports the captured upstream first-page candidates for both rows.
- [ ] Add negative coverage for invalid abbreviation paths so the new graph
  cannot create arbitrary one-letter source fallback behavior.
- [ ] Ensure the tests fail against the current implementation before the fix
  is applied.

### 3. Build A Bounded Abbreviation Span Graph

Implementation branch only.

- [ ] Extend the compiled-prism lookup surface with a bounded span query that
  maps raw input spans to canonical table codes accepted by the selected schema.
- [ ] Use schema spelling algebra/prism data to distinguish valid
  abbreviations from arbitrary single-letter table scans.
- [ ] Keep graph construction proportional to reachable input spans and
  canonical-code fanout; cap or reject pathological fanout with measured
  counters rather than unbounded allocation.
- [ ] Do not add heap mirrors of the selected table or prism.
- [ ] Record counters for span count, canonical-code candidates, rejected spans,
  graph edges, and graph build time in the benchmark metric export.

### 4. Route Sentence Lookup Through The Span Graph

Implementation branch only.

- [ ] Teach the sentence model to consume canonical-code spans for abbreviation
  inputs while preserving the M40 exact/range/prefix/phrase-index path for full
  pinyin inputs.
- [ ] Revisit the existing one-letter interior-boundary guard and either remove
  it for validated abbreviation spans or replace it with a schema-aware rule.
- [ ] Keep the abbreviation path behind a separate branch so full-pinyin rows
  never invoke abbreviation span expansion.
- [ ] Treat any full-pinyin long-row regression or any full-pinyin counter
  evidence touching abbreviation span expansion as a hard stop: revert, narrow
  the branch condition, or record a measured no-go before continuing.
- [ ] Preserve reachable-vertex pruning, prefix filtering, phrase-index
  counters, and bounded first-page export.
- [ ] Ensure `cszysmsrsd` and `zybfshmsru` produce candidate count, text,
  comments, order, and preedit matching the captured upstream oracle. If the
  oracle shows a sentence candidate first and matched lexicon candidates after
  it, Yune must match that captured shape.
- [ ] Keep long full-pinyin rows on the existing M40 fast path.

### 5. Preserve M40 Storage And Memory Wins

- [ ] Prove `rsmarisa_byte_backed` remains selected for Track A.
- [ ] Prove selected table/prism heap mirror bytes remain `0`.
- [ ] Prove source fallback remains false.
- [ ] Attribute any new abbreviation index/graph memory separately from
  existing sentence-model entries and translator code sets.
- [ ] If `normal_codes`, sentence-model `ModelEntry` strings, or abbreviation
  span storage become a top owner, record that as the next memory track instead
  of hiding it inside M42.

### 6. Measure Short-Key Fixed Overhead

- [ ] Re-run `ni`, `hao`, and `zhongguo` with `m37_metrics` counters after the
  abbreviation fix lands.
- [ ] Break down translator calls, lookup views visited, owned candidates
  materialized, candidates sorted, userdb merge, filter pipeline, ranker
  pipeline, ABI export, and context export for `ni` and `hao`.
- [ ] Confirm again that the upstream sentence model is not active for these
  rows before choosing a short-key optimization.
- [ ] Name the top remaining short-key owner in the root-cause report.

### 7. Optimize Short Keys Only If The Owner Is Clear

- [ ] If one owner dominates `ni`/`hao`, implement the smallest bounded fix that
  preserves output order, comments, userdb merge behavior, filters/rankers, and
  ABI/context semantics.
- [ ] If ownership is split or correctness risk is high, record a measured
  blocker/no-go and defer broader short-key work to a separate milestone.
- [ ] Do not trade away long-row latency, memory, startup/session, or
  abbreviation correctness for a short-key micro-win.

### 8. Final Benchmark And Closeout

- [ ] Run the final native benchmark with startup, session, `hao`, `ni`,
  `zhongguo`, both long Track A rows, `cszysmsrsd`, `zybfshmsru`, and the Track
  B 50+ guard row.
- [ ] Include final `summary.csv`, `samples.csv`, `m37_metrics.csv`,
  `startup_session_trace.csv`, `product_path_status.csv`, raw lookup evidence,
  memory evidence, and a final `oracle-vs-yune-candidates.json` or equivalent
  candidate-output artifact under
  `docs/reports/evidence/m42-abbreviation-sentence-parity/`.
- [ ] Update `docs/reports/yune-vs-librime-performance.md` and
  `docs/reports/yune-vs-librime-root-cause-analysis.md`, including charts and
  the optimization-history summary.
- [ ] Update `docs/roadmap.md`, `docs/requirements.md`, `docs/decisions.md`,
  and `docs/ledgers/milestone-history.md`.
- [ ] Move this plan to `docs/plans/completed/` only after all closeout gates
  pass.

## Required Final Gates

Do not mark M42 complete until these are run and the evidence is recorded:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

The final native benchmark command must include:

- startup/runtime-ready;
- session create/select/destroy;
- Track A rows: `hao`, `ni`, `zhongguo`,
  `ceshiyixiachangjushuruxingnengzenyang`,
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`,
  `cszysmsrsd`, and `zybfshmsru`;
- Track B guard row:
  `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`.

The final evidence must also include a Yune-vs-librime candidate-output
artifact for `cszysmsrsd` and `zybfshmsru`, with candidate count, text,
comments, order, preedit, schema metadata, and capture provenance.

## Closeout Criteria

M42 is complete only when all of these are true:

- `cszysmsrsd` and `zybfshmsru` are either behavior-comparable with upstream
  librime or the upstream oracle proves there is no meaningful abbreviation
  behavior target and the row is reclassified as a reporting/no-go correction;
- short-key work has either a measured safe improvement or a measured deferral;
- M40 long-row performance, storage, memory, bounded-output, and startup/session
  gates remain intact;
- reports and docs make native-engine-only claims;
- all final gates pass;
- this plan is moved from `docs/plans/active/` to `docs/plans/completed/`.
