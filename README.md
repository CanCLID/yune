# Yune

Yune is a Rust input-method engine with a librime-compatible C ABI surface.
It uses librime as a behavioral oracle, not as an implementation template:
existing RIME schemas and frontends should behave predictably through Yune, but
the internals stay idiomatic Rust and the long-term product direction is an
AI-native input engine that librime cannot provide.

## Current Status

Phase 1, the engine build-out and basic oracle-parity effort, is complete for
the named target set:

- upstream `rime/librime 1.17.0` for default core behavior and common-schema
  compatibility;
- TypeDuck-HK/librime `v1.1.2` for the TypeDuck `jyut6ping3` compatibility
  profile;
- `yune-web` browser integration, including the default-off local AI second
  pass;
- TypeDuck-Windows backend compatibility smoke through the named TypeDuck
  profile ABI.

This does not mean Yune is a bit-for-bit clone of librime. It means the current
named targets are fixture-backed and documented. Future work is Phase 2:
frontend/product development, ongoing dogfooding, AI productization, and
additional compatibility targets when a real frontend or schema needs them.

Read [docs/roadmap.md](docs/roadmap.md) for the live milestone state. The first
historical TypeDuck-Web-derived dogfooding batch, M24, is complete and
archived; future `yune-web` dogfood reports should start a new scoped plan
rather than reopening Phase 1.
The Phase 2 Windows plan is the next platform/product planning artifact.

## Compatibility Model

