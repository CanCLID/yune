# Yune Conventions & Reference

> **This document SUPERSEDES the former `.planning/codebase/` maps**
> (`ARCHITECTURE.md`, `STACK.md`, `STRUCTURE.md`, `CONVENTIONS.md`, `TESTING.md`,
> `INTEGRATIONS.md`, `CONCERNS.md`). It is the single navigable conventions/reference
> doc for the repo.

> **Code anchors drift.** File paths and symbol names below are authoritative; any
> `file:line` numbers are hints, not fixed anchors. Trust the symbol/path names and
> grep to locate them.

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

Yune is a Rust input-method engine fronted by a single **librime-shaped C ABI**.
The engine (`yune-core`) holds all deterministic input behavior behind traits;
the ABI crate (`yune-rime-api`) is the **only** compatibility surface. Everything
external consumes the engine through that ABI.

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
                  +-----------------+-----------------+
                  v                                   v
       +----------------------------+      +------------------------+
       | Core input engine          |      | Schema model subset    |
       | crates/yune-core/src/      |      | crates/yune-schema/    |
       | translators, filters,      |      | parse RIME YAML shape  |
       | rankers, dictionaries      |      +------------------------+
       +----------------------------+
```

**The three consumers of the C ABI** (all drive the same exported function table;
none reach into `yune-core` directly):

1. **CLI surrogate** — `crates/yune-cli/src/rime_frontend.rs`. The in-tree frontend
   stand-in. `run_frontend` calls `rime_get_api()` and drives the full librime
   lifecycle: setup → initialize → deploy → create-session → select-schema →
   process-key → read context/status/commit → destroy-session → cleanup → finalize
   (cleanup is RAII via `CleanupGuard`). `main.rs` dispatches `Frontend` (human/JSON)
   and `FrontendCheck` for fixture comparison, alongside the older direct-to-core
   `Run`/`Check` flow (`sample_core.rs`).
2. **TypeDuck-Web** — `yune-rime-api` compiles to `wasm32-unknown-emscripten`,
   exporting the simplified `yune_typeduck_*` C API in
   `crates/yune-rime-api/src/typeduck_web.rs`. The `@yune-ime/typeduck-runtime`
   TypeScript package (`packages/yune-typeduck-runtime/`) wraps the module; the
   upstream app integrates through the tracked seam
   `third_party/typeduck-web/yune-integration/adapter.ts`.
3. **TypeDuck-Windows** - parked TypeDuck compatibility-profile work. The default
   `rime_get_api()` table follows upstream librime 1.17.0 and does not expose the
   fork-only `config_list_append_*` slots. A drop-in `rime.dll` package requires
   a named TypeDuck profile ABI surface first (see [section 5](#5-c-abi-rules)).

**Direction is upstream-oracle-first.** M9 browser validation is complete,
M11's core/CLI AI layer is complete, and M12 closed the upstream oracle refresh
plus the first expanded upstream behavioral parity gate. The current baseline is
upstream `rime/librime 1.17.0` for default core behavior. M10 TypeDuck-Windows
is parked as a TypeDuck compatibility profile until an explicit profile surface
exists. See `docs/roadmap.md`, `docs/plans/archive/upstream-oracle-refresh.md`,
and `docs/plans/m12-upstream-behavioral-parity-closeout.md`.

**Behavior oracle.** The default compatibility oracle is upstream
`rime/librime 1.17.0` (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`). The
TypeDuck fork (`https://github.com/TypeDuck-HK/librime`, tag `v1.1.2`, commit
`74cb52b78fb2411137a7643f6c8bc6517acfde69`) is a TypeDuck compatibility-profile
oracle only. If upstream and TypeDuck disagree, upstream wins for core Yune;
TypeDuck behavior must be isolated behind a named TypeDuck-profile test,
fixture, adapter, or ABI note. These are referenced repositories, **NOT local
checkout paths**. librime is never linked or called at runtime.

