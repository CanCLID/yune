# Why Yune is slower than librime - root-cause analysis

Date: 2026-06-23

Companion report: [`yune-vs-librime-performance.md`](./yune-vs-librime-performance.md).
M33 changed the diagnosis. The old headline gap was partly unfair because Yune
loaded `stroke` reverse-lookup assets during `luna_pinyin` schema select while
librime did not. M33 fixed that fairness issue and added a build-once
translator cache. Startup and session medians are now in the same order of
magnitude as librime, but per-key lookup remains far slower.

## Current verdict after M33

It is not "Rust is slow" and it is not that Yune's AI-native direction prevents
classic performance. The remaining gap is a concrete representation gap:

1. **Resolved in M33: reverse-lookup load asymmetry.** Yune now defers the
   `stroke` reverse dictionary until the first reverse-lookup query, matching
   librime for no-reverse `luna_pinyin` typing.
2. **Resolved for reselect/session in M33: rebuild-per-select.** Yune now shares
   immutable built dictionary translators process-wide using schema and asset
   signatures, so repeated session select no longer reloads and re-expands the
   same schema.
3. **Still open: eager table-backed spelling algebra.** The live lookup structure
   is still a materialized `BTreeMap<String, Vec<Candidate>>` built from table
   payloads and spelling-algebra expansion. librime's hot path walks mmap-backed
   compiled structures and applies prism spelling data lazily.
4. **Still open: no mmap-backed runtime lookup.** Yune still reads/parses
   compiled artifacts into owned heap structures. Mmap was deferred because the
   M33 stop gate showed startup/session already improved dramatically, while the
   remaining product-relevant gap is per-key lookup representation.

## What changed in M33

Before M33, the public comparison showed roughly `96x` startup/session gaps on
the fresh M33 baseline:

| Row | Yune before | librime before | Ratio |
| --- | ---: | ---: | ---: |
| Startup/runtime-ready | `2,881,852.7 us` | `29,788.8 us` | `96.7x` |
| Session create/select/destroy | `2,985,364.0 us` | `30,998.8 us` | `96.3x` |

After lazy reverse lookup and the built-translator cache:

| Row | Yune after | librime after | Ratio |
| --- | ---: | ---: | ---: |
| Startup/runtime-ready | `47,788.2 us` | `27,628.3 us` | `1.7x` |
| Session create/select/destroy | `47,813.7 us` | `25,765.9 us` | `1.9x` |

This makes the old "Yune startup is about 100x slower" public claim stale and
unsafe. The fair post-M33 claim is narrower: startup/session are close enough for
public demo copy, but not faster than librime.

## Remaining hot path

Per-key rows did not improve:

| Input | Yune after | librime after | Ratio |
| --- | ---: | ---: | ---: |
| `ni` | `6,064.5 us` | `28.5 us` | `212.8x` |
| `hao` | `12,463.4 us` | `34.5 us` | `361.3x` |
| `zhongguo` | `37,572.3 us` | `1,479.8 us` | `25.4x` |

The current Yune lookup path still depends on the table-backed translator map:

- Exact lookup reads `entries_by_code.get(...)`.
- Completion walks `entries_by_code.range(...)`.
- Prefix fallback and sentence segmentation re-read `entries_by_code`.
- TypeDuck correction stress can scan `entries_by_code.keys()`.
- Candidate text, comments, quality, ordering, recomposition, and sentence data
  live in table-derived `Candidate` payloads, not in the prism.

M33 added the focused fixture test
`upstream_luna_pinyin_prism_fixture_does_not_contain_candidate_payloads` to lock
the lazy-prism spike result. The upstream prism can map a spelling such as `ni`
to syllable descriptors, but it does not contain candidate text bytes such as
`U+4F60`, `U+597D`, or `U+4E2D U+56FD`. A byte-identical lazy lookup therefore
needs a broader table-payload/index redesign, not just a prism walk.

## Why librime remains faster per key

librime's classic path is close to the floor for a table IME:

- Deploy once into compact compiled assets.
- Memory-map the table/prism files at runtime.
- Walk trie/prism structures directly.
- Load reverse lookup only on first reverse lookup.
- Share schema/dictionary state across sessions.

Yune now has the last two bullets for the measured no-reverse workload, but not
the mmap-backed lookup model. That is why startup/session are close and per-key
typing remains behind.

## Deferred work

The next performance milestone should not start from generic "make startup
faster" work. It should start from the exact remaining representation question:

- Can Yune build a byte-identical table-payload index that lets prism spelling
  lookup find candidate text/comment/order without pre-expanded `entries_by_code`
  materialization?
- Can that design preserve TypeDuck `jyut6ping3` profile behavior, sentence
  segmentation, prefix fallback, correction scans, and candidate quality?
- Once that lookup representation exists, mmap can pay off because the hot path
  would actually walk borrowed compiled structures instead of copying them into
  heap maps.

Until that milestone is opened with evidence, M33 should stay closed as the
bounded fairness/cache win.
