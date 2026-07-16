# Yune Conventions & Reference

This is the repository's canonical architecture, stack, ownership, testing,
integration, and documentation reference. It supersedes the removed
`.planning/codebase/` maps. Current sequencing lives in
[`roadmap.md`](./roadmap.md); completed outcomes live in
[`ledgers/milestone-history.md`](./ledgers/milestone-history.md).

Paths and symbol names below are authoritative. Line numbers in historical
plans or evidence are source-bound hints; locate current symbols by name.

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

Yune is a Rust input-method engine with a deterministic core and a single
librime-shaped compatibility layer.

```text
yune-cli                 yune-web / WASM             native package consumers
    |                           |                              |
    +---------------------------+------------------------------+
                                v
                  crates/yune-rime-api
          C ABI, sessions, config, deployment, processors,
                 schema install, yune_web_* adapter
                                |
                                v
                     crates/yune-core
          engine state, translators, filters, dictionaries,
                    ranking, userdb, AI sidecar
```

- `yune-core` owns deterministic engine behavior and typed internal models.
- `yune-rime-api` owns the upstream-shaped C ABI, process-global RIME service,
  schema/config/deployment integration, and the current processor pipeline.
- `yune-cli` is an in-tree frontend surrogate that exercises the real ABI.
- `packages/yune-web-runtime` is the reusable TypeScript/WASM bridge.
- `apps/yune-web` is the tracked browser harness and public demo, not the
  TypeDuck shipping product.
