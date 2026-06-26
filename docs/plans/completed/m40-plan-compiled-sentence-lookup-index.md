# M40 Compiled Sentence Lookup Index Plan

> **Status:** Complete - **Milestone:** M40 (compiled sentence lookup index) -
> **Created:** 2026-06-26 - **Closed:** 2026-06-26 - **Type:**
> engine-performance plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring Track A long continuous `luna_pinyin` sentence lookup closer
to same-run upstream librime by combining fast range lookup, reachable-vertex
pruning, prefix existence filtering, and a librime-shaped table phrase index,
without regressing startup, session lifecycle, short-input latency, memory, or
the M38/M39 mmap/`rsmarisa` hot path.

**Architecture:** M40 is a native-engine-only optimization over the M39
`UpstreamSentenceModel` owner. The end state is not a pile of unrelated
`HashMap` checks; it is a sentence lookup index shaped like librime's runtime:
start from reachable vertices, enumerate only valid spelling/code prefixes,
walk pre-indexed phrase/table ranges, and emit already-weighted bounded graph
edges. Transitional hash/range indexes are allowed only when they reduce the
measured owner and do not become a memory or startup regression.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), `UpstreamSentenceModel`,
`WordGraph`, `StaticTableTranslator`, `CompactTableStore`, mmap-backed
table/prism bytes, `rsmarisa`, native in-process benchmark harness, upstream
librime `1.17.0`, M37/M39 owner counters, working-set/peak memory sampling,
heap-owner profiling where available, and reports under
`docs/reports/evidence/`.

---

## Rationale

M39 closed the catastrophic long-input regression, but the remaining Track A
gap is still inside the sentence lookup owner:

| Row | M39 final Yune | Same-run librime | Ratio | Current owner shape |
| --- | ---: | ---: | ---: | --- |
| `ceshiyixiachangjushuruxingnengzenyang` | `514.903us` | `291.786us` | `1.765x` | `upstream_sentence_model_ns` around `459.8us/key`, about `241.1` prefix checks/key, about `3,564.2` entries/key. |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `917.961us` | `695.653us` | `1.320x` | `upstream_sentence_model_ns` around `836.9us/key`, about `608.6` prefix checks/key, about `6,344.6` entries/key. |

The current `word_graph_for_input` still has the old broad shape: it scans
every `(start, end)` substring boundary and performs sorted-vector range
lookups even when `start` is not reachable from position `0`. For the
59-character row the final-key substring space is `59 * 60 / 2 = 1,770`, and
the M39 average prefix-check counter across the incremental input sequence is
`608.6/key`, which matches the all-substrings shape.

Librime's sentence path avoids that shape. It keeps a set of reachable
vertices, calls the deployed prism's common-prefix search only from reachable
starts, walks a compiled table phrase index, and consumes weighted entry lists
that were sorted at deploy time. M40 should move Yune toward that data path.

This is intentionally an isolated engine-parity milestone. Web harness startup,
public-demo delivery, and application/product prioritization are separate
tracks. M40 exists because Yune's engine objective is not merely "pass the
current gate"; it is to keep pushing the core runtime toward librime-class
behavior across short, medium, incomplete, and long `luna_pinyin` inputs without
letting storage, startup, or memory regress.

## Strategy Bundle

M40 combines four strategies. They should be implemented as one coherent
sentence lookup design, not as unrelated optimizations.

| Strategy | M40 interpretation | Why it helps | Caveat |
| --- | --- | --- | --- |
| A. `HashMap`/range index | Build an exact-code range index over sentence model entries so `entries_for_code` is `O(1)` or one cheap range lookup instead of repeated `partition_point` searches. | Reduces per-substring lookup overhead. | Alone it keeps the O(L^2) scan. It must store ranges/ids into existing entries, not duplicate every text/code string. |
| B. Reachable-vertices pruning | Keep a reachable start set seeded with `0`; only process starts already reached by a graph edge. | Matches librime's `vertices` shape and skips invalid intermediate starts. | It changes graph construction order; behavior tests must prove partial candidates and final sentence ordering remain stable. |
| C. HashSet/prefix filter | Maintain valid-code and valid-prefix membership so invalid substrings are rejected before table/range lookup, and the inner loop can break when no longer on a valid prefix. | Avoids thousands of empty lookups for impossible spans. | A literal `HashSet<String>` may increase memory; prefer compact/interned strings or prefix-trie nodes if heap profiling shows cost. |
| D. Librime table phrase index | Introduce a `SentenceLookupIndex`/`SentencePhraseIndex` that represents head/trunk/tail-style prefix nodes with entry ranges ordered by weight. | This is the closest Yune equivalent to librime's compiled table phrase index and should supersede most ad hoc substring checks. | Do not eagerly build a large heap mirror at startup. The index must be lazy/shared/borrowed or the milestone fails its startup/memory gates. |

