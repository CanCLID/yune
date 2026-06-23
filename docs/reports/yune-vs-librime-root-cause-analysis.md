# Why Yune is slower than librime — root-cause analysis

Date: 2026-06-22

Companion to the measurement report: [`yune-vs-librime-performance.md`](./yune-vs-librime-performance.md)
(2026-06-23 benchmark). That report establishes *that* Yune is slower on the
shared upstream `luna_pinyin` C-ABI surface (startup `94.6×`, session `115.1×`,
per-key `23–313×`, resident memory `208.6 MiB` vs `0.9 MiB`). This document
explains *why*, from the source, and states honestly whether the architecture is
worse than librime's. The concrete remediation is
[`docs/plans/m33-plan-engine-native-lookup-performance.md`](../plans/m33-plan-engine-native-lookup-performance.md).

## TL;DR

It is **not** "Rust is slow" and it is **not** that a modern architecture cannot
beat librime. On this workload the current implementation made the **opposite bet
from librime on the three things that dominate cost**:

1. **No memory-mapping.** Yune `fs::read`s the whole `.table.bin` / `.prism.bin`
   / `.reverse.bin` into the heap and parses them into owned structures. librime
   `mmap`s those files and walks them in place.
2. **Eager spelling-algebra expansion into RAM.** At schema-load time Yune
   materializes every dictionary entry into a `Candidate` and expands the full
   fuzzy/abbreviation cross-product into an in-memory
   `BTreeMap<String, Vec<Candidate>>`. librime keeps the algebra in the prism and
   applies it **lazily during the trie walk** at query time, never materializing
   the cross-product.
3. **No build-once schema cache.** The entire load-materialize-expand build is
   redone on **every `RimeSelectSchema`**, which is why "session create/select"
   (`2.761 s`) is essentially as slow as cold startup (`2.722 s`).

All three are implementation/representation choices, not properties of Rust or of
"AI-native architecture." They are fixable, but fixing #2 is a real lookup-path
change, not a tweak (see *Reconciliation with M30* below).

## What librime's speed actually comes from

librime is fast on `luna_pinyin` for specific, well-understood reasons, and any
honest comparison has to name them:

- **Deploy-once, mmap-at-runtime.** The expensive work (compiling the source
  `.dict.yaml` into a prism double-array + a packed table) happens once at deploy
  time. At runtime librime `mmap`s `luna_pinyin.prism.bin` and
  `luna_pinyin.table.bin` and reads them in place. Startup ≈ `mmap()` + read a
  header; resident growth ≈ the pages actually touched (lazy page-in). This is
  why its startup delta is `0.9 MiB` and `~29 ms`. (Confirmed in the local clone at `C:\Users\laubonghaudoi\Documents\GitHub\librime`: `src/rime/dict/mapped_file.{cc,h}`, used by `table.cc` and `prism.cc`.)
- **Spelling algebra lives in the prism, applied lazily.** Fuzzy rules
  (`zh`↔`z`, etc.) and abbreviations are encoded into the prism's spelling map at
  build time. During a query librime walks the double-array trie and generates
  alternative spellings *on the path it is already taking* — it never builds the
  expanded entry set in memory.
- **Schema built once, sessions are cheap.** A `Schema`/`Dictionary` is
  constructed once and held process-wide; selecting a schema for a session is a
  near-free pointer assignment.

This is close to the floor for a classic table IME: `mmap` + a trie descent in
tens of microseconds. There is not much left to take from it on raw classic-path
latency.

## What Yune does today — with source evidence

### 1. Whole-file read + heap parse, never mmap

A search for `mmap`/`memmap` across `crates/` returns **zero hits**. The compiled
loader reads each artifact fully into a `Vec<u8>` and parses it into an owned
`TableDictionary`:

- `crates/yune-rime-api/src/schema_install.rs` — `load_schema_compiled_dictionary`
  does `fs::read(table_path)` (≈ line 893), `fs::read` for prism and reverse
  (≈ lines 896–908), then `parse_rime_table_bin_dictionary(&table_bytes)`
  (≈ line 941). The compiled prism is parsed via `parse_rime_prism_bin_payload`
  and merged as *advanced data* — it is **not** used as the live lookup trie.
