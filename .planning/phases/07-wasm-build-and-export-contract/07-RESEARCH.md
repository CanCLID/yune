# Phase 7: WASM Build And Export Contract - Research

**Researched:** 2026-05-04  
**Domain:** Rust C ABI exports, Emscripten `wasm32-unknown-emscripten`, browser filesystem contract  
**Confidence:** HIGH for current repo/export state and official Emscripten/Rust build mechanics; MEDIUM for exact script shape because Phase 7 leaves that to planner discretion

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
## Implementation Decisions

### WASM Target Contract
- **D-01:** Treat `wasm32-unknown-emscripten` as the intended browser build target for this milestone because TypeDuck-Web-style integration needs Emscripten C ABI exports and filesystem/runtime hooks, not `wasm-bindgen` as the primary contract.
- **D-02:** Phase 7 should define a single documented build command or script path that attempts the Emscripten build and reports a reproducible local-toolchain blocker when Emscripten is unavailable. Missing Emscripten locally is not a failure if the blocker is deterministic and native adapter tests remain the fallback.
- **D-03:** Keep the Rust adapter in `crates/yune-rime-api` for this phase. Do not create a separate adapter crate unless planning proves the export/build contract cannot be expressed safely from the existing cdylib crate.

### Export Retention Verification
- **D-04:** The required browser export surface is the seeded `yune_typeduck_*` API: init, process-key, select-candidate, delete-candidate, flip-page, deploy, customize, cleanup, response-json, response-handled, and free-response.
- **D-05:** Add a deterministic symbol/export verification path that works in native mode and, when Emscripten output exists, checks the generated WASM/JS artifact for the same required symbol names.
- **D-06:** Export verification should be adapter-specific and must not broaden or mutate the existing librime-shaped `RimeApi` function table. The Phase 7 contract verifies symbol presence for JS callers; Phase 8 owns typed JS call ergonomics.

### Local Toolchain Fallback
- **D-07:** Native adapter contract tests in `crates/yune-rime-api/tests/typeduck_web.rs` remain the authoritative fallback when the local machine cannot build or run the browser target.
- **D-08:** Toolchain detection should fail with an actionable, reproducible message naming the missing Emscripten/Rust target component rather than silently skipping all validation.
- **D-09:** The planner may choose whether the detection lives in a small script, Cargo alias-like documented command, integration test helper, or docs-first command block, provided CI/local behavior is explicit and easy to rerun.

### Browser Constraint Documentation
- **D-10:** Documentation must make the one-active-process-global-service constraint visible: `yune_typeduck_cleanup` finalizes the process-global RIME service, and multiple simultaneous TypeDuck states with different dirs are not supported by this first contract.
- **D-11:** Documentation must state browser host assumptions for MEMFS/IDBFS paths: shared data, user data, and `user_data_dir/build` must exist before init; schema/dictionary assets must be preloaded; persistence sync remains a JS host responsibility until Phase 9.
- **D-12:** Document required linker/export flags and known host assumptions without requiring upstream TypeDuck-Web source access in Phase 7. Upstream clone/replace testing is explicitly Phase 10.

### Claude's Discretion
- Choose the exact script name, artifact inspection command, and docs location during planning as long as the commands are deterministic, preserve MSRV/workspace quality gates, and keep owned behavior out of `lib.rs` facades.
- Prefer small, reviewable build-contract checks over introducing a full browser bundler, npm package, or TypeDuck-Web checkout in this phase.

### Deferred Ideas (OUT OF SCOPE)
## Deferred Ideas

- TypeScript wrapper types, response parsing/freeing enforcement, and browser keycode/mask mapping remain Phase 8.
- Browser virtual filesystem layout tests, asset preload orchestration, IDBFS sync, and stale/missing asset recovery remain Phase 9.
- Cloning upstream TypeDuck-Web, replacing its librime/WASM core with Yune, and real browser E2E validation remain Phase 10.
- AI-native provider, ranking, context, memory, and privacy work remains deferred until the TypeDuck-Web integration milestone produces a frontend exposure recommendation.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TYPEDUCK-WASM-01 | Developer can build the TypeDuck adapter for the intended Emscripten/WASM target or reproduce a documented local-toolchain blocker. | Use `wasm32-unknown-emscripten`; verify `rustup target add wasm32-unknown-emscripten` and `emcc` requirements; local audit currently finds Rust tools but no installed Emscripten target or `emcc`. [CITED: rust-lang/rust wasm32-unknown-emscripten docs] [VERIFIED: local tool audit] |
| TYPEDUCK-WASM-02 | The browser build preserves all required `yune_typeduck_*` exports for JS callers. | Required export list is already present in `typeduck_web.rs`, documented in `docs/typeduck-web-adapter.md`, and visible in native cdylib `nm` output; Emscripten should use `-sEXPORTED_FUNCTIONS=` with underscore-prefixed native symbol names. [VERIFIED: current codebase] [VERIFIED: native nm] [CITED: emscripten.org settings reference] |
| TYPEDUCK-WASM-03 | Native adapter contract tests remain the deterministic fallback when local browser/WASM tooling is unavailable. | `cargo test -p yune-rime-api --test typeduck_web` passes 4/4 tests from the main checkout and should be the fallback gate when Emscripten build prerequisites are missing. [VERIFIED: cargo test] |
</phase_requirements>

## Summary

