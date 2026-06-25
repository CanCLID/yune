# Why Yune is slower than librime - root-cause analysis

Date: 2026-06-24

Companion report: [`yune-vs-librime-performance.md`](./yune-vs-librime-performance.md).

## Current Verdict After M37

The remaining gap is not a generic Rust problem. It is now split between landed
upstream compact-storage work, landed TypeDuck product-path compiled storage,
landed M37 page-bounded product materialization, landed native mapped product
table storage, and remaining comparison/browser/residual lookup owners.

Resolved or improved:

1. **M33 fairness:** Yune no longer loads `stroke` reverse lookup during the
   no-reverse `luna_pinyin` startup/session comparison.
2. **M33 repeated schema/session cost:** immutable built dictionary translators
   are shared across compatible schema selects.
3. **M34 bounded first-page work:** short `luna_pinyin` typing can keep complete
   prefix enumeration but materialize only a bounded candidate window for the
   safe no-ranker/no-userdb/no-full-list-filter subset.
4. **M35 compact upstream storage:** upstream `luna_pinyin` can use compact
   table storage plus prism canonical-code lookup without retaining heap
   `entries_by_code` or expanded spelling-algebra aliases.
5. **M36 product compiled storage:** the shipped TypeDuck `jyut6ping3_mobile`
   path can rebuild stale unsupported product marisa blobs into Yune-readable
   table/prism/reverse artifacts during schema-scoped deploy, then run with
   `compiled_ready=true` instead of `SourceFallback`.
6. **M37 page-bounded product rows:** ordinary Track B product rows no longer
   materialize and sort the full product completion set before exporting one
   page.
7. **M37 mapped product table storage:** selected native product tables are
   byte-backed with `mapping_mode=mmap`, no `SourceFallback`, and no M36 owned
   no-marisa row mirror. Real TypeDuck marisa string-table payloads were also
   mmaped successfully with `rsmarisa`.

Still open:

1. **Track A upstream lookup still trails librime.** M36 Track A final rows keep
   the comparison gap visible: `hao` is `348.03x`, `ni` is `206.04x`, and
   `zhongguo` is `24.66x` slower than librime in the native in-process fair
   `luna_pinyin` comparison.
2. **Whole-process comparison memory remains high.** M36 product rows reduce the
   TypeDuck product working set, but Track A still has a large Yune-versus-librime
   working-set gap.
3. **Residual Track B `hai` owner is lookup-view scanning.** Final M37 `hai`
   still visits 19,918 lookup views, but now builds only 52 owned candidates and
   exports 5. A future win should target the product lookup/index path, not
   full-list materialization.
4. **`rsmarisa` full product query integration remains future design work.**
   `rsmarisa` can mmap the real marisa string-table payloads, but M37 did not
   select it for the hot route because the real product `.table.bin` also needs
   a multi-level phrase-index adapter.
5. **Browser delivery remains M31 work.** M37 changed native engine/deploy
   storage behavior only. It does not claim browser startup, browser typing,
   WASM, React, Cloudflare, or public-demo delivery wins.

## What changed in M34

M34 added an internal bounded candidate request and lazy engine window:

- `Translator::translate_with_context_and_request(...)` defaults to eager
  compatibility behavior.
- `StaticTableTranslator` uses bounded materialization only for the safe subset.
- Prefix enumeration remains complete under the current code-ordered heap map;
  only candidate clone/comment/preedit materialization is bounded.
- `Engine::refresh_candidates` uses the bounded path for short `luna_pinyin`
  input when filters/rankers/userdb allow it.
- Out-of-window candidate actions and full-list candidate iterators force a
  complete eager refresh.
- `RimeGetContext` receives a `candidate_list_complete` bit so it can report
  `is_last_page` honestly without materializing every candidate for first-page
  reads.
- A private `TableLookup` abstraction now covers exact/prefix/all-code queries
  for the current heap map.

The public C ABI did not change. `RimeApi`, `RimeCandidate`, and the TypeDuck
profile ABI remain isolated.

## What Changed In M35

M35 added the compact runtime storage substrate that M34 deliberately deferred:

- `TableLookup` now returns lightweight candidate views instead of heap
  `&[Candidate]` slices.
- `CompactTableStore` answers exact, prefix, and all-code queries without
  retaining per-row `Candidate` values.
- `RimePrismBinPayload::lookup_canonical_codes(...)` maps typed spellings to
  canonical table codes; table storage still supplies candidate payloads.
- `StaticTableTranslator` uses a private heap-or-compact storage enum.
- `schema_install` preserves parsed prism payloads and enables compact storage
  only for safe upstream `luna_pinyin`.
- TypeDuck `jyut6ping3` remains heap-backed by design.

The public C ABI still did not change. Default `RimeApi`, `RimeCandidate`, and
TypeDuck profile ABI slots are untouched.

## What Changed In M36

M36 made the TypeDuck product path measurable and compiled-active:

- Added a native Rust in-process benchmark harness for Track A (`luna_pinyin`
  Yune versus librime) and Track B (`jyut6ping3_mobile` Yune before/after),
  including startup, session, per-key, resident working set, peak working set,
  and product asset status.
- Recorded that shipped product `jyut6ping3` and `jyut6ping3_scolar` compiled
  blobs are stale relative to source dictionaries and rejected by Yune as
  unsupported marisa table/reverse plus unsupported prism versions.
- Added schema-scoped `RimeRunTask("workspace_update:<schema_id>")` support so
  the product benchmark can rebuild only the active schema's dictionary assets
  before measurement.
- Fixed deployment rebuilds to write prism artifacts to the configured
  translator prism stem. This matters for `jyut6ping3_mobile`, whose dictionary
  id is `jyut6ping3` but whose configured prism id is `jyut6ping3_mobile`.
- Preserved parsed prism payloads on the product translator path and enables
  compact storage for TypeDuck only when the path loaded compiled artifacts.
- Kept `RimeApi`, `RimeCandidate`, and TypeDuck profile ABI unchanged.

## What Changed In M37

M37 closed the M36 residual product-path gates:

- Added opt-in M37 counters for process key, translator time, lookup views,
  owned candidate materialization, sort/filter/userdb/ranker/AI spans, context
  page/full snapshot clones, ABI context export, and `RimeFreeContext`.
- Changed `RimeGetContext` to use a page snapshot for ordinary context reads,
  so page-only reads no longer require cloning the full candidate list.
- Extended the TypeDuck product bounded-refresh path so ordinary
  `jyut6ping3_mobile` rows materialize the current page plus bounded surplus
  instead of the full completion set.
- Changed compiled product table loading so the table translator can receive a
  `CompactTableStore` over mapped bytes directly, instead of parsing a full
  `TableDictionary` and rebuilding owned `String` rows.
- Added native mapped table byte ownership through the ABI loader while keeping
  `yune-core` free of local unsafe mmap code.
- Added product status evidence for `selected_storage`, `table_format`,
  `mapping_mode`, `source_fallback`, byte source length, stored entries, and
  `rsmarisa` probe results.
- Tried `rsmarisa 0.4.2` against real shared `jyut6ping3` and
  `jyut6ping3_scolar` marisa string-table payloads. Both mmaped successfully.
- Kept `RimeApi`, `RimeCandidate`, and TypeDuck profile ABI unchanged.

## Measured Shape

| Surface | Before | M34 after | Interpretation |
| --- | ---: | ---: | --- |
| native `ni` full ABI | `1,760.250 us` | `1,132.950 us` | bounded first-page/context win |
| native `ni` engine-only | `569.700 us` | `575.250 us` | raw lookup not solved |
| cross-engine `hao` | `13,336.800 us` | `12,216.900 us` | improved, still `348.1x` librime |
| cross-engine `ni` | `5,858.800 us` | `5,693.900 us` | improved, still `198.4x` librime |
| cross-engine `zhongguo` | `36,451.100 us` | `35,909.100 us` | modest improvement, still `26.0x` librime |
| peak working set | `182,874,112 bytes` | `182,333,440 bytes` | no footprint win |

M35 movement:

| Surface | M35 baseline | M35 after | Interpretation |
| --- | ---: | ---: | --- |
| native `hao` engine-only | `1092.879us` | `750.517us` | compact upstream path improves, target not met |
| native `ni` engine-only | `891.791us` | `697.044us` | compact upstream path improves, target not met |
| native `zhongguo` full ABI | `14759.755us` | `1527.055us` | spelling-algebra expansion removed from hot path |
| `spelling_algebra_expand` startup | `148570.200us` / `17784832 bytes` | `122.200us` / `0 bytes` | expanded alias heap removed |
| `translator_install` startup | `233169.800us` / `37556224 bytes` | `55155.800us` / `9822208 bytes` | retained upstream dictionary delta cut |
| fair `hao` | `15906.800us` | `12547.200us` | improved, still `354.4x` librime |
| fair `ni` | `9225.100us` | `5678.500us` | improved, still `197.9x` librime |
| fair `zhongguo` | `45608.600us` | `35848.500us` | improved, still `24.7x` librime |
| fair peak working set | `182910976 bytes` | `182444032 bytes` | whole-process footprint not solved |

TypeDuck full-ABI guard rows stayed heap-backed and within the M35 guard/no-go
expectation:

- `hai`: `18,900.742 us` -> `18,450.767 us` (`-2.4%`)
- `jigaajiusihaa`: `28,836.874 us` -> `26,953.441 us` (`-6.5%`)
- correction-on `jigaajiusihaa`: `24,811.675 us` -> `26,707.480 us` (`+7.6%`)