M40 also requires a measured verdict on cross-keystroke incrementality. The
primary plan is A/B/C/D because the current measured owner is inside a single
`word_graph_for_input` call. However, the final benchmark types every prefix of
the long inputs, so rebuilding the whole graph on every key may remain visible
after A/B/C/D. If post-index counters show repeated graph rebuild is the new
top owner, M40 must either add an incremental graph/cache path for the active
composition or record a measured no-go before closeout.

## Startup And Memory Caveats

The easiest way to make long input faster is also the easiest way to regress
startup and memory: build large hash maps, string sets, and phrase-index nodes
every time a schema is selected. M40 must avoid that trap.

Accepted startup/memory strategies:

- Build the sentence index lazily on first sentence-model use, or build it once
  per deployed dictionary checksum in an `Arc` cache shared by sessions.
- Store ranges, numeric ids, byte offsets, or interned code handles into
  existing entries instead of cloning candidate text/comment/code strings.
- Keep the index behind the existing Track A upstream `luna_pinyin` sentence
  path unless new evidence proves another schema needs it.
- Preserve `selected_storage=rsmarisa_byte_backed`, table/prism `mmap`, and
  selected heap mirror bytes `0` for Track A.
- Measure first-use index-build time separately from warm startup/session so a
  hidden lazy cost cannot be mistaken for a runtime win.
- If the runtime index is proven useful but its heap cost is too high, move the
  design toward a deployed byte-backed index artifact rather than accepting a
  persistent heap mirror.

Potential additional memory/startup strategies, if attribution shows they are
needed inside M40:

- Deduplicate `ModelEntry` text/code storage through compact ids or borrowed
  slices from existing compact table storage.
- Avoid building `character_codes` and vocabulary first-code maps for schemas
  or rows where preset vocabulary does not fire.
- Replace temporary `String` concatenation in phrase-code derivation with
  stack buffers or code-id paths.
- Reuse a schema-scoped immutable sentence index across session create/select
  cycles.
- Profile whole-process memory before and after with the same M39 working-set
  sampler and an owner table; do not optimize a guessed memory owner.

## Non-Negotiable Closeout Gates