- Per the measurement report's own caveat, for some upstream compiled sections
  Yune cannot consume the binary and falls back to parsing source
  `.dict.yaml` (`schema_install.rs` ≈ lines 840–847). That path is *even more*
  expensive (full YAML parse of a multi-MB dictionary).

Either way the model is "read whole file → parse into owned heap structures,"
which is the opposite of mmap-and-walk.

### 2. Eager materialization + spelling-algebra expansion (the allocation hog)

The live lookup structure is a fully materialized, fully expanded heap map:

- `crates/yune-core/src/translator/mod.rs` — `StaticTableTranslator` stores
  `entries_by_code: BTreeMap<String, Vec<Candidate>>` (struct ≈ lines 106–138).
  `from_dictionary` (≈ lines 198–215) turns every dictionary row into an owned
  `Candidate`.
- `with_spelling_algebra` (≈ lines 427–448) calls
  `algebra.expand_entries_with_normal_codes(...)` and writes the **expanded**
  set into `entries_by_code`. Fuzzy + abbreviation rules multiply the entry
  count, so the whole cross-product is resident before the first keystroke.
- The build is explicitly instrumented as the owner: `schema_install.rs` wraps
  these steps in `startup_trace::span("translator_index_build")` (≈ line 292) and
  `startup_trace::span("spelling_algebra_expand")` (≈ line 313). The repo's own
  benchmark history (M27 cached expanded variants; M29 trimmed a no-op
  allocation; M30 removed a duplicate copy) all targets this exact span.

This is what the memory note *"startup is allocation-bound, not compute-bound"*
refers to: the seconds are spent allocating and populating this expanded map, not
doing clever work.

### 3. The build is redone on every schema select

`RimeSelectSchema` unconditionally rebuilds the translator chain — there is no
cache keyed by schema id or asset checksum:

- `crates/yune-rime-api/src/schema_selection.rs` ≈ line 136 calls
  `install_schema_translator_chain(session, schema_id)` inside a
  `startup_trace::span("translator_install")` on every select.
- `install_schema_translator_chain`
  (`crates/yune-rime-api/src/schema_install.rs:23`) re-runs
  `load_schema_table_dictionary` → `from_dictionary` → `with_spelling_algebra`
  each time, then `session.engine.add_translator(translator)`.

This is the direct explanation for the most damning benchmark row: **session
create/select/destroy (`2.761 s`) ≈ full cold startup (`2.722 s`)**. Selecting the
schema pays the entire load-materialize-expand cost again.

### 4. A measurement-fairness factor: eager reverse-lookup load

`luna_pinyin.schema.yaml` configures a reverse lookup against the `stroke`
dictionary (`reverse_lookup: { dictionary: stroke }`, plus
`reverse_lookup_translator`). The two engines treat that config very differently
in the timed window:

- **Yune eager-loads `stroke` at schema-select** —
  `crates/yune-rime-api/src/schema_install.rs:365–403`
  (`install_schema_reverse_lookup_translator_from_config`) reads
  `stroke.{table,prism,reverse}.bin` (≈ 9.5 MB) into the heap, on top of
  luna_pinyin (≈ 13.3 MB).
- **librime lazy-loads the reverse dictionary on first use** — local clone
  `src/rime/gear/reverse_lookup_translator.cc:147`
  (`if (!initialized_) Initialize(); // load reverse dict at first use`). The
  `ni`/`hao`/`zhongguo` workloads never trigger reverse lookup, so librime loads
  **zero** `stroke` bytes while timed.

So ≈ 9.5 MB of the ≈ 22.8 MB compiled payload Yune processes at startup is
`stroke`, which librime skips entirely here — inflating Yune's startup, session,
and resident-memory rows relative to a strict like-for-like load. This does **not**
flip the result (luna_pinyin alone keeps Yune far behind), but it over-states the
gap. The M33 plan treats it as a fairness gate (`M33-PERF-08`): equalize
reverse-load before publishing a corrected comparison, and make Yune lazy-load
reverse dictionaries to match librime.

