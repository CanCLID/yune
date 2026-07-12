# Post-fix Luna performance source audit

Date: 2026-07-11

This is a read-only diagnostic companion for the accepted post-page-order-fix
macOS evidence. It does not change a benchmark, implementation, signed ceiling,
or acceptance rule.

> **Supersession boundary:** this source audit preceded the later prefix-
> behavior strata, metrics-on/off, allocator, API, hardware-counter, and
> cross-platform controls in the final report. Its “faster long rows” wording
> describes the unnormalized aggregate only. The later control proves those
> aggregate wins are concentrated in candidate-text-different prefixes; on
> text-matched prefixes Yune is `1.420x` / `1.204x` librime. The later paired
> control also quantifies M37 instrumentation tax at 1.0-2.9%. Where this audit
> says either issue was unresolved, the final
> [`../../README.md`](../../README.md) and
> [`../../report/artifact.json`](../../report/artifact.json) govern.

## Provenance and evidence labels

- Yune source audited: afb7079b71f7f9353845114ff3e310c0a38b9b87,
  observed clean and detached at audit time.
- librime source audited: 33e78140250125871856cdc5b42ddc6a5fcd3cd4,
  observed clean and detached at audit time.
- Measurements: accepted-baseline/run-1 through accepted-baseline/run-5 under
  /Users/laufei/yune-m59-post-fix-root-cause-20260711.
- The counter table contains medians over 400 complete-input samples: 80 key
  iterations in each of five accepted runs. Deterministic count columns were
  identical across the samples inspected.

Labels used below:

- **Verified:** supported by the accepted measurements and the cited source.
- **Observed:** directly visible in captured candidate output.
- **Source-verified:** the implementation structure is established, but its
  causal performance magnitude was not measured.
- **Hypothesis:** plausible, but requires a controlled measurement before it
  may be presented as a cause.

## Answer first

The raw post-fix aggregate is sharply split by input class:

| Input | Median Yune/librime ratio | Result |
| --- | ---: | --- |
| n | 4.123 | Yune is about 4.1 times slower |
| zh | 3.261 | Yune is about 3.3 times slower |
| 37-character input | 0.399 | Yune is about 2.5 times faster |
| 59-character input | 0.205 | Yune is about 4.9 times faster |

A ratio below 1 means the aggregate is lower for Yune. Later prefix-stratified
evidence shows the long aggregates are not behavior-normalized, so they do not
establish a large implementation-speed advantage. Short prefixes, especially
n and zh, remain a separate work-volume problem.

The strongest source-backed structural distinction is this:

- Yune selects and eagerly materializes a bounded surplus of candidates on
  every key, runs eager vector filters, and stores the resulting owned
  candidates before the ABI asks for a five-item page.
- librime builds its query machinery but represents translations, merged
  streams, filters, and the menu as lazy generators. GetContext advances those
  streams only far enough to construct the requested five-item page, plus any
  extra items required to pass filtering or deduplication.

This is a credible explanation for disproportionate short-prefix overhead, but
source shape alone does not quantify its causal contribution. A separate
instrumented control is still required. It also is not evidence of a generic
macOS defect: the same accepted Mac evidence shows Yune winning strongly on
long inputs.

## Counter-backed four-row profile

The full fields are in counter-summary.csv. Times below are per-complete-input
sample medians in nanoseconds. Nested timers overlap and must not be added as an
exact partition.

| Measure | n | zh | 37-char | 59-char |
| --- | ---: | ---: | ---: | ---: |
| Key operations | 1 | 2 | 37 | 59 |
| Total process-key time | 89,667 | 275,250 | 2,440,312 | 4,673,897 |
| Translator calls | 3 | 6 | 111 | 177 |
| Translator time | 88,167 | 270,521 | 2,332,874 | 4,501,166 |
| Bounded requested/selected | 7 | 40 | 740 | 1,180 |
| ABI candidates exported | 5 | 10 | 185 | 295 |
| Lookup views visited | 7 | 41 | 820 | 1,297 |
| Owned candidates materialized | 7 | 40 | 776 | 1,236 |
| Sentence-model calls | 2 | 4 | 34 | 59 |
| Sentence-model time | 28,542 | 136,541 | 245,334 | 423,208 |
| Exact lookup time | 292 | 833 | 631,752 | 1,227,000 |
| Prefix lookup time | 19,083 | 31,584 | 238,396 | 562,811 |
| Context export time | 875 | 1,792 | 28,085 | 43,837 |

Verified implications:

- Translator time accounts for about 98.3%, 98.3%, 95.6%, and 96.3% of
  process-key time for n, zh, 37, and 59 respectively. The dominant opportunity
  is inside translation, not the surrounding process loop.
- Context export is only about 1.0%, 0.7%, 1.2%, and 0.9% respectively. ABI
  page copying is real work, but it is not the present first-order cause.