- `M40-ENGINE-01` (fresh same-run benchmark): final evidence includes startup,
  session, `hao`, `ni`, `zhongguo`,
  `ceshiyixiachangjushuruxingnengzenyang`,
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`,
  `cszysmsrsd`, and `zybfshmsru` for Track A `luna_pinyin`, plus the existing
  Track B `jyut6ping3_mobile` 50+ no-regression row.
- `M40-ENGINE-02` (long-row parity): both Track A long rows must improve from
  M39 and finish within `1.25x` of same-run upstream librime. If same-run
  librime moves materially, the report must show both the ratio and absolute
  Yune movement from M39.
- `M40-ENGINE-03` (four-strategy activation): final counters prove A, B, C,
  and D are active or the milestone remains open. Required proof includes
  exact range-index hits, skipped unreachable starts, prefix-filter hits or
  prefix-trie terminations, phrase-index walks, and reduced binary-search or
  partition-point calls in the hot path.
- `M40-ENGINE-04` (owner movement): final counters show at least a `40%`
  reduction from M39 in code-prefix checks and table entries considered on the
  59-character row, or a stronger owner explanation with a measured blocker
  before any closeout.
- `M40-ENGINE-05` (startup/session guard): startup/runtime-ready and session
  create/select/destroy remain within `1.25x` of same-run librime and do not
  regress more than `5%` from M39 Yune medians unless the regression is fully
  attributed and removed before closeout.
- `M40-ENGINE-06` (short/medium guard): `hao`, `ni`, and `zhongguo` remain
  within `5x` of same-run librime and do not regress more than `5%` from M39
  Yune medians.
- `M40-ENGINE-07` (storage guard): Track A selected storage remains
  `rsmarisa_byte_backed`; table/prism bytes remain `mmap`; selected
  table/prism heap mirror bytes remain `0`; `source_fallback=false`; runtime
  `rsmarisa` exact/prefix counters remain positive.
- `M40-ENGINE-08` (memory guard): Track A peak working set does not regress
  more than `5%` from M39 final (`123,985,920 B`) and final evidence includes
  a memory-owner table for the sentence index. If a lazy first-use cache is
  added, evidence must include cold first-use, warm reuse, and retained heap.
- `M40-ENGINE-09` (bounded output/context): Track A target rows continue to use
  bounded first-page output and page-sized context export; no full-list
  fallback may become the new owner.
- `M40-ENGINE-10` (behavior): upstream-observable `luna_pinyin` behavior,
  sentence/lattice ordering, partial candidates, paging, selection, deletion,
  context reads, and touched compatibility paths remain green.
- `M40-ENGINE-11` (honest claims): reports remain native-engine-only. Browser,
  frontend, product, packaging, deployment, and public-demo speed claims remain
  outside M40.
- `M40-ENGINE-12` (incrementality verdict): final evidence reports whether
  cross-keystroke graph rebuild remains a material owner after A/B/C/D. If it
  is the top remaining long-row owner, M40 must implement an incremental
  graph/cache path or record a measured blocker before closeout.

## File Responsibilities

- `crates/yune-core/src/poet/mod.rs`: current `UpstreamSentenceModel`,
  `word_graph_for_input`, `entries_for_code`, vocabulary lookup, and sentence
  graph construction.
- `crates/yune-core/src/poet/index.rs`: create if the sentence index becomes
  large enough to deserve its own module. Owns `SentenceLookupIndex`,
  exact-code ranges, prefix membership, reachable-vertex graph lookup helpers,
  and phrase-index walking.
- `crates/yune-core/src/m37_metrics.rs`: extend with M40 sentence-index
  counters. Keep the existing exported metric names stable; a rename can be a
  separate mechanical cleanup.
- `crates/yune-core/src/translator/mod.rs`: keep bounded candidate request
  behavior and call the optimized upstream sentence model without widening
  Track B behavior.
- `crates/yune-core/src/dictionary/compiled_table.rs`: consult only if the
  final phrase-index implementation can borrow existing compact table/rsmarisa
  structure without cloning rows.
- `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`: add final
  counter columns, length-row summaries, memory rows, and raw index microbench
  rows.
- `scripts/benchmark-native-rime-inprocess.ps1`: preserve M39 inputs and add
  the incomplete-pinyin Track A rows.
- `docs/reports/evidence/m40-compiled-sentence-lookup-index/`: evidence root.
- `docs/reports/yune-vs-librime-performance.md`,
  `docs/reports/yune-vs-librime-root-cause-analysis.md`, `docs/roadmap.md`,
  `docs/requirements.md`, and `docs/decisions.md`: closeout docs.

---

## Task 0 - Rebaseline And Confirm The Owner

**Files:**

- Modify: `scripts/benchmark-native-rime-inprocess.ps1`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Create: `docs/reports/evidence/m40-compiled-sentence-lookup-index/phase-0-baseline/`

- [x] **Step 0.1: Confirm repo state**

Run:

```powershell
git fetch origin --prune
git status --short --branch --untracked-files=all
git log --oneline -5 --decorate
```

Expected: local worktree state is understood before editing. Preserve unrelated
staged changes unless the user explicitly widens scope.

- [x] **Step 0.2: Run fresh same-run native baseline**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m40-compiled-sentence-lookup-index\phase-0-baseline -Iterations 9 -SessionIterations 20 -KeyIterations 20 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -TrackBInputs "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

Expected: evidence includes Yune/librime Track A rows, Track B Yune
no-regression row, startup/session, working set, peak working set, and current
M39 owner counters.

- [x] **Step 0.3: Record baseline owner verdict**

Create:

```text
docs/reports/evidence/m40-compiled-sentence-lookup-index/phase-0-baseline/owner-verdict.md
```

Expected content:

- M40 target rows and M39/fresh baseline values.
- Per-row `upstream_sentence_model_ns`, code-prefix checks, table entries
  considered, vocabulary entries considered, graph edges, and output/context
  counters.
- Explicit statement that no code changes begin until the current top owner is
  still inside sentence graph lookup/indexing.

## Task 1 - Add M40 Sentence-Index Counters

**Files:**

- Modify: `crates/yune-core/src/m37_metrics.rs`
- Modify: `crates/yune-rime-api/src/lib.rs`
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Test: `crates/yune-core/src/tests/poet.rs`

- [x] **Step 1.1: Add counters before changing behavior**

Add counters for:

- sentence index build calls/time;
- exact range-index hits/misses;
- prefix-filter hits/misses;
- prefix-filter early breaks;
- reachable starts visited;
- unreachable starts skipped;
- phrase-index walk calls;
- phrase-index nodes visited;
- phrase-index entry ranges emitted;
- partition-point fallback calls.

Expected: existing M39 counters remain stable and new counters default to zero
when the feature path does not fire.

- [x] **Step 1.2: Add focused counter tests**

Add a small `UpstreamSentenceModel` test that runs a long enough artificial
input and asserts:

- baseline path records prefix checks;
- new counters are exported;
- counters reset correctly between measurements.

Run:

```powershell
cargo test -p yune-core upstream_sentence_model -- --nocapture
```

Expected: tests pass before behavior changes.

## Task 2 - Implement A: Exact Code Range Index

**Files:**

- Modify: `crates/yune-core/src/poet/mod.rs`
- Create if needed: `crates/yune-core/src/poet/index.rs`
- Test: `crates/yune-core/src/tests/poet.rs`

- [x] **Step 2.1: Introduce exact range index**

Build an immutable range index from code to `Range<usize>` over the existing
sorted `entries_by_code` vector. Store ranges into the vector; do not clone
entry text or code payloads into the index.

Expected: `entries_for_code` can return the same entry slice through the range
index and records range-index hits/misses.

- [x] **Step 2.2: Preserve ordering behavior**

Add a test with multiple entries sharing the same code and different weights.

Expected:

- candidate order matches pre-M40 behavior;
- range index returns the same entries as the old `partition_point` lookup;
- short rows keep existing behavior.

Run:

```powershell
cargo test -p yune-core upstream_sentence_model -- --nocapture
```

## Task 3 - Implement B And C: Reachable Vertices Plus Prefix Filter

**Files:**

- Modify: `crates/yune-core/src/poet/mod.rs`
- Modify: `crates/yune-core/src/poet/index.rs` if created
- Test: `crates/yune-core/src/tests/poet.rs`

- [x] **Step 3.1: Add prefix membership**

Create a valid-code and valid-prefix membership structure from the same code
set used by the exact range index. Prefer interned/borrowed code handles when
practical. If this requires owned `String` sets, record retained heap in the
Task 5 memory evidence and keep the implementation easy to replace with a
compact trie.

Expected: invalid substrings can be rejected without calling exact entry lookup.

- [x] **Step 3.2: Process only reachable starts**

Change `word_graph_for_input` so it starts with reachable vertex `0`, skips
unreachable starts, and inserts `end` when a table or vocabulary edge is added.

Expected: counters report skipped starts on long rows and behavior remains
stable.

- [x] **Step 3.3: Break impossible prefix scans**

When a substring is no longer a valid prefix, break the inner `end` scan for
that start.

Expected: code-prefix checks and empty exact lookups drop materially on the
37-character and 59-character rows.

Run:

```powershell
cargo test -p yune-core upstream_sentence_model -- --nocapture
cargo test -p yune-core translator:: -- --nocapture
```

## Task 4 - Implement D: Librime-Shaped Sentence Phrase Index

**Files:**

- Modify: `crates/yune-core/src/poet/mod.rs`
- Modify or create: `crates/yune-core/src/poet/index.rs`
- Modify if borrowing compact table internals is needed:
  `crates/yune-core/src/dictionary/compiled_table.rs`
- Test: `crates/yune-core/src/tests/poet.rs`
- Test: `crates/yune-core/src/tests/translator.rs`

- [x] **Step 4.1: Add phrase-index node model**

Introduce a `SentencePhraseIndex` that represents code-prefix nodes with:

- node id;
- child code/prefix transitions;
- exact entry range;
- optional phrase/tail range;
- pre-sorted weighted entry access.

This can be implemented over code strings first, but the interface must allow
future code-id or byte-backed nodes without changing `UpstreamSentenceModel`.

- [x] **Step 4.2: Route graph construction through phrase-index walks**

From each reachable start, walk the phrase index over input prefixes and emit
graph edges only when an exact entry range exists.

Expected:

- phrase-index walk counters are positive on both long Track A rows;
- the old all-substrings exact lookup path is not the hot owner;
- prefix filtering remains a guard, not the primary algorithm.

- [x] **Step 4.3: Preserve weighted candidate behavior**

Add tests for:

- longest valid segmentation still wins where M39 expected it;
- higher weight wins among same-span entries;
- partial candidates still have correct consumed length;
- incomplete pinyin rows `cszysmsrsd` and `zybfshmsru` do not produce invalid
  crashes or full-list explosions.

Run:

```powershell
cargo test -p yune-core poet:: -- --nocapture
cargo test -p yune-core translator:: -- --nocapture
```

- [x] **Step 4.4: Measure cross-keystroke rebuild cost**

Add counters that distinguish:

- graph construction for the current key;
- retained graph/index reuse from the previous key, if any;
- discarded graph work when the input is a one-character extension of the last
  composition;
- time spent extending the graph versus rebuilding it from position `0`.

If A/B/C/D remove the old all-substrings owner but repeated whole-graph rebuild
becomes the top remaining owner for the 37- or 59-character row, implement a
bounded incremental graph/cache path for the active composition. If behavior or
state-lifetime constraints make that unsafe in M40, record the measured blocker
in final evidence before closeout.

Expected: M40 does not miss a second-order owner created by typing the long row
as a sequence of prefixes.

## Task 5 - Memory And Startup Hardening

**Files:**

- Modify: `crates/yune-core/src/poet/mod.rs`
- Modify: `crates/yune-core/src/poet/index.rs` if created
- Modify: `crates/yune-rime-api/benches/native_inprocess_benchmark.rs`
- Create:
  `docs/reports/evidence/m40-compiled-sentence-lookup-index/phase-3-memory/`

- [x] **Step 5.1: Split cold first-use from warm reuse**

Add benchmark rows or metadata that separately reports:

- schema startup/runtime-ready;
- session create/select/destroy;
- first sentence-index build time;
- warm sentence-index reuse time;
- retained heap/working-set after index build.

Expected: a lazy index cannot hide a startup regression.

- [x] **Step 5.2: Remove unnecessary string duplication**

If memory evidence shows the range/prefix/phrase index retains duplicated
strings, replace the top duplication owner with ids, ranges, or borrowed
handles before closeout.

Expected: Track A peak does not regress more than `5%` from M39 final.

- [x] **Step 5.3: Record memory-owner evidence**

Write:

```text
docs/reports/evidence/m40-compiled-sentence-lookup-index/phase-3-memory/memory-owner-summary.md
```

Expected: summary names sentence-index retained heap, `ModelEntry` storage,
prefix membership storage, phrase-index storage, and any remaining whole-process
owners that are outside M40.

## Task 6 - Final Native Benchmark And Closeout Docs

**Files:**

- Create:
  `docs/reports/evidence/m40-compiled-sentence-lookup-index/phase-4-final-native/`
- Create:
  `docs/reports/evidence/m40-compiled-sentence-lookup-index/final-gates.md`
- Modify: `docs/reports/yune-vs-librime-performance.md`
- Modify: `docs/reports/yune-vs-librime-root-cause-analysis.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Modify: `docs/decisions.md`
- Moved on closeout from the active plans directory to
  `docs/plans/completed/m40-plan-compiled-sentence-lookup-index.md`

