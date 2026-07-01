# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Read These First

This repo already maintains authoritative agent-facing docs. Treat them as the
source of truth and read them before non-trivial work:

- **[AGENTS.md](AGENTS.md)** — project orientation, current phase/milestone state, key constraints, workflow preferences, and the release quality gate.
- **[docs/conventions.md](docs/conventions.md)** — the single reference for architecture, stack, repo structure, coding/testing conventions, and C ABI rules. Sections are numbered; cite them when explaining decisions.
- **[docs/roadmap.md](docs/roadmap.md)** — active milestone sequence and scope boundaries. There is no `TODO`/`FIXME` in code; the roadmap + conventions are the live issue inventory.
- Subtree guides override the root for their area: [apps/yune-web/AGENTS.md](apps/yune-web/AGENTS.md) and [packages/yune-web-runtime/AGENTS.md](packages/yune-web-runtime/AGENTS.md).

## What This Is

Yune is a Rust input-method engine (Pinyin/Jyutping → Chinese characters) that
is a **drop-in, behavior-compatible replacement for librime/RIME**. It reads the
same schema/dictionary YAML and exposes the same C ABI, but is idiomatic safe
Rust internally. Correctness is defined by an **external oracle** (real librime),
never by Yune itself.

## Commands

Rust (workspace = `crates/yune-core`, `yune-rime-api`, `yune-cli`):

```bash
cargo build
cargo test --workspace                                          # all Rust tests
cargo test -p yune-core --test upstream_luna_pinyin_parity      # upstream 1.17.0 oracle parity
cargo test -p yune-core --test cantonese_parity                 # TypeDuck v1.1.2 oracle parity
cargo test -p yune-rime-api --test yune_web                     # yune-web WASM/adapter ABI contract
cargo test -p yune-core --test upstream_luna_pinyin_parity -- <substring>   # single test by name
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings           # the lint gate (-D warnings)
```

TypeScript runtime (`@yune-ime/yune-web-runtime`):

```bash
npm --prefix packages/yune-web-runtime test     # vitest run
npm --prefix packages/yune-web-runtime run build # tsc -> dist/
```

Browser harness (`apps/yune-web`):

```bash
npm --prefix apps/yune-web install
npm --prefix apps/yune-web run start      # builds worker, runs vite --host
npm --prefix apps/yune-web run build      # production build
npm --prefix apps/yune-web run typecheck  # tsc --noEmit
```

WASM build: `scripts/yune-web-wasm-build.sh` (needs `wasm32-unknown-emscripten`
+ `emcc`/`emar`; **degrades gracefully** to `cargo test -p yune-rime-api --test
yune_web` when the toolchain is absent).

CLI surrogate:

```bash
cargo run -p yune-cli -- run "nihao "                                    # direct-to-core
cargo run -p yune-cli -- frontend --shared-data-dir <rime-data> \
  --user-data-dir <tmp> --schema luna_pinyin --sequence "nihao "        # full ABI path
```

**Do not run the full quality gate by default.** Per AGENTS.md, run fmt/clippy/
tests only when the touched path needs it, when asked, or when you need evidence
for a claim. For docs-only or narrow edits, prefer targeted checks.

## Architecture (the big picture)

```
yune-cli  ┐
yune-web  ┼─► C ABI layer (crates/yune-rime-api, rlib + cdylib) ─► core engine (crates/yune-core)
DLL host  ┘     RimeApi tables, sessions, processors, config/deploy        translators, filters, rankers, dicts
```

- **`yune-rime-api` is the only compatibility surface.** All three consumers
  (CLI surrogate `rime_frontend.rs`, the `yune_web_*` WASM adapter, native
  `rime.dll`) drive the same exported function table. None reach into
  `yune-core` directly. It builds as `rlib` (links into tests/CLI) + `cdylib`
  (the loadable artifact / WASM).
- **Key path:** frontend calls `RimeApi.process_key` → ABI-level processors in
  `yune-rime-api/src/processors/` (ascii composer, speller, selector, editor…)
  → falls through to `Engine::process_key_event` in `yune-core` (translators →
  sort → filters → optional ranker).