**AI-native input is an explicitly separate M11 layer** above librime
compatibility - not part of M9, M12, or the parked TypeDuck profile. The current implementation is still
core/CLI-only: `crates/yune-core/src/ai/` owns `AiCandidateProvider`,
`MockAiProvider`, `LocalModelProvider`, `AiWorker`, staged input-keyed results,
`AiContext` snapshots, `AiPrivacyPolicy`, and `MemoryStore`; the direct
`yune-cli run` path can opt into `--ai-provider mock` or `--ai-provider local`.
ABI, TypeDuck-Web, and parked TypeDuck-Windows frontends keep AI off. AI context defaults to
sensitive, remote providers are blocked before invocation under sensitive
context, and AI memory writes are suppressed under the same policy. AI memory
uses `.ai-memory` / `.ai-memory.txt` namespace helpers rather than librime
`*.userdb` files. Remote model backends and real frontend exposure remain future
explicit/default-off work.

**Key data flow (RIME key path):** Frontend obtains the table via `rime_get_api`
and calls `RimeApi.process_key` (`api_table.rs`, `RimeProcessKey`). `RimeProcessKey`
validates session/mask/keycode, converts keycodes into `yune_core::KeyEvent`
(`key.rs`), runs ABI-level processors (ascii composer, key binder, selector,
navigator, chord composer, recognizer, punctuation, speller, editor, shape), then
falls through unhandled keys to `Engine::process_key_event` (`engine.rs`). The
engine refreshes candidates (translators → sort by quality → filters → optional
ranker reorder). Commits buffer in `SessionState.unread_commit`; context/status
reads copy snapshots into caller-owned C structs (`context_api.rs`).

---

## 2. Stack & Build

The stack spans **two ecosystems** — Cargo/Rust and npm/Node — plus an
Emscripten/WASM cross-build.

**Languages:** Rust 2021 (MSRV `1.76`, set in workspace `Cargo.toml`); TypeScript
(ESM, ES2022/NodeNext, strict) for the runtime package; Markdown, JSON, YAML, and a
librime-shaped `extern "C"` ABI surface.

**Rust workspace crates** (`Cargo.toml`, resolver 2, BSD-3-Clause):
- `yune-core` — input engine: session state, translators, filters, candidate-ranking
  hook, key handling, punctuation, spelling algebra, dictionary parsing. Dep: `regex`.
- `yune-schema` — minimal standalone RIME schema-subset parser. Deps: `serde`, `serde_yaml`.
- `yune-rime-api` — RIME-style C ABI shim, session registry, config/deployment APIs,
  levers, function tables, the `yune_typeduck_*` WASM adapter. Deps: `libc`, `regex`,
  `serde_json`, `serde_yaml`, `yune-core`; dev-dep `libloading`.
  **`crate-type = ["rlib", "cdylib"]`** — the rlib links into tests/`yune-cli`; the
  cdylib is the artifact loaded by native frontends (as `rime.dll`) and compiled to WASM.
  The browser-loadable Emscripten module is linked through the tiny
  `typeduck_web_module` bin target so the build emits JS glue plus a companion `.wasm`.
- `yune-cli` — fixture runner + frontend surrogate. Deps: `yune-core`, `yune-rime-api`.

No `build.rs`, no `rust-toolchain.toml`, no `.cargo/config.toml` — use the active
developer toolchain.

**TypeScript runtime package** — `@yune-ime/typeduck-runtime` at
`packages/yune-typeduck-runtime` (`private`, `type=module`). Built with `tsc`
(`npm run build` → `dist/`); tested with Vitest (`npm test` → `vitest run`). Source
modules: `module.ts` (Emscripten bindings), `typeduck.ts` (`TypeDuckRuntime` lifecycle
class), `response.ts` (JSON decode), `keys.ts` (DOM `KeyboardEvent` → RIME key),
`filesystem.ts` (IDBFS persistence); public API re-exported from `index.ts`.

**Emscripten / WASM build** — `scripts/typeduck-wasm-build.sh`:
1. Builds the native cdylib, verifies its exports with `nm`.
2. If `wasm32-unknown-emscripten` target + Emscripten `emcc`/`emar` are present, builds
   `--bin typeduck_web_module` with RUSTFLAGS link-args `-sEXPORTED_FUNCTIONS` (the
   `_`-prefixed `yune_typeduck_*` list),
   `-sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString,FS,IDBFS`,
   `-sMODULARIZE=1`, `-sEXPORT_NAME=createYuneTypeduckModule`,
   `-sFORCE_FILESYSTEM=1`, and `-lidbfs.js`.
