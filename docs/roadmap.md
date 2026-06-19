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
`yune-cli run` path only, leaving the M9/M10 frontend surfaces unchanged. The
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

## Next up — M10: TypeDuck-Windows native backend

Web-first is now satisfied (M9 complete), so the previously **parked Windows
platform work can resume.** TypeDuck-Windows (a weasel fork) talks only to the
RIME C ABI, so swapping `librime → Yune` is contained **iff** Yune presents the
same ABI surface and emits the same candidate data. The contract is in
[`typeduck-windows-backend-requirements.md`](./typeduck-windows-backend-requirements.md);
the implementation plan and its execution notes are in
[`plans/yune-windows-contract-implementation-plan.md`](./plans/yune-windows-contract-implementation-plan.md).
A first pass already landed; the shared engine work (comment shaping, Cantonese
goldens, the cross-platform baseline) benefits both web and Windows. What remains
is the platform-specific native package and the real TypeDuck-Windows E2E.

**Status** (workspace tests green and clippy clean, independently verified):

| # | Contract item | State | Notes |
|---|---|---|---|
| 0 | Windows test baseline | ✅ Done | Was 233 failing (timestamp-shape mismatch poisoning a shared test lock); fixed with a cross-platform ctime formatter + poison-tolerant lock. |
| 1 | `config_list_append_{string,bool,int,double}` C ABI | ✅ Done | Implemented with create-on-missing semantics, wired into the `RimeApi` table; field order **verified to match the fork's `rime_api.h`** (right after `config_list_size`). |
| 2 | `RimeCandidate.comment` fork shaping | ✅ Covered for current v1.1.2 slices | `dictionary_lookup_filter` emits the `\f\r1,…\r0,…` panel format; `cantonese_parity` byte-asserts rich dictionary comments against the committed v1.1.2 fixture, the HR-5 browser-shaped native test byte-asserts the full real-assets path when local oracle build assets are present, and HR-6 locks the `"; "` reverse-lookup join plus schema-name prompt/preedit bytes against a dedicated v1.1.2 oracle fixture. |
| 3 | Cantonese/Jyutping parity suite vs v1.1.2 | 🟡 Partial | 4 active golden-locked tests; 5 behaviors (`combine_candidates`/`show_full_code`/`enable_sentence`, completion/prediction, correction, schema-menu hiding, userdb pronunciations) are honestly `#[ignore]`d pending captured goldens. |
| 4 | Native `rime.dll` + `.lib` + headers | 🟡 Scripted, unverified | `scripts/package-typeduck-windows.ps1` + [`plans/yune-windows-native-build.md`](./plans/yune-windows-native-build.md) build/package/smoke-check the artifact, but the build has not been independently verified on an MSVC host. |

The v1.1.2 oracle fixture used for items 2–3 is **genuine captured fork output**
(`crates/yune-core/tests/fixtures/typeduck-v1.1.2/`).

---

## Concrete next steps

In priority order (**web first, then Windows, then other platforms**):

1. **Keep the M9 web gates green on merge.** Preserve the reproducible Emscripten build, TypeScript runtime tests/build, TypeDuck-Web worker build, real-assets browser evidence, and native `typeduck_web` fallback.
2. **Capture the remaining shared Cantonese goldens.** Five fork-specific cases remain explicit ignored blockers pending TypeDuck v1.1.2 oracle captures: option-combination behavior, completion/prediction, correction, schema-menu hiding, and per-entry userdb pronunciations.
3. **Resume Windows, then other platforms.** Verify the native `rime.dll`/`.lib`/headers build on an MSVC host, including `rime_get_api`/`config_list_append_string` smoke and header field-order parity, then run the real TypeDuck-Windows E2E per the fork's `INTEGRATION_PLAN.md`. Other native frontends follow the same engine.
4. **Keep AI frontend exposure separate and default-off.** M11's CLI/core layer is complete; any future TypeDuck-Web, Windows, or other frontend exposure needs a new explicit plan and must preserve M9/M10 compatibility gates.

---

## Deferred / future

- **librime C++ plugin ABI** (Lua, octagram, predict, proto): deferred until a concrete frontend or distribution requires it; prefer Yune-native extension points first.
- **AI-native input layer (future frontend exposure)** — after the completed S1-S5 CLI/core mock/provider, worker/confidence, context/privacy, memory, and local-model slices, remaining AI-native work is product integration: exposing AI behind explicit defaults in real frontends without changing M9/M10 compatibility behavior. The architecture remains in [`plans/ai-native-design.md`](./plans/ai-native-design.md); S1 evidence and checklist live in [`plans/archive/ai-native-cli-slice-plan.md`](./plans/archive/ai-native-cli-slice-plan.md).

## Principles (carried forward)

- **Oracle, not template.** Treat librime as the behavior oracle, but prefer idiomatic Rust, cleaner abstractions, stronger typing, and deterministic tests over cloning librime's internal C++ structure.
- **Name the behavior.** Before adding a librime-derived mechanism, name the external behavior it protects (user-visible input, frontend ABI, schema/config semantics, or deployed-data compatibility). If it's only an internal librime detail, use a smaller Yune-native design, isolate it behind an adapter, or document a deferral.
- **Own each slice.** Choose the owning implementation module, matching test module, and librime comparison target before writing a compatibility slice. Keep `lib.rs`/`main.rs` as facades.
