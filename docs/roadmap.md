# Roadmap

Yune is a Rust input-method engine that uses **librime as a compatibility
oracle** while building toward an AI-native input engine librime cannot provide.
The strategy: make existing RIME schemas and frontends behave predictably through
Yune, measuring every difference against librime before accepting it, then layer
AI-native behavior on top as a separate product milestone.

> **Compatibility oracle.** Upstream librime latest stable is the default core
> behavior reference for user-visible behavior, schema semantics, standard ABI
> contracts, deployed data, and migration. The current pinned upstream target is
> `rime/librime 1.17.0`
> (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`):
> <https://github.com/rime/librime>. TypeDuck-specific behavior is referenced
> only as a compatibility profile against the TypeDuck fork (tag `v1.1.2`,
> commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`):
> <https://github.com/TypeDuck-HK/librime>. (Earlier docs referenced a local
> checkout path; treat the GitHub sources above as canonical.)

**Document map**
- This file — high-level roadmap (what's done, what's next).
- [`analysis.md`](./plans/archive/analysis.md) — founding architecture decisions (archived historical snapshot).
- [`CONVENTIONS.md`](./CONVENTIONS.md) — architecture, stack, structure, coding/testing conventions, integrations, and current risks (one consolidated reference).
- [`decisions.md`](./decisions.md) — the consolidated decision log (standing principles + `D-*` entries).
- [`requirements.md`](./requirements.md) — requirement IDs and their status.
- [`typeduck-windows-backend-requirements.md`](./typeduck-windows-backend-requirements.md) - the parked TypeDuck-Windows compatibility-profile contract.
- [`plans/`](./plans/) — per-stage implementation plans, findings, build notes, and validation artifacts (finished ones under `plans/archive/`).

> The GSD planning system (`.planning/`) has been retired; its durable content now lives in `decisions.md`, `requirements.md`, and `CONVENTIONS.md`.

---

## Compatibility goal — oracle as a floor, not a feature checklist

Yune treats librime as a **behavioral oracle, not a feature target.** Success is
**not** "reimplement 100% of librime 1.17.0" — that would contradict the standing
*"oracle, not template"* principle and add no product value, since librime
already exists. Success is that **the schemas and frontends Yune targets behave
correctly against the oracle**, every difference measured and either fixed or
documented, with the **AI-native layer librime cannot provide** riding on top of
that compatible base.

The goal has two horizons and one explicit non-goal:

- **(A) Target-driven compatibility — the near-term definition of "done."** A
  bounded, *named* set of schemas and frontends must behave correctly versus the
  oracle. Today that set is the `luna_pinyin` core path (vs the upstream `1.17.0`
  oracle) and the TypeDuck `jyut6ping3` profile (vs the `v1.1.2` oracle). "Done"
  is always relative to this named list — never an open-ended checklist. This is
  what M12 begins.
- **(B) Broad RIME compatibility — the expansion ambition.** Over time, widen the
  named set so the **common** RIME schemas and the **real third-party frontends**
  (ibus / fcitx / Squirrel / weasel, TypeDuck-Web / Windows) work predictably.
  Breadth is added schema-by-schema and frontend-by-frontend through the same
  oracle-measured parity harness — not in one leap, and not by cloning librime
  wholesale.
- **Non-goal: bit-for-bit feature parity with librime internals.** Reproducing
  every librime gear, plugin, and code path is out of scope. A librime feature is
  implemented only when a named (A)/(B) target needs it (*"name the behavior"*).

