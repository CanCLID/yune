# Yune / 新韵

Yune is a RIME-compatible input engine experiment for modern, contextual
typing.

新韵是一个面向现代上下文输入体验的输入法引擎实验项目。第一阶段不追求
完整重写 librime，而是先建立兼容边界、测试基准和可演进的 Rust 核心。

## Goals

- Keep RIME schema and frontend compatibility as explicit design constraints.
- Validate AI-assisted candidate ranking and contextual completion before
  replacing mature engine behavior.
- Build small Rust components that can be tested independently.
- Avoid depending on C++ plugin ABI compatibility for the first milestone.

## Workspace

- `crates/yune-core`: session state, composition, candidates, and engine traits.
- `crates/yune-schema`: RIME schema compatibility layer.
- `crates/yune-rime-api`: RIME-style C ABI shim and compatibility surface for
  frontend integration tests.
- `crates/yune-cli`: local test runner for input sequences and diagnostics.

## Current Compatibility Surface

Yune now has a deterministic compatibility harness, a focused RIME frontend
shim, and a frontend-style ABI test client:

1. Load a small RIME-style schema subset and table dictionary fixtures.
2. Feed deterministic key sequences through a Yune session.
3. Compare composition, candidate, commit, and status output against checked-in
   fixtures.
4. Exercise a RIME-style C ABI surface for sessions, context/status/commit,
   config, levers, schema lists, deployment helpers, and key processing.
5. Cover librime-compatible key handling for navigation, editing, selection,
   keypad keys, modifier fallbacks, `menu/alternative_select_keys`, and
   simulated key sequences.
6. Drive the ABI through a frontend-style compatibility client that uses the
   `RimeApi` function table instead of direct Rust internals.
7. Exercise deployment, maintenance, config opening, config mutation, levers,
   schema list, switcher, and plain user dictionary operations through
   librime-shaped APIs.
8. Compile deployed configs with librime-style `__include`, `__patch`, list
   append/merge, custom patch, build-info, and freshness handling.
9. Load schema-driven table dictionaries into sessions and validate real
   dictionary candidates through context paging and highlight APIs.
10. Parse more librime-compatible dictionary header and body forms, including
   inline and block `columns`, `import_tables`, YAML null and quoted scalar
   cases, optional document starts, BOM-prefixed headers, duplicate rows, literal
   hash-prefixed entries, and librime-style numeric weight prefixes.
11. Provide an AI ranking hook that can reorder candidates without blocking classic
   input behavior.

The current scope is still a compatibility subset. Yune does not yet aim to
replace librime's full plugin ecosystem, OpenCC conversion pipeline, spelling
algebra/prism compiler, compiled binary dictionary formats, or C++ plugin ABI.
