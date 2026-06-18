# Roadmap

Yune is a Rust input-method engine that uses **librime as a compatibility
oracle** while building toward an AI-native input engine librime cannot provide.
The strategy: make existing RIME schemas and frontends behave predictably through
Yune, measuring every difference against librime before accepting it, then layer
AI-native behavior on top as a separate product milestone.

> **Compatibility oracle.** Upstream librime is the behavior reference for
> user-visible behavior, schema semantics, ABI contracts, and migration:
> <https://github.com/rime/librime>. Windows-specific behavior is referenced
> against the TypeDuck fork (tag `v1.1.2`):
> <https://github.com/TypeDuck-HK/librime>. (Earlier docs referenced a local
> checkout path; treat the GitHub sources above as canonical.)

**Document map**
- This file — high-level roadmap (what's done, what's next).
- [`analysis.md`](./analysis.md) — founding architecture decisions.
- [`CONVENTIONS.md`](./CONVENTIONS.md) — architecture, stack, structure, coding/testing conventions, integrations, and current risks (one consolidated reference).
- [`decisions.md`](./decisions.md) — the consolidated decision log (standing principles + `D-*` entries).
- [`requirements.md`](./requirements.md) — requirement IDs and their status.
- [`typeduck-windows-backend-requirements.md`](./typeduck-windows-backend-requirements.md) — the parked Windows engine contract to resume after web validation.
- [`plans/`](./plans/) — per-stage implementation plans, findings, build notes, and validation artifacts (finished ones under `plans/archive/`).

> The GSD planning system (`.planning/`) has been retired; its durable content now lives in `decisions.md`, `requirements.md`, and `CONVENTIONS.md`.

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

Detail: [`plans/archive/compat-foundation-summary.md`](./plans/archive/compat-foundation-summary.md), [`plans/refactor-plan.md`](./plans/refactor-plan.md) (module/test ownership rules), and the [`decisions.md`](./decisions.md) log.

### M8: Real frontend validation & benchmarks
- Host-shaped native loader validates the full `rime_get_api` → setup → deploy → schema select → session → key process → context/status → commit → teardown lifecycle.
- Squirrel/macOS and ibus/fcitx Linux paths attempted and documented with reproducible blockers (not claimed as completed native integration).
- Frontend-sensitive benchmark baselines for session lifecycle, per-key processing, deploy/dictionary loading, and userdb learning/sync.
- **Outcome:** *GO WITH CONDITIONS* to begin AI-native candidate/ranking **design**.

Detail: [`plans/archive/real-frontend-validation-plan.md`](./plans/archive/real-frontend-validation-plan.md), [`plans/archive/frontend-validation/`](./plans/archive/frontend-validation/).

---

## In progress

> **Sequencing — web first.** The original plan stands: prove Yune in a real
> **web browser before** expanding to Windows and other native platforms. The
> M9 *NO-GO* was a *tooling* block, not a behavioral one — the engine has never
> actually run in a browser, so that verdict reflects absent evidence, not a
> failed frontend. Finishing browser validation is the current priority. Much of
> the Windows work already done is **shared engine work** (comment shaping,
> Cantonese goldens, the cross-platform baseline fix) and stays; only the
> Windows-*platform*-specific pieces wait their turn.

### M9: TypeDuck-Web browser validation *(current focus)*

Build-out is done — WASM/Emscripten export contract for the `yune_typeduck_*`
adapter, TypeScript bridge/runtime package, browser filesystem + IDBFS
persistence, and an app-shaped E2E seam against upstream TypeDuck-Web. The
Emscripten build now emits loadable `yune-typeduck.js`/`.wasm` glue, and a Node
smoke instantiates it, calls a `yune_typeduck_*` export, and performs an
Emscripten `FS` write/read. **What's missing is the full official browser
validation:** the real TypeDuck schema/E2E path has not been observed end to
end in a browser. A limited local dev-server smoke has run through the adapter,
but it does not replace the Phase 17 browser validation gate.

**Plan:** [`plans/typeduck-web-validation-plan.md`](./plans/typeduck-web-validation-plan.md) — the M9 execution plan.
Detail: [`plans/typeduck-web-adapter.md`](./plans/typeduck-web-adapter.md), [`plans/typeduck-web-integration-findings.md`](./plans/typeduck-web-integration-findings.md), [`plans/archive/ai-native-frontend-readiness.md`](./plans/archive/ai-native-frontend-readiness.md) (superseded NO-GO).

### M10: TypeDuck-Windows native backend *(started early; platform work deferred)*

TypeDuck-Windows (a weasel fork) talks only to the RIME C ABI, so swapping
`librime → Yune` is contained **iff** Yune presents the same ABI surface and
emits the same candidate data. The contract is in
[`typeduck-windows-backend-requirements.md`](./typeduck-windows-backend-requirements.md);
the implementation plan and its execution notes are in
[`plans/yune-windows-contract-implementation-plan.md`](./plans/yune-windows-contract-implementation-plan.md).
A first pass already landed — but under web-first sequencing the
**platform-specific** items (4, and surfacing the new ABI in 1 for the native
build) wait until the web path is validated, while the **shared engine** items
(2, 3) continue because the web path needs them too.