- Sentence-model time is about 31.8% for n and 49.6% for zh, but only about
  10.1% and 9.1% for the long inputs.
- Main exact plus prefix lookup is about 21.6% for n, 11.8% for zh, 35.7% for
  37, and 38.4% for 59. The remaining translator time is not cleanly
  partitioned by current counters.
- Yune materializes seven candidates to export five for n. For each generic
  Luna key, including both keys of zh and every key of the long rows, it selects
  up to twenty candidates to export five. This comes from a page size of five
  plus a surplus of two for the special short-key set and fifteen otherwise.
- The sentence scratch reports 28 incremental reuse hits for the 37-character
  input and 54 for the 59-character input. This is consistent with the low
  aggregate long-input ratios, but does not resolve the later-observed prefix
  behavior mismatch.

The counter named graph_rebuild_calls is broader than its name suggests. Its
instrumentation records sentence-graph update events, including incremental
extensions. Thus 34 and 59 events for the long inputs are not evidence that
Yune fully rebuilt the graph on every key. Explicit reuse counters and discarded
character counters must be read with it.

Release-build DP volume and candidate-state counters are compiled behind
debug-assertion guards and therefore remain zero in this evidence. Those zeros
do not prove that the DP did no work.

## Current Yune Luna path

### Per-key orchestration

**Source-verified.** A printable character is appended and refresh is invoked in
crates/yune-core/src/engine.rs:536-548. Refresh chooses a bounded request in
engine.rs:1392-1474. The constants in engine.rs:50-53 establish a five-item
page, general surplus fifteen, and short-key surplus two. The short-key set is
h, ha, hao, n, and ni in translator/mod.rs:6780-6782; zh is not special-cased.

Every installed translator is called, its candidate batch is extended into an
owned vector, the vector is sorted and filtered, and the resulting candidates
are stored in context at engine.rs:1492-1661. The uniquifier eagerly drains and
deduplicates that vector at filter/mod.rs:13-52.

The deployed Luna schema lists punctuation, an empty custom table, reverse
lookup, and the script translator. The empty table is rejected as having no
usable dictionary path by schema_install.rs:1969-1981 and 424-430. The observed
three translator calls per key are therefore consistent with punctuation,
reverse lookup, and script. Punctuation and reverse lookup are tag-gated, so an
invocation does not imply that each performed substantial work. Present
counters do not time the translators separately.

### Bounded script translation

**Source-verified and counter-backed.** UpstreamScript declares bounded support
at translator/mod.rs:3365-3383. It builds exact and prefix iterators, retains a
bounded set of references, orders them, and then eagerly creates owned
Candidate values at translator/mod.rs:3780-4177. The sentence result is capped
to min(request, 5) at translator/mod.rs:4315-4377. Candidate owns text, comment,
and preedit Strings in state.rs:1-8, while the context holds a Vec of owned
candidates in state.rs:273-282.

The main table iterators are themselves lazy. QueryTable exposes view-based
iteration in dictionary/query_table.rs:8-77, and the mmap-backed rsmarisa exact
and prefix iterators live at dictionary/compiled_table.rs:1688-1767. The eager
step occurs when the bounded request is converted into owned candidate objects
and then passed through vector filters.

The accepted product path is rsmarisa_byte_backed with table and prism mmap
enabled, no source fallback, and 498,564 stored entries. The memory-owner report
shows zero rows and bytes in both the leading fetch seed and leading fetch
index. Current Track A therefore uses the checksum-current prism/direct route,
not the heap leading-index fallback.

### Sentence graph and incremental reuse

**Source-verified.** The sentence model keeps scratch state containing prior
input, graph states, paths, phrases, prefix states, and exact spans at
poet/mod.rs:1190-1221. It either extends that scratch or rebuilds it at
poet/mod.rs:1711-1748. Full construction is at poet/mod.rs:2039-2080;
incremental extension and result extraction are at poet/mod.rs:2754-3121.

The untoned Luna path does not enter the separate toned direct
surface/phrase-family branch guarded by untoned_dictionary at
translator/mod.rs:2477-2489. It would be incorrect to attribute current Luna
cost to that toned-only branch.

Luna does use leading-syllable reachability. The bounded leading injection is
at translator/mod.rs:4267-4313 and the longest-first family scan is at
translator/mod.rs:5587-5758. That scan first probes for a matching single and
then scans again to materialize the family. Those calls are outside the
record_exact_lookup timer at translator/mod.rs:916-939. Their cost is therefore
part of the currently unpartitioned translator residual.

### Context export

**Source-verified and counter-backed.** Yune clones the current page in
engine.rs:1133-1162. The ABI takes the snapshot and allocates preedit, candidate,
and comment strings in context_api.rs:90-300. Only the current page is exported.
The low context-export shares above make this a lower-priority optimization
than translation.