- [x] **Step 6.1: Run final benchmark**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m40-compiled-sentence-lookup-index\phase-4-final-native -Iterations 9 -SessionIterations 20 -KeyIterations 20 -TrackAInputs "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" -TrackBInputs "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" -DeployProductBeforeBenchmark
```

Expected: all M40 closeout gates have direct evidence.

- [x] **Step 6.2: Run quality gates**

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

Expected: all pass.

- [x] **Step 6.3: Update reports and ledgers**

Update final reports with:

- final same-run Yune/librime ratios;
- sentence-index bottleneck map;
- A/B/C/D strategy gate table;
- memory/startup no-regression table;
- explicit browser/frontend/product exclusion.

Expected: no report claims application-visible speed wins from native evidence.

## Out Of Scope

| Item | Reason |
| --- | --- |
| Web harness startup optimization | Paused as a separate effort; M40 is native-engine-only. |
| Product/frontend/browser speed claims | Need separate real-browser evidence and are not closed by M40. |
| Track B `jyut6ping3_mobile` rewrite | Track B is a no-regression guard unless fresh evidence shows it shares the Track A sentence-index owner. |
| Learned `.gram`/octagram support | Still deferred until a named target needs learned grammar behavior. |
| Full librime C++ plugin ABI | Not required for this sentence lookup owner. |

## Plan Self-Review

- Spec coverage: A, B, C, and D each have an implementation task and a closeout
  counter gate.
- Startup/memory caveat: explicit gates block eager heap mirrors and hidden lazy
  first-use costs.
- Scope: native engine only; web harness optimization remains separate.
- Behavior: upstream-observable behavior and existing M38/M39 storage/candidate
  gates are preserved.