**Status** (workspace tests green and clippy clean, independently verified):

| # | Contract item | State | Notes |
|---|---|---|---|
| 0 | Windows test baseline | ✅ Done | Was 233 failing (timestamp-shape mismatch poisoning a shared test lock); fixed with a cross-platform ctime formatter + poison-tolerant lock. |
| 1 | `config_list_append_{string,bool,int,double}` C ABI | ✅ Done | Implemented with create-on-missing semantics, wired into the `RimeApi` table; field order **verified to match the fork's `rime_api.h`** (right after `config_list_size`). |
| 2 | `RimeCandidate.comment` fork shaping | 🟡 Panel bytes proven; joiner/prompt pending | `dictionary_lookup_filter` emits the `\f\r1,…\r0,…` panel format; transport already existed. The byte-parity test now feeds authored TSV source rows through the real filter and asserts the golden bytes (**non-circular** — fixed on main). Remaining: the `"; "` reverse-lookup join and schema-name-in-prompt still need a dedicated v1.1.2 oracle case, and the source rows are authored in-test rather than extracted from the shipped `.dict.yaml`. |
| 3 | Cantonese/Jyutping parity suite vs v1.1.2 | 🟡 Partial | 2 active golden-locked tests; 5 behaviors (`combine_candidates`/`show_full_code`/`enable_sentence`, completion/prediction, correction, schema-menu hiding, userdb pronunciations) are honestly `#[ignore]`d pending captured goldens. |
| 4 | Native `rime.dll` + `.lib` + headers | 🟡 Scripted, unverified | `scripts/package-typeduck-windows.ps1` + [`plans/yune-windows-native-build.md`](./plans/yune-windows-native-build.md) build/package/smoke-check the artifact, but the build has not been independently verified on an MSVC host. |

The v1.1.2 oracle fixture used for items 2–3 is **genuine captured fork output**
(`crates/yune-core/tests/fixtures/typeduck-v1.1.2/`).

---

## Next

Concrete, in priority order (**web first, then Windows, then other platforms**):

1. **Fix the TypeDuck-Web adapter mismatches** (WI-2 of the M9 plan): update the tracked adapter/patch source, then sync it into the ignored TypeDuck-Web checkout before browser E2E. The tracked `adapter.ts` currently passes the whole candidate object as `text` and reads non-existent context-level `comments`/`highlighted_candidate_index`; align it to the real runtime shape (`candidate.text`, `candidate.comment`, `context.highlighted`).
2. **Run the TypeDuck-Web E2E in a real browser** — composition, candidate paging/selection/deletion, commit, deploy, customize, persistence — and record a *real* GO/NO-GO based on observed behavior. This is the validation the original plan called for.
3. **Keep the loadable WASM artifact green.** The documented build now produces `yune-typeduck.js`/`.wasm` and smokes `cwrap`/`FS`; preserve that gate while browser filesystem and E2E work proceeds.
4. **Land the remaining shared engine parity** (benefits web *and* Windows): the dictionary-panel comment byte-parity is now proven non-circularly from authored source rows — extend it with the `"; "` reverse-lookup joiner and schema-name-in-prompt oracle cases (and, ideally, real `.dict.yaml` rows), and capture the remaining Cantonese goldens to activate the 5 ignored tests.
5. **Keep tracking honest.** (Done on main: the future-dated "verified" claim was removed and the circular parity test reworked; the roll-up is set to partial.) Keep statuses evidence-based as Phase 17 proceeds.
6. **Then Windows, then other platforms.** Once the browser path is validated: verify the native `rime.dll`/`.lib`/headers build on an MSVC host (incl. the `rime_get_api`/`config_list_append_string` smoke check and header field-order parity), then run the real TypeDuck-Windows E2E per the fork's `INTEGRATION_PLAN.md`. Other native frontends (Squirrel/macOS, ibus/fcitx Linux) follow the same engine.

---

## Deferred / future

- **librime C++ plugin ABI** (Lua, octagram, predict, proto): deferred until a concrete frontend or distribution requires it; prefer Yune-native extension points first.
- **AI-native input layer** — a separate product milestone above the compatibility foundation. AI may provide candidates, rerank, use context, and keep memory **only** through source-labeled, local-first, non-blocking interfaces with strict timeout/fallback and privacy policy. AI must never replace or block classic RIME input paths, and must not auto-commit by default. Baseline behavior must work with local/mock providers; remote LLM calls are optional enhancements.

## Principles (carried forward)

- **Oracle, not template.** Treat librime as the behavior oracle, but prefer idiomatic Rust, cleaner abstractions, stronger typing, and deterministic tests over cloning librime's internal C++ structure.
- **Name the behavior.** Before adding a librime-derived mechanism, name the external behavior it protects (user-visible input, frontend ABI, schema/config semantics, or deployed-data compatibility). If it's only an internal librime detail, use a smaller Yune-native design, isolate it behind an adapter, or document a deferral.
- **Own each slice.** Choose the owning implementation module, matching test module, and librime comparison target before writing a compatibility slice. Keep `lib.rs`/`main.rs` as facades.