Phase 7 should plan a build/export contract around the already-seeded TypeDuck adapter in `crates/yune-rime-api/src/typeduck_web.rs`, not around missing symbols or a new adapter crate. [VERIFIED: current codebase] The current adapter exports 11 `#[no_mangle] unsafe extern "C"` / `extern "C"` symbols, and the native `cdylib` build exposes all 11 names in `/Users/trenton/Projects/yune/target/debug/libyune_rime_api.dylib`. [VERIFIED: current codebase] [VERIFIED: native nm]

The intended browser target is `wasm32-unknown-emscripten`; Rust’s official target documentation states that this target uses the Emscripten compiler toolchain and requires the `emcc` linker. [CITED: rust-lang/rust wasm32-unknown-emscripten docs] Emscripten’s official docs state that JS-callable C functions must be preserved with `EXPORTED_FUNCTIONS`, native symbols use underscore-prefixed names in that list, and `cwrap`/`ccall` require exported runtime methods when accessed from external JS. [CITED: emscripten.org preamble/settings docs]

Local execution should be deterministic even without browser tooling: the repository currently has Rust/Cargo/Rustup available under `/Users/trenton/.cargo/bin`, but `wasm32-unknown-emscripten`, `emcc`, `emar`, `wasm-nm`, and `wasm-objdump` are not available in the audited environment. [VERIFIED: local tool audit] The planner should therefore create one command/script that first checks these prerequisites, then either builds/verifies artifacts or exits with an actionable blocker while still running native fallback tests. [VERIFIED: CONTEXT.md decisions]

**Primary recommendation:** Add a small adapter-specific build/check script plus export-list file, extend `docs/typeduck-web-adapter.md`, and keep native `typeduck_web` tests as the mandatory fallback gate; do not add owned build behavior to `crates/yune-rime-api/src/lib.rs`. [VERIFIED: CONTEXT.md decisions] [VERIFIED: current codebase]

## Project Constraints (from CLAUDE.md)

No `/Users/trenton/Projects/yune/CLAUDE.md` file exists in the current project checkout, so there are no CLAUDE.md-specific directives to apply. [VERIFIED: filesystem check]

Project skill discovery found no project skill definitions under `/Users/trenton/Projects/yune/.claude/skills/` or `/Users/trenton/Projects/yune/.agents/skills/`. [VERIFIED: filesystem check]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Rust TypeDuck C/WASM export surface | API / Backend ABI layer (`yune-rime-api`) | Browser / Client JS caller | The Rust ABI crate owns exported `yune_typeduck_*` symbols; browser JS only calls them through generated Emscripten glue. [VERIFIED: current codebase] |
| Emscripten build contract | Build tooling | API / Backend ABI layer | The build command must compile `yune-rime-api` for `wasm32-unknown-emscripten` and retain ABI exports without changing runtime behavior. [CITED: rust-lang/rust wasm32-unknown-emscripten docs] [VERIFIED: CONTEXT.md decisions] |
| Native fallback validation | Test infrastructure | API / Backend ABI layer | `crates/yune-rime-api/tests/typeduck_web.rs` directly validates the adapter contract when Emscripten is unavailable. [VERIFIED: current codebase] [VERIFIED: cargo test] |
| Browser virtual filesystem assumptions | Browser / Client host | API / Backend ABI layer | Emscripten MEMFS/IDBFS mounting and `FS.syncfs` are JS host responsibilities; the Rust adapter receives only path strings. [CITED: emscripten.org filesystem docs] [VERIFIED: docs/typeduck-web-adapter.md] |
| Runtime lifecycle / one active service | API / Backend ABI layer | Browser / Client host | `yune_typeduck_cleanup` finalizes process-global RIME service state, so callers must not assume independent simultaneous services with different dirs. [VERIFIED: typeduck_web.rs] [VERIFIED: CONTEXT.md decisions] |

## Standard Stack

### Core

| Library / Tool | Version | Purpose | Why Standard |
|----------------|---------|---------|--------------|
| Rust workspace / Cargo | `rustc 1.95.0`, `cargo 1.95.0` locally; project MSRV `1.76` | Compile and test workspace crates. | The project is a Rust workspace and `Cargo.toml` declares Rust 2021 plus MSRV 1.76. [VERIFIED: local tool audit] [VERIFIED: Cargo.toml] |
| `yune-rime-api` | `0.1.0` | Owns RIME-style C ABI, TypeDuck adapter exports, and native fallback tests. | Existing seeded adapter lives in this crate and its manifest already declares `crate-type = ["rlib", "cdylib"]`. [VERIFIED: cargo metadata] [VERIFIED: crates/yune-rime-api/Cargo.toml] |
| `wasm32-unknown-emscripten` Rust target | Not installed locally | Browser/WASM compilation target for the TypeDuck adapter. | Official Rust docs define it as the Emscripten-backed WebAssembly target and show installing it with `rustup target add wasm32-unknown-emscripten`. [CITED: rust-lang/rust wasm32-unknown-emscripten docs] [VERIFIED: local tool audit] |
| Emscripten SDK (`emcc`, `emar`) | Not installed locally | Link Rust output into Emscripten JS/WASM artifacts and provide runtime/filesystem hooks. | Official Rust docs state `wasm32-unknown-emscripten` requires the Emscripten toolchain and specifically the `emcc` linker. [CITED: rust-lang/rust wasm32-unknown-emscripten docs] [VERIFIED: local tool audit] |
| Emscripten `EXPORTED_FUNCTIONS` | Emscripten setting | Preserve `yune_typeduck_*` symbols for JS callers. | Official Emscripten docs state functions must be exported with `EXPORTED_FUNCTIONS` to be callable through `ccall`/`cwrap` and preserved through optimization; native symbols require underscore prefixes. [CITED: emscripten.org settings reference] |
| Cargo integration tests | Built-in Cargo test harness | Deterministic fallback validation. | Current native adapter tests pass and exercise lifecycle, JSON state, candidate actions, deploy/customize, null handling, and response freeing. [VERIFIED: cargo test] [VERIFIED: current codebase] |