- Native TypeDuck/Yune Windows package compatibility remains testable here;
  Windows TSF, UI, installer, diagnostics, and product delivery belong to
  [`CanCLID/yune-windows`](https://github.com/CanCLID/yune-windows).

**Oracle authority.** Default core behavior follows upstream
`rime/librime 1.17.0` at
`33e78140250125871856cdc5b42ddc6a5fcd3cd4`. Canonical Jyutping candidate
ordering, segmentation, fallback, and completion use that engine with pinned
`rime/rime-cantonese`. TypeDuck-HK/librime `v1.1.2` at
`74cb52b78fb2411137a7643f6c8bc6517acfde69` is profile-only for multilingual
comments, lookup payloads, display/profile behavior, fork-only ABI controls,
and grandfathered fixture-backed candidate guards. M58's blast-radius audit is
complete; the preferred future `jyut6ping3_typeduck` id still requires explicit
user sign-off. These are validation sources, never runtime dependencies or
assumed local checkout paths.

**AI-native behavior is a separate layer.** The default-off AI provider,
context, privacy, staged-result, and memory types live under
`crates/yune-core/src/ai/`. Classic input remains provider-free and usable
without network access. Browser AI uses an explicit second pass; native product
AI exposure requires a separate product and engine boundary decision.

**Current boundary caveat.** The strategic shape is core engine plus thin ABI,
but today's complete RIME key path is not a thin adapter: schema-driven
processors live in `crates/yune-rime-api/src/processors/` before falling through
to `yune_core::Engine`. Extract processor semantics into a core-owned Rust API
only when a real non-ABI consumer needs the full path. Such extraction must be
behavior-preserving and retain the oracle, ABI, and browser gates.

---

## 2. Stack & Build

The repository spans Cargo/Rust, npm/TypeScript, and Emscripten/WASM.

- **Rust:** edition 2021, MSRV 1.76, workspace resolver 2, MIT. The workspace
  contains `yune-core`, `yune-rime-api`, and `yune-cli`.
- **Native ABI artifact:** `yune-rime-api` builds as both `rlib` and `cdylib`;
  release builds use LTO, one codegen unit, `panic = "abort"`, and stripping.
- **TypeScript runtime:** `@yune-ime/yune-web-runtime` is a private ESM package
  built with `tsc` and tested with Vitest.
- **Browser build:** [`scripts/yune-web-wasm-build.sh`](../scripts/yune-web-wasm-build.sh)
  builds the release `wasm32-unknown-emscripten` module, verifies the native and
  browser export contract, and smokes JS/WASM plus filesystem access. Missing
  Emscripten tooling may fall back to the native `yune_web` ABI test unless the
  caller explicitly requires a browser artifact.
- **Public deployment:** `.github/workflows/deploy-yune-web.yml` defines the
  deployment-maintenance release boundary. It classifies every `main` push,
  builds and runs the unchanged
  WEB03-11 gate without deployment credentials, seals one source/hash-identified
  artifact, proves it on a Cloudflare preview, and promotes those exact bytes by
  Wrangler direct upload. Cloudflare Git production/preview auto-builds are
  disabled before activation, and each credentialed upload checks that API
  interlock, so shared Pages build-host scheduling cannot turn benchmark timer
  noise into a failed deployment.
  `apps/yune-web/public-demo/cloudflare-pages-build.sh` remains the local
  compatibility reproduction entrypoint.
- **Native packaging:** `scripts/package-typeduck-windows.ps1` and
  `scripts/package-yune-windows.ps1` produce upstream-shaped default headers
  plus named profile headers. Profile-only slots never widen default
  `rime_get_api()`.

There is no `build.rs`, root `rust-toolchain.toml`, or root
`.cargo/config.toml`; ordinary development uses the active toolchain. Release
and deployment scripts may pin their own toolchains and must record them.

**Web surface terminology** — keep these distinct:

- `packages/yune-web-runtime/`: reusable Yune-owned TypeScript/WASM bridge.
- `apps/yune-web/`: canonical tracked Vite harness, browser evidence surface,
  and public demo.
- `apps/yune-web/source/`: ignored historical TypeDuck-Web reference checkout.
- `apps/yune-web/patches/`: retained migration/reference patches, not the live
  application source of truth.
- A separate TypeDuck-Web checkout: external product integration, requiring a
  separately scoped product track.

---

## 3. Repository Structure

| Path | Owner |
| --- | --- |
| `crates/yune-core/src/` | Deterministic engine, candidates/state, translators, filters, dictionaries, spelling, punctuation, userdb model, AI sidecar |
| `crates/yune-core/tests/` | External-oracle parity and provenance fixtures |
| `crates/yune-rime-api/src/` | C ABI, sessions, FFI memory, config/deployment, schema integration, processors, web adapter |
| `crates/yune-rime-api/tests/` | ABI, dynamic-loader, frontend-host, abuse, product-path, and web integration tests |
| `crates/yune-cli/src/` | CLI surrogate and fixture/transcript rendering |
| `packages/yune-web-runtime/` | TypeScript Emscripten runtime wrapper |
| `apps/yune-web/` | Tracked browser harness, public demo, Playwright gates, shipped schema assets |
| `scripts/` | Build, benchmark, packaging, curation, evidence, and policy tools |
| `fixtures/` | CLI and frontend-trace fixtures |
| `docs/` | Contracts, roadmap, requirements, decisions, plans, ledgers, reports, provenance |

**Where to add behavior:**

- Core engine/state → the owning `yune-core` module plus its owning tests.
- Translator/filter/dictionary behavior → the corresponding `yune-core`
  submodule; schema installation remains in `yune-rime-api`.
- RIME ABI function → shape in `abi.rs`/`api_table.rs`, implementation in the
  owning ABI module, and public-surface tests.
- Schema processor → `yune-rime-api/src/processors/<name>.rs`, with per-session
  state and installer wiring in their owning modules.
- Browser ABI → `yune-rime-api/src/web_runtime.rs`,
  `scripts/yune-web-exports.txt`, TypeScript wrapper, and matching tests.
- CLI behavior → `args.rs` plus the owning frontend/sample/render module.

Avoid generic utility modules until at least two ownership areas genuinely
share the helper.

---

## 4. Coding Conventions

- Rust modules, functions, fields, and locals use `snake_case`; types and traits
  use `UpperCamelCase`; C mirrors use `Rime*` and `#[repr(C)]`.
- TypeScript uses `UpperCamelCase` for types/classes and preserves
  `snake_case` JSON fields that mirror the Rust response.
- Librime-shaped exported functions use `RimePascalCase`.
- Yune browser exports use `yune_web_*`; the 14-name allowlist in
  `scripts/yune-web-exports.txt` is binding. Add or rename an export only with
  the allowlist, Rust ABI tests, and TypeScript contract updated together.
- Parsers return owned error types. CLI boundaries return errors; C ABI
  boundaries return librime-shaped false/null values and validate pointers and
  strings before use.
- Every unsafe block has a local `// SAFETY:` justification; unsafe FFI
  signatures carry Rustdoc `# Safety` sections. Keep raw pointer ownership and
  `RimeFree*` pairing inside ABI modules, never `yune-core`.
- When mirroring librime behavior, name the specific upstream construct or
  observable contract. Do not comment obvious control flow.
- `.gitattributes` and `.editorconfig` own encoding/EOL policy. Do not commit
  CRLF into normalized source, scripts, or byte-exact fixtures.
- Canonical Rust checks are `cargo fmt --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`. Unsafe-free crates
  inherit the workspace `forbid(unsafe_code)` policy; ABI-facing crates carry
  explicit local lint tables for required FFI.

---

## 5. C ABI Rules

The detailed support boundary lives in
[`contracts/engine-support-contract.md`](./contracts/engine-support-contract.md).

- **`RimeApi` field order is the ABI.** Match upstream `rime_api.h` and its
  size/version rules. Never insert or reorder a default slot without header and
  oracle evidence plus slot-lock updates.
- **Fork-only slots stay profile-only.** TypeDuck list-append helpers are
  exposed through named profile accessors, not default `rime_get_api()`.
- **Default structs remain upstream-shaped.** Do not widen `RimeCandidate` or
  other default ABI types for browser, AI, or product convenience.
- **Export families stay synchronized.** `yune_web_*` additions require Rust,
  allowlist, linker, TypeScript, and abuse/ABI coverage in the same slice.
- **FFI failures are contained.** All discovered exports belong behind the
  established panic/error boundary. Release `panic = "abort"` remains an
  explicit product policy; owning tests must prove defined bad inputs do not
  reach a panic.

---

## 6. Module & Test Ownership

Each behavior slice owns a production module and an owning test module.
`lib.rs` and `main.rs` remain facades and orchestration glue. Mechanical moves
do not change behavior or weaken assertions.

- Unit tests: `<crate>/src/tests/<slice>.rs`.
- Integration/parity tests and external fixtures: `<crate>/tests/`.
- Shared test helpers: the owning test module's explicit helper surface.
- CLI fixtures: top-level `fixtures/`.
- Public re-exports: crate/module facades, not ad hoc cross-module imports.

---

## 7. Testing Conventions

```bash
cargo test --workspace
cargo test -p yune-rime-api --test yune_web
cargo test -p yune-rime-api --test abi_abuse
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix packages/yune-web-runtime test
npm --prefix packages/yune-web-runtime run build
```

Run only the load-bearing subset for a narrow change; milestone/release work
uses the exact owning gate. Do not describe a partial subset as the full gate.

**Oracle-driven and non-circular.** Capture expected bytes/behavior from the
external oracle, run Yune's production path over those external bytes, and
compare. A Yune encoder/decoder round trip or fixture-to-manifest comparison is
not behavior parity. Fixture metadata records engine/schema pins, capture
commands, source-row policy, and contains no local absolute paths.

**Public surfaces over internals.** ABI/frontend tests obtain and call the
exported function table or `yune_web_*` surface. Dynamic-loader tests load the
actual cdylib. Browser-visible claims require real-browser, real-asset evidence;
native fallback tests do not prove browser behavior.

**Blocked and evidence-only tests are explicit.** A behavior blocker uses
`#[ignore = "blocked: ..."]` with a `panic!()` body. Evidence-only probes use
an `evidence-only:`/`evidence capture:` reason and explicit output location.
Never silently pass a missing oracle or degraded asset path.

**Test techniques.** Standard assertions and focused hand-written fakes remain
the norm. `proptest` is used where property generation materially strengthens
ABI-abuse validation; do not add a second framework casually.

**Cross-platform state.** Assert portable shapes, not platform-specific values.
The session registry uses a recovering mutex after M56; other process-global
registries may still panic on poison. Tests touching global runtime state must
use the shared serialization guard. Release panic policy is separate from
test/unwind behavior.

---

## 8. Integrations

- **librime/rime-cantonese/TypeDuck oracles:** validation and fixture capture
  only; never linked or called at runtime. Fixture paths name their provenance.
- **yune-web/Emscripten/IDBFS:** the TypeScript runtime owns WASM lifecycle,
  response-pointer pairing, browser filesystem preparation, and explicit
  persistence sync. The app adapter maps responses into the harness UI.
- **Windows packages:** this repo owns engine/package/profile-API boundaries and
  package smokes. The dedicated Windows repository owns product behavior.
- **OpenCC:** `SimplifierFilter` implements a named in-process subset using
  checked-in data. It is not the full OpenCC library; unsupported conversions
  require a named target and oracle evidence.
- **Local storage:** user dictionaries and AI memory are local, separate
  namespaces with explicit lifecycle and privacy rules.
- **Automation:** `.github/workflows/evidence-growth.yml` enforces the tracked
  evidence-growth policy. The engine has no required hosted service, auth,
  database, or HTTP-webhook dependency.

---

## 9. Key Risks / Concerns (current)

- **Oracle/profile drift:** canonical Jyutping, upstream core, and TypeDuck
  profile claims must never be inferred from a shared schema id. Name engine,
  schema source, commit, and lane.
- **Profile leakage:** TypeDuck-tuned behavior in shared core code requires
  explicit profile/config wiring or separate upstream evidence that it is
  global.
- **Process-global service:** one active RIME service per WASM instance remains
  the supported model. Runtime paths, registries, notifications, and caches are
  process-wide; hosts serialize setup/deployment/workspace mutation with use.
- **Shared-cache lifecycle:** immutable translators and byte-backed indexes may
  survive finalize for same-root reuse. Cleanup-all, root changes, and workspace
  mutation clear sessions plus shared caches before filesystem mutation.
- **Core/ABI ownership debt:** the full processor pipeline remains ABI-owned.
  Extraction is trigger-gated by a real non-ABI consumer.
- **Compiled-asset delivery:** source/compiled staleness, manifests, public
  asset splitting, worker delivery, and persistence can invalidate otherwise
  correct engine behavior. Keep fail-closed product and deployment gates.
- **Performance evidence:** native, browser, product, and platform counters are
  separate lanes. Current results and bottlenecks live only in the
  [`performance dashboard`](./reports/yune-vs-librime-performance.md).
- **Approximation boundaries:** local userdb storage and the focused OpenCC
  implementation preserve named observable behavior; they are not claims of
  LevelDB or full OpenCC implementation parity.

---

## 10. Planning Docs

Canonical ownership:

- `roadmap.md`: current sequence, scope, and readiness.
- `requirements.md`: requirement definitions, status, and traceability.
- `decisions.md`: durable decisions and rationale.
- `ledgers/milestone-history.md`: completed milestone outcomes.
- `plans/active`, `plans/reference`, `plans/completed`: execution authority,
  reference designs, and historical completed records amended only by an
  explicit dated correction or addendum.

**Evidence retention.** Raw native benchmark output belongs under the
user-level external evidence root selected by benchmark scripts, never in a Git
worktree. Import only the curator allowlist with
`scripts/curate-compact-evidence.py`. Run
`python3 scripts/check-evidence-growth.py --repo-root .` for evidence changes.
The policy rejects raw metric classes, unexcepted files over 5 MiB, and curated
packets over 10 MiB. Removed historical leaves remain recoverable through
`docs/ledgers/evidence-pruning/current-ledger.csv`; do not rewrite signed
history.

**Markdown.** `docs/.prettierrc.json` uses `proseWrap: "never"`. Prefer one
source line per paragraph/list item where practical; tables, code, paths, and
commands may stay long. Validate with `markdownlint-cli2` when a docs gate
requires it.

**Plan records.** Every plan under `plans/active`, `plans/reference`, or
`plans/completed` opens immediately below its title with a status banner whose
separate `Status` and `Milestone` fields name its state and owning
milestone/stage. Use
`Updated` for active/reopened/parked records and `Closed` for
complete/finished/superseded records. Plan filenames use the established
`m<nn>-<type>-<topic>.md` or Phase 2
`p<phase>-<track><number>-<type>-<topic>.md` form. Finished or superseded plans
move to `plans/completed/`, never disappear. Keep plan state consistent with
the roadmap and milestone ledger.

Behavior-sensitive commits with generic subjects include a short body or
evidence pointer so the invariant is recoverable without reconstructing the
entire diff.

---

_Last reviewed: 2026-07-15. M59 is complete; WEB03-11 is maintenance; M60 is
the sole next milestone. Windows product execution is external, while this repo
retains engine/package/profile-API ownership._