## Pinned librime path

**Source-verified.** librime runs its processor chain and recomposes on context
updates at src/rime/engine.cc:99-168. It invokes configured translators and adds
their Translation objects and filters to the Menu at engine.cc:203-230.

The important difference is evaluation strategy:

- Translation is generator-shaped, with Next and Peek, at
  src/rime/translation.h:17-39.
- Menu prepares only the requested page and CreatePage(5, 0) requests five
  visible items at src/rime/menu.cc:15-57.
- MergedTranslation chooses its next item lazily at
  src/rime/translation.cc:101-159.
- Uniquifier advances its underlying translation lazily only as needed to skip
  duplicates at src/rime/gear/uniquifier.cc:14-70.
- ScriptTranslator creates a new ScriptTranslation per query at
  src/rime/gear/script_translator.cc:207-235. Candidate enumeration then occurs
  through lazy Next/Peek paths at script_translator.cc:512-673.
- Table::Query constructs accessors from the syllable graph at
  src/rime/dict/table.cc:571-629, while dictionary chunks materialize and
  partial-sort the next DictEntry lazily at
  src/rime/dict/dictionary.cc:127-296.
- GetContext creates the first page and copies only its candidates at
  src/rime_api_impl.h:209-306.

librime is therefore not “lazy everywhere”: it still creates a composition,
syllable graph, collectors, and accessors. Its page consumption, merged
translation, uniquifier, and dictionary-entry enumeration are lazy. Yune's
sentence scratch has a cross-prefix reuse capability that librime's newly
constructed ScriptTranslation does not expose in the same form. That distinction
fits the observed class split: librime avoids some surplus work on tiny pages,
while Yune amortizes and reuses sentence work on long inputs.

## Candidate behavior is not uniformly equivalent

**Observed.** The accepted candidate snapshots show:

- The 37-character and 59-character first-page candidate texts and order match
  exactly between Yune and librime after the page-order fix.
- Their preedit formatting still differs: librime reports spaced syllables;
  Yune reports raw unspaced input.
- For n, only the first candidate matches. librime reports
  你, 那, 呢, 能, 年 with blank comments; Yune reports
  你, 那, 拿, 哪, 納 with na comments on candidates two through five.
- For zh, only the first candidate matches. librime reports
  中, 這, 之後, 最後, 着; Yune reports 中, 炸, 渣, 扎, 紮,
  with zha comments after the first candidate.

This matters to performance work: a page-driven or short-prefix optimization
must first lock the intended behavior for n and zh against the governing oracle.
It must not silently optimize a page whose ordering or comments are already
different.

## What this audit does and does not establish

Established:

1. The post-fix Mac result is row-class-specific, not a blanket lack of Yune
   advantage.
2. Final long-input page text/order parity is present, and the aggregate ratio
   is below 1; later prefix-stratified evidence prevents treating that as a
   behavior-normalized speed claim.
3. Short prefixes remain substantially slower.
4. Yune eagerly owns and filters a surplus candidate batch; librime consumes a
   lazy page pipeline.
5. Sentence work is a much larger fraction of n/zh than of long-input time.
6. Context export is small, while a large portion of translator time remains
   unattributed by current phase counters.

Not established:

1. The exact number of nanoseconds caused by eager materialization versus any
   other translator phase.
2. A generic Apple allocator, compiler, CPU, or macOS scheduling defect.
3. This audit did not establish the cost of Yune metrics instrumentation. The
   later paired control measures it at 1.0-2.9%. The benchmark enables Yune's M37
   counters inside the timed key sample at
   crates/yune-rime-api/benches/native_inprocess_benchmark.rs:213-221 and
   597-620, while librime has
   no equivalent symbols. The metrics implementation performs enabled checks,
   timers, and atomic updates in m37_metrics.rs:337-350 and 950-997. This is a
   real measurement asymmetry; the later paired result shows it is not the
   multi-fold cause.
4. That a DP rewrite is the primary answer. The model is only about 9-10% of
   the measured long-input time, and incremental reuse is already active.

## Diagnostic conclusion

No evidence here supports calling the post-fix state a single platform bug.
The strongest current interpretation is an engine-path workload split:

- Long Luna inputs have low aggregate ratios after reconciliation, but the
  later behavior strata show that almost every Yune-faster prefix emits
  different candidate text. No behavior-normalized advantage follows.
- Tiny prefixes expose fixed translation/model setup and eager surplus
  materialization costs that librime's lazy first-page machinery can avoid or
  defer.
- Yune-side instrumentation overhead is subsequently quantified at 1.0-2.9%
  and is not the primary gap.
- Candidate differences on n and zh prevent treating a raw performance rewrite
  as behavior-neutral until oracle expectations are resolved.

That conclusion is diagnostic, not an authorization to change implementation or
thresholds.
