# Roadmap

Yune is a Rust input-method engine that uses **librime as a compatibility oracle** while building toward an AI-native input engine librime cannot provide. The strategy is target-driven: make named RIME schemas and frontends behave predictably through Yune, measure every compatibility difference against the relevant oracle, and layer AI-native behavior on top only through explicit product milestones.

> **Compatibility oracle.** Upstream librime latest stable is the default core behavior reference for user-visible behavior, schema semantics, standard ABI contracts, deployed data, and migration. The current pinned upstream target is `rime/librime 1.17.0` (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`): <https://github.com/rime/librime>. TypeDuck-specific behavior is referenced only as a compatibility profile against the TypeDuck fork (tag `v1.1.2`, commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`): <https://github.com/TypeDuck-HK/librime>. These are referenced upstream/fork repositories, not local checkout paths.

## Document Map

- This file - current roadmap dashboard, active sequence, scope boundaries, and readiness gates.
- [`ledgers/milestone-history.md`](./ledgers/milestone-history.md) - historical milestone ledger split out of this roadmap.
- [`conventions.md`](./conventions.md) - architecture, stack, structure, coding/testing conventions, integrations, current risks, and planning-doc rules.
- [`decisions.md`](./decisions.md) - standing principles plus project-wide decision log.
- [`requirements.md`](./requirements.md) - requirement IDs and status, including the closed M37 engine gates.
- [`ledgers/fork-parity-ledger.md`](./ledgers/fork-parity-ledger.md) - source of truth for Cantoboard/TypeDuck fork improvements versus upstream `1.17.0`.
- [`reports/yune-vs-librime-performance.md`](./reports/yune-vs-librime-performance.md) and [`reports/yune-vs-librime-root-cause-analysis.md`](./reports/yune-vs-librime-root-cause-analysis.md) - current performance comparison and diagnosis.
- [`plans/completed/m37-plan-engine-hyper-optimization.md`](./plans/completed/m37-plan-engine-hyper-optimization.md) - completed engine hyper-optimization milestone.
- [`plans/active/p2-win01-plan-typeduck-windows-next.md`](./plans/active/p2-win01-plan-typeduck-windows-next.md) - Phase 2 Windows product/frontend plan.
- [`plans/active/m32-plan-ai-native-public-demo-product-layer.md`](./plans/active/m32-plan-ai-native-public-demo-product-layer.md) - later AI-native product/demo expansion.
- [`plans/`](./plans) - active, reference, and completed plans, findings, contracts, and validation artifacts; finished plans live under [`plans/completed/`](./plans/completed).

> The GSD planning system (`.planning/`) has been retired; durable planning now lives under `docs/`.

## Current Snapshot

| Lane | Current state | Next decision or gate |
| --- | --- | --- |
| Core compatibility | Phase 1 named-target baseline is complete: upstream `luna_pinyin` / common schemas track upstream `1.17.0`; TypeDuck `jyut6ping3` tracks TypeDuck-HK/librime `v1.1.2` through explicit profile surfaces | Preserve upstream-first defaults and TypeDuck-profile isolation on every new change |
| Engine performance | M33-M37 closed fairness, bounded upstream candidate work, compact upstream storage, compiled-active TypeDuck product storage, page-bounded product materialization, and native mapped product table storage | Keep Track A comparison caveats visible; target future engine work only from fresh owner evidence |
| Windows product | M10 backend/profile smoke and P2-WIN-02 Yune boundary compatibility are complete; the remaining Notepad raw-ASCII issue is classified as TSF input-delivery/frontend-shell work | Resume **P2-WIN-01** unless the user explicitly chooses another track |
| Public web demo | M31 published `yune-web` at <https://yune-web.pages.dev> with scoped output-standard evidence, Cloudflare Pages deployment, cache evidence, and no browser speed claim | Future public-demo changes need a new scoped plan and fresh browser evidence |
| AI-native product | M11 core/CLI AI and M13 default-off local web exposure are complete | **M32** remains later; it should not run ahead of Windows by default |
| Future platform work | TypeDuck-Web product integration and iOS keyboard support remain future product/platform tracks | Start a named track before touching those product repositories or platform contracts |

## Authoritative Sequence

1. **P2-WIN-01 TypeDuck-Windows product/frontend** - primary product track after the M37 engine closeout.
2. **M32 AI-native public demo/product expansion** - later, unless deliberately reprioritized.