### Supporting

| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| `serde_json` | `1.0.149` locked; registry version verified as `1.0.149` | Adapter JSON response serialization. | Keep because `typeduck_web.rs` already uses `serde_json::json` for response payloads. [VERIFIED: Cargo.lock] [VERIFIED: crates.io via cargo info/search] [VERIFIED: typeduck_web.rs] |
| `regex` | `1.12.3` locked; registry version verified as `1.12.3` | Existing schema/key processing dependency. | Preserve; not Phase 7-specific but part of `yune-rime-api` build closure. [VERIFIED: Cargo.lock] [VERIFIED: crates.io via cargo info/search] |
| `serde_yaml` | `0.9.34+deprecated` locked; registry version verified as `0.9.34+deprecated` | Existing RIME config/schema YAML behavior. | Preserve for deploy/customize/runtime config paths; do not change during build-contract work. [VERIFIED: Cargo.lock] [VERIFIED: crates.io via cargo info/search] |
| `libc` | `0.2.186` locked; latest stable line for current dependency is still `0.2.186` while crates.io also lists `1.0.0-alpha.3` | Existing native ABI support. | Preserve current 0.2 line unless a separate dependency policy decision is made. [VERIFIED: Cargo.lock] [VERIFIED: crates.io via cargo info/search] |
| `libloading` | `0.8.9` locked; crates.io also lists `0.9.0` | Existing dynamic-loader tests. | Preserve for native ABI loader coverage; not required for browser build itself. [VERIFIED: Cargo.lock] [VERIFIED: crates.io via cargo info/search] |
| `nm` / `llvm-nm` | Available as `llvm-nm, compatible with GNU nm` | Inspect native dynamic library symbols. | Use for native export verification after `cargo build -p yune-rime-api`. [VERIFIED: local tool audit] [VERIFIED: native nm] |
| `wasm-nm` or `wasm-objdump` | Not installed locally | Inspect generated Emscripten WASM exports when available. | Use only when Emscripten artifacts exist; otherwise report deterministic blocker and rely on native fallback. [VERIFIED: local tool audit] |
| Node.js | `v24.14.1` local | Optional JS artifact text inspection or Emscripten-generated JS smoke checks. | Useful for scripts, but Phase 7 should not introduce npm/bundler/package infrastructure. [VERIFIED: local tool audit] [VERIFIED: CONTEXT.md decisions] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `wasm32-unknown-emscripten` | `wasm32-unknown-unknown` + `wasm-bindgen` | Out of scope because D-01 locks Emscripten as the target needed for C ABI exports and filesystem/runtime hooks. [VERIFIED: CONTEXT.md decisions] |
| Adapter-specific build script | Full npm package/bundler | Out of scope because Phase 7 should prefer small build-contract checks and leave wrapper/package work to Phase 8. [VERIFIED: CONTEXT.md decisions] |
| Native `typeduck_web` tests | Browser E2E / TypeDuck-Web checkout | Browser E2E and upstream TypeDuck-Web source access are Phase 10, not Phase 7. [VERIFIED: CONTEXT.md decisions] |
| Export-list verification | Broad `RimeApi` function table mutation | Out of scope because D-06 requires adapter-specific verification and forbids broadening/mutating the librime-shaped function table for this contract. [VERIFIED: CONTEXT.md decisions] |

**Installation / setup commands:**

```bash
# Rust target installation required before a real Emscripten build.
rustup target add wasm32-unknown-emscripten

# Emscripten SDK must be installed and activated so emcc/emar are on PATH.
# Exact emsdk installation path is host-specific; Phase 7 should detect, not vendor, emsdk.
```

**Version verification:** Cargo package versions were verified with `cargo metadata`, `Cargo.lock`, and `cargo info/search --registry crates-io`; local tools were verified with `rustc --version`, `cargo --version`, `rustup --version`, `command -v`, and version probes. [VERIFIED: cargo metadata] [VERIFIED: Cargo.lock] [VERIFIED: local tool audit]

## Architecture Patterns

### System Architecture Diagram

```text
Developer command
  |
  v
Phase 7 build/check script
  |
  +--> Prerequisite detection
  |      |
  |      +--> rustup target missing? --> Actionable blocker + run native fallback tests
  |      |
  |      +--> emcc/emar missing? -----> Actionable blocker + run native fallback tests
  |      |
  |      +--> wasm inspection missing? -> Build may run; export check reports inspection blocker
  |
  v
Cargo build -p yune-rime-api --target wasm32-unknown-emscripten
  |
  v
Emscripten link flags
  |
  +--> -sEXPORTED_FUNCTIONS=[_yune_typeduck_*]
  +--> -sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString
  +--> filesystem runtime available to JS host
  |
  v
Generated JS/WASM artifact
  |
  +--> Artifact export verification with wasm-nm/wasm-objdump or generated JS symbol scan
  |
  v
Browser / TypeDuck-Web host in later phases
  |
  +--> Mount MEMFS/IDBFS and preload shared/user/build files
  +--> Call yune_typeduck_init/process/action/deploy/customize/cleanup
  +--> Pair every owned response with yune_typeduck_free_response
```