3. Copies the browser-loadable pair to
   `target/wasm32-unknown-emscripten/debug/yune-typeduck.js` and
   `target/wasm32-unknown-emscripten/debug/yune-typeduck.wasm`, verifies exports, then
   instantiates it in Node and proves one `yune_typeduck_*` call plus one `FS` write/read.
4. If the target/toolchain is absent, **degrades gracefully** to the native fallback
   `cargo test -p yune-rime-api --test typeduck_web`.

The exported-symbol contract is `scripts/typeduck-exports.txt` (the 11 `yune_typeduck_*`
names — see [§4](#4-coding-conventions)).

**Native packaging** - `scripts/package-typeduck-windows.ps1` is parked during
M12 closeout and fails fast. The default `RimeApi` follows upstream
`rime/librime 1.17.0`, so the previous TypeDuck fork-only
`config_list_append_string` slot smoke is not valid against default
`rime_get_api()`. Re-enable the script only after a named TypeDuck profile ABI
surface exists and its slot smoke is re-derived from TypeDuck-HK/librime
`v1.1.2` `rime_api.h`.

**Web integration seam** — the upstream TypeDuck-Web app is vendored at
`third_party/typeduck-web/source`; the Yune seam is
`third_party/typeduck-web/yune-integration/` (`adapter.ts`, `assets.ts`, etc.), which
adapts `@yune-ime/typeduck-runtime` into the upstream app. In-browser M9 validation runs
through this seam.

**Key deps & cross-platform note:** `libc::ctime_r` is used only on
`all(unix, not(target_os = "emscripten"))`; on Windows + Emscripten/WASM a pure-Rust
`format_ctime_utc` fallback is used (both in `lib.rs`, `librime_signature_modified_time`).
`serde_yaml` parses RIME YAML; `serde_json` serializes the WASM adapter's response;
`regex` powers spelling algebra, recognizer/speller patterns, and comment formatting.

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
|   |-- yune-schema/src/lib.rs         # Standalone typed RIME schema-subset parser
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
|-- third_party/typeduck-web/          # Vendored upstream + Yune integration seam
|   |-- source/                        # Upstream app (own librime/)
|   |-- yune-integration/ (adapter.ts, assets.ts, ...)
|   |-- e2e/ (Playwright)  / patches/  / typeduck-web.lock.json
|-- docs/                              # roadmap.md, decisions.md, requirements.md, plans/ (+ archive/),
|   |                                  #   typeduck-windows-backend-requirements.md, this file
|-- fixtures/                          # sample-*.json CLI fixtures + frontend-traces/
`-- .planning/codebase/                # Former generated maps (superseded by this doc)
```

**Where to add new behavior:**
- **Core engine** → `crates/yune-core/src/engine.rs` (+ `state.rs` for shape changes);
  test in `src/tests/engine.rs`.
- **Core translator / filter** → `crates/yune-core/src/translator/mod.rs` or
  `filter/mod.rs`; export via `lib.rs`; install (when schema-driven) in
  `yune-rime-api/src/schema_install.rs`; test in `src/tests/{translator,filter}.rs`.
- **Dictionary / encoder** → `crates/yune-core/src/dictionary/{source,compiled,encoder}.rs`.
- **RIME ABI function** → shape in `abi.rs` + `api_table.rs` (field order matches the fork
  header — see [§5](#5-c-abi-rules)); implement in the owning module
  (`context_api.rs`, `config_api.rs`, `deployment.rs`, `levers.rs`, ...); export via `lib.rs`.
- **TypeDuck-Web** → `crates/yune-rime-api/src/typeduck_web.rs`; new exports MUST be added
  to `scripts/typeduck-exports.txt`; TS in `packages/.../src/<area>.ts` + matching test.
- **Schema processor** → `crates/yune-rime-api/src/processors/<name>.rs`; per-session state
  in `session.rs`; installer call in `schema_selection.rs`/`schema_install.rs`.
- **CLI** → `args.rs` (parse), `main.rs` (dispatch), `sample_core.rs`/`rime_frontend.rs`,
  `transcript.rs`/`render.rs` (output).
- **Avoid a generic utility module** unless two-plus ownership areas need the same helper.

---

## 4. Coding Conventions

**Naming:**
- Rust modules `snake_case`; directory roots use `mod.rs` (a sibling facade may re-point
  via `#[path = "..."]`, e.g. `userdb.rs`). Crate package names kebab-case (`yune-core`).
- Functions/methods/locals/fields `snake_case`; booleans use `is_`/`has_` prefixes.
- Types (structs, enums, traits, errors) `UpperCamelCase`. C ABI mirror types are `Rime`-
  prefixed and `#[repr(C)]`. Tests are long behavior-specific `snake_case` sentences.
- TypeScript: `UpperCamelCase` interfaces/classes; `snake_case` JSON-mirroring fields
  (`page_size`, `page_no`, `is_last_page`); named `Error` subclasses carry failures.

**The TWO export families — do not mix them:**
- **librime-shaped ABI → `RimePascalCase`.** `#[no_mangle] extern "C"` functions mirroring
  librime's C ABI, e.g. `RimeConfigOpen` (`config_api.rs`), `RimeSetup` (`runtime.rs`).
- **Yune-owned WASM/browser ABI → `snake_case` `yune_typeduck_*`.** The 11 exports in
  `typeduck_web.rs`: `yune_typeduck_init`, `process_key`, `select_candidate`,
  `delete_candidate`, `flip_page`, `deploy`, `customize`, `cleanup`, `response_json`,
  `response_handled`, `free_response`. These names are an **explicit export contract**
  enforced by the allowlist `scripts/typeduck-exports.txt` and `-sEXPORTED_FUNCTIONS`.
  Add or rename an exported C function → **update the allowlist** or the WASM build
  silently drops it.

**Error handling:**
- Library parsers return custom error types implementing `Display`/`Error`
  (`KeySequenceParseError`, `SchemaParseError`, `TableDictionaryParseError`,
  `TableEncoderFormulaError`). CLI `run` returns `Result<(), String>` → stderr +
  `ExitCode::FAILURE`.
- C ABI functions return librime-shaped `Bool`/null instead of panicking; validate null
  pointers and invalid C strings at the boundary. Use `expect` only for internal invariants
  and test setup. Preserve librime-shaped fallback behavior explicitly in compatibility code.

**FFI safety:** FFI functions use explicit `unsafe extern "C" fn` signatures plus Rustdoc
`# Safety` sections and local `// SAFETY:` comments next to unsafe blocks (see
`config_api.rs`, `runtime.rs`, `ffi_memory.rs`).

**Name the external librime behavior.** When code mirrors librime, name the specific
upstream construct (class/function/field), not just "compatibility", so a reviewer can
trace it to the oracle — e.g. `// librime's Signature::Sign stores a trimmed ctime(3)
string.` in `lib.rs`. (librime is named in comments hundreds of times across the ABI crate.)
Avoid restating straightforward control flow. The TS runtime uses no TSDoc — it expresses
contracts through exported interfaces and named `Error` subclasses.

**EOL policy:** `.gitattributes` sets `* text=auto eol=lf` with explicit exceptions
(`*.bat`/`*.cmd` CRLF, `*.sh` LF, binaries `*.wasm`/`*.dll`/`*.so`/`*.exe` never normalized).
`.editorconfig` enforces UTF-8, LF, final newline, trailing-whitespace trim (`*.md` exempt
from the trim). The repo is developed on Windows but ships shell/WASM tooling — **do not
commit CRLF into normalized files**, or `.sh` scripts and byte-exact fixtures break.

**Formatting & lint gate:** `cargo fmt` (no repo `rustfmt.toml`). Quality gate:
`cargo clippy --workspace --all-targets -- -D warnings`. Root `Cargo.toml`
`[workspace.lints]` declares `rust.unsafe_code = "forbid"` and clippy
`all`/`pedantic = "warn"` — but **member crates do not opt in** with
`[lints] workspace = true`, so the forbid does not currently block the `unsafe extern "C"`
ABI code (this is tech debt — see [§9](#9-key-risks--concerns-current)). Public pure
accessors/constructors commonly carry `#[must_use]`. Imports group as std → external →
local (`crate::`/`super::`); no custom path aliases.

---

## 5. C ABI Rules

**`RimeApi` field order IS the ABI.** The `#[repr(C)]` function table in
`crates/yune-rime-api/src/abi.rs` is accessed by struct-pointer offset, so the order of its
fields is the actual C contract. Core ABI fields follow upstream `rime/librime`
headers; explicit TypeDuck-profile fork-only fields follow the TypeDuck fork's
`rime_api.h`. Never insert mid-struct without matching oracle/header evidence. A
misplaced field silently breaks every native frontend.

- **Fork-only slots:** `config_list_append_bool` / `_int` / `_double` / `_string`
  do not exist in upstream librime and are not exposed by default
  `rime_get_api()`. Their helper implementations remain in `config_api.rs` with
  direct TypeDuck-profile tests. If a named TypeDuck profile ABI surface is
  added later, its slot order must be re-derived from the TypeDuck fork's
  `rime_api.h`.
- **Locks:** default slot positions are locked by `assert_api_slot!` tests in
  `crates/yune-rime-api/src/tests/abi.rs` against upstream `1.17.0`. Never
  reorder/insert without updating these locks and confirming against the
  relevant upstream or explicit TypeDuck profile header. Historical TypeDuck
  slot rationale lives in `docs/plans/yune-windows-native-build.md` and
  `yune-windows-contract-implementation-plan.md`.
- **Comment panel:** `yune-core` ships `DictionaryLookupFilter` (filter name
  `"dictionary_lookup_filter"`, `filter/mod.rs`) emitting the TypeDuck comment-panel bytes —
  a leading `\u{000c}` form-feed, per-row `\r` markers, a `1`/`0` primary flag, and
  comma-joined multilingual dictionary fields. These bytes are golden (see [§7](#7-testing-conventions)).
- **Unsafe boundary:** keep all FFI pointer work, `CString`/`Box`/`Vec` raw conversions, and
  `RimeFree*` pairing in the ABI modules (`context_api.rs`, `candidate_api.rs`, `config_api.rs`,
  `ffi_memory.rs`, `typeduck_web.rs`) — never in `yune-core`.

---

## 6. Module & Test Ownership

**Own each slice.** Each behavior slice owns its production module *and* its tests.
`lib.rs`/`main.rs` stay thin **facades** — re-exports + orchestration glue only. Adding owned
engine/processor/config/ABI behavior directly into a facade hides ownership boundaries; put
it in the focused module instead.

- **Unit tests** live under `<crate>/src/tests/<slice>.rs` behind `#[cfg(test)] mod tests`
  (e.g. `yune-core/src/tests/{engine,filter,translator}.rs`;
  `yune-rime-api/src/tests/*.rs` with shared helpers in `tests/mod.rs`).
- **Integration / parity tests + oracle fixtures** live under `<crate>/tests/` (e.g.
  `yune-core/tests/cantonese_parity.rs` with goldens in `tests/fixtures/typeduck-v1.1.2/`;
  `yune-rime-api/tests/{frontend_client,dynamic_loader,frontend_hosts,typeduck_web}.rs`).
- **CLI sample fixtures** stay in the top-level `fixtures/` as `sample-*.json`.

Public re-exports stay centralized in crate facades; use module roots (`dictionary/mod.rs`,
`processors/mod.rs`, etc.) as barrels where they define ownership boundaries.

---

## 7. Testing Conventions

**Runner:** Rust built-in harness via Cargo; Vitest for the TS runtime. Standard assertions
only (`assert_eq!`, `assert!`, `panic!`) — no property/snapshot/mocking framework; mocking is
hand-written fakes (`CommentTranslator` in `tests/engine.rs`; `test/fake-{filesystem,module}.ts`).

```bash
cargo test --workspace                              # all Rust tests
cargo test -p yune-rime-api --test typeduck_web     # TypeDuck-Web ABI/adapter contract
cargo test -p yune-core --test upstream_luna_pinyin_parity # upstream 1.17.0 oracle parity
cargo test -p yune-core --test cantonese_parity     # v1.1.2 oracle parity
cargo clippy --workspace --all-targets -- -D warnings
npm test  # (in packages/yune-typeduck-runtime) -> vitest run
```

**Oracle-driven, NON-circular parity.** Compatibility tests capture expected bytes/behavior
from the **external oracle** (upstream librime / TypeDuck fork v1.1.2) into a checked-in
fixture, then run Yune's **real production path** and assert it reproduces the oracle output.
**Never derive the expected value from Yune itself.** Canonical example:
`upstream_luna_pinyin_parity.rs` uses official upstream `rime/librime 1.17.0`
release-binary fixtures for `luna_pinyin`. Curated mechanics fixtures feed
captured upstream dictionary/vocabulary rows through Yune's real
`TableDictionary` and `StaticTableTranslator` path. Full selection fixtures must
include every competing upstream dictionary row for the tested code plus
relevant `essay.txt` rows for every in-scope candidate so ranking cannot
silently use default/zero essay weights. Any behavior affected by menu state,
paging, selection, commit, filters, or options must drive Yune's real `Engine`
path or an equivalent full-pipeline harness; translator-direct output is only
mechanics coverage. `oracle_fixture_provenance.rs` scans all upstream
`luna_pinyin` JSON files for oracle identity, schema repository commits, capture
commands, source-row policies, and absence of local absolute cache paths.
Unsupported upstream behavior stays as an ignored test with a blocker string and
`panic!()` body, not as undocumented absence. TypeDuck profile example:
`cantonese_parity.rs` feeds raw TypeDuck TSV source rows through the real
`DictionaryLookupFilter` and compares the emitted comment against the golden
`tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json` (each comment begins with
the panel marker `\u{000c}\r1,`). A companion test locks the fixture's pinned
engine/tag/commit metadata.

**TypeDuck rich-comment E2E reproducibility.** The browser-shaped
`typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments` integration
test may use local TypeDuck v1.1.2 oracle build artifacts under
`target/typeduck-oracle/v1.1.2/rime-user/build` to prove the full
`jyut6ping3_mobile` runtime path emits the rich `\f\r1,.../\r0,...` comment
payload. That `target/` tree is ignored local oracle state, so the test must
emit an explicit skip reason when those build assets are absent and must never
silently pass against a degraded three-column fallback. The committed
clean-checkout byte-parity guarantee is still
`cargo test -p yune-core --test cantonese_parity`.

**`#[ignore]` must carry a documented blocker.** A blocked behavior gets a *named* test
marked `#[ignore = "blocked: <what is missing>"]` whose body `panic!()`s — never silently
drop a slice. The reason names the precise blocker (usually a missing oracle fixture). See the
5 ignored parity tests in `cantonese_parity.rs`.

**Tests exercise the public surface, not internals.** ABI/frontend tests obtain the table via
`rime_get_api()` and call its members, or call the exported `yune_typeduck_*` functions — the
same surface a real frontend uses. `dynamic_loader.rs` `dlopen`s the built cdylib and resolves
`rime_get_api`, the strongest "drive through the real ABI" guarantee.

**Cross-platform hygiene.** Yune builds for Unix, Windows (MSVC cdylib), and
`wasm32-unknown-emscripten`; tests must not assert platform-specific values.
- *Identical ctime shape.* The librime signature timestamp is computed two ways
  (`libc::ctime_r` on Unix, `format_ctime_utc` on non-Unix/emscripten) but both yield the same
  `ctime(3)` shape. Tests assert the *shape* (field count, weekday/month tokens, `HH:MM:SS`
  layout, numeric year) via `assert_librime_ctime_shape` in `tests/mod.rs`, never a value.
- *Poison-tolerant test locks vs panic-on-poison production locks.* Test-only lock helpers are
  poison-tolerant (`.unwrap_or_else(PoisonError::into_inner)`, `test_guard`/
  `notification_events_lock` in `tests/mod.rs`) so one failing test cannot cascade. **Production
  locks intentionally stay panic-on-poison** (`.expect("...should not be poisoned")`, e.g.
  `session.rs`, `lib.rs`). Know which side you are on. Hold a serializing
  `let _guard = test_guard();` in any test touching process-wide runtime state.

**Native adapter contract as the WASM-absent fallback.** When the
`wasm32-unknown-emscripten` target or `emcc`/`emar` are unavailable,
`scripts/typeduck-wasm-build.sh` deliberately runs `cargo test -p yune-rime-api --test
typeduck_web` so the WASM adapter contract is still validated without browser tooling.
Real-browser M9 validation is the web-first goal beyond this fallback.

**Fork-only ABI helper tests.** `tests/config_api.rs` guards the parked
TypeDuck-profile list-append helper behavior (`config_list_append_*`
round-trips). The default `rime_get_api()` table no longer exposes those
fork-only slots; its config-list contract is upstream `1.17.0`.

No coverage tooling/threshold is configured. Browser-level E2E for TypeDuck-Web is the M9 goal;
until then the `typeduck_web` ABI tests + native fallback + Vitest suite are the safety net.

---

## 8. Integrations

**librime oracle (validation-only, no runtime dependency).** Core Yune targets
upstream `github.com/rime/librime @ 1.17.0`. TypeDuck-Web/Windows profile tests
may target `github.com/TypeDuck-HK/librime @ v1.1.2`. Golden fixture
directories must name the oracle, e.g. `upstream-1.17.0/` or
`typeduck-v1.1.2/`. Never derive expected bytes from Yune, and never link or
call librime at runtime.

**TypeDuck-Web / Emscripten / IDBFS.** The `yune_typeduck_*` adapter (`typeduck_web.rs`)
exports 11 functions over the `rime_get_api()`/`rime_levers_get_api()` tables. The TS runtime
consumes the WASM module via Emscripten `cwrap`/`UTF8ToString`. **Browser persistence** uses an
Emscripten **IDBFS** mount over a virtual data dir, flushed with `FS.syncfs`:
`packages/.../src/filesystem.ts` defines `prepareTypeDuckFilesystem` (writes `default.yaml`,
`<schema>.schema.yaml`, `<dict>.dict.yaml`, `build/`) and explicit sync boundaries
(`syncFromPersistenceBeforeInit`; `syncToPersistenceAfterMutation` / `deployAndSync` /
`customizeAndSync` / `syncAfterUserDataChange`). The upstream seam adapter translates Yune's
`TypeDuckResponse` (`handled`, `commits`, `context.preedit`, `context.candidates`) into the
upstream `RimeResult` shape and parses key strings (`a`, `{BackSpace}`, `{Release+Enter}`) via
`keyEventToRimeKey`; patch scope is intentionally minimal.

**weasel / TypeDuck-Windows native.** Parked TypeDuck-profile work. The old
package path is retained as reference material, but `scripts/package-typeduck-windows.ps1`
now fails fast because default `rime_get_api()` follows upstream `1.17.0`.
Native packaging can resume only after a named TypeDuck profile ABI surface
exists (see [section 5](#5-c-abi-rules)). Frontend-host integration tests:
`tests/frontend_hosts/{native,native_frontends}.rs`.

**OpenCC.** `SimplifierFilter` (`filter/mod.rs`) honors a *limited subset* of librime OpenCC
config names — `from_opencc_config` maps e.g. `t2s`/`hk2s` (traditional→simplified), `t2tw`
(traditional→Taiwan). This is a **built-in approximation, not the real OpenCC library**; wired
during install in `schema_install.rs`.

**Other in-process seams:** RIME schema/config/dictionary YAML (the primary compatibility
boundary, via `serde_yaml` + local helpers); the frontend notification callback
(`RimeNotificationHandler`, `notifications.rs`); the module registry (`RimeModule`,
`modules.rs`); the AI ranking extension point (`CandidateRanker`/`MockAiRanker`, the seam for
the separate later AI layer). **No auth, no databases, no HTTP webhooks, no CI workflows** — all
calls are local library/CLI/FFI (native or in-browser WASM). User dictionaries are plain local
`*.userdb` files; sync snapshots are `*.userdb.txt`.

---

## 9. Key Risks / Concerns (current)

**M9 web validation is complete — residual risk is regression (keep gates
green).** The engine now runs through the real TypeDuck-Web browser E2E: the
HR-5 real-assets matrix passes (composition, paging, selection, deletion,
Space/phrase commit, deploy, customize, persistence sync, reload, dictionary
panel) against `jyut6ping3_mobile`, and HR-7 recorded **GO WITH CONDITIONS**.
The remaining concern is keeping it green: preserve the reproducible Emscripten
build, the runtime tests/build, the TypeDuck-Web worker build, and the committed
browser evidence on every merge. Files: `typeduck_web.rs`,
`packages/yune-typeduck-runtime/`, `scripts/typeduck-wasm-build.sh`,
`docs/plans/archive/typeduck-web-validation-plan.md`.

**Upstream latest-stable behavioral closeout is complete for the first
`luna_pinyin` gate.** Default core behavior and default `RimeApi` follow
upstream `rime/librime 1.17.0`. The active gate covers curated single-code
mechanics, full `ni` dictionary selection with essay weights, Engine
paging/selection/commit, reverse lookup, punctuation/symbols, and supported
option paths. `zhongguo` phrase/language-model output, `ascii_punct` processor
bypass, and punctuation immediate commit remain explicit ignored blockers.
TypeDuck-derived fixtures remain profile-only unless separate upstream goldens
prove the same behavior.

**TypeDuck-Windows ABI/package work is parked and not yet proven as a current
profile.** A pre-M12 package smoke built `rime.dll`/`rime.lib`/headers and
checked a TypeDuck fork-only slot, but that smoke is archived evidence only and
is not valid against the default upstream `rime_get_api()` table. The script now
fails fast until a named TypeDuck profile ABI surface exists. The real
TypeDuck-Windows/weasel frontend E2E also remains blocked: the pinned fork
checkout lacked the referenced integration-plan files, and local frontend build
tools (`msbuild.exe`, `cmake.exe`, etc.) were not on PATH.

**Comment-parity oracle gaps (narrowed).** HR-6 added oracle coverage for the reverse-lookup
`"; "` joiner (`comments.join("; ")` in `filter/mod.rs`) and schema-name-in-prompt parity, so
those are now byte-locked. **Still not golden-covered:** five Cantonese/Jyutping parity cases
remain `#[ignore]`d pending dedicated v1.1.2 goldens (options `combine_candidates`/`show_full_code`/
`enable_sentence`; completion/prediction; correction minimal-distance + m-abbreviation; schema-
menu hiding; per-entry userdb pronunciation). Regressions in those pass CI because no
oracle asserts them.

**Process-global single RIME service.** Runtime paths, sessions, module pointers, notifications,
state-label cache, API tables, and switcher registries are process-wide singletons (e.g.
`runtime_paths()` is `OnceLock<Mutex<RuntimePaths>>`). The TypeDuck-Web handoff contract requires
**exactly one active process-global RIME service per WASM instance** with host-owned MEMFS/IDBFS
layout and explicit host-driven sync; multiple concurrent engines/schemas in one instance are out
of scope, and these singletons are load-bearing for that model. `yune_typeduck_init` drives
setup/initialize/create_session/select_schema against this single global service.

**Other notable items (condensed):** workspace lints are declared but not enabled by member
crates (command-line discipline only); the core facade `lib.rs` (~3.3k lines) and ABI facade
`lib.rs` (~1.9k lines) still own residual inline tests / cross-module glue; production session
locks panic on poison (a scaling limit); dictionary lookup is linear and clones large snapshots
(performance); the userdb store is file-backed (not LevelDB); `SimplifierFilter` is an OpenCC
approximation. No production `TODO`/`FIXME` markers exist — use `docs/roadmap.md`
and this doc as the active issue inventory.

## 10. Planning Docs

Planning, decisions, and conventions live under `docs/` — there is no external planning tool (the GSD system was retired). Layout: `docs/roadmap.md` (the milestone map), `docs/decisions.md`, `docs/requirements.md`, this file, and `docs/plans/` (per-stage plans / findings / build notes, with finished ones under `docs/plans/archive/`).

**Every doc under `docs/plans/` (and `archive/`) opens with a status banner as its second line, and the banner MUST name the milestone/stage it belongs to** so its scope is clear at a glance:

```
> **Status:** <Active|Reopened|Parked|Finished|Superseded> · **Milestone:** M<n> (short name) · **Updated|Closed:** YYYY-MM-DD · **Type:** <execution plan|findings|reference|record>
```

- **`Milestone` is a required field, kept separate from `Status`.** Write `**Status:** Parked · **Milestone:** M10 (…)`, never `**Status:** Parked (M10)`. Append the within-milestone stage where useful, e.g. `**Milestone:** M9 — stage HR-3`.
- Use `Updated:` for `Active`/`Reopened`/`Parked` docs and `Closed:` for `Finished`/`Superseded` ones.
- `grep -rn "Status:" docs/plans` is the at-a-glance dashboard of every plan — its milestone/stage and its state.
- **Finished or superseded plans move to `docs/plans/archive/`** (banner flipped accordingly), never deleted — the trail stays.
- The milestone → plans → status mapping lives in `docs/roadmap.md`; keep a plan's banner milestone consistent with the roadmap.

---

*Last reviewed: 2026-06-19 - M12 upstream behavioral parity closeout complete; default RimeApi follows upstream 1.17.0 and TypeDuck-Windows ABI/package work is parked pending a named profile surface.*