Trigger-gated, not scheduled: extracting the full processor pipeline from `yune-rime-api` into `yune-core` lands only when a real non-ABI consumer such as an iOS package or Yune-native frontend needs the full input path. Do not milestone that extraction speculatively.

## M37 Closeout

M37 is complete. It closed the M36 residual engine gates for the native product path without making browser claims.

**Starting evidence:**

- M36 Track B product rows improved materially after fresh Yune-readable compiled artifacts: `ngohaig` `14,943.043us` -> `3,465.057us`, `loengjathau` `16,309.045us` -> `3,754.855us`, and `jigaajiusihaa` `27,633.869us` -> `5,065.308us`.
- Product max peak working set dropped from `1000.4 MB` to `885.3 MB`.
- M36 Track A still trails librime widely: `hao` `348.03x`, `ni` `206.04x`, and `zhongguo` `24.66x` slower.
- Track B `hai` remains the sharp residual product clue at `15,241.000us`, the shortest and worst final product row.
- Browser delivery was not touched by M36; native engine wins are not browser wins.

**Closeout evidence:**

- Phase 0 attributed `hai` to full candidate materialization/filtering: 19,918 lookup views, 19,918 owned candidates, 11,289 sorted/stored rows, and only 5 ABI-exported candidates.
- Final product rows are page-bounded: final `hai` still visits 19,918 lookup views but builds 52 owned candidates, sorts/stores 48, page-clones 5, and exports 5.
- Selected product storage is byte-backed and native-mapped: `selected_storage=byte_backed`, `table_format=yune_no_marisa_compact`, `mapping_mode=mmap`, `source_fallback=false`.
- `rsmarisa 0.4.2` was tried against real TypeDuck `jyut6ping3` and `jyut6ping3_scolar` marisa string-table payloads; both mmaped successfully. It was not selected as the hot route because full query replacement still needs a multi-level phrase-index adapter.
- `hai` moved from the M36 final `15,241.000us` to `8,336.800us` (`-45.3%`).
- Track B product median working set moved from the M36 final about `777 MB` row plateau to about `365-369 MB`; peak moved from `928,350,208 B` to `504,377,344 B`.
- Track A remains comparison evidence and still trails librime widely; no browser startup/typing claim is made because M37 did not rebuild release WASM or run real browser proof.

## Track Map

| Track | Scope | Current source of truth |
| --- | --- | --- |
| Engine performance | Native full-ABI and engine data-path optimization; Track A fair comparison and Track B TypeDuck product rows stay separate | [`plans/completed/m37-plan-engine-hyper-optimization.md`](./plans/completed/m37-plan-engine-hyper-optimization.md), performance reports, and `docs/reports/evidence/` |
| Windows product/frontend | Yune-first TypeDuck-Windows product shell, TSF input delivery, candidate UI, packaging, and interactive smoke | [`plans/active/p2-win01-plan-typeduck-windows-next.md`](./plans/active/p2-win01-plan-typeduck-windows-next.md) |
| Public web/demo delivery | `yune-web` packaging, Cloudflare deployment, payload/cache strategy, browser smoke, and browser-only UX | [`plans/completed/m31-plan-yune-web-public-demo-readiness.md`](./plans/completed/m31-plan-yune-web-public-demo-readiness.md) |
| AI-native product | Local-first AI UX, memory/privacy controls, public-demo posture, and any explicit remote-provider decision | [`plans/active/m32-plan-ai-native-public-demo-product-layer.md`](./plans/active/m32-plan-ai-native-public-demo-product-layer.md) |
| Compatibility breadth | Named schema/frontend additions with fresh oracle fixtures | [`requirements.md`](./requirements.md), [`decisions.md`](./decisions.md), and per-milestone plans |
| Historical record | Completed milestone outcomes and reference/provenance pointers | [`ledgers/milestone-history.md`](./ledgers/milestone-history.md) |

## Milestone Ledger

