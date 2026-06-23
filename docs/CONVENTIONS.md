# Yune Conventions & Reference

> **This document SUPERSEDES the former `.planning/codebase/` maps** (`ARCHITECTURE.md`, `STACK.md`, `STRUCTURE.md`, `CONVENTIONS.md`, `TESTING.md`, `INTEGRATIONS.md`, `CONCERNS.md`). It is the single navigable conventions/reference doc for the repo.

> **Code anchors drift.** File paths and symbol names below are authoritative; any `file:line` numbers are hints, not fixed anchors. Trust the symbol/path names and grep to locate them.

## Contents

1. [Overview & Architecture](#1-overview--architecture)
2. [Stack & Build](#2-stack--build)
3. [Repository Structure](#3-repository-structure)
4. [Coding Conventions](#4-coding-conventions)
5. [C ABI Rules](#5-c-abi-rules)
6. [Module & Test Ownership](#6-module--test-ownership)
7. [Testing Conventions](#7-testing-conventions)
8. [Integrations](#8-integrations)
9. [Key Risks / Concerns](#9-key-risks--concerns-current)
10. [Planning Docs](#10-planning-docs)

---

## 1. Overview & Architecture

Yune is a Rust input-method engine fronted by a single **librime-shaped C ABI**. The engine (`yune-core`) holds all deterministic input behavior behind traits; the ABI crate (`yune-rime-api`) is the **only** compatibility surface. Everything external consumes the engine through that ABI.

```text
+----------------------+   +--------------------------+   +-------------------------+
| CLI surrogate        |   | TypeDuck-Web (WASM)      |   | TypeDuck-Windows native |
| yune-cli             |   | yune_typeduck_* adapter  |   | drop-in rime.dll        |
| rime_frontend        |   | + @yune-ime/typeduck-rt  |   |                         |
+----------+-----------+   +------------+-------------+   +-----------+-------------+
           |                            |                             |
           +--------------+-------------+--------------+--------------+
                          v                            v
            +-------------------------------------------------------+
            | RIME compatibility / runtime adapter layer            |
            | crates/yune-rime-api/src/ (rlib + cdylib)             |
            | RimeApi/RimeLeversApi tables, sessions, key routing,  |
            | config, deployment, schema install, processors        |
            +-----------------------+-------------------------------+
                                    |
                  v
       +----------------------------+
       | Core input engine          |
       | crates/yune-core/src/      |
       | translators, filters,      |
       | rankers, dictionaries      |
       +----------------------------+
```

**The three consumers of the C ABI** (all drive the same exported function table; none reach into `yune-core` directly):

1. **CLI surrogate** — `crates/yune-cli/src/rime_frontend.rs`. The in-tree frontend stand-in. `run_frontend` calls `rime_get_api()` and drives the full librime lifecycle: setup → initialize → deploy → create-session → select-schema → process-key → read context/status/commit → destroy-session → cleanup → finalize (cleanup is RAII via `CleanupGuard`). `main.rs` dispatches `Frontend` (human/JSON) and `FrontendCheck` for fixture comparison, alongside the older direct-to-core `Run`/`Check` flow (`sample_core.rs`).
2. **TypeDuck-Web** — `yune-rime-api` compiles to `wasm32-unknown-emscripten`, exporting the simplified `yune_typeduck_*` C API in `crates/yune-rime-api/src/typeduck_web.rs`. The `@yune-ime/typeduck-runtime` TypeScript package (`packages/yune-typeduck-runtime/`) wraps the module; the internal browser harness integrates through the tracked seam `third_party/typeduck-web/yune-integration/adapter.ts`.
3. **TypeDuck-Windows** - completed TypeDuck compatibility-profile work. The default `rime_get_api()` table follows upstream librime 1.17.0 and does not expose the fork-only `config_list_append_*` slots. M19 added a named, opt-in `rime_get_typeduck_profile_api()` surface for those slots. The M10 resume completed native package/header smoke, packaged DLL lifecycle, x64 TypeDuck-Windows build/link, and stock real-server IPC frontend smoke through that profile.

**Direction is upstream-oracle-first.** M9 browser validation is complete, M11's core/CLI AI layer is complete, and M12 closed the upstream oracle refresh plus the first expanded upstream behavioral parity gate. The current baseline is upstream `rime/librime 1.17.0` for default core behavior. M10 TypeDuck-Windows is complete as a TypeDuck compatibility profile through the named M19 ABI surface. See `docs/roadmap.md`, `docs/plans/archive/m12-plan-upstream-oracle-refresh.md`, and `docs/plans/archive/m12-plan-upstream-behavioral-parity-closeout.md`.

**Behavior oracle.** The default compatibility oracle is upstream `rime/librime 1.17.0` (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`). The TypeDuck fork (`https://github.com/TypeDuck-HK/librime`, tag `v1.1.2`, commit `74cb52b78fb2411137a7643f6c8bc6517acfde69`) is a TypeDuck compatibility-profile oracle only. If upstream and TypeDuck disagree, upstream wins for core Yune; TypeDuck behavior must be isolated behind a named TypeDuck-profile test, fixture, adapter, or ABI note. These are referenced repositories, **NOT local checkout paths**. librime is never linked or called at runtime.

**AI-native input is an explicitly separate layer** above librime compatibility - not part of M9, M12, or the TypeDuck compatibility profile. `crates/yune-core/src/ai/` owns `AiCandidateProvider`, `MockAiProvider`, `LocalModelProvider`, `AiWorker`, staged input-keyed results, `AiContext` snapshots, `AiPrivacyPolicy`, and `MemoryStore`; the direct `yune-cli run` path can opt into `--ai-provider mock` or `--ai-provider local`. M13 exposes the local provider through TypeDuck-Web only, default-off, using a provider-free first pass (`yune_typeduck_process_key`) and a Rust/WASM second pass (`yune_typeduck_stage_ai`) requested by the browser worker after classic rendering. TypeDuck-Windows and other native frontends currently keep AI off; native AI exposure is future product work. AI context defaults to sensitive, remote providers are blocked before invocation under sensitive context, and AI memory writes are suppressed under the same policy. AI memory uses `.ai-memory` / `.ai-memory.txt` namespace helpers rather than librime `*.userdb` files. Remote model backends and additional frontend exposure remain future explicit/default-off work.

**Key data flow (RIME key path):** Frontend obtains the table via `rime_get_api` and calls `RimeApi.process_key` (`api_table.rs`, `RimeProcessKey`). `RimeProcessKey` validates session/mask/keycode, converts keycodes into `yune_core::KeyEvent` (`key.rs`), runs ABI-level processors (ascii composer, key binder, selector, navigator, chord composer, recognizer, punctuation, speller, editor, shape), then falls through unhandled keys to `Engine::process_key_event` (`engine.rs`). The engine refreshes candidates (translators → sort by quality → filters → optional ranker reorder). Commits buffer in `SessionState.unread_commit`; context/status reads copy snapshots into caller-owned C structs (`context_api.rs`).

**Current boundary caveat.** The intended long-term product shape is still `yune-core` as the deterministic engine and `yune-rime-api` as the compatibility surface, but today's full RIME key path is not a thin adapter: the schema-driven processor pipeline lives in `crates/yune-rime-api/src/processors/` and falls through to `Engine::process_key_event`. That is acceptable for librime-shaped frontends and the current TypeDuck-Web WASM contract, but it is architectural debt for future non-librime/native product frontends. When a Yune-native frontend, iOS package, or other non-ABI host needs the full input pipeline, extract processor semantics into a core-owned Rust API and leave `yune-rime-api` as the C ABI/session/config adapter. Do not do this as a speculative rewrite; do it as a behavior-preserving extraction with the existing oracle/browser gates unchanged.

---

## 2. Stack & Build

The stack spans **two ecosystems** — Cargo/Rust and npm/Node — plus an Emscripten/WASM cross-build.

**Languages:** Rust 2021 (MSRV `1.76`, set in workspace `Cargo.toml`); TypeScript (ESM, ES2022/NodeNext, strict) for the runtime package; Markdown, JSON, YAML, and a librime-shaped `extern "C"` ABI surface.

**Rust workspace crates** (`Cargo.toml`, resolver 2, BSD-3-Clause):

- `yune-core` — input engine: session state, translators, filters, candidate-ranking hook, key handling, punctuation, spelling algebra, dictionary parsing. Dep: `regex`.
- `yune-rime-api` — RIME-style C ABI shim, session registry, config/deployment APIs, schema parsing/install, levers, function tables, the `yune_typeduck_*` WASM adapter. Deps: `libc`, `regex`, `serde_json`, `serde_yaml`, `yune-core`; dev-dep `libloading`. **`crate-type = ["rlib", "cdylib"]`** — the rlib links into tests/`yune-cli`; the cdylib is the artifact loaded by native frontends (as `rime.dll`) and compiled to WASM. The browser-loadable Emscripten module is linked through the tiny `typeduck_web_module` bin target so the build emits JS glue plus a companion `.wasm`.
- `yune-cli` — fixture runner + frontend surrogate. Deps: `yune-core`, `yune-rime-api`.

No `build.rs`, no `rust-toolchain.toml`, no `.cargo/config.toml` — use the active developer toolchain.

**TypeScript runtime package** — `@yune-ime/typeduck-runtime` at `packages/yune-typeduck-runtime` (`private`, `type=module`). Built with `tsc` (`npm run build` → `dist/`); tested with Vitest (`npm test` → `vitest run`). Source modules: `module.ts` (Emscripten bindings), `typeduck.ts` (`TypeDuckRuntime` lifecycle class), `response.ts` (JSON decode), `keys.ts` (DOM `KeyboardEvent` → RIME key), `filesystem.ts` (IDBFS persistence); public API re-exported from `index.ts`.

**Emscripten / WASM build** — `scripts/typeduck-wasm-build.sh`:

1. Builds the native cdylib, verifies its exports with `nm`.
2. If `wasm32-unknown-emscripten` target + Emscripten `emcc`/`emar` are present, builds `--bin typeduck_web_module` with RUSTFLAGS link-args `-sEXPORTED_FUNCTIONS` (the `_`-prefixed `yune_typeduck_*` list), `-sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString,FS,IDBFS`, `-sMODULARIZE=1`, `-sEXPORT_NAME=createYuneTypeduckModule`, `-sFORCE_FILESYSTEM=1`, and `-lidbfs.js`.
3. Copies the browser-loadable pair to `target/wasm32-unknown-emscripten/debug/yune-typeduck.js` and `target/wasm32-unknown-emscripten/debug/yune-typeduck.wasm`, verifies exports, then instantiates it in Node and proves one `yune_typeduck_*` call plus one `FS` write/read.
4. If the target/toolchain is absent, **degrades gracefully** to the native fallback `cargo test -p yune-rime-api --test typeduck_web`.

The exported-symbol contract is `scripts/typeduck-exports.txt` (the 14 `yune_typeduck_*` names — see [§4](#4-coding-conventions)).

**Native packaging** - `scripts/package-typeduck-windows.ps1` now builds the Windows package and runs the packaged TypeDuck-profile smoke. The default `RimeApi` follows upstream `rime/librime 1.17.0`, so the script packages an upstream-shaped `rime_api.h` plus `rime_typeduck_profile_api.h`; fork-only `config_list_append_*` slots are verified only through `rime_get_typeduck_profile_api()`. The TypeDuck-Windows package also ships upstream `rime_api_deprecated.h` / `rime_api_stdbool.h` and has packaged `rime_api.h` include the deprecated declarations, because the pinned frontend source includes `<rime_api.h>` while calling `RimeSetup`-style direct symbols. This is a header-compatibility measure only; it must not widen default ABI structs or add TypeDuck fork slots to default `rime_get_api()`.

**Web surface terminology** — keep three similarly named surfaces distinct: `packages/yune-typeduck-runtime/` is the reusable Yune-owned TypeScript/WASM runtime bridge; `third_party/typeduck-web/` is the patched internal TypeDuck-Web harness used for browser evidence, demos, and regression work; a separate checkout of `TypeDuck-HK/TypeDuck-Web` is the real web IME product and belongs to a future product-integration track. Do not treat the runtime bridge as a UI product, and do not treat the internal harness as the shipping product.

**Web integration seam** — the upstream TypeDuck-Web app is vendored at `third_party/typeduck-web/source`; the Yune seam is `third_party/typeduck-web/yune-integration/` (`adapter.ts`, `assets.ts`, etc.), which adapts `@yune-ime/typeduck-runtime` into the upstream app. In-browser validation for M9/M13/M16 and M20 runs through this internal harness. M24/M25 closed the dogfooding/demo-hardening batches against this harness, M27 closed the measured startup/runtime-init owner, and M28 closed segment-aware partial selection through the same harness as final browser evidence; future browser dogfood loops should start a new scoped plan and preserve those archived evidence baselines rather than reopening Phase 1.

**Key deps & cross-platform note:** `libc::ctime_r` is used only on `all(unix, not(target_os = "emscripten"))`; on Windows + Emscripten/WASM a pure-Rust `format_ctime_utc` fallback is used (both in `lib.rs`, `librime_signature_modified_time`). `serde_yaml` parses RIME YAML; `serde_json` serializes the WASM adapter's response; `regex` powers spelling algebra, recognizer/speller patterns, and comment formatting.

---

## 3. Repository Structure

```text
yune/
|-- Cargo.toml / Cargo.lock            # Workspace manifest + locked deps
|-- README.md / AGENTS.md              # Overview / contributor guidance
|-- .editorconfig / .gitattributes     # EOL + encoding policy (see §4)
|-- crates/
|   |-- yune-core/                     # Deterministic core engine
|   |   |-- src/
|   |   |   |-- lib.rs                 # Public facade; declares `mod tests`
|   |   |   |-- engine.rs              # Engine state machine + candidate refresh
|   |   |   |-- state.rs               # Candidate/context/status/snapshot structs
|   |   |   |-- key.rs                 # RIME key-sequence parser + typed keys
|   |   |   |-- punctuation.rs / spelling_algebra.rs / comment_format.rs
|   |   |   |-- userdb.rs              # Core user-dict model + commit scoring
|   |   |   |-- dictionary/            # source + compiled prism/reverse/table + encoder
|   |   |   |-- translator/            # echo, table, reverse, history, switch, schema-list
|   |   |   |-- filter/                # Uniquifier, SingleChar, Charset, DictionaryLookup, ...
|   |   |   `-- tests/                 # Unit tests: engine.rs, filter.rs, translator.rs
|   |   `-- tests/
|   |       |-- upstream_luna_pinyin_parity.rs
|   |       |                          # Oracle parity vs upstream 1.17.0
|   |       |-- oracle_fixture_provenance.rs
|   |       |-- cantonese_parity.rs    # Oracle parity vs captured TypeDuck v1.1.2
|   |       |-- fixtures/upstream-1.17.0/
|   |       |                          # Captured upstream fixtures + provenance README
|   |       `-- fixtures/typeduck-v1.1.2/  # Captured TypeDuck profile fixture + README
|   |-- yune-rime-api/                 # RIME-shaped C ABI crate (rlib + cdylib)
|   |   |-- benches/frontend_baselines.rs  # [[bench]] (harness = false)
|   |   |-- tests/                     # Integration tests driving the exported ABI
|   |   |   |-- dynamic_loader.rs      # Loads cdylib via libloading, drives rime_get_api
|   |   |   |-- frontend_client.rs     # Function-table integration client
|   |   |   |-- frontend_hosts.rs + frontend_hosts/  # mod, native, native_frontends, typeduck_web
|   |   |   `-- typeduck_web.rs        # TypeDuck-Web C ABI integration
|   |   `-- src/
|   |       |-- bin/typeduck_web_module.rs  # Emscripten JS+WASM linker anchor
|   |       |-- lib.rs                 # ABI facade, key routing, shared glue
|   |       |-- abi.rs                 # C ABI structs + function-table types
|   |       |-- api_table.rs           # Static RimeApi/RimeLeversApi builders
|   |       |-- typeduck_web.rs        # yune_typeduck_* C entry points
|   |       |-- session.rs / context_api.rs / candidate_api.rs
|   |       |-- config.rs / config_api.rs / config_compiler.rs
|   |       |-- deployment.rs / runtime.rs
|   |       |-- schema_api.rs / schema_install.rs / schema_selection.rs
|   |       |-- key_table.rs / levers.rs / notifications.rs / modules.rs
|   |       |-- ffi_memory.rs / resource_id.rs
|   |       |-- userdb.rs (#[path] facade) + userdb/  # file_store, record, recovery, snapshot, store, sync
|   |       |-- processors/            # ascii_composer, chord_composer, editor, key_binder,
|   |       |                          #   navigator, punctuation, recognizer, selector, shape, speller
|   |       `-- tests/                 # Focused ABI/unit compatibility modules
|   `-- yune-cli/src/                  # CLI surrogate frontend
|       |-- main.rs / args.rs / sample_core.rs / fixture.rs / transcript.rs / render.rs
|       `-- rime_frontend.rs           # RIME ABI-backed frontend surrogate
|-- packages/yune-typeduck-runtime/    # npm pkg @yune-ime/typeduck-runtime (TS WASM wrapper)
|   |-- src/ (filesystem, index, keys, module, response, typeduck .ts)
|   `-- test/ (Vitest *.test.ts + fakes) / dist/ (generated)
|-- scripts/
|   |-- typeduck-wasm-build.sh         # Emscripten/WASM build (wasm32-unknown-emscripten)
|   |-- typeduck-exports.txt           # Exported-symbol allowlist (yune_typeduck_*)
|   `-- package-typeduck-windows.ps1   # Native packaging (-> rime.dll/.lib + headers)
|-- third_party/typeduck-web/          # Internal patched TypeDuck-Web harness + Yune seam
|   |-- source/                        # Upstream app (own librime/)
|   |-- yune-integration/ (adapter.ts, assets.ts, ...)
|   |-- e2e/ (Playwright)  / patches/  / typeduck-web.lock.json
|-- docs/                              # roadmap.md, decisions.md, requirements.md, plans/ (+ archive/),
|   |                                  #   typeduck-windows-backend-requirements.md, this file
|-- fixtures/                          # sample-*.json CLI fixtures + frontend-traces/
`-- .planning/codebase/                # Former generated maps (superseded by this doc)
```

**Where to add new behavior:**

- **Core engine** → `crates/yune-core/src/engine.rs` (+ `state.rs` for shape changes); test in `src/tests/engine.rs`.
- **Core translator / filter** → `crates/yune-core/src/translator/mod.rs` or `filter/mod.rs`; export via `lib.rs`; install (when schema-driven) in `yune-rime-api/src/schema_install.rs`; test in `src/tests/{translator,filter}.rs`.
- **Dictionary / encoder** → `crates/yune-core/src/dictionary/{source,compiled,encoder}.rs`.
- **RIME ABI function** → shape in `abi.rs` + `api_table.rs` (field order matches the fork header — see [§5](#5-c-abi-rules)); implement in the owning module (`context_api.rs`, `config_api.rs`, `deployment.rs`, `levers.rs`, ...); export via `lib.rs`.
- **TypeDuck-Web** → `crates/yune-rime-api/src/typeduck_web.rs`; new exports MUST be added to `scripts/typeduck-exports.txt`; TS in `packages/.../src/<area>.ts` + matching test.
- **Schema processor** → `crates/yune-rime-api/src/processors/<name>.rs`; per-session state in `session.rs`; installer call in `schema_selection.rs`/`schema_install.rs`.
- **CLI** → `args.rs` (parse), `main.rs` (dispatch), `sample_core.rs`/`rime_frontend.rs`, `transcript.rs`/`render.rs` (output).
- **Avoid a generic utility module** unless two-plus ownership areas need the same helper.

---

## 4. Coding Conventions

**Naming:**

- Rust modules `snake_case`; directory roots use `mod.rs` (a sibling facade may re-point via `#[path = "..."]`, e.g. `userdb.rs`). Crate package names kebab-case (`yune-core`).
- Functions/methods/locals/fields `snake_case`; booleans use `is_`/`has_` prefixes.
- Types (structs, enums, traits, errors) `UpperCamelCase`. C ABI mirror types are `Rime`- prefixed and `#[repr(C)]`. Tests are long behavior-specific `snake_case` sentences.
- TypeScript: `UpperCamelCase` interfaces/classes; `snake_case` JSON-mirroring fields (`page_size`, `page_no`, `is_last_page`); named `Error` subclasses carry failures.

**The TWO export families — do not mix them:**

- **librime-shaped ABI → `RimePascalCase`.** `#[no_mangle] extern "C"` functions mirroring librime's C ABI, e.g. `RimeConfigOpen` (`config_api.rs`), `RimeSetup` (`runtime.rs`).
- **Yune-owned WASM/browser ABI → `snake_case` `yune_typeduck_*`.** The 14 exports in `typeduck_web.rs`: `yune_typeduck_init`, `process_key`, `select_candidate`, `delete_candidate`, `flip_page`, `deploy`, `customize`, `set_option`, `set_ai_enabled`, `stage_ai`, `cleanup`, `response_json`, `response_handled`, `free_response`. These names are an **explicit export contract** enforced by the allowlist `scripts/typeduck-exports.txt` and `-sEXPORTED_FUNCTIONS`. Add or rename an exported C function → **update the allowlist** or the WASM build silently drops it.

**Error handling:**

- Library parsers return custom error types implementing `Display`/`Error` (`KeySequenceParseError`, `TableDictionaryParseError`, `TableEncoderFormulaError`). CLI `run` returns `Result<(), String>` → stderr + `ExitCode::FAILURE`.
- C ABI functions return librime-shaped `Bool`/null instead of panicking; validate null pointers and invalid C strings at the boundary. Use `expect` only for internal invariants and test setup. Preserve librime-shaped fallback behavior explicitly in compatibility code.

**FFI safety:** FFI functions use explicit `unsafe extern "C" fn` signatures plus Rustdoc `# Safety` sections and local `// SAFETY:` comments next to unsafe blocks (see `config_api.rs`, `runtime.rs`, `ffi_memory.rs`).

**Name the external librime behavior.** When code mirrors librime, name the specific upstream construct (class/function/field), not just "compatibility", so a reviewer can trace it to the oracle — e.g. `// librime's Signature::Sign stores a trimmed ctime(3) string.` in `lib.rs`. (librime is named in comments hundreds of times across the ABI crate.) Avoid restating straightforward control flow. The TS runtime uses no TSDoc — it expresses contracts through exported interfaces and named `Error` subclasses.

**EOL policy:** `.gitattributes` sets `* text=auto eol=lf` with explicit exceptions (`*.bat`/`*.cmd` CRLF, `*.sh` LF, binaries `*.wasm`/`*.dll`/`*.so`/`*.exe` never normalized). `.editorconfig` enforces UTF-8, LF, final newline, trailing-whitespace trim (`*.md` exempt from the trim). The repo is developed on Windows but ships shell/WASM tooling — **do not commit CRLF into normalized files**, or `.sh` scripts and byte-exact fixtures break.

**Formatting & lint gate:** `cargo fmt` (no repo `rustfmt.toml`). Quality gate: `cargo clippy --workspace --all-targets -- -D warnings`. Root `Cargo.toml` `[workspace.lints]` declares `rust.unsafe_code = "forbid"` and clippy `all`/`pedantic = "warn"` with explicit existing-debt exceptions. Unsafe-free crates inherit the workspace table (`yune-core`), so `unsafe` fails there. ABI/FFI-facing crates (`yune-rime-api` and the ABI-driving `yune-cli`) use explicit non-inheriting lint tables with `unsafe_code = "allow"` because the workspace `forbid(unsafe_code)` cannot be locally overridden. Public pure accessors/constructors commonly carry `#[must_use]`. Imports group as std → external → local (`crate::`/`super::`); no custom path aliases.

---

## 5. C ABI Rules

**`RimeApi` field order IS the ABI.** The `#[repr(C)]` function table in `crates/yune-rime-api/src/abi.rs` is accessed by struct-pointer offset, so the order of its fields is the actual C contract. Core ABI fields follow upstream `rime/librime` headers; explicit TypeDuck-profile fork-only fields follow the TypeDuck fork's `rime_api.h`. Never insert mid-struct without matching oracle/header evidence. A misplaced field silently breaks every native frontend.

- **Fork-only slots:** `config_list_append_bool` / `_int` / `_double` / `_string` do not exist in upstream librime and are not exposed by default `rime_get_api()`. Their helper implementations remain in `config_api.rs` with direct TypeDuck-profile tests. M19 exposes them only through the named `rime_get_typeduck_profile_api()` accessor, whose appended slot order is documented in `docs/plans/m19-reference-typeduck-profile-abi.md`. Future TypeDuck-profile slots still require fresh fork-header evidence.
- **Locks:** default slot positions are locked by `assert_api_slot!` tests in `crates/yune-rime-api/src/tests/abi.rs` against upstream `1.17.0`. Never reorder/insert without updating these locks and confirming against the relevant upstream or explicit TypeDuck profile header. Historical TypeDuck slot rationale lives in `docs/plans/m10-reference-typeduck-windows-native-build.md` and `m10-reference-typeduck-windows-contract.md`.
- **Comment panel:** `yune-core` ships `DictionaryLookupFilter` (filter name `"dictionary_lookup_filter"`, `filter/mod.rs`) emitting the TypeDuck comment-panel bytes — a leading `\u{000c}` form-feed, per-row `\r` markers, a `1`/`0` primary flag, and comma-joined multilingual dictionary fields. These bytes are golden (see [§7](#7-testing-conventions)).
- **Unsafe boundary:** keep all FFI pointer work, `CString`/`Box`/`Vec` raw conversions, and `RimeFree*` pairing in the ABI modules (`context_api.rs`, `candidate_api.rs`, `config_api.rs`, `ffi_memory.rs`, `typeduck_web.rs`) — never in `yune-core`.

---

## 6. Module & Test Ownership

**Own each slice.** Each behavior slice owns its production module _and_ its tests. `lib.rs`/`main.rs` stay thin **facades** — re-exports + orchestration glue only. Adding owned engine/processor/config/ABI behavior directly into a facade hides ownership boundaries; put it in the focused module instead.

- **Unit tests** live under `<crate>/src/tests/<slice>.rs` behind `#[cfg(test)] mod tests` (e.g. `yune-core/src/tests/{engine,filter,translator}.rs`; `yune-rime-api/src/tests/*.rs` with shared helpers in `tests/mod.rs`).
- **Integration / parity tests + oracle fixtures** live under `<crate>/tests/` (e.g. `yune-core/tests/cantonese_parity.rs` with goldens in `tests/fixtures/typeduck-v1.1.2/`; `yune-rime-api/tests/{frontend_client,dynamic_loader,frontend_hosts,typeduck_web}.rs`).
- **CLI sample fixtures** stay in the top-level `fixtures/` as `sample-*.json`.

Public re-exports stay centralized in crate facades; use module roots (`dictionary/mod.rs`, `processors/mod.rs`, etc.) as barrels where they define ownership boundaries.

---

## 7. Testing Conventions

**Runner:** Rust built-in harness via Cargo; Vitest for the TS runtime. Standard assertions only (`assert_eq!`, `assert!`, `panic!`) — no property/snapshot/mocking framework; mocking is hand-written fakes (`CommentTranslator` in `tests/engine.rs`; `test/fake-{filesystem,module}.ts`).

```bash
cargo test --workspace                              # all Rust tests
cargo test -p yune-rime-api --test typeduck_web     # TypeDuck-Web ABI/adapter contract
cargo test -p yune-core --test upstream_luna_pinyin_parity # upstream 1.17.0 oracle parity
cargo test -p yune-core --test cantonese_parity     # v1.1.2 oracle parity
cargo clippy --workspace --all-targets -- -D warnings
npm test  # (in packages/yune-typeduck-runtime) -> vitest run
```

**Oracle-driven, NON-circular parity.** Compatibility tests capture expected bytes/behavior from the **external oracle** (upstream librime / TypeDuck fork v1.1.2) into a checked-in fixture, then run Yune's **real production path** and assert it reproduces the oracle output. **Never derive the expected value from Yune itself.** Canonical example: `upstream_luna_pinyin_parity.rs` uses official upstream `rime/librime 1.17.0` release-binary fixtures for `luna_pinyin`. Curated mechanics fixtures feed captured upstream dictionary/vocabulary rows through Yune's real `TableDictionary` and `StaticTableTranslator` path. Full selection fixtures must include every competing upstream dictionary row for the tested code plus relevant `essay.txt` rows for every in-scope candidate so ranking cannot silently use default/zero essay weights. Any behavior affected by menu state, paging, selection, commit, filters, or options must drive Yune's real `Engine` path or an equivalent full-pipeline harness; translator-direct output is only mechanics coverage. `oracle_fixture_provenance.rs` scans all upstream `luna_pinyin` JSON files for oracle identity, schema repository commits, capture commands, source-row policies, and absence of local absolute cache paths. Unsupported upstream behavior stays as an ignored test with a blocker string and `panic!()` body, not as undocumented absence. TypeDuck profile example: `cantonese_parity.rs` feeds raw TypeDuck TSV source rows through the real `DictionaryLookupFilter` and compares the emitted comment against the golden `tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json` (each comment begins with the panel marker `\u{000c}\r1,`). A companion test locks the fixture's pinned engine/tag/commit metadata.

**TypeDuck rich-comment E2E reproducibility.** The browser-shaped `typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments` integration test may use local TypeDuck v1.1.2 oracle build artifacts under `target/typeduck-oracle/v1.1.2/rime-user/build` to prove the full `jyut6ping3_mobile` runtime path emits the rich `\f\r1,.../\r0,...` comment payload. That `target/` tree is ignored local oracle state, so the test must emit an explicit skip reason when those build assets are absent and must never silently pass against a degraded three-column fallback. The committed clean-checkout byte-parity guarantee is still `cargo test -p yune-core --test cantonese_parity`.

**`#[ignore]` must carry a documented blocker.** A blocked behavior gets a _named_ test marked `#[ignore = "blocked: <what is missing>"]` whose body `panic!()`s - never silently drop a slice. The reason names the precise blocker, usually a missing oracle fixture. As of M24, `cantonese_parity.rs` has no ignored cases; the remaining ignored parity blockers live in the M19 breadth tests such as `upstream_cangjie_parity.rs`, `upstream_double_pinyin_parity.rs`, and `upstream_zhuyin_parity.rs`.

**Tests exercise the public surface, not internals.** ABI/frontend tests obtain the table via `rime_get_api()` and call its members, or call the exported `yune_typeduck_*` functions — the same surface a real frontend uses. `dynamic_loader.rs` `dlopen`s the built cdylib and resolves `rime_get_api`, the strongest "drive through the real ABI" guarantee.

**Cross-platform hygiene.** Yune builds for Unix, Windows (MSVC cdylib), and `wasm32-unknown-emscripten`; tests must not assert platform-specific values.

- _Identical ctime shape._ The librime signature timestamp is computed two ways (`libc::ctime_r` on Unix, `format_ctime_utc` on non-Unix/emscripten) but both yield the same `ctime(3)` shape. Tests assert the _shape_ (field count, weekday/month tokens, `HH:MM:SS` layout, numeric year) via `assert_librime_ctime_shape` in `tests/mod.rs`, never a value.
- _Poison-tolerant test locks vs panic-on-poison production locks._ Test-only lock helpers are poison-tolerant (`.unwrap_or_else(PoisonError::into_inner)`, `test_guard`/ `notification_events_lock` in `tests/mod.rs`) so one failing test cannot cascade. **Production locks intentionally stay panic-on-poison** (`.expect("...should not be poisoned")`, e.g. `session.rs`, `lib.rs`). Know which side you are on. Hold a serializing `let _guard = test_guard();` in any test touching process-wide runtime state.

**Native adapter contract as the WASM-absent fallback.** When the `wasm32-unknown-emscripten` target or `emcc`/`emar` are unavailable, `scripts/typeduck-wasm-build.sh` deliberately runs `cargo test -p yune-rime-api --test typeduck_web` so the WASM adapter contract is still validated without browser tooling. Real-browser M9 validation is the web-first goal beyond this fallback.

**Fork-only ABI helper tests.** `tests/config_api.rs` guards the TypeDuck-profile list-append helper behavior (`config_list_append_*` round-trips), which M19 named and M10 consumed through `rime_get_typeduck_profile_api()`. The default `rime_get_api()` table does not expose those fork-only slots; its config-list contract is upstream `1.17.0`.

No coverage tooling/threshold is configured. Browser-level E2E for TypeDuck-Web is the M9 goal; until then the `typeduck_web` ABI tests + native fallback + Vitest suite are the safety net.

---

## 8. Integrations

**librime oracle (validation-only, no runtime dependency).** Core Yune targets upstream `github.com/rime/librime @ 1.17.0`. TypeDuck-Web/Windows profile tests may target `github.com/TypeDuck-HK/librime @ v1.1.2`. Golden fixture directories must name the oracle, e.g. `upstream-1.17.0/` or `typeduck-v1.1.2/`. Never derive expected bytes from Yune, and never link or call librime at runtime.

**TypeDuck-Web / Emscripten / IDBFS.** The `yune_typeduck_*` adapter (`typeduck_web.rs`) exports 14 functions over the `rime_get_api()`/`rime_levers_get_api()` tables and the Yune-owned AI sidecar. The TS runtime consumes the WASM module via Emscripten `cwrap`/`UTF8ToString`. **Browser persistence** uses an Emscripten **IDBFS** mount over a virtual data dir, flushed with `FS.syncfs`: `packages/.../src/filesystem.ts` defines `prepareTypeDuckFilesystem` (writes `default.yaml`, `<schema>.schema.yaml`, `<dict>.dict.yaml`, `build/`) and explicit sync boundaries (`syncFromPersistenceBeforeInit`; `syncToPersistenceAfterMutation` / `deployAndSync` / `customizeAndSync` / `syncAfterUserDataChange`). The upstream seam adapter translates Yune's `TypeDuckResponse` (`handled`, `commits`, `context.preedit`, `context.candidates`) into the upstream `RimeResult` shape and parses key strings (`a`, `{BackSpace}`, `{Release+Enter}`) via `keyEventToRimeKey`; M13 maps `enableAI` to the runtime-only `set_ai_enabled` flag and requests `stage_ai` as a serialized second action. Source labels for AI rows come from engine snapshot data aligned to the rendered page, not from `RimeCandidate`; patch scope is intentionally minimal.

**weasel / TypeDuck-Windows native.** Completed TypeDuck-profile work. The old package path is retained as reference material, and `scripts/package-typeduck-windows.ps1` now runs current TypeDuck-profile package smoke while keeping default `rime_get_api()` upstream-shaped. Native packaging, x64 TypeDuck-Windows build/link, and stock real-server IPC frontend smoke are verified through the named TypeDuck profile. Frontend-host integration tests: `tests/frontend_hosts/{native,native_frontends}.rs`.

**OpenCC.** `SimplifierFilter` (`filter/mod.rs`) honors a focused subset of librime OpenCC config names. `t2s` uses checked-in OpenCC `TS*` source dictionaries, `hk2s` runs the TypeDuck-required HK reverse stage plus the TS stage from `crates/yune-core/src/opencc/data/`, and `t2tw` remains the small built-in Taiwan-variant path. This is still an in-process approximation, not the real OpenCC library; wired during install in `schema_install.rs`.

**Other in-process seams:** RIME schema/config/dictionary YAML (the primary compatibility boundary, via `serde_yaml` + local helpers); the frontend notification callback (`RimeNotificationHandler`, `notifications.rs`); the module registry (`RimeModule`, `modules.rs`); the AI ranking extension point (`CandidateRanker`/`MockAiRanker`, the seam for the separate later AI layer). **No auth, no databases, no HTTP webhooks, no CI workflows** — all calls are local library/CLI/FFI (native or in-browser WASM). User dictionaries are plain local `*.userdb` files; sync snapshots are `*.userdb.txt`.

---

## 9. Key Risks / Concerns (current)

**M9/M13 TypeDuck-Web validation is complete — residual risk is regression (keep gates green).** The engine now runs through the real TypeDuck-Web browser E2E: the HR-5 real-assets matrix passes (composition, paging, selection, deletion, Space/phrase commit, deploy, customize, persistence sync, reload, dictionary panel) against `jyut6ping3_mobile`, and HR-7 recorded **GO WITH CONDITIONS**. M13 added the default-off, local-only AI second pass with browser evidence for AI-off identity, source labels, no default AI auto-commit, and explicit AI selection. Preserve the reproducible Emscripten build, runtime tests/build, TypeDuck-Web worker build, native `typeduck_web` fallback, committed browser evidence, and default-off AI scenarios on every merge. Files: `typeduck_web.rs`, `packages/yune-typeduck-runtime/`, `scripts/typeduck-wasm-build.sh`, `docs/plans/archive/m09-plan-typeduck-web-validation.md`, `docs/plans/archive/m13-plan-ai-native-frontend-exposure.md`.

**Upstream latest-stable behavioral closeout is complete for the first `luna_pinyin` gate.** Default core behavior and default `RimeApi` follow upstream `rime/librime 1.17.0`. The active gate covers curated single-code mechanics, full `ni` dictionary selection with essay weights, Engine paging/selection/commit, reverse lookup, punctuation/symbols, and supported option paths. M17 adds the upstream null-grammar `luna_pinyin` sentence/lattice path, and M18 adds the previously blocked `ascii_punct` bypass plus punctuation immediate commit behavior. Learned `.gram`/octagram grammar and broader contextual translation remain deferred until a named target needs them. TypeDuck-derived fixtures remain profile-only unless separate upstream goldens prove the same behavior.

**TypeDuck-Windows ABI/package work is complete as a TypeDuck compatibility profile.** A pre-M12 package smoke built `rime.dll`/`rime.lib`/headers and checked a TypeDuck fork-only slot, but that old smoke is archived evidence only and is not valid against the default upstream `rime_get_api()` table. M19 added the named `rime_get_typeduck_profile_api()` accessor for the list-append fork slots, and M10 completed the current profile package/header smoke, packaged DLL dynamic-loader lifecycle, TypeDuck-Windows x64 build/link evidence, and stock TypeDuckServer/TestTypeDuckIPC real-server IPC smoke through that profile surface. The default table and `RimeCandidate` remain upstream-shaped. Future TypeDuck-Windows modernization is Phase 2 product/frontend work, not a reason to widen the default ABI.

**TypeDuck `jyut6ping3` fork-parity arc is closed with explicit browser limits.** HR-6 added oracle coverage for the reverse-lookup `"; "` joiner (`comments.join("; ")` in `filter/mod.rs`) and schema-name-in-prompt parity, so those are byte-locked. The remaining Cantonese/Jyutping gaps were tracked by M14-M16, not loose backlog: M14 captured TypeDuck-HK/librime `v1.1.2` goldens, M15 implemented dictionary-driven engine behavior, and M16 committed TypeDuck-Web browser evidence for the app-exposed `jyut6ping3_mobile` surface plus M13 AI. The active `cantonese_parity` cases now cover options (`combine_candidates`/`show_full_code`/ `enable_sentence`), completion, and correction against M14 fixtures. Deploy-only schema variants, schema-menu hiding, correction UI detail, and per-entry userdb pronunciation are documented browser/userdb inspection limits; do not turn those into unqualified browser claims without a new TypeDuck-Web UI or native inspection surface.

**Profile isolation is a live guardrail.** TypeDuck `v1.1.2` heuristics may live in shared core types only when installed or configured by a named TypeDuck profile, or when separate upstream oracle evidence proves the same behavior is global. A `TYPEDUCK_*` constant in shared translator code that affects default upstream schemas is a red flag: thread it through typed translator config / schema-profile install wiring, or rename it neutrally only after proving it is not profile-specific. M23 applied this to the M21-GAP-01 sentence word penalty: `TYPEDUCK_SENTENCE_WORD_PENALTY = 21.0` is now threaded through typed translator config and installed only for the `jyut6ping3` TypeDuck profile, so default upstream schemas such as `luna_pinyin` do not inherit it.

**Process-global single RIME service.** Runtime paths, sessions, module pointers, notifications, state-label cache, API tables, and switcher registries are process-wide singletons (e.g. `runtime_paths()` is `OnceLock<Mutex<RuntimePaths>>`). The TypeDuck-Web handoff contract requires **exactly one active process-global RIME service per WASM instance** with host-owned MEMFS/IDBFS layout and explicit host-driven sync; multiple concurrent engines/schemas in one instance are out of scope, and these singletons are load-bearing for that model. `yune_typeduck_init` drives setup/initialize/create_session/select_schema against this single global service.

**Core/ABI boundary drift is known debt.** The RIME processor pipeline is currently owned by `yune-rime-api`, not `yune-core`, so the complete production input path is most naturally driven through `RimeProcessKey` / the TypeDuck C adapter. That is fine for compatibility validation, but it should not become the only way a future Yune-native frontend, iOS package, or product host can use the full engine. The extraction trigger is a real non-ABI consumer; the extraction rule is behavior-preserving movement of processor semantics toward `yune-core`, not a rewrite or a weakening of the C ABI gates. M18 made the narrow punctuation processor behavior needed by upstream fixtures available in `yune-core` (`ascii_punct`, direct commit, confirm-unique preview, pair preview, and list cycling), but the broader processor-pipeline extraction remains trigger-gated.

**Other notable items (condensed):** workspace lints are inherited by unsafe-free crates, while ABI/FFI crates keep explicit local lint tables documenting their unsafe exceptions; the orphaned `yune-schema` crate was removed in M23, leaving production schema parsing/install owned by `yune-rime-api`; the inline core facade tests and oversized ABI test modules were split into behavior-owned files in M23; M18 added Yune-owned binary dictionary writers and a rebuild executor (`build_table_bin`, `build_reverse_bin`, `build_prism_bin`, `execute_rebuild_plan`) whose table/reverse bytes are Yune-native round-trippable artifacts, not upstream marisa-compatible outputs; `yune-rime-api/src/lib.rs` still owns production glue; production session locks panic on poison (a scaling limit); dictionary/runtime performance still has measured debt: `StaticTableTranslator` has an `entries_by_code` map for exact/ranged lookup, but the parsed prism/double-array is not the hot runtime lookup index, M30 removed duplicate steady-state expanded-entry storage for spelling-algebra-backed translators while retaining a builder-only source stream to preserve row order, the TypeDuck dynamic-correction branch still scans `entries_by_code.keys()` but M26 now prunes impossible-length codes before the restricted-distance matrix, candidate materialization still clones output snapshots, TypeDuck-Web startup/schema-selection/runtime init is now closed by M27 with native owner spans, Windows working-set evidence, and a spelling-algebra startup optimization, and TypeDuck partial candidate selection is now closed by M28 with segment-aware commit/recomposition plus FORK-PARITY-03 learning preservation; the userdb store is file-backed (not LevelDB); `SimplifierFilter` is an OpenCC approximation. No production `TODO`/`FIXME` markers exist — use `docs/roadmap.md` and this doc as the active issue inventory.

## 10. Planning Docs

Planning, decisions, and conventions live under `docs/` — there is no external planning tool (the GSD system was retired). Layout: `docs/roadmap.md` (the milestone map), `docs/decisions.md`, `docs/requirements.md`, this file, and `docs/plans/` (per-stage plans / findings / build notes, with finished ones under `docs/plans/archive/`).

**Markdown source style:** docs are formatted with Prettier configured to preserve long prose lines (`docs/.prettierrc.json`, `proseWrap: "never"`). Prefer one source line per paragraph or list item where practical; do not hard-wrap prose only to satisfy a line-length limit. Tables, code fences, headings, long URLs, paths, and command lines may also stay long when that is more readable. Validate docs from the repo root with `markdownlint-cli2`; on Windows PowerShell, use `npx.cmd` if the `npx.ps1` shim is blocked.

**Every doc under `docs/plans/` (and `archive/`) opens with a status banner as its second line, and the banner MUST name the milestone/stage it belongs to** so its scope is clear at a glance:

```markdown
> **Status:** <Active|Reopened|Parked|Complete|Finished|Superseded> · **Milestone:** M<n> (short name) · **Updated|Closed:** YYYY-MM-DD · **Type:** <execution plan|findings|reference|record>
```

- **`Milestone` is a required field, kept separate from `Status`.** Write `**Status:** Parked · **Milestone:** M10 (…)`, never `**Status:** Parked (M10)`. Append the within-milestone stage where useful, e.g. `**Milestone:** M9 — stage HR-3`.
- Use `Updated:` for `Active`/`Reopened`/`Parked` docs and `Closed:` for `Complete`/`Finished`/`Superseded` ones.
- **Plan filenames use `m<two-digit milestone>[-<stage>]-<doc-type>-<short-topic>.md` for Phase 1 milestones and `p<phase>-<track><number>-<doc-type>-<short-topic>.md` for Phase 2 product/platform tracks.** Allowed type tokens are `plan`, `design`, `reference`, `analysis`, `findings`, `record`, and `audit`. Example active names: `m11-design-ai-native.md`, `m14-plan-typeduck-v112-golden-capture.md`, `m09-reference-typeduck-web-adapter.md`, and `p2-win01-plan-typeduck-windows-next.md`. Archived records follow the same style when created or normalized; older multi-milestone archive records may use a span such as `m05-m07-record-foundation-refactor.md`.
- `grep -rn "Status:" docs/plans` is the at-a-glance dashboard of every plan — its milestone/stage and its state.
- **Finished or superseded plans move to `docs/plans/archive/`** (banner flipped accordingly), never deleted — the trail stays.
- The milestone → plans → status mapping lives in `docs/roadmap.md`; keep a plan's banner milestone consistent with the roadmap.

---

_Last reviewed: 2026-06-22 - M0-M30 are complete for the Phase 1 named-target baseline, completed TypeDuck-Web dogfooding tracks, measured performance-hardening slices, TypeDuck-Web startup/runtime-init follow-up, segment-aware TypeDuck partial selection, and engine representation performance. M30 removed duplicate steady-state expanded-entry storage for spelling-algebra-backed translators, reducing TypeDuck single-startup ready pressure from about `1.10GB` to about `838MB` without claiming browser latency wins. M27 added native startup owner spans, Windows working-set evidence, browser fresh/reload startup evidence, a spelling-algebra startup optimization, AI-control no-loading evidence, and regenerated TypeDuck-Web patch checks. M28 added capture-not-confirm TypeDuck v1.1.2 fixture coverage, native/API/browser tests for prefix selection without raw-tail commit, and FORK-PARITY-03 learning preservation for whole-sentence versus true partial commits. M17 upstream `luna_pinyin` null-grammar sentence/lattice parity, M19 breadth schemas, M18 deployment/processor depth, and M23 architecture hardening remain complete. M24/M25 TypeDuck-Web dogfooding is complete with browser evidence and regenerated patch checks. M13 TypeDuck-Web AI exposure, M14 TypeDuck `jyut6ping3` v1.1.2 capture, M15 dictionary-driven engine parity, and M16 TypeDuck-Web browser validation remain complete; default RimeApi follows upstream 1.17.0. M10 TypeDuck-Windows is complete as a TypeDuck compatibility profile through the named profile ABI. Phase 2 product/platform work begins with `docs/plans/p2-win01-plan-typeduck-windows-next.md`, blocked for Windows product work by the P2-WIN-02 boundary compatibility plan._