So **"the engine is done" is never absolute** — it reads *"the current target set
is green against the oracle; everything else is deferred-and-documented."* The
product north star is the AI-native layer on top of a compatible base, not parity
for its own sake. *(Ratified as a standing principle and `D-25` in
[`decisions.md`](./decisions.md#standing-principles).)*

---

## Completed

### M0–M4: Foundation
- Rust workspace (`yune-core`, `yune-schema`, `yune-rime-api`, `yune-cli`); core session/candidate types; CLI smoke test.
- Deterministic compatibility harness: recorded fixtures, JSON output for context/candidates/commit/status, workspace tests.
- RIME-style schema subset: processors, segmentors, translators, filters as named components; config patch/include behavior.
- Table dictionary prototype with deterministic lookup and ranking.
- Non-blocking candidate reranking trait with a mock ranker; classic ordering preserved as fallback.

### M5–M7: RIME ABI, schema, and data compatibility
- Focused RIME-style C ABI (`yune-rime-api`): sessions, context/status/commit, config, levers, schema lists, deployment, modules, runtime options, key processing — driven through the exported `RimeApi` function table.
- Broad librime-compatible key-table coverage and aligned core/ABI key handling.
- Schema-loaded compatibility across the high-value processor/segmentor/translator/filter set (speller, editor, navigator, selector, chord, punctuation, shape; abc/ascii/affix/matcher segmentors; table/script/r10n/reverse-lookup/history/switch/schema-list translators; simplifier/uniquifier/charset/reverse-lookup filters).
- Source `.dict.yaml` parsing aligned with librime/yaml-cpp edge cases; `import_tables`; preset vocabulary; table-encoder primitives; checksum/rebuild-plan groundwork.
- Compiled `.table.bin`/`.prism.bin`/`.reverse.bin` payload consumption and rebuild execution; correction/tolerance data in the compiled path.
- UserDB compatibility beyond the plain-text shim: storage, snapshot/restore/recovery/sync, transaction rollback, learning, frequency updates, predictive lookup.

Detail: [`plans/archive/compat-foundation-summary.md`](./plans/archive/compat-foundation-summary.md), [`plans/archive/refactor-plan.md`](./plans/archive/refactor-plan.md) (module/test ownership rules), and the [`decisions.md`](./decisions.md) log.

### M8: Real frontend validation & benchmarks
- Host-shaped native loader validates the full `rime_get_api` → setup → deploy → schema select → session → key process → context/status → commit → teardown lifecycle.
- Squirrel/macOS and ibus/fcitx Linux paths attempted and documented with reproducible blockers (not claimed as completed native integration).
- Frontend-sensitive benchmark baselines for session lifecycle, per-key processing, deploy/dictionary loading, and userdb learning/sync.
- **Outcome:** *GO WITH CONDITIONS* to begin AI-native candidate/ranking **design**.

Detail: [`plans/archive/real-frontend-validation-plan.md`](./plans/archive/real-frontend-validation-plan.md), [`plans/archive/frontend-validation/`](./plans/archive/frontend-validation/).

### M9: TypeDuck-Web browser validation

- Emscripten build emits loadable `yune-typeduck.js`/`.wasm` glue, and a Node
  smoke instantiates it, calls a `yune_typeduck_*` export, and performs an
  Emscripten `FS` write/read.
- TypeDuck-Web adapter maps runtime `candidate.text`, `candidate.comment`, and
  `context.highlighted` into the upstream candidate panel shape.
- Patched TypeDuck-Web worker calls the modular Emscripten factory, mounts IDBFS,
  fetches real `public/schema` assets before init, and runs in a real browser.
- **Browser result:** the HR-5 real-assets matrix passes for composition,
  candidate list, paging, selection, deletion, Space commit, phrase commit,
  deploy, customize, persistence sync, reload survival, and dictionary-panel
  rendering against `jyut6ping3_mobile`; the committed byte-parity guarantee
  for rich dictionary comments is the `cantonese_parity` fixture, with the
  browser-shaped native rich-comment test enabled when local v1.1.2 oracle
  build assets are present. HR-6 also locks the shared reverse-lookup `"; "`
  joiner and schema-prompt bytes against the TypeDuck v1.1.2 oracle.
- **Outcome:** **GO WITH CONDITIONS** for AI-native frontend exposure. This
  supersedes the old tooling-blocked Phase 10 NO-GO and the interim hardening
  NO-GO: real browser compatibility is proven, but AI-native behavior remains
  disabled by default in real frontends until the M11 provider/ranking/privacy
  contracts are proven and explicitly enabled.

Detail: [`plans/archive/typeduck-web-validation-plan.md`](./plans/archive/typeduck-web-validation-plan.md), [`plans/typeduck-web-adapter.md`](./plans/typeduck-web-adapter.md), [`plans/archive/typeduck-web-integration-findings.md`](./plans/archive/typeduck-web-integration-findings.md), [`plans/archive/ai-native-frontend-readiness.md`](./plans/archive/ai-native-frontend-readiness.md) (HR-7 recommendation).

### M11: AI-native input layer — S1–S5 CLI/core complete *(2026-06-18; frontend exposure deferred)*

The AI-native layer (M11) is implemented in `crates/yune-core` and the direct
`yune-cli run` path only, leaving the TypeDuck-Web and TypeDuck-Windows frontend surfaces unchanged. The
core exposes an `AiCandidateProvider` interface, deterministic `MockAiProvider`,
and an `AiWorker` (provider execution is CLI-orchestrated outside
`Engine::refresh_candidates`; the engine consumes only staged, input-keyed
results); structured `Ai { provider, confidence }` source metadata with
fixed-point confidence; one merge function that pins the top classic candidate
at index 0; a default-sensitive `AiPrivacyPolicy` that blocks remote providers
before invocation and gates learning; an inspectable / clearable / disable-able
`MemoryStore` kept **outside** the librime `*.userdb` namespace; and a
deterministic local rule-backed provider (`yune-cli run --ai-provider local`).
All eight S1–S5 safety criteria are independently verified — source-labeled,
classic-first, non-blocking, no default auto-commit, **no userdb leak**,
privacy-gated, deterministic fallback. Real frontend exposure remains deferred
and default-off (see *Deferred / future*).

Detail: [`plans/ai-native-design.md`](./plans/ai-native-design.md) (living architecture), [`plans/archive/ai-native-cli-slice-plan.md`](./plans/archive/ai-native-cli-slice-plan.md) (S1 record).

---

## Current baseline - M12: Upstream Behavioral Parity Closeout complete

Yune's core engine now tracks upstream `rime/librime 1.17.0` as the default
oracle target. M12 turned TypeDuck behavior into an explicit compatibility
profile instead of the default engine truth. The expanded M12 closeout captures
`luna_pinyin` behavior from the official upstream Windows MSVC release binary
and checks Yune against those bytes for curated single-code mechanics, full
`ni` dictionary selection with essay weights, Engine paging/selection/commit,
reverse lookup, punctuation/symbol candidates, and supported option paths
(`zh_hans` single-code conversion and full-shape punctuation first candidate).
The phrase/language-model surface (`zhongguo` full-page sentence output),
`ascii_punct` processor bypass, and punctuation immediate-commit processor
behavior are fixture-backed ignored blockers, not hidden parity claims.

Detail: [`plans/archive/upstream-oracle-refresh.md`](./plans/archive/upstream-oracle-refresh.md) and [`plans/archive/m12-upstream-behavioral-parity-closeout.md`](./plans/archive/m12-upstream-behavioral-parity-closeout.md).

**Status**:

| # | Work item | State | Notes |
|---|---|---|---|
| 0 | Pin upstream oracle | Done | Upstream `1.17.0` commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4` is the default core target; provenance is checked in and the official Windows MSVC release binary is available for behavioral byte capture. |
| 1 | Fixture naming policy | Done | Fixture manifests and the provenance guard test distinguish `upstream-1.17.0` from `typeduck-v1.1.2`. |
| 2 | TypeDuck assumption audit | Done | Existing TypeDuck-derived behavior is classified in `docs/plans/archive/m12-coverage-audit.md`. |
| 3 | First upstream parity slice | Done | Default `RimeApi` ABI parity was refreshed to `rime/librime 1.17.0`; fork-only `start_quick` and `config_list_append_*` slots are excluded from the core table. |
| 4 | First upstream behavioral fixture | Done | `luna-pinyin-basic.json` is captured from the official upstream `1.17.0` binary and checked by `upstream_luna_pinyin_parity`. |
| 5 | Expanded upstream behavioral fixtures | Done | `luna-pinyin-selection`, `actions`, `reverse-lookup`, `punctuation`, and `options` fixtures are captured from the official release binary with provenance enforced by `oracle_fixture_provenance`. |
| 6 | Full-pipeline parity gates | Done | Active `upstream_luna_pinyin_parity` coverage drives Yune's real parser, dictionary, translator, filter, and Engine paths; unsupported phrase/language-model and processor-only edges are explicit ignored blockers. |

---

## Parked - M10: TypeDuck-Windows native backend

TypeDuck-Windows remains valuable, but it is no longer the active core-engine
priority. Its work is parked as a TypeDuck compatibility profile until Yune has
a named TypeDuck profile ABI surface.

Archived pre-M12 M10 evidence is preserved: Windows test trust, fork-only
`config_list_append_*` helper behavior, current TypeDuck comment shaping
fixtures, and a historical native `rime.dll`/`.lib`/headers package smoke. That
package smoke is not an active or valid gate for the default upstream
`rime_get_api()` table after M12. Remaining TypeDuck work is still blocked by
five uncaptured v1.1.2 Cantonese/Jyutping goldens and the real TypeDuck-Windows
frontend E2E.

Detail: [`typeduck-windows-backend-requirements.md`](./typeduck-windows-backend-requirements.md),
[`plans/yune-windows-contract-implementation-plan.md`](./plans/yune-windows-contract-implementation-plan.md),
and [`plans/yune-windows-native-build.md`](./plans/yune-windows-native-build.md).

## Concrete next steps

In priority order:

1. **Preserve the upstream-first baseline.** Keep default `RimeApi` and core behavior aligned to upstream `1.17.0`; add new TypeDuck fork-only behavior only behind an explicit profile surface.
2. **Broaden upstream parity schema-by-schema.** Reuse the M12 fixture/provenance harness for the next named upstream schema or for parked `luna_pinyin` blockers when a target needs them.
3. **Keep M9 web gates green on merge.** Preserve the reproducible Emscripten build, TypeScript runtime tests/build, TypeDuck-Web worker build, real-assets browser evidence, and native `typeduck_web` fallback.
4. **Keep AI frontend exposure separate and default-off.** M11's CLI/core layer is complete; any future TypeDuck-Web, Windows, or other frontend exposure needs a new explicit plan and must preserve compatibility gates.
5. **Resume TypeDuck profile work only with a named surface.** Return to TypeDuck-Windows packaging after the profile ABI is defined and fork-header slot smoke is re-derived.

---

## Beyond M12 — trajectory & scope ledger

Priority is set by what a *named* (A)/(B) target needs, not by librime's feature
list. The broad arc after M12's first behavioral-parity slice:

1. **Breadth (toward B)** — bring more target schemas through the M12 parity
   harness (e.g. Shuangpin, Cangjie, Zhuyin), each measured against the upstream
   `1.17.0` oracle.
2. **Depth** — close the parity-*blocking* engine gaps, but only as a target
   demands them: the grammar / language model (sentence quality),
   processor-level punctuation/ascii-punctuation behavior, spelling-algebra
   prism generation, and binary-dictionary / deployment writing. These are
   today's known gaps (see the ledger).
3. **TypeDuck profile reconciliation** — un-ignore the five Cantonese / Jyutping
   goldens and decide, per difference, fix-in-core versus keep-as-profile.
4. **AI-native frontend exposure** — M11's CLI/core layer → real frontends,
   default-off until proven (see *Deferred / future*).

### Scope ledger

A living map so "parity" always names a target. Deferred rows move into *in
scope* only as a named target needs them; nothing here commits to a timeline, and
the *Non-goal* column is not a backlog. Standing deferrals also appear in
*Deferred / future* below.

| In scope — target-driven, measured | Deferred — implement when a target needs it | Non-goal |
|---|---|---|
| `luna_pinyin` core vs upstream `1.17.0` oracle | Grammar / language model (poet / octagram); processor-level punctuation/ascii-punctuation parity | Bit-for-bit parity with librime internals |
| TypeDuck `jyut6ping3` profile vs `v1.1.2` oracle | Broader OpenCC dictionary coverage beyond the currently covered M12 `zh_hans` slice | librime C++ plugin ABI as a requirement |
| Common RIME schemas, as breadth (B) is added | Spelling-algebra prism generation; binary-dict / deployment writing | Cloud inference as a hard dependency |
| AI-native layer (M11) on the compatible base | `contextual_translation`, `unity_table_encoder`, deeper gear coverage | Replacing or altering classic input paths by default |

---

## Deferred / future

- **librime C++ plugin ABI** (Lua, octagram, predict, proto): deferred until a concrete frontend or distribution requires it; prefer Yune-native extension points first.
- **AI-native input layer (future frontend exposure)** - after the completed S1-S5 CLI/core mock/provider, worker/confidence, context/privacy, memory, and local-model slices, remaining AI-native work is product integration: exposing AI behind explicit defaults in real frontends without changing upstream-core, TypeDuck-Web, or parked TypeDuck-Windows compatibility behavior. The architecture remains in [`plans/ai-native-design.md`](./plans/ai-native-design.md); S1 evidence and checklist live in [`plans/archive/ai-native-cli-slice-plan.md`](./plans/archive/ai-native-cli-slice-plan.md).

## Principles (carried forward)

The standing principles that govern all current and future work — librime as oracle not template, name-the-protected-behavior, own-each-slice, AI-native as a separate local-first layer, fixtures before module replacement, deferred plugin ABI, and upstream-first oracle sequencing — have one canonical home: [`decisions.md` → Standing principles](./decisions.md#standing-principles).
