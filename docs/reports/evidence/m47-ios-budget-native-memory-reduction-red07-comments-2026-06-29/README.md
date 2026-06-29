# M47 RED-07 Comments-Intact Lookup Storage Evidence

Date: 2026-06-29

Harness: Windows native `RimeApi` probe using `PROCESS_MEMORY_COUNTERS_EX`
`WorkingSetSize` / `PrivateUsage`, plus the test-local counting allocator in
`crates/yune-rime-api/tests/native_memory_probe.rs`. These are Windows proxy
numbers, not iOS `phys_footprint`.

## Verdict

RED-07 confirms the rich TypeDuck dictionary/comment payload was the dominant
gap between the lean lower bound and a TypeDuck-like keyboard profile. The fix
keeps rich comments but stores lookup records as indexed byte-backed compiled
payloads instead of retained `HashMap<String, Vec<DictionaryLookupRecord>>`
owners.

| Profile | Before steady / peak | After steady / peak | Private after | Allocator live after | Comments |
| --- | ---: | ---: | ---: | ---: | --- |
| Lean lower bound | 56.7 / 61.8 MB | 56.9 / 61.3 MB | 23.3 MB | 16.0 MB | intentionally omitted |
| Comments-intact keyboard | 164.0 / 171.2 MB | 78.7 / 89.1 MB | 33.7 MB | 25.6 MB | retained |
| Full mobile | 194.0 / 201.3 MB | 91.4 / 102.4 MB | 41.6 MB | 32.3 MB | retained |

Lookup owner class moved from `heap_owned_required` to
`shared_or_overlapping` byte-backed payload rows:

- Comments-intact keyboard: `compact_table.lookup_records`
  `31,920,140 B` heap plus `dictionary_lookup_filter.lookup_records`
  `50,695,595 B` heap became `14,219,092 B` + `17,498,253 B`
  shared/overlapping byte-backed payload rows.
- Full mobile: primary `31,920,140 B`, secondary `13,769,158 B`, and filter
  `50,695,595 B` heap rows became `14,219,092 B`, `5,536,522 B`, and
  `17,498,253 B` shared/overlapping byte-backed payload rows.

The bulk was live retained lookup/comment heap, not allocator-retained free
memory. RED-07 removes that heap retention while preserving visible rich
comments. The next blocker is still steady resident size above the 48 MB target:
comments-intact keyboard is 78.7 MB and full mobile is 91.4 MB, with remaining
clean/shared mmap payload and compact index/code owners to reduce or make
product-optional.

## Files

- `profile-summary.csv` / `profile-summary.json` - profile before/after table.
- `lean-lower-bound*/` - lower-bound probe rows with lookup/comment payloads
  disabled.
- `comments-intact-keyboard-before/` and `comments-intact-keyboard-after/` -
  rich comments retained, grave-prefix reverse UI omitted.
- `full-mobile-before/` and `full-mobile-after/` - no opt-outs.
- `rich-comments/rich-comment-zouhapci.json` - representative visible
  candidate comments proving TypeDuck dictionary-panel bytes survive RED-07.

Each probe folder contains `phase-memory.csv/json`, `create-session-events.*`,
`owner-attribution.*`, and `summary.json`.

## TypeDuck-iOS Reference

Reference checkout: `TypeDuck-HK/TypeDuck-iOS` at
`094abb53cf6d6254b997de90d1a96239088d3f7b`; schema submodule
`TypeDuck-HK/schema` at `dde24bc8dfd6c3dd836af0f7e57a12d7652631a9`.

- RIME candidate comments carry the dictionary payload:
  `CantoboardFramework/RimeKit/RKRimeSession.mm:99-123` copies candidate text
  and comment; `CandidateInfo.swift:21-35` parses comments into entries;
  `CandidateEntry.swift:38-42` defines multilingual dictionary fields.
- Swift mostly parses visible/opened candidate comments:
  `CandidatePaneView.swift:738-751`, `:822-834`, and `:791-809`. Caveat:
  grouping modes can bulk-parse loaded candidates.
- English/Unihan/ngram support stores are separate from RIME candidate comments:
  English and Unihan use `LevelDbTable`; ngram is a separate mmap-backed
  `.ngram` format.
- Asset-size reference: TypeDuck-iOS comparable six RIME bins total
  `16,646,044 B`; current Yune comparable six bins total `50,674,920 B`; Yune's
  WEB03 15-file compiled set totals `70,830,098 B`; TypeDuck-iOS support stores
  total `13,044,998 B`.
