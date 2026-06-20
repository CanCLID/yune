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
- [`m00-analysis-founding.md`](./plans/archive/m00-analysis-founding.md) — founding architecture decisions (archived historical snapshot).
- [`CONVENTIONS.md`](./CONVENTIONS.md) — architecture, stack, structure, coding/testing conventions, integrations, and current risks (one consolidated reference).
- [`decisions.md`](./decisions.md) — the consolidated decision log (standing principles + `D-*` entries).
- [`requirements.md`](./requirements.md) — requirement IDs and their status.
- [`typeduck-windows-backend-requirements.md`](./typeduck-windows-backend-requirements.md) - the parked TypeDuck-Windows compatibility-profile contract.
- [`fork-parity-ledger.md`](./fork-parity-ledger.md) — the single source of truth for *every* Cantoboard + TypeDuck fork improvement vs upstream `1.17.0`, with origin, category, and Yune status (done / todo / non-goal). Sourced from [`CANTOBOARD_LIBRIME_REBASE_SUMMARY.md`](./CANTOBOARD_LIBRIME_REBASE_SUMMARY.md) and [`REBASE_SUMMARY_SINCE_D8BC266D.md`](./REBASE_SUMMARY_SINCE_D8BC266D.md).
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

Detail: [`plans/archive/m05-m07-record-compat-foundation-summary.md`](./plans/archive/m05-m07-record-compat-foundation-summary.md), [`plans/archive/m05-m07-record-foundation-refactor.md`](./plans/archive/m05-m07-record-foundation-refactor.md) (module/test ownership rules), and the [`decisions.md`](./decisions.md) log.

### M8: Real frontend validation & benchmarks
- Host-shaped native loader validates the full `rime_get_api` → setup → deploy → schema select → session → key process → context/status → commit → teardown lifecycle.
- Squirrel/macOS and ibus/fcitx Linux paths attempted and documented with reproducible blockers (not claimed as completed native integration).
- Frontend-sensitive benchmark baselines for session lifecycle, per-key processing, deploy/dictionary loading, and userdb learning/sync.
- **Outcome:** *GO WITH CONDITIONS* to begin AI-native candidate/ranking **design**.

Detail: [`plans/archive/m08-plan-real-frontend-validation.md`](./plans/archive/m08-plan-real-frontend-validation.md), [`plans/archive/frontend-validation/`](./plans/archive/frontend-validation/).

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

Detail: [`plans/archive/m09-plan-typeduck-web-validation.md`](./plans/archive/m09-plan-typeduck-web-validation.md), [`plans/m09-reference-typeduck-web-adapter.md`](./plans/m09-reference-typeduck-web-adapter.md), [`plans/archive/m09-findings-typeduck-web-integration.md`](./plans/archive/m09-findings-typeduck-web-integration.md), [`plans/archive/m09-record-ai-native-frontend-readiness.md`](./plans/archive/m09-record-ai-native-frontend-readiness.md) (HR-7 recommendation).

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

Detail: [`plans/m11-design-ai-native.md`](./plans/m11-design-ai-native.md) (living architecture), [`plans/archive/m11-plan-ai-native-cli-slice.md`](./plans/archive/m11-plan-ai-native-cli-slice.md) (S1 record).

### M12: Upstream behavioral parity closeout (upstream `1.17.0`)

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

Detail: [`plans/archive/m12-plan-upstream-oracle-refresh.md`](./plans/archive/m12-plan-upstream-oracle-refresh.md) and [`plans/archive/m12-plan-upstream-behavioral-parity-closeout.md`](./plans/archive/m12-plan-upstream-behavioral-parity-closeout.md).

**Status**:

| # | Work item | State | Notes |
|---|---|---|---|
| 0 | Pin upstream oracle | Done | Upstream `1.17.0` commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4` is the default core target; provenance is checked in and the official Windows MSVC release binary is available for behavioral byte capture. |
| 1 | Fixture naming policy | Done | Fixture manifests and the provenance guard test distinguish `upstream-1.17.0` from `typeduck-v1.1.2`. |
| 2 | TypeDuck assumption audit | Done | Existing TypeDuck-derived behavior is classified in `docs/plans/archive/m12-audit-coverage.md`. |
| 3 | First upstream parity slice | Done | Default `RimeApi` ABI parity was refreshed to `rime/librime 1.17.0`; fork-only `start_quick` and `config_list_append_*` slots are excluded from the core table. |
| 4 | First upstream behavioral fixture | Done | `luna-pinyin-basic.json` is captured from the official upstream `1.17.0` binary and checked by `upstream_luna_pinyin_parity`. |
| 5 | Expanded upstream behavioral fixtures | Done | `luna-pinyin-selection`, `actions`, `reverse-lookup`, `punctuation`, and `options` fixtures are captured from the official release binary with provenance enforced by `oracle_fixture_provenance`. |
| 6 | Full-pipeline parity gates | Done | Active `upstream_luna_pinyin_parity` coverage drives Yune's real parser, dictionary, translator, filter, and Engine paths; unsupported phrase/language-model and processor-only edges are explicit ignored blockers. |

### M13: AI-native frontend exposure

The first test of the product thesis: take M11's completed CLI/core AI layer to a
**real frontend** — default-off, local-first, and gated by the same safety
invariants already proven in the CLI. M11 owns the hard parts (the
`AiCandidateProvider` trait, `MockAiProvider`/`LocalModelProvider`, the
input-keyed merge that pins the top classic candidate at index 0, the
default-sensitive `AiPrivacyPolicy`, and the `MemoryStore` kept outside the
librime `*.userdb` namespace). M13 carries that surface — **unchanged and still
safe** — across the frontend boundary on **TypeDuck-Web**, the only
GO-WITH-CONDITIONS frontend (M9). No core or TypeDuck compatibility behavior
changes.

**Orchestration decision.** M11 requires the per-key path to *never* run provider
code — it only reads an already-staged result. The browser has no CLI host, and
`AiWorker` uses `std::thread`, which does not port to Emscripten. M13 honors the
invariant with **two passes in Rust**: `yune_typeduck_process_key` stays
unchanged and returns the classic response (so AI-off is byte-identical and the
key path never invokes the provider); a new `yune_typeduck_stage_ai` export then
runs the `LocalModelProvider` **synchronously** and stages an input-keyed result,
which the worker requests **after** rendering classic. Classic input is never
delayed; AI rows arrive as a bounded **second-pass update** on the
off-main-thread worker. The async / second-Web-Worker port is deferred.

**Non-goals (deferred).** Remote LLM providers; the async background-worker port;
exposure through Windows or other native frontends; any change to classic-input
defaults.

**Status**:

| # | Work item | State | Notes |
|---|---|---|---|
| 0 | Browser AI orchestration | Done | `process_key` stays provider-free; `yune_typeduck_stage_ai` runs the `LocalModelProvider` in Rust as a second pass after classic renders. |
| 1 | Default-off + opt-in toggle | Done | AI is invisible until enabled; disabling AI clears staged rows for the current input so the visible candidate page returns to classic output. |
| 2 | Source-labeled candidates in the panel | Done | AI rows render after the classic top candidate with `source: "ai:local"` from engine snapshot data aligned to the rendered page; `RimeCandidate` remains unchanged. |
| 3 | Commit-boundary safety in the browser | Done | Space/Return/default commits classic; explicit AI selection never writes librime userdb, and sensitive-default browser context suppresses AI-memory learning. |
| 4 | Privacy in browser context | Done | Browser context has no app/field signal and defaults to **sensitive**; M13 ships local only and keeps remote providers out of scope. |
| 5 | Browser-E2E safety evidence | Done | Native `typeduck_web`, TS runtime tests/build, and the real TypeDuck-Web Playwright M13 scenarios cover AI-off identity, AI labels, no auto-commit, and explicit AI selection. |

**Outcome:** M13 proves the web surface of the product thesis. The M11 safety
invariants (classic-first, non-blocking classic path, no default AI auto-commit,
no userdb leak, privacy-gated local provider, deterministic fallback) now hold
through TypeDuck-Web. AI candidates render as a second-pass update, are labeled,
and never preempt classic index 0. Classic input remains byte-identical with AI
off, and disabling AI clears stale staged rows for the current input. This
supersedes the *Deferred / future* "AI-native frontend exposure" item for the web
surface only; native frontend exposure remains deferred.

Detail: [`plans/archive/m13-plan-ai-native-frontend-exposure.md`](./plans/archive/m13-plan-ai-native-frontend-exposure.md) (execution plan) and [`plans/m11-design-ai-native.md`](./plans/m11-design-ai-native.md) (architecture).

### M14–M16: TypeDuck-Web fork parity

The chosen next arc: complete the **TypeDuck `jyut6ping3` target** so the
TypeDuck-Web example behaves like the fork — the named (A) target from the
*Compatibility goal*, not feature-completeness for its own sake. **Key gap-map
finding:** `jyut6ping3` is dictionary-driven (`script_translator` +
`dictionary_lookup_filter` + `simplifier`); it does **not** use
`poet`/`octagram`/grammar, so this arc needs **golden capture + dictionary-driven
features**, *not* the upstream language model (that is Track 2 / M17). Five
behaviors are fixture-backed in `cantonese_parity.rs`, with browser-only gaps
listed explicitly below.

#### M14 — Capture the TypeDuck v1.1.2 Cantonese goldens

Parameterize the scenario-capable `oracle-rime-probe.cs` (its traits currently
hardcode upstream `1.17.0` identity) — or add a TypeDuck v1.1.2 capture wrapper —
with the **correct v1.1.2 oracle identity** (modules, distribution name/version,
provenance) and a `jyut6ping3` fixture composer, then capture goldens from the
v1.1.2 oracle binary (oracle-measured, non-circular).

| # | Work item | State | Notes |
|---|---|---|---|
| 0 | v1.1.2 capture wrapper | Done | `oracle-rime-probe.cs` has TypeDuck v1.1.2 identity support and `scripts/capture-typeduck-jyutping.ps1` composes provenance-stamped fixtures. |
| 1 | Option-toggle goldens | Done | `jyut6ping3-m14-options.json` captures deploy-time variants for `combine_candidates`, `show_full_code`, and `enable_sentence` at multiple input lengths. |
| 2 | Completion + correction goldens | Done | `jyut6ping3-m14-completion-correction.json` captures completion and correction variants, including the `nri` correction difference. |
| 3 | Schema-menu goldens | Done | `jyut6ping3-m14-schema-menu.json` captures the emitted `RimeGetSchemaList` one-schema vs multi-schema surface; `hide_lone_schema` / `hide_caret` UI decoration is deferred to the M16 browser assertion. |
| 4 | userdb-pronunciation **feasibility spike** | Done | `jyut6ping3-m14-userdb.json` proves levers export is available and captures a learned `nei5` userdb row. |

#### M15 — Dictionary-driven feature parity

Complete. Yune's real engine now passes the M14 dictionary-driven goldens without
adding a language model or changing the upstream ABI.

| # | Work item | State | Notes |
|---|---|---|---|
| 0 | `combine_candidates` | Done | Same-text rows coalesce with multi-primary TypeDuck dictionary comments when `translator/combine_candidates` is enabled; separate mode remains available. |
| 1 | `show_full_code` | Done | Affix-aware table lookup and cangjie short/full-code comments match the M14 side-lookup fixture. |
| 2 | `enable_sentence` | Done | Viterbi dictionary sentence candidates plus sentence-aware lookup comments reproduce the `ngohaigo` M14 row. |
| 3 | completion + correction | Done | Completion/correction fixture paths are active in `cantonese_parity`; correction uses the real spelling-algebra path. |
| 4 | OpenCC `hk2s` data | Done | `SimplifierFilter` now loads checked-in OpenCC source dictionaries for the `hk2s` chain instead of a hardcoded char slice. |

#### M16 — TypeDuck-Web fork-parity validation

Complete with documented browser-surface limits. The real TypeDuck-Web Playwright
matrix now covers the browser-supported `jyut6ping3_mobile` surface against the
M14 goldens where the app exposes it: default combined candidates, sentence
composition, completion, simplification, the existing M9 smoke flows, and the
M13 default-off AI scenarios. M15 remains the authoritative real-engine proof
for deploy-only variants (`common:/separate_candidates`, `common:/show_full_code`)
and correction details that TypeDuck-Web does not expose as independent browser
selectors. Schema-menu hiding and per-entry userdb pronunciation remain explicit
browser/userdb inspection gaps, backed by the M14 emitted-schema-list and levers
export fixtures rather than claimed as browser UI coverage.

Detail: [`plans/archive/m14-plan-typeduck-v112-golden-capture.md`](./plans/archive/m14-plan-typeduck-v112-golden-capture.md), [`plans/archive/m15-plan-typeduck-dictionary-driven-parity.md`](./plans/archive/m15-plan-typeduck-dictionary-driven-parity.md), and [`plans/archive/m16-plan-typeduck-web-parity-validation.md`](./plans/archive/m16-plan-typeduck-web-parity-validation.md).

> **Scope of M14–M16, and what it did *not* cover.** M14–M16 closed the *captured*
> `jyut6ping3_mobile` browser surface. A fuller audit of **all** Cantoboard + TypeDuck
> fork improvements vs upstream `1.17.0` now lives in
> [`fork-parity-ledger.md`](./fork-parity-ledger.md). Two corrections it records:
> (1) **F2 (`santai`→身體/身體健康 prefix prediction) and F4 (auto-compose-only-as-fallback)
> are upstream `1.17.0` behaviors, not fork inventions** — Yune preserves them by tracking
> upstream, not by porting fork code; only the fork's *prediction ranking* differs.
> (2) **The Cantonese 容錯 (fuzzy) ruleset was previously stripped on dictionaries >50k
> entries** (`schema_install.rs:237-260`); the production `jyut6ping3_scolar` dict is
> ~127k rows, so this became the highest-priority TypeDuck Cantonese engine-parity
> backlog item and is now covered by a real-dictionary golden.
>
> **Backlog closeout:** FORK-PARITY-01..09 are now implemented or explicitly decided.
> F08 keeps upstream `1.17.0` ranking semantics while preserving long-entry prediction
> and adding profile controls for prediction thresholds / never-first behavior. F09 is
> intentionally UI-side for TypeDuck-Web display-language selection.

### M20: Web demo showcase controls

M20 turned this repo's patched TypeDuck-Web harness into Yune's canonical
internal browser playground for demoing, stress-testing, and comparing the
behavior already proven by M9, M13, M14-M16, and the FORK-PARITY backlog. It is a
**separate web/demo track**,
not a reopened M13: M13 remains the completed default-off AI frontend exposure
milestone, while M20 made the browser demo highly controllable, honest,
inspectable, and useful for manual dogfooding before deeper M17-M19 work.

This is not the same surface as a separately cloned `TypeDuck-HK/TypeDuck-Web`
product checkout. `packages/yune-typeduck-runtime/` remains the reusable Yune
runtime bridge, `third_party/typeduck-web/` is the internal patched harness, and
the real TypeDuck-Web web IME product should get its own future integration
track.

The milestone exposes only controls backed by real runtime behavior:
schema/deploy-time knobs through `customize()` (`enable_completion`,
`enable_correction`, `enable_sentence`, learning, `combine_candidates`,
`prediction_never_first`, and one measured, fine-grained prediction threshold
with a real-assets-calibrated `santai` cutoff and documented range bounds) and
live session options through `setOption()` (`ascii_mode`, `full_shape`,
`simplification`). Browser-visible controls have before/after assertions,
including learned prediction ranking for `prediction_never_first`; Input Memory
records a visible learned-prediction on-state but an explicit browser-surface
N/A for memory-off suppression, and Auto-correction records explicit
browser-surface N/A for `nri` candidate rendering while retaining
`cantonese_parity` as the engine proof. Deploy-time controls whose current
browser panel effect is not independently visible keep persisted
`jyut6ping3_mobile.custom.yaml` assertions, without being counted as
candidate-output proof. Display-only controls are grouped separately and prove
visible rendering changes for the current browser-reachable display surface.
Static or
default-on engine features such as `santai`
-> `身體健康`, Cantonese fuzzy/容錯, letter-to-tone input, reverse lookup,
`show_full_code` reachability/N-A, and AI second-pass behavior are presented as
guided scenarios rather than fake toggles. The M20 UI keeps grouped candidates
as a documented demo default, while the checked-in raw mobile assets still
record the `common:/separate_candidates` patch. For the current
`jyut6ping3_mobile` browser schema, reverse/Cangjie/show-full-code evidence
records the missing `cangjie` namespace as N/A instead of exposing a fake
control. `ascii_punct` is explicitly not exposed as a working toggle until M18
implements the processor-level behavior.
As Yune adds browser-safe engine features, this playground should gain either an
active control or a guided scenario so the web surface stays useful for future
regression hunts and product demos.

Evidence lives under
`third_party/typeduck-web/e2e/results/m20-showcase-controls/`. The completed
plan is archived at
[`plans/archive/m20-plan-web-demo-showcase-controls.md`](./plans/archive/m20-plan-web-demo-showcase-controls.md).

---

## Parked

### M10: TypeDuck-Windows native backend

TypeDuck-Windows remains valuable, but it is no longer the active core-engine
priority. Its work is parked as a TypeDuck compatibility profile until Yune has
a named TypeDuck profile ABI surface.

Archived pre-M12 M10 evidence is preserved: Windows test trust, fork-only
`config_list_append_*` helper behavior, current TypeDuck comment shaping
fixtures, and a historical native `rime.dll`/`.lib`/headers package smoke. That
package smoke is not an active or valid gate for the default upstream
`rime_get_api()` table after M12. Remaining TypeDuck-Windows work is still
blocked by a named profile ABI surface and the real TypeDuck-Windows frontend
E2E; the TypeDuck-Web Cantonese gaps are now fixture-backed under M14-M16 with
engine coverage and explicit browser/userdb limits.

Detail: [`typeduck-windows-backend-requirements.md`](./typeduck-windows-backend-requirements.md),
[`plans/m10-reference-typeduck-windows-contract.md`](./plans/m10-reference-typeduck-windows-contract.md),
and [`plans/m10-reference-typeduck-windows-native-build.md`](./plans/m10-reference-typeduck-windows-native-build.md).

## Concrete next steps

In priority order:

1. **Preserve the upstream-first baseline.** Keep default `RimeApi` and core behavior aligned to upstream `1.17.0`; add new TypeDuck fork-only behavior only behind an explicit profile surface.
2. **Keep M9/M13/M16/M20 web gates green on merge.** Preserve the reproducible Emscripten build, TypeScript runtime tests/build, TypeDuck-Web worker build, real-assets browser evidence, native `typeduck_web` fallback, default-off M13 AI scenarios, and M20 showcase-control honesty checks.
3. **Advance Track 2 (M17-M19) opportunistically.** The upstream language model, prism generation, deployment-write, and breadth schemas now follow the upstream-first scope ledger after the M14-M16 TypeDuck-Web closeout and M20 browser playground work.
4. **Extend the M20 playground only with browser-safe supported features.** Add active controls or guided scenarios for new browser-safe engine behavior, and keep unsupported behavior absent or documented instead of partially exposed.
5. **Resume TypeDuck profile work only with a named surface.** Return to TypeDuck-Windows packaging after the profile ABI is defined and fork-header slot smoke is re-derived.
6. **Add a future TypeDuck-Web product-integration track before changing a separately cloned TypeDuck-Web product checkout.** Treat `TypeDuck-HK/TypeDuck-Web` as the dedicated web IME product, not as the M20 harness or the runtime bridge.
7. **Add a future iOS keyboard-developer track before TypeDuck iOS work starts.** Treat the Cantoboard/TypeDuck iOS build repositories as platform-integration provenance, not as engine-parity code to port. The track should define Yune-native iOS packaging, Swift/Obj-C host bindings, resource bundling, sandboxed userdb/storage, keyboard-extension lifecycle limits, and mobile-specific configuration hooks.

---

## Planned / Next up

Priority is set by what a *named* (A)/(B) target needs, not by librime's feature
list. **TypeDuck `jyut6ping3` reconciliation (M14–M16) and the M20 browser
playground are complete** (see *Completed* above). The remaining engine-depth arc
is **Track 2 (broad upstream depth):**

- **M17 - Upstream sentence / language model (poet)** - the statistical LM for
  `luna_pinyin` sentence/lattice parity (the two blocked upstream tests). The
  heavy item; *not* required for TypeDuck-Web parity.
- **M18 — Deployment & processor depth** — spelling-algebra prism generation, a
  public binary-dictionary write API, and the `ascii_punct` / immediate-commit
  punctuation processor behaviors.
- **M19 — Breadth (toward B)** — more upstream schemas through the M12 harness
  (Shuangpin, Cangjie, Zhuyin); resume TypeDuck-Windows.
- **M21 — TypeDuck-Web product comparison** — a documented protocol (off the parity
  critical path) comparing the Yune harness against the deployed `typeduck.hk/web`
  product as a behavior/feel target (the `v1.1.2` fixtures stay the hard oracle); runs
  after M20. See [`plans/m21-plan-typeduck-web-product-comparison.md`](./plans/m21-plan-typeduck-web-product-comparison.md).
- **AI-native frontend expansion** — the proven TypeDuck-Web surface stays
  default-off; Windows and other native frontend exposure wait for their own
  safety evidence.
- **iOS keyboard developer track** — future TypeDuck iOS or third-party keyboard
  work should build on the closed Cantonese engine-parity behavior, then define a
  Yune-native iOS package/host contract. Cantoboard/TypeDuck iOS build repos are
  reference material for static-linking, resource deployment, and keyboard-host
  constraints, not a C++ implementation template.

## Scope ledger

A living map so "parity" always names a target. Deferred rows move into *in
scope* only as a named target needs them; nothing here commits to a timeline, and
the *Non-goal* column is not a backlog. Standing deferrals also appear in
*Deferred / future* below.

| In scope — target-driven, measured | Deferred — implement when a target needs it | Non-goal |
|---|---|---|
| `luna_pinyin` core vs upstream `1.17.0` oracle | Grammar / language model (poet / octagram); processor-level punctuation/ascii-punctuation parity | Bit-for-bit parity with librime internals |
| TypeDuck `jyut6ping3` profile vs `v1.1.2` oracle | Browser/userdb UI evidence after M15 engine parity; broader OpenCC phrase/config breadth beyond the checked-in `hk2s` source chain | librime C++ plugin ABI as a requirement |
| Common RIME schemas, as breadth (B) is added | Spelling-algebra prism generation; binary-dict / deployment writing | Cloud inference as a hard dependency |
| AI-native layer (M11) on the compatible base | `contextual_translation`, `unity_table_encoder`, deeper gear coverage | Replacing or altering classic input paths by default |
| TypeDuck-Web / TypeDuck profile surfaces, when named | iOS keyboard developer SDK: package format, Swift/Obj-C host API, resource bundle/deploy model, sandboxed userdb/storage, and mobile config hooks | Treating iOS build scripts as engine semantics |

---

## Deferred / future

- **librime C++ plugin ABI** (Lua, octagram, predict, proto): deferred until a concrete frontend or distribution requires it; prefer Yune-native extension points first.
- **AI-native input layer (future native/frontend expansion)** - after M13, TypeDuck-Web has a default-off local AI surface with browser safety evidence. Remaining AI-native product integration is exposing equivalent gates in additional real frontends without changing upstream-core, TypeDuck-Web classic behavior, or parked TypeDuck-Windows compatibility behavior. The architecture remains in [`plans/m11-design-ai-native.md`](./plans/m11-design-ai-native.md); CLI evidence lives in [`plans/archive/m11-plan-ai-native-cli-slice.md`](./plans/archive/m11-plan-ai-native-cli-slice.md), and web exposure evidence lives in [`plans/archive/m13-plan-ai-native-frontend-exposure.md`](./plans/archive/m13-plan-ai-native-frontend-exposure.md).
- **iOS keyboard developer support** - future TypeDuck iOS work needs its own
  track. The fork-parity ledger now classifies the Cantoboard/TypeDuck iOS build
  rows as deferred platform integration: useful for Apple build/static-linking,
  resource deployment, and keyboard-extension constraints, but outside the
  completed engine-parity backlog.

## Principles (carried forward)

The standing principles that govern all current and future work — librime as oracle not template, name-the-protected-behavior, own-each-slice, AI-native as a separate local-first layer, fixtures before module replacement, deferred plugin ABI, and upstream-first oracle sequencing — have one canonical home: [`decisions.md` → Standing principles](./decisions.md#standing-principles).