| Milestone or track | Status | Current roadmap meaning |
| --- | --- | --- |
| M0-M24 | Complete | Phase 1 named-target engine/basic oracle parity is complete; history lives in [`ledgers/milestone-history.md`](./ledgers/milestone-history.md). |
| M25-M30 | Complete | TypeDuck-Web dogfooding and early performance follow-ups are closed; future web dogfood needs a new scoped ledger. |
| P2-WIN-02 | Complete | Yune-side TypeDuck Windows raw-comment/session boundary is fixed; remaining interactive issue is TSF/frontend-shell work. |
| M31 | Complete | `yune-web` public demo is deployed; no browser startup/typing win is claimed. |
| M33-M37 | Complete | Recent engine-performance work is closed through product compiled-active storage, page-bounded materialization/context export, native mapped product table storage, `hai` movement, and product memory movement. |
| P2-WIN-01 | Draft / next product track | Resume unless deliberately prioritized later. |
| M32 | Planned / later | AI-native product/demo expansion; do not let it delay Windows by default. |

## Scope Ledger

A living map so "parity" always names a target. Deferred rows move into scope only as a named target needs them; nothing here commits to a timeline, and the non-goal column is not a backlog.

| In scope - target-driven, measured | Deferred - implement when a target needs it | Non-goal |
| --- | --- | --- |
| `luna_pinyin` core versus upstream `1.17.0`, including completed M17 null-grammar sentence/lattice and M18 punctuation processor slices | Learned `.gram`/octagram grammar, contextual translation, and broader plugin-backed gears until a named target needs them | Bit-for-bit parity with librime internals |
| TypeDuck `jyut6ping3` profile versus TypeDuck-HK/librime `v1.1.2` | Browser/userdb UI evidence or broader profile behavior only when a named product target needs it | Making TypeDuck behavior the default upstream ABI/core truth |
| Common RIME schemas added through explicit breadth milestones | Further schema breadth only with fresh oracle fixtures and owning tests | Unbounded schema checklist work |
| AI-native layer on the compatible base | M32 product/demo expansion and any remote-provider decision | Replacing or altering classic input paths by default |
| Product/platform frontends when named | TypeDuck-Web product integration, iOS keyboard SDK, resource bundle/deploy model, sandboxed storage, mobile config hooks | Treating frontend repos or platform build scripts as engine semantics |
| Engine performance for named product rows | Browser delivery/cache speed work unless backed by real browser evidence | Claiming native wins as browser wins |

## Deferred / Future

- **librime C++ plugin ABI** (Lua, octagram, predict, proto): deferred until a concrete frontend or distribution requires it; prefer Yune-native extension points first.
- **AI-native input layer beyond M13:** M32 owns richer local-first AI UX, privacy/memory controls, public-demo evidence, and any explicit remote-provider decision. Until then, proven web AI remains default-off/local-only.
- **Queryable table+prism storage after M37:** the native product path now uses byte-backed mapped Yune product tables. Future storage work should target full `rsmarisa` hot-path selection only after a multi-level phrase-index adapter proves byte identity across table, reverse, prism, prediction, correction, paging, learning, and rich-comment gates. Browser byte-backed loading still needs separate WASM and real-browser evidence before public delivery claims.
- **Post-M31 `yune-web` follow-ups:** future public-demo changes need a new scoped plan, browser evidence, no telemetry/secrets, and clear separation between delivery/cache claims and Rust engine latency claims.
- **OpenCC output-standard breadth:** M31 exposes only browser-honest Hong Kong Traditional and `hk2s` Simplified controls. Broader standards need named engine/runtime/browser evidence.
- **TypeDuck-Web product integration:** use a new product-integration track before changing a separately cloned `TypeDuck-HK/TypeDuck-Web` checkout. The internal `apps/yune-web/` harness is not the shipping TypeDuck product.
- **TypeDuck-Web userdb reset/debug controls:** the internal harness now has read-only active-userdb inspection; a later debug slice should add explicit clear/reset controls, preserve sandboxed browser storage boundaries, and tighten the Input Memory off-state so learned userdb candidates are suppressed immediately instead of only persisting `translator/enable_user_dict=false`.
- **iOS keyboard developer support:** future TypeDuck iOS or third-party keyboard work needs its own Yune-native package/host contract.

## Principles

The standing principles that govern all current and future work - librime as oracle not template, name-the-protected-behavior, own-each-slice, AI-native as a separate local-first layer, fixtures before module replacement, deferred plugin ABI, and upstream-first oracle sequencing - have one canonical home: [`decisions.md` -> Standing principles](./decisions.md#standing-principles).
