# M55 Phase 2 Poet Storage Design

Date: 2026-07-03

Status: design checkpoint; implementation not started in this commit.

## Owner

Phase 2 owns the two largest confirmed byte-backable Track A Luna sentence-model owners from `phase-1-attribution/owner-budget.csv`:

- `poet.vocabulary`: `53,644,752 B`
- `poet.entries_by_code`: `18,694,662 B`

Goal: move both from retained heap rows to mmap-backed compiled storage while preserving candidate bytes and keeping the full M55 ratchet green.

## Artifact

Add a Yune-native compiled artifact named `<dictionary>.poet.bin` beside the existing `<dictionary>.table.bin` and `<schema>.prism.bin` artifacts.

Version tag:

```text
YUNE-POET/1\0
```

Header fields, little-endian:

- magic/version tag
- source dictionary checksum
- section directory offset/count
- entries section offset/count
- entry text pool offset/byte length
- entry code pool offset/byte length
- entry code range table offset/count
- normal vocabulary section offset/count
- normal vocabulary first-code index offset/count
- abbreviation vocabulary section offset/count
- abbreviation vocabulary first-code index offset/count
- character-code table offset/count
- abbreviation character-code table offset/count

The artifact is offset-served. Runtime readers should borrow strings from the mapped byte source by `(start, end)` ranges and read entry/vocabulary rows by fixed-width descriptors. The in-memory model may keep small structural indexes where they are already guarded (`poet.lookup_index`) but must not clone `poet.vocabulary` or `poet.entries_by_code` payloads into retained heap.

## Data Layout

Entries:

- sort order must match the current `compare_model_entry_by_code` order exactly;
- each row stores `text_range`, `code_id`, and `weight`;
- text bytes use the same concatenated UTF-8 pool as current `ModelStringPool`;
- code bytes plus code ranges use the current deduplicated sorted-by-entry code pool.

Vocabulary:

- rows store `text_range`, `chars_range`, and `weight`;
- `chars_range` points into a UTF-32LE character pool or equivalent fixed-width codepoint pool;
- first-code indexes store `(code_range, vocabulary_row_index)` and must be sorted/deduped identically to `build_model_vocabulary_index`;
- normal and abbreviation vocabularies are distinct sections because TypeDuck and abbreviation lanes may diverge.

Character-code tables:

- store `(char, code_range_list)` for normal and abbreviation character code maps;
- code lists must be sorted and deduped exactly as the current constructor does.

## Crate Split

`yune-core`:

- define a `PoetByteSource` trait analogous to `CompactTableByteSource`;
- parse and validate `YUNE-POET/1` bytes;
- expose a byte-backed `UpstreamSentenceModel` storage variant that serves `entries_by_code`, vocabulary rows, and code maps by offsets;
- keep all parsing and lookup code safe Rust; no `unsafe` in `yune-core`;
- emit `memory_owner_rows()` for `poet.entries_by_code` and `poet.vocabulary` as `mmap_file_backed` when the source mapping mode is `mmap`, with heap remnants recorded separately and expected below `1 MB` combined.

`yune-rime-api`:

- mmap `<dictionary>.poet.bin` on native using `memmap2`, mirroring `MappedCompiledTableBytes`;
- use owned bytes on WASM/fallback paths, with no WASM memory claim;
- pass the byte source into schema install when constructing the upstream sentence model;
- write/rebuild the artifact during deploy/rebuild next to table/prism artifacts.

## Stale And Corrupt Artifact Rejection

The parser must reject:

- missing or unsupported magic/version;
- source dictionary checksum mismatch;
- truncated header or section directory;
- out-of-bounds string/row ranges;
- invalid UTF-8 string ranges;
- invalid codepoint ranges;
- row counts exceeding explicit sanity limits.

Rejection must be loud. If source dictionary inputs are available during deploy, the deploy/rebuild path may regenerate `<dictionary>.poet.bin`; if runtime selection sees a stale/corrupt artifact without source data, it must record a compiled-rejection reason and fall back through the existing source-load path only as an explicit degraded path visible in diagnostics. Add tests for wrong version and truncated artifact.

## Track A Benchmark Flow

The Track A benchmark run root is copied from the upstream librime oracle build, so it will not contain Yune-native `poet.bin` files by default. Extend `scripts/benchmark-native-rime-inprocess.ps1` Track A prep after `Prepare-UpstreamRun "track-a-yune"`:

1. run an untimed Yune deploy/compile step against the Track A run root using the Yune DLL;
2. ensure `<run-root>\user\build\luna_pinyin.poet.bin` exists;
3. record its path, byte size, checksum, and modified time in the run evidence;
4. assert the artifact modified time is after the current Yune binary build time before trusting memory numbers.

Timed benchmark phases remain unchanged.

## Verification

Phase 2 close requires:

- stale/corrupt artifact tests;
- `upstream_luna_pinyin_parity`;
- `cantonese_parity`;
- product-path CLI before/after candidate byte comparison for the 37-char row, 59-char row, and two fixture sentences;
- `memory-owner-profile.csv` showing `poet.vocabulary` and `poet.entries_by_code` as mmap-backed with heap remnants under `1 MB` combined;
- two consecutive full M55 ratchet runs green before tightening the memory ceiling.

## Open Risk

Byte-backed access can add per-lookup latency. The inherited 59-character row was `2.447x` in Phase 1, exactly at its ceiling, so Phase 2 implementation must measure early against that row and stop as no-go if the mmap path cannot keep the committed ceiling without retained heap caches.