## Mapping mechanisms to the benchmark rows

| Benchmark row | Dominant cause |
|---|---|
| Startup `2.722 s` vs `28.8 ms` | §1 whole-file parse + §2 eager expansion (no mmap, no lazy algebra) |
| Session create/select `2.761 s` vs `24.0 ms` | §3 rebuild-per-select redoes §1+§2 every time |
| Resident delta `208.6 MiB` vs `0.9 MiB` | §2 the expanded map is fully resident; §1 parsed copies vs librime's lazy page-in |
| Per-key `ni` `193×`, `hao` `313×` | heap-map lookup + per-query `expanded_lookup_specs` work vs an mmap trie descent; Yune's fixed per-key overhead dominates short inputs |
| Per-key `zhongguo` `23×` | gap shrinks only because librime's own candidate/lattice work grows on the longer input; Yune's sentence-DP (which still clones path vectors) also grows |

> The startup, session, and resident-memory rows are *additionally* inflated by Yune eager-loading the `stroke` reverse-lookup dictionary (≈ 9.5 MB) that librime never loads during these workloads (§4). The per-key rows are not affected by this.

## Reconciliation with the prior performance milestones

This analysis is consistent with — not a contradiction of — M26–M30:

- **M27** reduced the `spelling_algebra_expand` owner by caching expanded
  variants; **M29** removed a no-op allocation; **M30** removed a *duplicate*
  steady-state copy of the expanded entries (`1,103,331,328` → `839,217,152`
  bytes single-startup). Each was a real win *inside the materialize-and-expand
  model* — none of them changed the model itself.
- The memory note and M30 closeout call **lazy expansion "infeasible."** Read
  precisely, that means infeasible **within the current lookup architecture**,
  because every lookup path assumes a pre-expanded `entries_by_code` map.
  Eliminating the eager expansion (§2) therefore requires reworking lookup to walk
  the prism/double-array, which is exactly Lever 2 of the remediation plan — an
  architectural change, deliberately gated behind a feasibility spike.
- The components needed already exist: M18 added a pure-Rust Darts double-array
  (`crates/yune-core/src/dictionary/double_array.rs`) and parses real upstream
  prism sections. They are currently used for validation/advanced-data merging,
  not as the live query structure.

## Verdict: is our architecture worse than librime's?

Honestly, on this classic `luna_pinyin` surface: **yes — the current data path is
measurably worse**, for the three concrete reasons above. But the cause is *unbuilt
optimizations*, not Rust and not the AI-native design. Yune skipped mmap, lazy
spelling algebra, and build-once caching; librime's entire speed budget comes from
those three.

Two things follow, and both should be said plainly:

- **Matching librime is achievable** (the design is known and the parser/double-
  array pieces are already in-tree), but expect "same order of magnitude," not a
  blowout win — librime is near the floor for a classic table IME.
- **Beating librime meaningfully on raw classic-path latency is unlikely**, and
  was never the real differentiation. The product thesis (per the roadmap) is the
  **AI-native layer librime cannot host**. The right engine goal is to **stop
  losing embarrassingly on the classic path** so the AI layer — not pinyin lookup
  speed — is what stands out.

## What closing the gap takes

In rough order of impact (full detail and acceptance gates in
[`m33-plan-engine-native-lookup-performance.md`](../plans/m33-plan-engine-native-lookup-performance.md)):

1. **Build-once schema cache** — stop rebuilding on every select. Cheapest, large
   win for the session/startup-on-reselect rows.
2. **Lazy spelling algebra at query time** over the prism/double-array, instead of
   pre-expanding into RAM — removes most of the `2.7 s` and the `208 MiB`.
   Architectural; spike-gated because M30 flagged it infeasible in the current
   lookup design.
3. **mmap the compiled table/prism** (zero-copy) instead of `fs::read` +
   parse-to-heap — further cuts resident memory and load time; pays off fully only
   once lookup walks the prism (Lever 2).