This diagram intentionally stops before TypeScript wrapper and real TypeDuck-Web E2E because those are deferred to Phases 8 and 10. [VERIFIED: CONTEXT.md decisions]

### Recommended Project Structure

```text
/Users/trenton/Projects/yune/
├── crates/yune-rime-api/
│   ├── src/typeduck_web.rs          # Existing adapter-owned exported symbols
│   ├── src/lib.rs                   # Facade only; keep as `mod typeduck_web; pub use typeduck_web::*;`
│   ├── tests/typeduck_web.rs        # Existing native fallback contract tests
│   └── Cargo.toml                   # Existing rlib/cdylib crate type
├── scripts/                         # Recommended home for deterministic build/export checks [ASSUMED]
│   ├── typeduck-wasm-build.sh       # Detect target/emcc, build if possible, report blocker if not [ASSUMED]
│   └── typeduck-exports.txt         # Canonical 11-symbol export list [ASSUMED]
└── docs/typeduck-web-adapter.md     # Extend with build command, flags, blockers, host assumptions
```

The `scripts/` directory does not currently exist in the observed project root, so the exact script location is a planner decision under D-09. [VERIFIED: filesystem check] [VERIFIED: CONTEXT.md decisions]

### Pattern 1: Adapter-Owned Export List

**What:** Keep a canonical adapter-specific list of the 11 `yune_typeduck_*` symbols and use it for native and Emscripten verification. [VERIFIED: current codebase]

**When to use:** Use in every Phase 7 build/check path so docs, native `nm`, and Emscripten exports cannot drift. [VERIFIED: CONTEXT.md decisions]

**Required symbols:**

```text
yune_typeduck_init
yune_typeduck_process_key
yune_typeduck_select_candidate
yune_typeduck_delete_candidate
yune_typeduck_flip_page
yune_typeduck_deploy
yune_typeduck_customize
yune_typeduck_cleanup
yune_typeduck_response_json
yune_typeduck_response_handled
yune_typeduck_free_response
```

Each name above is present in `crates/yune-rime-api/src/typeduck_web.rs`, documented in `docs/typeduck-web-adapter.md`, and visible in native cdylib `nm` output with platform-leading underscores on macOS. [VERIFIED: current codebase] [VERIFIED: native nm]

### Pattern 2: Deterministic Toolchain Detection Before Build

**What:** Check `rustup`, installed `wasm32-unknown-emscripten`, `emcc`, and `emar` before invoking the browser build. [CITED: rust-lang/rust wasm32-unknown-emscripten docs]

**When to use:** Always run detection first so missing browser tooling produces the same actionable blocker locally and in CI. [VERIFIED: CONTEXT.md decisions]

**Example blocker text:**

```text
TypeDuck WASM build blocked: missing wasm32-unknown-emscripten Rust target.
Install with: rustup target add wasm32-unknown-emscripten
Native fallback still available: cargo test -p yune-rime-api --test typeduck_web
```

```text
TypeDuck WASM build blocked: missing Emscripten linker `emcc` on PATH.
Install/activate Emscripten SDK so `emcc` and `emar` are available, then rerun this command.
Native fallback still available: cargo test -p yune-rime-api --test typeduck_web
```

The local audit currently needs both blocker paths because the Rust target and Emscripten tools are missing. [VERIFIED: local tool audit]

### Pattern 3: Emscripten Export Flags

**What:** Pass an explicit export list to Emscripten with underscore-prefixed native names. [CITED: emscripten.org settings reference]

**When to use:** Use whenever producing the browser JS/WASM artifact; otherwise optimized Emscripten output may omit functions that JS expects to call. [CITED: emscripten.org preamble/settings docs]

**Example flag shape:**

```bash
-sEXPORTED_FUNCTIONS=_yune_typeduck_init,_yune_typeduck_process_key,_yune_typeduck_select_candidate,_yune_typeduck_delete_candidate,_yune_typeduck_flip_page,_yune_typeduck_deploy,_yune_typeduck_customize,_yune_typeduck_cleanup,_yune_typeduck_response_json,_yune_typeduck_response_handled,_yune_typeduck_free_response
-sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString
```

`EXPORTED_RUNTIME_METHODS=ccall,cwrap` is required when those preamble helpers are accessed from outside generated code, and `UTF8ToString` is needed by the current documented JS response-copy pattern. [CITED: emscripten.org preamble docs] [VERIFIED: docs/typeduck-web-adapter.md]

### Pattern 4: Native Fallback Gate

**What:** Run the existing adapter integration test when Emscripten tooling is missing. [VERIFIED: current codebase]

**When to use:** Use on every local/CI path where `emcc` or the target is unavailable; this proves the adapter contract remains intact even without browser artifacts. [VERIFIED: CONTEXT.md decisions]

**Command:**

```bash
cargo test -p yune-rime-api --test typeduck_web
```

The command passed locally with 4 tests, 0 failures. [VERIFIED: cargo test]

### Anti-Patterns to Avoid