- **Known debt:** the schema-driven processor pipeline lives in `yune-rime-api`,
  not `yune-core`. Acceptable for librime-shaped frontends; extract toward core
  *only* when a real non-ABI host needs it, as a behavior-preserving move.
- **AI is a separate, default-off, local-only layer** (`yune-core/src/ai/`),
  outside the deterministic classic path. In the web harness it is a second pass
  (`yune_web_stage_ai`) requested after classic rendering — never inside
  `process_key`.

Where to add behavior: see [docs/conventions.md §3](docs/conventions.md)
("Where to add new behavior").

## Non-Obvious Rules That Cause Real Mistakes

- **`RimeApi` field order IS the ABI.** The `#[repr(C)]` table in
  `yune-rime-api/src/abi.rs` is accessed by struct-pointer offset. Never
  reorder/insert mid-struct without matching upstream-header evidence; slot
  positions are locked by `assert_api_slot!` tests in `src/tests/abi.rs`. A
  misplaced field silently breaks every native frontend.
- **Default `rime_get_api()` stays upstream-shaped (librime 1.17.0).** TypeDuck
  fork-only slots (`config_list_append_*`) are exposed *only* via
  `rime_get_typeduck_profile_api()`. Do not widen the default table for
  TypeDuck-only behavior.
- **Two export families — never mix:** librime-shaped FFI is `RimePascalCase`
  (`#[no_mangle] extern "C"`); the Yune browser ABI is `snake_case` `yune_web_*`
  (14 functions in `web_runtime.rs`). Adding/renaming a `yune_web_*` export
  **requires updating `scripts/yune-web-exports.txt`** or the WASM build silently
  drops it.
- **Tests are oracle-driven and non-circular.** Capture expected bytes from the
  external oracle (upstream librime / TypeDuck v1.1.2) into a checked-in fixture,
  run Yune's real production path, assert it matches. **Never derive expected
  values from Yune.** Unsupported behavior is a named `#[ignore = "blocked:
  ..."]` test with a `panic!()` body — no silent gaps.
- **`unsafe_code = "forbid"`** workspace-wide. `yune-core` inherits this (no
  `unsafe` allowed). The ABI/FFI crates (`yune-rime-api`, `yune-cli`) use
  explicit local lint tables with `unsafe_code = "allow"`; keep all FFI pointer
  work / raw conversions / `RimeFree*` pairing in the ABI modules, never in
  `yune-core`.
- **EOL policy:** `.gitattributes` normalizes to LF (with `*.bat`/`*.cmd` CRLF,
  binaries untouched). Developed on Windows but ships `.sh` + byte-exact
  fixtures — do not commit CRLF into normalized files.
- **librime is never linked or called at runtime** — it is a validation oracle
  only, referenced by upstream repo/commit, not a local checkout path.
- **`apps/yune-web/source/` is ignored reference only**; the Yune-owned state is
  the seam under `apps/yune-web/src/yune-integration/`. Browser-visible claims
  require real-browser (Playwright) evidence — see
  [apps/yune-web/e2e/yune-browser-smoke.md](apps/yune-web/e2e/yune-browser-smoke.md).
- **A "done / gates passed / verified" report is a claim, not a fact — verify
  it.** Independently re-run the load-bearing check before accepting or building
  on another session's, agent's, or handoff's work. Repeat offenders here: a
  closeout that claimed gates passed but never ran the broad `cargo clippy
  --workspace --all-targets -- -D warnings` (lint errors don't fail `cargo
  test`/`build`, so they slip a partial check); an "oracle" test that compared
  two checked-in files or round-tripped through Yune's own builder instead of
  executing the real path over externally-captured oracle bytes (a byte-parity
  bug shipped green behind it); and README claims drifting ahead of the measured
  evidence. When you close a milestone, run the *exact* named gate commands and
  list them; when you review, re-run the gate/behavior the finding hinges on.
  Full detail: AGENTS.md "Verification Discipline".
