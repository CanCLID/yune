# Yune / 新韵

Yune is a Rust input-method engine with a librime-compatible C ABI surface.
It parses RIME schemas and dictionaries, runs a full input processor pipeline,
and exposes ~60 librime-shaped API functions so existing frontends can drive it
without code changes.

新韵是一个 Rust 输入法引擎，提供与 librime 兼容的 C ABI 接口。它解析 RIME
方案与词典，运行完整的输入处理器管线，并暴露约 60 个与 librime 同形的 API
函数，现有前端无需修改即可接入。

## Goals

- Maintain RIME schema and frontend compatibility as explicit design
  constraints, validated through fixture-driven testing.
- Build a clean, modular Rust core with typed translators, filters, and
  processors instead of cloning librime's C++ internals.
- Keep every compatibility difference measurable against librime before it is
  accepted.
- Provide an AI ranking hook (`CandidateRanker` trait) that can reorder
  candidates without blocking classic input behavior.

## Workspace

| Crate | Description |
|-------|-------------|
| `yune-core` | Engine, session state, composition, candidates, translators, filters, ranker trait, key parsing, spelling algebra, punctuation, dictionary parsing (YAML source + compiled `.table.bin` / `.prism.bin` / `.reverse.bin`), table encoding, user dictionary. |
| `yune-rime-api` | Librime-shaped C ABI shim: session registry, runtime paths, config compiler (`__include` / `__patch` / list merge / freshness), deployment, schema installation, 9-processor input pipeline, config/context/candidate/levers APIs, function table (`RimeApi` + `RimeLeversApi`), TypeDuck Web WASM adapter. |
| `yune-cli` | CLI test harness: run key sequences through core or ABI pipeline, compare output against checked-in JSON fixtures. |

## Current Compatibility Surface

1. **Engine core** — typed `Engine` with pluggable translators, filters, and an
   async-friendly `CandidateRanker` trait. Processes key events, manages
   composition, produces commits, and integrates with a user dictionary for
   learning.

2. **Translators** — echo, static table, punctuation, history, reverse lookup,
   switch, and schema-list translators. Table translator supports
   `columns`, `import_tables`, BOM-prefixed headers, numeric weight prefixes,
   and custom column orders.

3. **Filters** — charset filter, reverse lookup filter, simplifier,
   single-character filter, tagged filter, and uniqueness filter.

4. **Input processor pipeline** — 9 schema-loaded processors:
   speller, selector, navigator, key binder, editor, ascii composer, chord
   composer, punctuation, and recognizer. Each mirrors librime's processing
   stages and can be configured through schema YAML.

5. **Dictionary parsing** — reads RIME table dictionary YAML source files and
   the three compiled binary formats: `Rime::Table/4.0`, `Rime::Prism/4.0`, and
   `Rime::Reverse/4.0`. Supports checksum validation, spelling maps, correction
   rules, and reverse lookup entries.

6. **Config compiler** — librime-style `__include`, `__patch`, list
   append/merge, custom patches, build-info, and timestamp-based freshness
   handling.

7. **C ABI surface** — ~60 `extern "C"` functions matching the librime API:
   setup, initialization, finalization, session lifecycle, key processing,
   context/status/commit reads, config open/read/write/iterate, schema
   selection, deployment, maintenance, sync, candidate iteration/paging, user
   dictionary operations, and `RimeApi` / `RimeLeversApi` function tables.

8. **Schema lifecycle** — parse schema YAML → install processors, translators,
   filters, and segment tags → deploy to workspace → select into sessions.

9. **CLI fixture harness** — deterministic key-sequence replay through both the
   core engine and the full ABI pipeline, with JSON output compared against
   checked-in fixtures under `fixtures/`.

10. **Frontend-style tests** — a `frontend_client.rs` integration test drives
    the `RimeApi` function table like a real frontend would, and a
    `dynamic_loader.rs` test loads the compiled `cdylib` via `libloading`.

11. **TypeDuck Web adapter** — simplified C API (`yune_typeduck_*`) designed
    for WASM consumption, with JSON state exchange.

12. **User dictionary** — in-memory store with commit tracking, file-backed
    persistence, snapshots, sync, and recovery.

## Quick Start

```sh
# Build everything
cargo build

# Run all tests (unit + integration + frontend ABI)
cargo test --workspace

# Quality gate
cargo clippy --workspace --all-targets -- -D warnings

# Run a key sequence through the core engine
cargo run -p yune-cli -- run "nihao "

# Check core output against a fixture
cargo run -p yune-cli -- check fixtures/sample-nihao.json

# Run through the full ABI pipeline
cargo run -p yune-cli -- frontend \
  --shared-data-dir /path/to/rime-data \
  --user-data-dir /tmp/yune-user \
  --schema luna_pinyin \
  --sequence "nihao "
```

## Not Yet Implemented

- **AI/ML ranking** — the `CandidateRanker` trait and `MockAiRanker` exist, but
  no real model is integrated.
- **OpenCC conversion** — simplified/traditional Chinese conversion is not yet
  wired in.
- **Spelling algebra / prism compiler** — compiled prisms can be read but not
  generated from source.
- **Binary dictionary writing** — compiled formats are read-only.
- **C++ plugin ABI** — out of scope for the current milestone.

## License

BSD-3-Clause