- **Assuming symbols are missing:** The seeded TypeDuck symbols already exist and are natively exported; Phase 7 should preserve/verify them, not recreate them. [VERIFIED: current codebase] [VERIFIED: native nm]
- **Moving adapter logic into `lib.rs`:** `lib.rs` currently only wires `mod typeduck_web; pub use typeduck_web::*;`; owned behavior should stay in `typeduck_web.rs`, scripts, docs, or tests. [VERIFIED: current codebase] [VERIFIED: architecture map]
- **Replacing Emscripten with wasm-bindgen:** D-01 locks Emscripten for this milestone. [VERIFIED: CONTEXT.md decisions]
- **Silently skipping browser validation:** Missing Emscripten is acceptable only when the command reports a reproducible blocker and runs native fallback validation. [VERIFIED: CONTEXT.md decisions]
- **Adding TypeScript/bundler/TypeDuck-Web checkout work:** Those are explicitly deferred to later phases. [VERIFIED: CONTEXT.md decisions]
- **Broadening `RimeApi`:** Export verification is adapter-specific and must not mutate the librime-shaped function table. [VERIFIED: CONTEXT.md decisions]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Browser-target compilation | Custom Rust-to-WASM pipeline | Cargo with `--target wasm32-unknown-emscripten` plus Emscripten toolchain | Rust officially supports the Emscripten target and requires the Emscripten linker. [CITED: rust-lang/rust wasm32-unknown-emscripten docs] |
| JS-callable symbol retention | Ad hoc generated JS wrappers that assume exports exist | Emscripten `-sEXPORTED_FUNCTIONS=` with canonical adapter list | Emscripten docs state functions must be exported to be callable via `ccall`/`cwrap` and preserved through optimization. [CITED: emscripten.org preamble/settings docs] |
| JS runtime helper exposure | Manual copies of Emscripten preamble helpers | `-sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString` | Emscripten docs provide a standard mechanism for exposing runtime methods on `Module`. [CITED: emscripten.org preamble docs] |
| Browser persistence | Rust-side IndexedDB logic or custom persistence shim | JS host mounts MEMFS/IDBFS and calls `FS.syncfs` | Emscripten’s filesystem docs define MEMFS/IDBFS and `FS.syncfs`; the Rust adapter only takes path strings. [CITED: emscripten.org filesystem docs] [VERIFIED: docs/typeduck-web-adapter.md] |
| Native fallback validation | New fake ABI harness | Existing `crates/yune-rime-api/tests/typeduck_web.rs` | Current tests already exercise the seeded adapter contract and pass locally. [VERIFIED: current codebase] [VERIFIED: cargo test] |
| Export inspection | Manual visual review of docs | Scripted `nm` / `wasm-nm` / generated JS symbol scan against canonical list | Deterministic symbol checks satisfy D-05 and reduce drift. [VERIFIED: CONTEXT.md decisions] |

**Key insight:** The hard part is not inventing a new adapter; the hard part is making symbol retention, missing-tool behavior, and browser host assumptions reproducible enough that later TypeScript and TypeDuck-Web phases can depend on them. [VERIFIED: current codebase] [VERIFIED: CONTEXT.md decisions]

## Common Pitfalls

### Pitfall 1: Confusing Rust `no_mangle` With Emscripten Export Retention

**What goes wrong:** A function exists in Rust source but is not callable from JS because Emscripten optimization removed it or did not expose it. [CITED: emscripten.org preamble/settings docs]

**Why it happens:** `#[no_mangle] extern "C"` gives a stable native symbol name, but Emscripten still requires exported functions to be listed for `ccall`/`cwrap` access. [CITED: emscripten.org preamble/settings docs] [CITED: rust-lang/rust symbol docs]

**How to avoid:** Generate or maintain `EXPORTED_FUNCTIONS` from the canonical `yune_typeduck_*` list with underscore-prefixed names. [CITED: emscripten.org settings reference]

**Warning signs:** Generated JS `Module.cwrap('yune_typeduck_init', ...)` fails at runtime; artifact inspection does not show `_yune_typeduck_init`. [CITED: emscripten.org preamble docs]

### Pitfall 2: Treating Missing Emscripten As Success

**What goes wrong:** CI/local scripts skip all browser-target checks and still pass without proving either the build or fallback contract. [VERIFIED: CONTEXT.md decisions]

**Why it happens:** External toolchain checks are often hidden behind optional local setup. [ASSUMED]

**How to avoid:** Script exits should distinguish “build verified” from “browser build blocked, native fallback passed.” [VERIFIED: CONTEXT.md decisions]

**Warning signs:** Logs do not name missing `wasm32-unknown-emscripten`, `emcc`, or `emar`; no `typeduck_web` fallback test output appears. [VERIFIED: local tool audit] [VERIFIED: CONTEXT.md decisions]

### Pitfall 3: Expanding Phase 7 Into TypeScript or Browser E2E

**What goes wrong:** The phase grows into npm packaging, TypeScript wrapper design, browser persistence orchestration, or upstream TypeDuck-Web replacement. [VERIFIED: CONTEXT.md decisions]

**Why it happens:** Emscripten artifacts are adjacent to JS integration, but the phase boundary is only build/export contract. [VERIFIED: CONTEXT.md decisions]

**How to avoid:** Limit modifications to scripts/checks/docs/native tests unless a tiny code change is required to preserve the existing adapter exports. [VERIFIED: CONTEXT.md decisions]

**Warning signs:** New `package.json`, bundler config, upstream TypeDuck-Web checkout, or browser E2E harness appears in Phase 7. [VERIFIED: CONTEXT.md decisions]

### Pitfall 4: Forgetting Process-Global Lifecycle Constraints

