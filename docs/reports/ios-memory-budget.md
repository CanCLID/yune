# iOS-Budget Native Memory Report

Date: 2026-06-28

This report measures Yune's **native single-active-schema memory** against an iOS
keyboard-extension budget (hard **64 MB**, target **48 MB**). It is a different
lens from
[`yune-vs-librime-performance.md`](./yune-vs-librime-performance.md) (which tracks
librime *parity*): here the bar is a **product memory budget**, because the
intended product is a Cantoboard-style iOS keyboard
([Cantoboard](https://github.com/Cantoboard/Cantoboard),
[librime-ios-build](https://github.com/Cantoboard/librime-ios-build)). The
reduction work it motivates is tracked by
[`plans/active/m47-plan-ios-budget-native-memory-reduction.md`](../plans/active/m47-plan-ios-budget-native-memory-reduction.md).

Findings were adversarially verified (2026-06-28 workflow: 4 refute agents +
synthesis); the calibration corrections below are folded in.

## Harness hygiene (read first)

**Every memory number must name its harness.** Three different processes give
three different numbers; only one is the iOS proxy:

| Source | jyut6ping3_mobile | What it is |
| --- | ---: | --- |
| Initial lean native probe | **~298 MB steady / 482 MB peak** | Original M47 baseline: real `RimeApi`, mmap path, prebuilt assets, one schema per fresh Rust process |
| Current lean native probe after RED-03 | **69.4 MB steady / 80.8 MB peak** | Current Windows proxy after RED-01 side lookup opt-out, RED-02 prism byte-backed runtime storage, and RED-03 compact lookup-record opt-out |
| .NET in-process benchmark (Track B) | ~436 MB after-ready / 504 MB peak | A .NET process hosting **both** Yune *and* librime DLLs — polluted upper bound, **not** the iOS number |
| Browser WASM | 160 MiB | WASM linear memory, **no mmap**, owned tables — a different deployment |

The earlier "Jyutping native ~504 MB" headline was the dual-DLL harness; the
iOS-relevant steady was initially **~298 MB** and is now **69.4 MB** after RED-03.

## Methodology

Probe: [`crates/yune-rime-api/tests/native_memory_probe.rs`](../../crates/yune-rime-api/tests/native_memory_probe.rs)
(evidence-only, `#[ignore]`). It drives the native `RimeApi`
(`setup`→`initialize`→`deploy`→`create_session`→`select_schema`→`process_key`)
with `prebuilt_data_dir` set to the committed `.bin` bundle, so `deploy()`
**reuses + mmaps** the compiled assets instead of rebuilding — exactly how an iOS
keyboard loads precompiled bundle assets. It reads this process's own
`WorkingSet64`/`PeakWorkingSet64` at each phase; one schema per fresh process
removes cross-schema contamination; the ~5 MB harness baseline is recorded and
subtracted. Verified faithful (C1, high confidence): the native load path mmaps
via `memmap2` (`schema_install.rs`), and the search order returns the deployed
staging copy.

## Measurement caveat (what this proxy does and does not say)

Windows `WorkingSet64` is **total resident physical memory**, including clean,
shared, file-backed (mmap'd) pages. iOS jetsam accounts a keyboard extension
roughly by **`phys_footprint` (dirty + compressed memory)**, which *excludes*
clean mmap'd pages that can be evicted and re-faulted. So the iOS number for the
same engine could be **lower** than this Windows working set — but, as GPT's
review put it, *a ~300 MB native session footprint does not become 48 MB by
platform accounting alone*. M47 Phase 0 now records Windows `PrivateUsage` and
allocator-live evidence for the dirty/private side of that gap; treat every
number here as a Windows proxy that justifies the work, not as an iOS
`phys_footprint`. The on-device figure is a later Phase 2 validation gate.

**Independent reproduction (GPT review, 2026-06-28, separate Windows machine):**
after_deploy `14.9`, after_session `297.5`, after_select/steady `302.3`, peak
`486.0` MB — same shape and magnitude as below (deploy cheap; `create_session`
is the owner; ~300 MB steady / ~486 MB peak).

## Findings

### 1. The cost is at `create_session`, not `deploy` (verified, high confidence)

jyut6ping3_mobile, lean probe, phase by phase:

| Phase | Working set |
| --- | ---: |
| baseline (harness) | 5.2 MB |
| after `setup`+`initialize` | 5.9 MB |
| **after `deploy()`** | **10.8 MB** |
| **after `create_session()`** | **293.4 MB** (+283 MB) |
| after `select_schema` | 298.3 MB |
| after typing | 298.4 MB |
| **steady** | **298.4 MB** |
| **peak** | **482.0 MB** |

`deploy()` (compile/reuse) is nearly free. The +283 MB lands at
`create_session()`, which eagerly loads the workspace **default** schema's
dictionary and materializes heap structures
(`apply_initial_schema_to_session` → `install_schema_translator_chain` →
`CompactTableStore::from_table_bin_byte_source` + `read_yune_table_advanced_payload`
+ `StaticTableTranslator::from_compact_table_store`, which collects
`all_codes()` into a `normal_codes` HashSet).

### 2. It scales with the default dictionary, not a fixed cost (verified)

By patching `default.yaml`'s `schema_list` to isolate the default schema that
`create_session` loads:

| Default/active schema | Steady WS | Peak WS |
| --- | ---: | ---: |
| luna_pinyin | **62.6 MB** | 294 MB |
| cangjie5 | **94.9 MB** | 102 MB |
| jyut6ping3_mobile | **~298 MB** | 482 MB |

The earlier "~constant ~300 MB across schemas" was an artifact of `create_session`
always loading the default `jyut6ping3` before any switch. Only jyut shows a large
steady↔peak gap (a ~184 MB transient that frees back); luna's 294 MB peak frees to
62 MB steady.

### 3. What it is *not* (verified)

- **Not the sentence model.** A/B disabling `enable_sentence`+`enable_completion`
  for jyut moved steady 298→297 MB (≈ 0). `poet`/sentence-model owner rows are 0
  bytes for jyut (`with_upstream_sentence_model` is only used for luna_pinyin).
- **Not OpenCC.** All `.ocd2` simplifier dictionaries total **77 KB**.

### 4. Pre-Phase-0 classification left ~235 MB un-owned

Before M47 Phase 0, the owner profile
([`track-b-yune-product/memory-owner-profile.csv`](./evidence/current-performance-dashboard-2026-06-28/native-current-benchmark/track-b-yune-product/memory-owner-profile.csv)):

| Owner | Bytes | Class |
| --- | ---: | --- |
| `compact_table.lookup_records` (primary) | 31.9 MB | heap_owned_required |
| `compact_table.lookup_records` (secondary) | 8.1 MB | heap_owned_required (dictionary loaded twice) |
| `compact_table.syllabary_codes` | ~4.2 MB | heap_owned_reducible |
| `compact_table.storage` (table) | ~15 MB | **mmap_file_backed** (not private heap) |
| **Classified heap-required** | **~44 MB** | |

Against the lean probe's **298 MB** steady, classified heap (~44 MB) + mmap'd
file pages (~18 MB, which are clean/shared, not private dirty) left **~235 MB
un-owned**. That was the Phase 0 blocker, not the final attribution. The closeout
below adds private bytes, allocator live/high-water, filter owner rows, and
inside-load phase markers.

## Phase 0 attribution closeout (Windows-measured, 2026-06-28)

Evidence:
[`evidence/m47-ios-budget-native-memory-attribution-2026-06-28/`](./evidence/m47-ios-budget-native-memory-attribution-2026-06-28/)
from the extended native probe. This is Windows `WorkingSetSize` /
`PrivateUsage` evidence plus a test-local counting allocator, **not** iOS
`phys_footprint`.

Two runs matter:

| Run | Steady WS | Peak WS | Steady private | Allocator live | Allocator high-water | Named heap owners | Clean mmap estimate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Default-list baseline (`default.yaml` first schema, then select `jyut6ping3_mobile`) | **301.2 MB** | **485.1 MB** | **284.7 MB** | **191.5 MB** | **415.1 MB** | 152.7 MB after `create_session` | 14.5 MB after `create_session` |
| Isolated mobile default (`YUNE_MEM_DEFAULT=jyut6ping3_mobile`) | **223.9 MB** | **231.5 MB** | **202.4 MB** | **155.0 MB** | **164.8 MB** | **96.6 MB** | **19.0 MB** |

Phase 0 verdict: the steady-state bulk is **not primarily allocator-retained
free memory**. The isolated mobile run ends at `~202 MB` private bytes with
`~155 MB` allocator-live bytes and only `~165 MB` allocator high-water, so an
allocator decay strategy cannot be the first branch. The remaining unclassified
steady mass is live retained heap / process-private memory that needs owner
reduction.

The formerly "un-owned" bucket is now smaller and more concrete:

- Default-list `create_session`: `~285 MB` private, `~220 MB` allocator live,
  `~153 MB` named heap, `~15 MB` clean mmap estimate. Remaining unclassified
  private is roughly `~132 MB`; unclassified Rust-allocator live is roughly
  `~68 MB`.
- Isolated mobile: `~202 MB` private, `~155 MB` allocator live, `~97 MB` named
  heap, `~19 MB` clean mmap estimate. Remaining unclassified private is roughly
  `~106 MB`; unclassified Rust-allocator live is roughly `~58 MB`.

The double/side load is confirmed and owner-named:

| Owner | Bytes | Required for normal keyboard candidate text? | Verdict |
| --- | ---: | --- | --- |
| Primary `jyut6ping3` compact table `lookup_records` | 31.9 MB | No; rich lookup/comment records, not candidate generation | Lazy/skip candidate for keyboard-only path |
| Secondary `luna_pinyin_yune_reverse` compact table `lookup_records` | 13.8 MB | No for unprefixed Jyutping; used by grave-prefix Mandarin reverse lookup UI | Lazy-load on reverse trigger or optional UI pack |
| `dictionary_lookup_filter.lookup_records` (`jyut6ping3_scolar`) | 50.7 MB | No; dictionary-panel/comment enrichment filter | Highest-leverage lazy/optional branch |

This table selected RED-01 first and remains the Phase 0 pre-reduction ordering.
The post-RED-01 prism attribution correction below supersedes the next-branch
order: run prism storage/lazy parsing before reverse/UI lazy loading. Keep
candidate-output correctness gates green; this is not an allocator-strategy
branch.

## RED-01 reduction closeout (Windows-measured, 2026-06-28)

Evidence:
[`evidence/m47-ios-budget-native-memory-reduction-red01-2026-06-28/`](./evidence/m47-ios-budget-native-memory-reduction-red01-2026-06-28/)
from the same native probe. This is Windows `WorkingSetSize` / `PrivateUsage`
evidence, not iOS `phys_footprint`.

RED-01 adds an explicit keyboard-profile gate:
`dictionary_lookup_filter/load_lookup_records: false`. When set, schema install
skips loading and retaining `jyut6ping3_scolar` lookup records. The default is
still eager (`true`), so public/web and TypeDuck-rich-comment paths remain
unchanged unless a profile opts out.

| Metric | Before | After | Delta |
| --- | ---: | ---: | ---: |
| Steady WS | **223.9 MB** | **169.2 MB** | **-54.7 MB** |
| Steady private | **202.2 MB** | **147.7 MB** | **-54.5 MB** |
| Steady allocator live | **155.0 MB** | **104.9 MB** | **-50.1 MB** |
| Peak WS | **231.6 MB** | **217.3 MB** | **-14.3 MB** |
| Named heap owners | **96.6 MB** | **48.2 MB** | **-48.4 MB** |

The measured owner delta is direct: `dictionary_lookup_filter.lookup_records`
went from `50,695,595 B` / `127,144` records in the before owner profile to no
owner row in the after profile. Candidate correctness remains covered by the
committed Jyutping asset regression; dictionary-panel comments and TypeDuck
comment parity remain preserved on the default/eager path. For the keyboard
profile, dictionary-panel enrichment is intentionally disabled, not lazily
loaded on first request.

RED-01 is a meaningful reduction but not close to the target: after the branch,
isolated `jyut6ping3_mobile` is still **~3.5x** over the 48 MB steady target and
**~3.4x** over the 64 MB peak target. The named heap total in this RED-01 table
was later corrected by the prism attribution section below.

## Post-RED-01 prism attribution correction (Windows-measured, 2026-06-28)

Evidence:
[`evidence/m47-ios-budget-native-memory-prism-attribution-2026-06-28/`](./evidence/m47-ios-budget-native-memory-prism-attribution-2026-06-28/)
from the same native probe and RED-01 keyboard-profile opt-out. This is an
owner-table correction, not a reduction branch.

| Metric | RED-01 owner table | Corrected owner table | Delta |
| --- | ---: | ---: | ---: |
| Steady WS | **169.2 MB** | **169.1 MB** | noise only |
| Steady private | **147.7 MB** | **146.6 MB** | noise only |
| Steady allocator live | **104.9 MB** | **104.9 MB** | **0.0 MB** |
| Peak WS | **217.3 MB** | **217.2 MB** | noise only |
| Named heap owners | **48.2 MB** | **86.6 MB** | **+38.4 MB** |
| Named total owners | **70.3 MB** | **108.7 MB** | **+38.4 MB** |

The corrected owner rows name parsed prism payload heap retained by
`StaticTableTranslator`:

| Owner | Bytes | Verdict |
| --- | ---: | --- |
| `prism.spelling_map` | `31,337,240 B` | Largest newly named owner; parsed spelling descriptor vectors. |
| `prism.double_array_units` | `8,896,560 B` | Parsed Darts double-array units. |
| `prism.corrections_tolerance` | `96 B` | Empty correction/tolerance vector headers in this profile. |
| `prism.tips_payload` | `0 B` | No non-empty descriptor tips in this profile. |

The primary `jyut6ping3_mobile` prism accounts for `28,767,208 B` of spelling
map and `8,388,632 B` of double-array units; `luna_pinyin_yune_reverse` accounts
for `2,570,032 B` and `507,928 B`. `dictionary_lookup_filter.lookup_records`
remains absent, so RED-01's keyboard-profile skip is still active.

## RED-02 prism runtime storage closeout (Windows-measured, 2026-06-28)

Evidence:
[`evidence/m47-ios-budget-native-memory-reduction-prism-storage-2026-06-28/`](./evidence/m47-ios-budget-native-memory-reduction-prism-storage-2026-06-28/)
from the same native probe and RED-01 keyboard-profile opt-out. The `before/`
folder is the 682be75a prism-attribution baseline; `current/` is the RED-02
runtime parser result.

RED-02 keeps the public owned prism parser intact for fixture/facade tests, but
compiled native runtime loading now stores a `RimePrismRuntimePayload` that reads
the Darts double-array units and spelling-map descriptors from the existing
compiled prism byte source instead of expanding them into heap-owned `Vec<u32>`
and `Vec<Vec<RimePrismSpellingDescriptor>>` structures.

| Metric | Before | After | Delta |
| --- | ---: | ---: | ---: |
| Steady WS | **169.1 MB** | **138.1 MB** | **-31.0 MB** |
| Steady private | **146.6 MB** | **101.8 MB** | **-44.8 MB** |
| Steady allocator live | **104.9 MB** | **66.6 MB** | **-38.3 MB** |
| Peak WS | **217.2 MB** | **172.1 MB** | **-45.1 MB** |
| Named heap owners | **86.6 MB** | **48.2 MB** | **-38.4 MB** |

| Owner | Before | After | Verdict |
| --- | ---: | ---: | --- |
| `prism.double_array_units` | `8,896,560 B` heap | `8,896,512 B` mmap-backed | Moved to byte-backed runtime reads. |
| `prism.spelling_map` | `31,337,240 B` heap | `11,955,056 B` mmap-backed | Moved to lazy byte-backed descriptor reads; raw descriptor bytes are smaller than heap expansion. |
| `prism.corrections_tolerance` | `96 B` heap | `96 B` heap | Unchanged small parsed metadata. |
| `compact_table.lookup_records` | `45,689,298 B` heap | `45,689,298 B` heap | Largest remaining heap owner. |

RED-02 is a successful reduction but not a budget closeout: the isolated mobile
keyboard profile is still roughly **2.9x** over the 48 MB steady target and
**2.7x** over the 64 MB peak target on the Windows proxy harness. The next
measured blocker is compact lookup-record materialization, not prism storage.

## RED-03 compact lookup-record closeout (Windows-measured, 2026-06-28)

Evidence:
[`evidence/m47-ios-budget-native-memory-reduction-lookup-records-2026-06-28/`](./evidence/m47-ios-budget-native-memory-reduction-lookup-records-2026-06-28/)
from the same native probe, with RED-01 enabled and RED-03 applied through the
temporary deployed schema's `translator/load_lookup_records: false` and
`luna_pinyin/load_lookup_records: false` settings. The committed public schema
bundle is unchanged, and the parser default remains eager.

RED-03 keeps table assets and default rich lookup behavior intact, but compact
runtime parsing can now validate and walk the `YUNE-LOOKUP` advanced payload
without retaining `HashMap<String, Vec<DictionaryLookupRecord>>` when a
translator namespace opts out.

| Metric | Before | After | Delta |
| --- | ---: | ---: | ---: |
| Steady WS | **137.8 MB** | **69.4 MB** | **-68.4 MB** |
| Steady private | **102.8 MB** | **30.2 MB** | **-72.6 MB** |
| Steady allocator live | **66.6 MB** | **20.4 MB** | **-46.2 MB** |
| Peak WS | **172.1 MB** | **80.8 MB** | **-91.3 MB** |
| Named heap owners | **48.2 MB** | **4.6 MB** | **-43.6 MB** |

| Owner | Before | After | Verdict |
| --- | ---: | ---: | --- |
| Primary `jyut6ping3_mobile` `compact_table.lookup_records` | `31,920,140 B` / `127,144` records | `48 B` / `0` records | Skipped in keyboard-profile run. |
| Secondary `luna_pinyin_yune_reverse` `compact_table.lookup_records` | `13,769,158 B` / `70,807` records | `48 B` / `0` records | Skipped in keyboard-profile run. |
| `compact_table.lookup_records` total | `45,689,298 B` / `197,951` records | `96 B` / `0` records | RED-03 measured heap owner removed. |
| `compact_table.syllabary_codes` total | `4,850,892 B` | `4,850,892 B` | Top remaining named heap owner. |

Mmap/file-backed owner rows stayed flat at `40,740,505 B`, so this branch
removed live retained heap rather than clean file mappings. The steady Windows
working set is now roughly **1.45x** over the 48 MB target, and the peak is still
roughly **1.26x** over the 64 MB target.

## Verdict (Windows-measured)

Against the 64 MB hard budget, the original isolated `jyut6ping3_mobile` baseline
was **~4.7x over** the 48 MB steady target and **~3.6x over** the 64 MB peak
target. After RED-03, the keyboard-profile run is still **~1.45x over** steady and
**~1.26x over** peak. Cangjie5 remains ~1.5x over at steady; luna_pinyin is
borderline-under at steady but ~4.6x over at peak. Even a small-schema base is
~60-95 MB before profile-specific UI deferral. The multilingual Jyutping keyboard
**does not fit** in its current shape, and closing the gap remains
**architecture-level work, not tuning**. The portable levers (defer reverse/UI
payloads, bound transients, slim the mobile profile, then revisit remaining
named heap) are Windows-implementable and benefit every platform. **No
"iOS-ready" claim** until a later real-Apple-device validation pass.

## Next work

RED-01, RED-02, and RED-03 are complete. Start the next branch with reverse/UI
lazy loading: defer `script_translator@luna_pinyin` /
`luna_pinyin_yune_reverse` until the grave-prefix reverse lookup path is used,
or isolate it as an optional UI pack for keyboard-extension builds. Keep both
steady and peak visible; the current RED-03 peak is still **80.8 MB**. Do not use
allocator changes as the next branch unless a later allocator-specific probe
shows a steady live-vs-resident gap that Phase 0/RED-01/RED-02/RED-03 did not
show.

## Evidence

- Phased + per-schema working set:
  [`evidence/ios-memory-budget-2026-06-28/native-working-set-by-phase.csv`](./evidence/ios-memory-budget-2026-06-28/native-working-set-by-phase.csv)
- Owner profile (Track B):
  [`evidence/current-performance-dashboard-2026-06-28/native-current-benchmark/track-b-yune-product/memory-owner-profile.csv`](./evidence/current-performance-dashboard-2026-06-28/native-current-benchmark/track-b-yune-product/memory-owner-profile.csv)
- M47 Phase 0 attribution:
  [`evidence/m47-ios-budget-native-memory-attribution-2026-06-28/`](./evidence/m47-ios-budget-native-memory-attribution-2026-06-28/)
- M47 RED-01 reduction:
  [`evidence/m47-ios-budget-native-memory-reduction-red01-2026-06-28/`](./evidence/m47-ios-budget-native-memory-reduction-red01-2026-06-28/)
- M47 post-RED-01 prism attribution correction:
  [`evidence/m47-ios-budget-native-memory-prism-attribution-2026-06-28/`](./evidence/m47-ios-budget-native-memory-prism-attribution-2026-06-28/)
- M47 RED-02 prism runtime storage reduction:
  [`evidence/m47-ios-budget-native-memory-reduction-prism-storage-2026-06-28/`](./evidence/m47-ios-budget-native-memory-reduction-prism-storage-2026-06-28/)
- M47 RED-03 compact lookup-record reduction:
  [`evidence/m47-ios-budget-native-memory-reduction-lookup-records-2026-06-28/`](./evidence/m47-ios-budget-native-memory-reduction-lookup-records-2026-06-28/)
- Probe: [`crates/yune-rime-api/tests/native_memory_probe.rs`](../../crates/yune-rime-api/tests/native_memory_probe.rs)