The companion performance report now embeds M35 visualizations for native
watched-row movement, fair cross-engine gap, and dictionary-local memory versus
whole-process peak. Those charts intentionally keep the remaining librime gap
visible instead of turning the compact-storage win into a broad performance
claim.

M36 movement:

| Surface | M36 baseline | M36 final | Interpretation |
| --- | ---: | ---: | --- |
| product startup ready | `201811.100us` / `818.7 MB` | `175424.800us` / `738.8 MB` | compiled product assets reduce startup and working set |
| product session create/select/destroy | `243946.900us` / `806.7 MB` | `219919.200us` / `726.6 MB` | product schema/session still costly but lower |
| product `ngohaig` | `14943.043us` / `823.2 MB` | `3465.057us` / `741.5 MB` | product typing win from compiled-active path |
| product `loengjathau` | `16309.045us` / `823.7 MB` | `3754.855us` / `741.5 MB` | product typing win from compiled-active path |
| product `jigaajiusihaa` | `27633.869us` / `824.8 MB` | `5065.308us` / `741.5 MB` | long product typing row improves materially |
| Track A `hao` | - | Yune `4072.000us`, librime `11.700us` | comparison gap remains `348.03x` |
| Track A `ni` | - | Yune `2977.300us`, librime `14.450us` | comparison gap remains `206.04x` |
| Track A `zhongguo` | - | Yune `4403.738us`, librime `178.600us` | comparison gap remains `24.66x` |

Product max peak working set drops from `1000.4 MB` to `885.3 MB` across the
measured Track B rows. The product status CSV records `compiled_ready=true` for
both `jyut6ping3` and `jyut6ping3_scolar` in the final run.

M37 movement:

| Surface | M36 final | M37 final | Interpretation |
| --- | ---: | ---: | --- |
| product `hai` | `15241.000us` / `741.5 MB` | `8336.800us` / `350.3 MB` | residual row moved after page-bounded materialization and mapped storage |
| product `ngohaig` | `3465.057us` / `741.5 MB` | `1861.586us` / `350.9 MB` | ordinary product row is now page-bounded |
| product `loengjathau` | `3754.855us` / `741.5 MB` | `2164.609us` / `348.0 MB` | ordinary product row is now page-bounded |
| product `jigaajiusihaa` | `5065.308us` / `741.5 MB` | `3189.085us` / `352.3 MB` | long row remains byte-identical while reducing memory |
| product peak working set | `928350208 bytes` | `504377344 bytes` | mapped/byte-backed storage removes the M36 row-mirror plateau |
| final `hai` materialization | 19,918 owned candidates in phase 0 | 52 owned candidates | full-list materialization is no longer the `hai` owner |
| final `hai` context export | 5 page clones / 5 ABI candidates | 5 page clones / 5 ABI candidates | context export remains page-bounded |

The final product status records `selected_storage=byte_backed`,
`table_format=yune_no_marisa_compact`, `mapping_mode=mmap`,
`source_fallback=false`, and fresh table/prism/reverse artifacts for both
product dictionaries. The selected byte source lengths are `15,248,382 B` for
`jyut6ping3` and `27,325,622 B` for `jyut6ping3_scolar`.

## Why Librime Remains Faster

librime's classic table path has a compact deployed data model and a lazy
candidate iterator:

- deployed table/prism assets are compact and mmap-friendly;
- prism/spelling lookup is an index into table payloads;
- candidates are exposed through page/iterator-oriented APIs;
- full candidate payload materialization is avoided until needed;
- reverse lookup is lazy;
- schema/dictionary state is shared.

Yune now has lazy reverse lookup, build-once translator sharing, a narrow
bounded first-page path, compact upstream `luna_pinyin` table+prism storage, a
compiled-active TypeDuck product path through Yune-readable rebuilt assets,
page-bounded ordinary TypeDuck product materialization, and native mapped product
table bytes. It still falls back to eager/full-list behavior for unsafe
userdb/ranker, correction-heavy, and filter cases, and it still keeps Track A
whole-process comparison memory far above librime.

## Follow-Up After M37

M37 closes the native product-path hyper-optimization milestone. The safe
follow-up order is now:

1. Keep Track A and Track B separated in all public claims. Product-path wins
   come from Track B before/after evidence; fair upstream `luna_pinyin`
   comparison ratios remain caveats, not product headlines.
2. Treat full `rsmarisa` hot-path selection as future storage-design work. The
   string-table mmap probe is green, but a phrase-index adapter is still needed
   before `rsmarisa` can replace the selected byte-backed Yune table route.
3. Do not broaden bounded/lazy TypeDuck candidate windows without byte-identical
   coverage for paging, filters/rankers, correction/tolerance, context snapshots,
   partial selection, default-confirm recomposition, long composition, and
   userdb learning.
4. Route browser startup, browser typing, TypeDuck-Web public delivery,
   Cloudflare/cache, and OpenCC UI work to M31 unless a future engine milestone
   explicitly changes runtime-visible files and records fresh browser proof.