**What goes wrong:** Documentation or tests imply multiple independent TypeDuck states with different dirs can run simultaneously. [VERIFIED: CONTEXT.md decisions]

**Why it happens:** `YuneTypeDuckState` looks per-instance, but cleanup finalizes the process-global RIME API state. [VERIFIED: typeduck_web.rs]

**How to avoid:** Keep tests serialized with the existing mutex pattern and document one active process-global service. [VERIFIED: current codebase] [VERIFIED: CONTEXT.md decisions]

**Warning signs:** Parallel adapter tests initialize different runtime dirs at once; docs omit the cleanup/finalize side effect. [VERIFIED: current codebase]

### Pitfall 5: Browser Filesystem Assumptions Drift From Adapter Reality

**What goes wrong:** The browser host calls `init` before shared data, user data, and `user_data_dir/build` are mounted/preloaded. [VERIFIED: docs/typeduck-web-adapter.md]

**Why it happens:** Emscripten has an in-memory filesystem by default, while persistence requires explicit IDBFS mounting and syncing. [CITED: emscripten.org filesystem docs]

**How to avoid:** Document that MEMFS/IDBFS setup, asset preload, and `FS.syncfs(true/false)` remain JS host responsibilities until Phase 9. [CITED: emscripten.org filesystem docs] [VERIFIED: CONTEXT.md decisions]

**Warning signs:** `yune_typeduck_init` returns null in browser because deployed config/schema/dictionary files are absent. [VERIFIED: typeduck_web.rs] [VERIFIED: docs/typeduck-web-adapter.md]

## Code Examples

Verified patterns from official/current sources:

### Native Symbol Verification

```bash
cargo build -p yune-rime-api
nm -gU target/debug/libyune_rime_api.dylib | grep 'yune_typeduck_'
```

Expected local native output currently includes all 11 names with leading underscores on macOS. [VERIFIED: native nm]

### Emscripten Target Prerequisite

```bash
rustup target add wasm32-unknown-emscripten
```

Rust’s official documentation gives this target installation command. [CITED: rust-lang/rust wasm32-unknown-emscripten docs]

### Emscripten Export Flag Shape

```bash
-sEXPORTED_FUNCTIONS=_yune_typeduck_init,_yune_typeduck_process_key,_yune_typeduck_select_candidate,_yune_typeduck_delete_candidate,_yune_typeduck_flip_page,_yune_typeduck_deploy,_yune_typeduck_customize,_yune_typeduck_cleanup,_yune_typeduck_response_json,_yune_typeduck_response_handled,_yune_typeduck_free_response
-sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString
```

Emscripten docs state native symbols require underscore prefixes in `EXPORTED_FUNCTIONS` and runtime methods such as `ccall`/`cwrap` must be exported to access them externally. [CITED: emscripten.org settings/preamble docs]

### Native Fallback Test Gate

```bash
cargo fmt --all -- --check
cargo test -p yune-rime-api --test typeduck_web
```

Both commands passed in the current checkout; the test target ran 4 tests successfully. [VERIFIED: cargo test]

### Browser FS Sync Responsibilities

```js
FS.syncfs(true, (err) => {
  if (err) throw err;
  // call yune_typeduck_init after persistent data is loaded into the mounted FS
});

FS.syncfs(false, (err) => {
  if (err) throw err;
  // data has been flushed back to IndexedDB-backed storage
});
```

Emscripten docs define `FS.syncfs(true)` as populating from persistent storage and `FS.syncfs(false)` as saving to persistent storage. [CITED: emscripten.org filesystem docs]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Assuming `yune-rime-api` has no `cdylib` output | Current `crates/yune-rime-api/Cargo.toml` declares `crate-type = ["rlib", "cdylib"]` | Before this Phase 7 research; codebase map is stale on this point | Planning should not add `cdylib` as if absent; it should preserve and verify it. [VERIFIED: current codebase] |
| Treating TypeDuck adapter as missing | Current `typeduck_web.rs` exports the seeded 11-symbol bridge | Seeded before formal Phase 7 planning | Planning should focus on build/export retention, not adapter creation. [VERIFIED: current codebase] [VERIFIED: STATE.md] |
| Browser validation as a Phase 6 exploratory gap | Phase 7 now locks an Emscripten build/export contract before JS wrapper and filesystem phases | Phase 7 context gathered 2026-05-02 | Build/script/docs must create a stable handoff for Phases 8–10. [VERIFIED: CONTEXT.md] |
| Docs list Emscripten export config as missing | Phase 7 should fill in export-list/linker flags and blocker behavior | Current docs still list Emscripten export-list/linker configuration as not included | Extend `docs/typeduck-web-adapter.md` during Phase 7. [VERIFIED: docs/typeduck-web-adapter.md] |

**Deprecated/outdated:**

- `.planning/codebase/STACK.md` line stating `yune-rime-api` lacks `cdylib` is stale; current `crates/yune-rime-api/Cargo.toml` has `crate-type = ["rlib", "cdylib"]`. [VERIFIED: current codebase] [VERIFIED: .planning/codebase/STACK.md]
- Any plan that uses `wasm-bindgen` as the primary TypeDuck contract contradicts D-01. [VERIFIED: CONTEXT.md decisions]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `scripts/` is an acceptable location for the Phase 7 build/check script and export-list file. | Recommended Project Structure | Low: D-09 allows planner discretion, so a different location is acceptable if deterministic and documented. |
| A2 | Missing Emscripten checks commonly get hidden behind optional local setup. | Common Pitfalls | Low: mitigation is still required by D-08 even if the cause differs. |

