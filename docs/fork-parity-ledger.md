# Fork Parity Ledger — Cantoboard + TypeDuck improvements vs upstream librime

> **Status:** Active · **Created:** 2026-06-19 · **Type:** reference / planning backbone
>
> The single source of truth for *what the Cantonese librime forks changed versus
> upstream `rime/librime`, and what Yune has done about each change.* Every claim
> here is measured against the pinned upstream oracle
> **`rime/librime 1.17.0`** (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`), not
> against the fork's own (older) base. Roadmap/requirements defer to this file for
> fork-improvement status.

## Sources & method

- **Source inventory (what the forks changed):**
  [`CANTOBOARD_LIBRIME_REBASE_SUMMARY.md`](./CANTOBOARD_LIBRIME_REBASE_SUMMARY.md)
  (Cantoboard's 18 master commits + side-branches, and which TypeDuck adopted) and
  [`REBASE_SUMMARY_SINCE_D8BC266D.md`](./REBASE_SUMMARY_SINCE_D8BC266D.md) (the full
  TypeDuck delta `d8bc266d..v1.1.2`). These are **LLM-generated provenance notes**; every
  row below was re-verified against the on-disk fork source and live upstream 1.17.0.
- **Upstream comparison:** fork source on disk at
  `target/typeduck-oracle/v1.1.2/{librime-src, rime-dictionary-lookup-filter-src, schema-src}`
  (a shallow clone — no local history, so no `git diff`) vs upstream files fetched
  from `https://raw.githubusercontent.com/rime/librime/33e78140…/`.
- **Yune status:** verified by searching `crates/yune-core` and `crates/yune-rime-api`.

## Fork lineage & the version gap (read this first)

```
rime/librime  (upstream oracle = 1.17.0)
      │
      ├─► Cantoboard/librime  (Cantonese + iOS fork, ~2021; 18 commits)
      │        │  TypeDuck cherry-picked ~5: correction ordering (733eedc8),
      │        │  PreferUserPhrase (d02f26fc), vector syllable-graph (5c3c7ba),
      │        │  mobile keymap (ac4bddd5), start_quick (a7b7148c)
      │        ▼
      └─► TypeDuck-HK/librime  (large original body of work)
               + merged upstream up to ~librime 1.11.0
               = tag v1.1.2  (74cb52b…)  ← our profile oracle
```

**The version gap is the central subtlety.** TypeDuck v1.1.2 sits on a **~librime
1.11.0** base; Yune's oracle is **1.17.0**. Several behaviors the fork pioneered on
1.11.0 were *independently* added by upstream before 1.17.0 — so they are **not fork
deltas for Yune** and must not be credited to the fork. Conversely, the fork's own
ranking/threshold choices for those behaviors can still differ from 1.17.0's, which
is a *tuning* question, not a missing feature.

## Yune-vs-librime architecture crib sheet

Use this table before deciding whether to port a fork change. The ledger tracks
behavior Yune must preserve or decline; it does **not** imply that Yune should
copy the librime or fork implementation shape.

| Area | librime / forks | Yune current design | Porting implication |
|---|---|---|---|
| Runtime dependency | librime is the engine. Forks patch the C++ engine and plugins directly. | librime is an oracle only; Yune never links or calls it at runtime. | Capture expected behavior from the oracle, then implement the smallest Rust-owned behavior that satisfies the named target. |
| Default ABI | Forks may add or reorder ABI slots for their own frontends. | Default `rime_get_api()` follows upstream 1.17.0; TypeDuck-only slots stay out of the default table. | Fork-only ABI belongs behind a named TypeDuck profile surface, not in the default ABI. |
| Profile-specific tuning | Forks can tune ranking, correction, sentence, and prediction constants in shared C++ code for their own product. | Yune keeps upstream/default behavior separate from TypeDuck-profile behavior, even when both use the same Rust translator types. M23 gated the `21.0` TypeDuck sentence word penalty through typed translator config so default upstream schemas no longer inherit it. | TypeDuck-calibrated constants must be installed through an explicit profile predicate or typed translator config. A `TYPEDUCK_*` constant read unconditionally by shared core is a red flag unless upstream oracle evidence proves the behavior is global and the name is made neutral. |
| Core/input split | librime gears/processors live inside the C++ engine. | Yune's deterministic translators, filters, dictionaries, ranking, userdb model, and AI layer live mostly in `yune-core`; the RIME processor pipeline, schema install/config glue, sessions, and FFI table currently live in `yune-rime-api`. M23 completed the bounded hardening around this boundary without moving processors. | Processor semantics in `yune-rime-api` are compatibility-centric current state, not the ideal long-term native-engine boundary. Extract processor semantics into `yune-core` when a non-librime frontend or iOS/native product path needs a Rust API that does not tunnel through the C ABI. The processor extraction itself stays trigger-gated (D-28). |
| Schema model | librime loads schemas through its C++ config/deployer stack. | M23 removed the orphaned `crates/yune-schema` parser crate. Production schema parsing/install remains in `yune-rime-api` (`config.rs`, `schema_install.rs`, `schema_selection.rs`). | Keep schema behavior oracle-driven in the live `yune-rime-api` path. Reintroduce a standalone schema crate only when a concrete production consumer needs that Rust API. |
| Compiled dictionary artifacts | librime writes marisa-backed table/reverse internals plus a Darts prism. | M18 added public Yune-owned writers: `.table.bin` / `.reverse.bin` use Yune's round-trippable `YUNE-*` payloads, while `.prism.bin` contains a real pure-Rust Darts double-array spelling index. These generated table/reverse bytes are for Yune's readers/deployer path, not upstream librime consumption. | Preserve observable candidate/spelling behavior, checksums, and Yune round-trips. Do not chase marisa/upstream table byte identity unless a named external consumer requires upstream-consumable artifacts. |
| Syllable graph / spelling | Forks can tune C++ `SyllableGraph` behavior and pruning. | Yune generally models spelling expansions and penalties directly in Rust data structures; some C++ graph hazards are architecturally inapplicable. | Preserve observable candidates, preedit, comments, commits, and option behavior; do not port C++ graph internals just because a fork touched them. |
| Plugin ABI | librime supports C++ plugin ecosystems; TypeDuck uses a dictionary-lookup plugin. | Yune defers the C++ plugin ABI and implements named plugin behavior natively when a target needs it (`DictionaryLookupFilter`, OpenCC subset, etc.). | Port plugin behavior as Rust-owned target behavior, not as a general C++ plugin host, unless a future distribution explicitly requires the plugin ABI. |
| User data | librime uses LevelDB-style userdb and fork-specific tweaks. | Yune uses a file-backed compatible abstraction for classic userdb, and keeps AI memory in a separate `.ai-memory` namespace. | Match librime-observable learning/export/import behavior; never leak AI selections into `*.userdb`. |
| Product surfaces | Fork repos combine engine, packaging, mobile build, and product concerns. | Yune separates the reusable runtime bridge, internal TypeDuck-Web harness, deployed product comparison, TypeDuck profile ABI, and future iOS/native packaging tracks. | Classify each fork change as engine parity, ABI/profile, product UI, packaging, or non-goal before implementing. |

## Legend

- **Category** — `fork-engine-code` (C++ engine change absent upstream) · `fork-plugin`
  (TypeDuck-only `rime-dictionary-lookup-filter`; upstream bundles none) ·
  `fork-schema-data` (fork-authored schema/algebra data on upstream primitives) ·
  `fork-schema-config` (upstream option the fork enables/values) ·
  `upstream-1.17.0` (already upstream — **not** a fork delta).
- **In 1.17.0?** — does the pinned upstream oracle already have it.
- **Yune status** — `done` · `partial` · `missing` · `non-goal` · `n/a` (architecturally inapplicable).
- **Decision** — `preserve✓` (done, keep) · `preserve-todo` (genuine delta, not yet matched) ·
  `non-goal` · `do-not-preserve` · `decide` (needs a product call).

## Headline findings

1. **Two of the team's "six improvements" are NOT fork deltas vs 1.17.0.** **F2**
   (`santai`→身體/身體健康 prefix prediction) and **F4** (auto-compose only as fallback)
   are upstream-default behaviors the fork merely keeps on. So is `max_corrections=4`
   and `fix_schema_list_order`. Yune gets these from tracking 1.17.0 — preserve them
   by *not regressing*, not by porting fork code.
2. **The real fork deltas to preserve** are **F1** (Cantonese algebra), **F5**
   (`DictionaryLookupFilter` plugin), **F6 / `combine_candidates`** (engine), plus
   `show_full_code`, `hide_lone_schema`, `letter_to_tone`, the full 容錯 ruleset, and
   the Cantoboard-origin correction/user-phrase fixes.
3. **FORK-PARITY-01..07 are preserved.** The production Cantonese algebra now runs on
   the real ~127k-entry `jyut6ping3` dictionary; `PreferUserPhrase`, per-entry userdb
   pronunciation recovery, `hide_lone_schema`, correction fidelity, letter-tone
   preedit, and TypeDuck full-/half-width labels are implemented and tested.
4. **F08 is intentionally scoped, not full fork-ranking byte parity.** Yune follows
   upstream `1.17.0` ranking semantics, preserves long-entry completion (for example,
   `santai` can surface `身體健康`), and exposes profile controls for raw-weight
   prediction thresholds and prediction-never-first behavior.
5. **F09 is a UI-side decision.** `display_languages` gloss-column selection belongs
   in TypeDuck-Web; the engine emits stable lookup payloads without adding a
   language-filtering engine branch.
6. **`show_full_code` is a fork *engine* addition** (upstream `translator_commons`/
   `table_translator` have no such option) — Yune ported it correctly; don't relabel
   it as "an upstream option."
7. **The M14–M16 work itself is solid** (reviewed separately): real-engine
   implementations, no ABI change, 12/12 active parity tests green. The gaps above are
   *coverage/scope* gaps the M14–M16 goldens were not designed to catch.

---

## A. Runtime IME behaviors

| # | Behavior | Origin | Fork commit(s) | Category | In 1.17.0? | Yune status | Decision |
|---|---|---|---|---|---|---|---|
| F1 | `m` → 唔 **+** m-initial syllables (冇…) — schema `ng→m` derive + abbreviation + tone_ignore | TypeDuck | schema `include.yaml` | `fork-schema-data` | no | done ¹ | preserve✓ |
| F1e | engine half: syllable-graph pruning preserves abbreviation under perfect match | TypeDuck | `41684211`,`3aa87595` | `fork-engine-code` | no | **n/a** ² | non-goal |
| — | Full Cantonese 容錯 ruleset (`lv1_laanjam`,`lv2_upper`,`shortcuts`,`lv2_lower`,abbrev) | TypeDuck | schema `include.yaml` | `fork-schema-data` | no | done ¹ | preserve✓ |
| — | `letter_to_tone`/`tone_to_letter` (type v/x/q for tones) | TypeDuck | schema `include.yaml` | `fork-schema-data` | no | done ³ | preserve✓ |
| F2 | `santai` → 身體 **+** 身體健康 (prefix/word completion) | upstream | (`#848` upstream) | `upstream-1.17.0` | **yes** | done (default-on) | preserve✓ (don't disable) |
| — | Fork *prediction controls* (freq≥100-style threshold, prediction-not-first) | TypeDuck | `a01dd1af`,`245543ec` | `fork-engine-code` | partial⁴ | done (scoped) | preserve✓ (upstream ranking + knobs) |
| — | Exact fork prediction metadata/preedit (`matching_code_size`) | TypeDuck | `a01dd1af` | `fork-engine-code` | no | n/a ⁴ | do-not-preserve |
| F3 | Option to disable auto-composition (`enable_sentence:false` switch) | TypeDuck | `5e50fcdb` | `fork-schema-config` | yes (option) | done | preserve✓ |
| F4 | Auto-compose only when no exact phrase (sentence fallback gating) | upstream | — | `upstream-1.17.0` | **yes** | done | preserve✓ |
| — | Composition prefers fewer syllables / tuned word penalty | TypeDuck | `2ea5f56f`,`c1938644` | `fork-engine-code` | no | done (scoped M21-GAP-01) ⁵ | preserve✓ (oracle-backed scope only) |
| F5 | Reverse/dict lookup shows candidate text **+** looked-up pronunciation(s), joined `"; "` | TypeDuck | `3e90bf97`,`3f7b9a36` + plugin | `fork-plugin` | no | done | preserve✓ |
| F6 | Homographs: all pronunciations as separate rows / folded comments (`combine_candidates`) | TypeDuck | `0b5dd737`,`97b193f7` | `fork-engine-code` | no | done | preserve✓ |
| — | `show_full_code` (full input code / cangjie-root `\v` comment) | TypeDuck | `d8667c92` | `fork-engine-code` | **no** ⁶ | done | preserve✓ |
| — | `hk2s` OpenCC simplification (HK-trad → simplified) | TypeDuck | schema `template.yaml` | `fork-schema-config` | yes (gear) | done ⁷ | preserve✓ |
| — | `always_show_comments:true` (force comment render) | TypeDuck | `88e36264` | `fork-schema-config` | yes (option) | missing (latent no-op) ⁸ | preserve-todo (low) |
| — | `hide_lone_schema` (hide switcher when one schema) | TypeDuck | `838e3d41`,`83924c37` | `fork-engine-code` | no | done | preserve✓ |
| — | `nul alternative_select_keys` (free digit keys for tone input) | TypeDuck | schema `include.yaml` | `fork-schema-config` | yes (option) | done | preserve✓ |
| — | `display_languages` multilingual gloss columns (en/ur/ne/hi/id) | TypeDuck | plugin + web adapter | `fork-plugin` | no | n/a (UI-side) | non-goal engine-side |
| — | Correction ranked behind normal; corrections only from normal spellings | Cantoboard→TD | `733eedc8`→`2f79c3ab` | `fork-engine-code` | no | done ⁹ | preserve✓ |
| — | Correction penalty scales by edit distance; discard non-minimal-distance (`kCorrection`) | TypeDuck | `c77d5375`,`81e13724` | `fork-engine-code` | no | done | preserve✓ |
| — | `enable_correction` independent of `enable_completion` | TypeDuck | `585f4656` | `fork-engine-code` | no | done ¹⁰ | preserve✓ |
| — | `PreferUserPhrase` (user-dict not preferred by equal code length alone) | Cantoboard→TD | `d02f26fc`→`76da593b` | `fork-engine-code` | no | done ¹¹ | preserve✓ |
| — | Per-entry userdb element/full-code pronunciation recovery after commit | TypeDuck | `d057fb75`,`e2c8c4f0`,`124b6836` | `fork-engine-code` | no | done | preserve✓ |
| — | `全形`/`半形` state labels (vs `全角`/`半角`) | TypeDuck | `5fe09db5` | `fork-schema-data` | no | done ¹² | preserve✓ |
| — | Reverse lookup always shows schema name | TypeDuck | `578a55c2` | `fork-engine-code` | no | likely done ¹³ | preserve✓ (verify) |
| — | Mobile corrector keymap (iOS/Android) + no digit autocorrect | Cantoboard→TD | `ac4bddd5`→`8dc9e9c4` | `fork-engine-code` | no | **non-goal** ¹⁴ | non-goal |

**Notes**
1. ¹ The large-dictionary algebra filter was relaxed for the TypeDuck Cantonese profile, and a real-dictionary golden now covers the production `jyut6ping3` path. A follow-up fix also keeps generated one-letter abbreviation aliases from acting as interior sentence boundaries without suppressing normal one-letter dictionary codes in other schemas.
2. ² Yune has no librime `SyllableGraph`; abbreviation spellings are flat penalized entries that coexist with normal ones by construction, so the C++ "perfect match disqualifies abbreviation" hazard cannot arise. Architecturally inapplicable.
3. ³ Implemented through the TypeDuck profile's `preedit_format` wiring; partial letter-tone completion keeps raw preedit where the fork does.
4. ⁴ Upstream 1.17.0 has its *own* word completion (`#848`, which the fork excluded from its 1.11.0 merge). Product decision: do not chase exact fork ranking byte parity; preserve upstream completion/ranking, keep long-entry prediction visible, and expose `prediction_weight_threshold` / `prediction_frequency_threshold` plus `prediction_never_first` profile controls. M21-GAP-02 narrows this for the TypeDuck `jyut6ping3` profile only: the v1.1.2 fixture `jyut6ping3-m21-prediction-ranking.json` shows one long prediction interleaved ahead of single-character matches for `santai`, `sigin`, `gwongdung`, and `hoenggong`, so Yune applies a calibrated prediction candidate limit of 1 on that profile without broad fork ranking byte parity.
5. ⁵ M21-GAP-01 provided the future oracle-backed scenario: TypeDuck v1.1.2 composes `loengnincin`, `leoicijyu`, `loengjathau`, `geijatcin`, and `gamjatheoi` as top-1 dictionary sentences while pre-fix Yune composed high-frequency short-piece garbage. Yune now preserves a scoped log-space sentence word-penalty heuristic for the TypeDuck `jyut6ping3` profile path, effectively preferring fewer pieces with frequency as a tiebreak across the six locked fixture cases. M23 made the scoping real by threading the `21.0` penalty through translator config and installing it only behind the TypeDuck `jyut6ping3` profile predicate; default upstream schemas keep the neutral default. M17 later added the upstream `luna_pinyin` null-grammar poet path with its separate `-13.815510557964274` penalty. Do not merge these scoring spaces; broaden either corpus only with fresh oracle fixtures.
6. ⁶ Verified: upstream `translator_commons.{h,cc}` and `table_translator.cc` (33e7814) have no `show_full_code` member/accessor — it is a fork engine addition. Yune ported it (`translator/mod.rs`). The `\v`-prefix + cangjie-root xlit is fork-schema-data on top.
7. ⁷ Implemented data-driven via checked-in OpenCC source dicts; note the chain omits `TSCharactersExt` (immaterial for Cantonese output).
8. ⁸ Yune has no `spelling_hints` suppression, so comments already render unconditionally — the override is a latent no-op until `spelling_hints` exists. Low priority.
9. ⁹ Correction fidelity now includes distance-scaled penalties, minimal-distance filtering, normal-spelling restriction, and dictionary correction gating.
10. ¹⁰ `enable_correction` is wired independently from `enable_completion` in schema install and core translator behavior.
11. ¹¹ The weighted `PreferUserPhrase` gate replaced the flat userdb bonus for equal/longer-code ordering.
12. ¹² Data-driven; no Rust ABI change was needed. The TypeDuck-profile schema asset and goldens carry the Traditional `全形`/`半形` strings.
13. ¹³ Plausibly covered by the M9/HR-6 reverse-lookup schema-prompt parity; not separately verified. Spot-check before claiming.
14. ¹⁴ Yune has no near-key corrector at all, and a compile-time desktop/mobile keymap is meaningless for a platform-agnostic engine. If ever ported, make it data/config-driven.

---

## B. ABI / RIME API surface

| Behavior | Origin | Fork commit(s) | Category | In 1.17.0? | Yune status | Decision |
|---|---|---|---|---|---|---|
| `RimeConfigListAppend{Bool,Int,Double,String}` | TypeDuck | `2944f7d1`,`70b91220` | `fork-engine-code` | no | **done** (impl+tested, exposed through M19 `rime_get_typeduck_profile_api()`, absent from default table by design) | preserve✓ |
| Individually-exported `rime_get_api` symbols | TypeDuck | `980074cb` | (convention) | yes (librime convention) | done (Yune dual-exports) | preserve✓ |
| `start_quick` / `RimeStartQuick` slot | Cantoboard→TD | `a7b7148c`→`02627c08` | `fork-engine-code` | no | **non-goal** (excluded from default 1.17.0 table; profile-only if ever needed) | non-goal |
| `RimeCandidate` + `double quality` field | TypeDuck | `93159863` | `fork-engine-code` | no | **non-goal** (Yune pins upstream 3-pointer ABI; `quality` is internal engine state) | non-goal |

> Per the standing upstream-first rule (D-25), the default `rime_get_api()` table tracks
> upstream 1.17.0; fork-only ABI slots are reserved for a *named TypeDuck profile ABI
> surface*. M19 added the opt-in `rime_get_typeduck_profile_api()` accessor for
> `config_list_append_*`; Windows packaging and real frontend E2E remain separate
> parked work.

---

## C. Build / platform / packaging — deferred platform integration, not engine parity

All catalogued for awareness. These are not porting obligations for the
completed Cantonese engine-parity arc, because Yune has its own Rust build,
deployment, and persistence model. They are also not permanent product
non-goals: if Yune becomes the engine behind a TypeDuck iOS keyboard or an iOS
developer SDK, handle the iOS rows in a separate platform-integration track with
its own packaging, storage, lifecycle, and host-API requirements.

| Behavior | Origin | Commit(s) | Decision |
|---|---|---|---|
| iOS cross-compile, glog bump, Xcode/CMake wiring | Cantoboard | `a9563f7`,`b261736`,`820c4dd` | deferred iOS platform track |
| Disable dict recompilation on iOS | Cantoboard | `1a4a80e3`,`a7b7148c` | deferred iOS deploy/resource policy |
| Prevent schema/config update & dict build on startup | TypeDuck | `24f4b381`(rev),`df41bc9a` | deferred platform deploy policy (Yune controls its own deploy) |
| Schema submodule instead of checked-in minimal data | TypeDuck | `7a1245fe` | deferred packaging/source-management decision |
| WASM path-leakage reduction (`__FILE_NAME__`, `-ffile-prefix-map`) | TypeDuck (uncommitted) | — | non-goal for engine parity; revisit under WASM release hardening if needed |
| leveldb `Schedule()` synchronous tweak; OpenCC Emscripten CMake | TypeDuck (uncommitted) | — | non-goal for engine parity; revisit by target runtime |
| Boost URL / VS env / CI workflow / clang-format | TypeDuck | various | non-goal |

For future iOS work, use `Cantoboard/librime-cantoboard` and
`TypeDuck-HK/librime-ios-build` as provenance for iOS build, static-linking, and
keyboard-host constraints. Do not treat those repositories as a request to port
C++ internals into Yune, change the upstream `RimeApi` table, or extend
`RimeCandidate`; the iOS surface should be a Yune-native package/host contract.

---

## D. Experiments & reverts — DO NOT preserve

Recorded so nobody re-derives a dead end.

| Item | Origin | Status | Decision |
|---|---|---|---|
| `IndexCode` fixed-size struct (Table::Query perf) | Cantoboard | not picked by TypeDuck | do-not-preserve |
| Top-30 `DictEntryIterator` partial sort | Cantoboard | **reverted** on master | do-not-preserve |
| Incremental search / edge-finding side branches | Cantoboard | experimental, not picked | do-not-preserve |
| Sentence caching | Cantoboard | side branch, not picked | do-not-preserve |
| Abbreviation/index encoding side branch | Cantoboard | not picked | do-not-preserve |
| Vector-backed syllable-graph indices | Cantoboard→TD `34e706e2` | C++ micro-opt | do-not-preserve (n/a to Rust) |
| Reverse-lookup word-by-word fallback | Cantoboard `52b09e22` | **not** in v1.1.2 | do-not-preserve |
| Sort reverse lookup by weight | Cantoboard `29bab991` | **not** in v1.1.2 | do-not-preserve |
| Reverted: increase correction penalty, aggressive completion tuning, Windows settings patch | TypeDuck | reverted | do-not-preserve |

---

## Closed product decisions

1. **Prediction ranking:** do not chase full TypeDuck ranking byte parity. Yune keeps
   upstream `1.17.0` ranking behavior except for the M21-GAP-02
   oracle-backed TypeDuck `jyut6ping3` prediction-count limit, preserves
   long-entry completion, and exposes profile controls for raw-weight/frequency
   thresholds plus prediction-never-first behavior.
2. **Composition word penalty:** a scoped M21-GAP-01 word-penalty fix is preserved
   for oracle-backed TypeDuck dictionary sentence composition. This remains narrower
   than full fork ranking byte parity.
3. **`display_languages`:** TypeDuck-Web owns gloss-column language selection. The
   engine keeps stable lookup payload shape/order and does not add engine-side
   language filtering.
4. **iOS support:** iOS keyboard support is a future Yune platform-integration
   track, not part of the completed Cantonese engine-parity backlog. Preserve the
   engine behavior already closed here, then define iOS packaging, resource
   bundling, sandboxed persistence, and host APIs separately.

## Completed arc — "TypeDuck Cantonese engine-parity" backlog

FORK-PARITY-01..09 are implemented or explicitly decided. Future work should preserve
these gates while advancing the upstream-first Track 2 work and any named TypeDuck
profile ABI surface separately.