- The default oracle is upstream `rime/librime 1.17.0`
  (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`).
- TypeDuck fork behavior is profile-only, pinned to TypeDuck-HK/librime
  `v1.1.2` (`74cb52b78fb2411137a7643f6c8bc6517acfde69`).
- The default `rime_get_api()` table remains upstream-shaped.
- Fork-only TypeDuck ABI slots live behind the opt-in
  `rime_get_typeduck_profile_api()` accessor.
- Oracle fixtures are non-circular: expected bytes come from the upstream or
  TypeDuck oracle, not from Yune output.

## Workspace

| Path | Purpose |
|---|---|
| `crates/yune-core` | Deterministic Rust engine: schema-driven processors, translators, filters, candidates, UserDB, OpenCC subset, dictionary parsing/writing, spelling algebra, AI staging, and ranking hooks. |
| `crates/yune-rime-api` | Librime-shaped C ABI: session lifecycle, config/deploy/schema APIs, function tables, TypeDuck profile ABI, native frontend tests, and the `yune-web` WASM-facing API. |
| `crates/yune-cli` | CLI surrogate for driving core or ABI paths and comparing checked-in fixtures. |
| `packages/yune-web-runtime` | TypeScript wrapper around the `yune-web` WASM API. |
| `apps/yune-web` | Canonical `yune-web` browser harness: upstream-derived shell, Yune patch, adapter, browser tests, public-demo package, and evidence. |
| `docs` | Roadmap, decisions, requirements, conventions, fork-parity ledger, and execution plans. |

## What Works

- RIME schema/config handling: `__include`, `__patch`, list merge/append,
  custom patches, deploy freshness, schema installation, and schema switching.
- Processor pipeline: speller, selector, navigator, key binder, editor, ASCII
  composer, chord composer, punctuation, and recognizer.
- Translators and filters: table/script paths, history, reverse lookup,
  punctuation, schema list, simplifier/OpenCC subset, charset, uniqueness,
  dictionary lookup, and TypeDuck-specific profile behavior.
- Dictionary support: source `.dict.yaml`, imports, table encoder pieces,
  compiled table/prism/reverse formats, public binary writers, rebuild
  execution, and fixture-backed ranking behavior for named targets.
- User data: local user dictionary storage, learning, snapshots, sync, recovery,
  and profile-safe separation from AI memory.
- C ABI compatibility: upstream-shaped default `RimeApi`, `RimeLeversApi`,
  config/context/candidate/session/deploy APIs, dynamic-loader tests, and
  frontend-shaped lifecycle tests.
- yune-web: Vite/React/Tailwind upstream-derived app, Yune runtime adapter, browser
  evidence, multi-schema playground behavior, and default-off local AI rows.
- TypeDuck-Windows: packaged Yune DLL/header smoke, build/link evidence, and
  stock TypeDuck server/client IPC smoke through the TypeDuck profile ABI.

## Performance

Yune treats librime as a *behavioral* oracle, not a performance oracle. The
current upstream-`luna_pinyin` comparison is fair after M33: Yune now lazy-loads
the `stroke` reverse dictionary and shares built dictionary translators across
session selects. M33 produced a real startup/session improvement, but the
numbers must be read as cold versus warm: a fresh Yune process still pays a
`~909 ms` first schema build and peaks around `183 MB`, while warm re-selects
are around `48 ms`. Per-key lookup still trails librime by a wide margin, so no
typing-speed, memory-footprint, or browser-speed win is claimed.

Current analysis and evidence:

- [docs/reports/yune-vs-librime-performance.md](docs/reports/yune-vs-librime-performance.md)
  records the current measurement and caveats.
- [docs/reports/yune-vs-librime-root-cause-analysis.md](docs/reports/yune-vs-librime-root-cause-analysis.md)
  explains the remaining root cause after M33.
- [docs/plans/completed/m33-plan-engine-native-lookup-performance.md](docs/plans/completed/m33-plan-engine-native-lookup-performance.md)
  records the completed fairness/cache pass.

## AI-Native Layer

The AI foundation is already present, but intentionally conservative:

- M11 added the core/CLI AI layer: provider trait, local/mock providers,
  staged input-keyed results, privacy classification, separate AI memory, and
  no classic-path auto-commit.
- M13 exposes that layer in the web harness now canonical as `yune-web`: default-off, local-only,
  second-pass `stage_ai` flow. Classic candidates render first; AI rows are
  source-labeled and never become the default commit.
- Remote providers, richer contextual translation, native frontend AI exposure,
  and product UX are Phase 2 work, not Phase 1 parity requirements.

## Quick Start

```powershell
# Build Rust workspace
cargo build

# Run all Rust tests
cargo test --workspace

# Rust quality gate
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings

# TypeScript runtime tests/build
npm --prefix packages/yune-web-runtime test
npm --prefix packages/yune-web-runtime run build

# Run a key sequence through the core engine
cargo run -p yune-cli -- run "nihao "

# Run through the ABI-shaped frontend surrogate
cargo run -p yune-cli -- frontend `
  --shared-data-dir C:\path\to\rime-data `
  --user-data-dir C:\temp\yune-user `
  --schema luna_pinyin `
  --sequence "nihao "
```

For `yune-web` browser work, read
[apps/yune-web/e2e/yune-browser-smoke.md](apps/yune-web/e2e/yune-browser-smoke.md)
and the current plan or archived M24 baseline under `docs/plans/`.

## Key Documentation

- [docs/conventions.md](docs/conventions.md) - architecture, coding rules,
  testing conventions, ABI rules, integrations, and current risks.
- [docs/roadmap.md](docs/roadmap.md) - phase status, completed milestones,
  active work, and Phase 2 direction.
- [docs/decisions.md](docs/decisions.md) - decision log and standing
  principles.
- [docs/requirements.md](docs/requirements.md) - requirement IDs and status.
- [docs/ledgers/fork-parity-ledger.md](docs/ledgers/fork-parity-ledger.md) - Cantoboard and
  TypeDuck fork deltas versus upstream.
- [docs/plans/](docs/plans/) - active plans and archived execution records.

## Non-Goals And Deferred Work

- Bit-for-bit librime internals or full C++ plugin ABI compatibility.
- Widening the default upstream `RimeApi` for TypeDuck-only behavior.
- Cloud inference as a hard dependency.
- Remote AI providers without explicit privacy/product gates.
- Native frontend AI exposure before a named platform track proves the UX and
  safety model.
- Treating the TypeDuck-Web product repo, TypeDuck-Windows, iOS, or other frontend repos as
  engine semantics. They are product/platform tracks that consume Yune.

## License

MIT for Yune's original code. Checked-in third-party schemas, dictionaries,
fixtures, generated data, and provenance materials keep their upstream terms;
see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