## Open Questions

1. **Should the canonical export list be a plain text file, a shell array, or generated from Rust source?**
   - What we know: D-05 requires deterministic native and Emscripten export verification, and the 11-symbol list is stable in current code. [VERIFIED: CONTEXT.md decisions] [VERIFIED: current codebase]
   - What's unclear: The exact script/data-file shape is left to planner discretion by D-09. [VERIFIED: CONTEXT.md decisions]
   - Recommendation: Use a plain text export list consumed by both native and Emscripten checks; it is reviewable and avoids parsing Rust source in the build contract. [ASSUMED]

2. **Which artifact should be inspected after an Emscripten build?**
   - What we know: D-05 requires checking generated WASM/JS artifacts when they exist. [VERIFIED: CONTEXT.md decisions]
   - What's unclear: Local Emscripten tools are unavailable, so this session could not observe the exact artifact names Cargo/Emscripten will produce for this crate. [VERIFIED: local tool audit]
   - Recommendation: Script should search the expected target directory for generated `.wasm` and `.js` files and use the strongest available inspector (`wasm-nm`, `wasm-objdump`, then JS text scan as fallback). [ASSUMED]

3. **Should Phase 7 run full workspace tests?**
   - What we know: `cargo fmt --all -- --check` and `cargo test -p yune-rime-api --test typeduck_web` passed locally; project guidance uses focused tests and workspace tests when shared behavior changes. [VERIFIED: cargo test] [VERIFIED: PROJECT.md]
   - What's unclear: If Phase 7 only adds scripts/docs and no shared Rust behavior changes, a full workspace test may be optional. [ASSUMED]
   - Recommendation: Require focused native adapter test for every plan; run `cargo test --workspace` if Rust source or shared build manifest behavior changes beyond script/docs. [VERIFIED: PROJECT.md] [ASSUMED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust compiler | Cargo build/tests | yes | `rustc 1.95.0` at `/Users/trenton/.cargo/bin/rustc` | Add `/Users/trenton/.cargo/bin` to PATH if shell cannot find Rust. [VERIFIED: local tool audit] |
| Cargo | Workspace build/tests | yes | `cargo 1.95.0` at `/Users/trenton/.cargo/bin/cargo` | Add `/Users/trenton/.cargo/bin` to PATH if shell cannot find Cargo. [VERIFIED: local tool audit] |
| Rustup | Target detection/install instruction | yes | `rustup 1.29.0` at `/Users/trenton/.cargo/bin/rustup` | Document manual target install if Rustup unavailable. [VERIFIED: local tool audit] |
| `wasm32-unknown-emscripten` target | Real Emscripten build | no | — | Reproducible blocker + native `typeduck_web` tests. [VERIFIED: local tool audit] |
| `emcc` | Emscripten linker | no | — | Reproducible blocker + native `typeduck_web` tests. [VERIFIED: local tool audit] |
| `emar` | Emscripten archive tool / toolchain sanity | no | — | Reproducible blocker + native `typeduck_web` tests. [VERIFIED: local tool audit] |
| `nm` / `llvm-nm` | Native export verification | yes | `llvm-nm, compatible with GNU nm` | Use platform `nm` equivalent. [VERIFIED: local tool audit] |
| `wasm-nm` | WASM export verification | no | — | Use `wasm-objdump` if available; otherwise generated JS text scan as lower-confidence fallback. [VERIFIED: local tool audit] [ASSUMED] |
| `wasm-objdump` | WASM export verification | no | — | Use `wasm-nm` if available; otherwise generated JS text scan as lower-confidence fallback. [VERIFIED: local tool audit] [ASSUMED] |
| Node.js | Optional generated JS smoke/text checks | yes | `v24.14.1` | Python/text scan for artifact checks if Node is not needed. [VERIFIED: local tool audit] |
| Python 3 | Optional script helpers | yes | `Python 3.12.13` | POSIX shell-only script. [VERIFIED: local tool audit] |

**Missing dependencies with no fallback:**

- None for planning/native fallback; real browser artifact production is blocked until `wasm32-unknown-emscripten`, `emcc`, and `emar` are installed. [VERIFIED: local tool audit]

**Missing dependencies with fallback:**

- `wasm32-unknown-emscripten`, `emcc`, and `emar`: fallback is deterministic blocker output plus `cargo test -p yune-rime-api --test typeduck_web`. [VERIFIED: local tool audit] [VERIFIED: cargo test]
- `wasm-nm` / `wasm-objdump`: fallback is generated JS symbol text scan when artifacts exist, but this should be marked lower confidence than a WASM export-table inspector. [ASSUMED]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No authentication layer is present in this build/export phase. [VERIFIED: PROJECT.md] |
| V3 Session Management | yes | Treat RIME session/service lifecycle as process-global runtime state; serialize native tests and document one active service. [VERIFIED: typeduck_web.rs] [VERIFIED: CONTEXT.md decisions] |
| V4 Access Control | no | No user/tenant authorization model is present in this phase. [VERIFIED: PROJECT.md] |
| V5 Input Validation | yes | Keep null pointer checks, C string conversion checks, and runtime logical resource/path validation at the ABI boundary; do not bypass existing resource-ID controls. [VERIFIED: typeduck_web.rs] [VERIFIED: PROJECT.md] |
| V6 Cryptography | no | No cryptographic behavior is introduced by Phase 7. [VERIFIED: CONTEXT.md decisions] |
| V8 Data Protection | yes | Browser persistence must remain host-controlled through MEMFS/IDBFS and explicit sync; do not add hidden data exfiltration or remote dependency. [CITED: emscripten.org filesystem docs] [VERIFIED: PROJECT.md] |
| V12 File and Resources | yes | Browser host must create/preload shared data, user data, and `user_data_dir/build`; Rust code must continue treating runtime resource identifiers as logical IDs. [VERIFIED: docs/typeduck-web-adapter.md] [VERIFIED: PROJECT.md] |

### Known Threat Patterns for Rust C ABI + Emscripten Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Null or invalid C string pointers from JS/C callers | Tampering / Denial of Service | Preserve current null checks and `CString` conversion failures returning null/FALSE. [VERIFIED: typeduck_web.rs] |
| Use-after-free of adapter response pointers | Tampering / Denial of Service | Centralize response ownership docs: copy JSON immediately and always call `yune_typeduck_free_response`; Phase 8 owns typed wrapper enforcement. [VERIFIED: docs/typeduck-web-adapter.md] [VERIFIED: CONTEXT.md decisions] |
| Path traversal through runtime resource IDs | Tampering | Do not bypass existing logical resource-ID validation requirement from project constraints. [VERIFIED: PROJECT.md] |
| Missing browser assets causing failed init | Denial of Service | Document host preloading requirements and fail deterministically when `init` cannot select schema. [VERIFIED: typeduck_web.rs] [VERIFIED: docs/typeduck-web-adapter.md] |
| Simultaneous independent services corrupt process-global state | Tampering / Denial of Service | Document one active process-global service and serialize tests/host lifecycle. [VERIFIED: typeduck_web.rs] [VERIFIED: CONTEXT.md decisions] |
| Silent omission of exports from optimized artifact | Denial of Service | Verify `EXPORTED_FUNCTIONS` and inspect generated artifact before treating browser build as valid. [CITED: emscripten.org settings docs] [VERIFIED: CONTEXT.md decisions] |

## Sources

### Primary (HIGH confidence)

- `/rust-lang/rust` via Context7 CLI — `wasm32-unknown-emscripten` target requirements and install command. [CITED: https://github.com/rust-lang/rust/blob/main/src/doc/rustc/src/platform-support/wasm32-unknown-emscripten.md]
- `/websites/emscripten` via Context7 CLI — `EXPORTED_FUNCTIONS`, native underscore prefix, `EXPORTED_RUNTIME_METHODS`, `ccall`/`cwrap`, MEMFS/IDBFS, `FS.mount`, and `FS.syncfs`. [CITED: https://emscripten.org/docs/tools_reference/settings_reference.html] [CITED: https://emscripten.org/docs/api_reference/preamble.js.html] [CITED: https://emscripten.org/docs/api_reference/Filesystem-API.html] [CITED: https://emscripten.org/docs/porting/files/file_systems_overview.html]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/typeduck_web.rs` — seeded adapter symbols, lifecycle, JSON response, cleanup behavior. [VERIFIED: current codebase]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/tests/typeduck_web.rs` — native fallback contract tests and serialized process-global pattern. [VERIFIED: current codebase]
- `/Users/trenton/Projects/yune/docs/typeduck-web-adapter.md` — current symbol list, response ownership, JS flow, filesystem contract, missing Emscripten docs gap. [VERIFIED: current codebase]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/Cargo.toml` — `rlib`/`cdylib` crate type and dependencies. [VERIFIED: current codebase]
- Local commands: `cargo fmt --all -- --check`, `cargo test -p yune-rime-api --test typeduck_web`, `cargo build -p yune-rime-api`, `nm -gU target/debug/libyune_rime_api.dylib`. [VERIFIED: cargo/native nm]

### Secondary (MEDIUM confidence)

- `cargo info/search --registry crates-io` — registry versions for `serde_json`, `regex`, `serde_yaml`, `libc`, and `libloading`. [VERIFIED: crates.io via cargo]
- `/Users/trenton/Projects/yune/.planning/codebase/STACK.md`, `ARCHITECTURE.md`, `TESTING.md`, `PROJECT.md` — project patterns and constraints, with one stale `cdylib` note superseded by current manifest. [VERIFIED: project planning docs]

### Tertiary (LOW confidence)

- Script location and artifact-inspection fallback details are recommendations under planner discretion because D-09 permits several implementation shapes and local Emscripten artifacts could not be generated in this environment. [ASSUMED]

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — current Rust workspace, Cargo metadata, locked dependencies, native exports, and official Rust/Emscripten target docs were verified in-session. [VERIFIED: cargo metadata] [VERIFIED: current codebase] [CITED: official docs]
- Architecture: HIGH — phase decisions and current code agree that TypeDuck adapter behavior belongs in `yune-rime-api` and `lib.rs` remains facade glue. [VERIFIED: CONTEXT.md decisions] [VERIFIED: current codebase]
- Pitfalls: MEDIUM — export retention and filesystem pitfalls are backed by official docs; script/CI failure-mode pitfalls include some implementation judgment. [CITED: official docs] [ASSUMED]

**Research date:** 2026-05-04  
**Valid until:** 2026-06-03 for project-local code facts; 2026-05-11 for Emscripten command details if toolchain versions change quickly. [ASSUMED]
