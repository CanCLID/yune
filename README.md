# Yune

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rust-1.76%2B-orange.svg)](https://www.rust-lang.org)

**Languages:** English | [简体中文](README.zh-CN.md) | [粵語](README.yue.md)

> The engine that turns your typing into Chinese characters.
> Type `nihao`, get 你好. Type `nei5 hou2`, get 你好 in Cantonese.
> Built in Rust - native ABI, browser WASM, and CLI paths.

## Contents

- [What Yune Does](#what-yune-does)
- [Why It Exists](#why-it-exists)
- [How It Works](#how-it-works)
- [Current Status](#current-status)
- [Compatibility](#compatibility)
- [Performance](#performance)
- [Quick Start](#quick-start)
- [Quality Checks](#quality-checks)
- [Repository Layout](#repository-layout)
- [Documentation](#documentation)
- [Non-Goals](#non-goals)
- [Contributing](#contributing)
- [License](#license)

## What Yune Does

You type romanized Chinese (Pinyin for Mandarin, Jyutping for Cantonese) on a
standard keyboard. Yune converts it to the right Chinese characters in real time.

Under the hood, Yune reads RIME-style dictionary and configuration files for its
named targets and supported common-schema behavior. The goal is predictable
compatibility with existing RIME schemas when a target has oracle evidence, not
a universal all-schema claim.

**[yune-web.pages.dev](https://yune-web.pages.dev)** — try it in your browser.

### Capabilities

- RIME schema and config handling: `__include`, `__patch`, custom patches, deploy
  freshness, schema installation, and schema switching.
- Full input pipeline: speller, selector, navigator, key binder, editor, ASCII
  composer, chord composer, punctuation, recognizer, translators, and filters.
- Dictionary support: source `.dict.yaml`, imports, Yune-native compiled
  table/prism/reverse artifacts, rebuild execution, and fixture-backed ranking
  verified against the reference engine.
- C ABI compatibility: upstream-shaped default `RimeApi` and `RimeLeversApi`,
  config/context/candidate/session/deploy APIs, dynamic-loader tests, and
  frontend lifecycle tests.
- TypeDuck profile behavior: fork-only ABI slots exposed through
  `rime_get_typeduck_profile_api()`, rich Cantonese dictionary comments,
  TypeDuck-Web browser evidence, and TypeDuck-Windows backend/profile/IPC smoke
  evidence.
- Browser runtime: `@yune-ime/yune-web-runtime`, the `yune-web` Vite app,
  multi-schema browser harness (jyut6ping3, cangjie5, luna_pinyin, and more),
  UI language switching, output standard selection, public demo, and Playwright
  evidence.
- AI foundation: provider trait, local/mock providers, staged AI rows, privacy
  policy, separate AI memory, and default-off browser exposure.

## Why It Exists

RIME has been the backbone of open-source Chinese input for over a decade. It
works well. But it's a large C++ codebase that's difficult to change, test, or
embed in modern environments like browsers and mobile apps.

Yune rebuilds the engine from scratch in Rust with three goals:

**Run everywhere.** The same core engine can be exposed as a native
librime-shaped shared library for desktop host experiments, as WebAssembly for
browser-based input, or as a CLI tool for testing and benchmarking.

**Be testable.** Covered behavior is verified byte-for-byte against the relevant
RIME-family oracle. Instead of porting C++ code (and inheriting its bugs and
assumptions), Yune runs the oracle as a behavior reference: capture what it
outputs for a given input, then assert Yune produces the exact same result. This
preserves compatibility without cargo-culting a 15-year-old C++ architecture.

**Prepare for AI-native input.** The engine has a built-in, default-off AI layer.
In the future, an on-device language model could suggest completions or
corrections alongside traditional dictionary candidates — without slowing down
the classic path and without sending your typing to a cloud service.

## How It Works

```
keystrokes  ──►  spelling algebra  ──►  dictionary lookup  ──►  ranking & filtering  ──►  commit text
                    (normalize)          (find candidates)        (sort, deduplicate)        (output)
```

The pipeline is built from swappable Rust traits — translators, filters, and
rankers — rather than a monolithic class hierarchy. Want to plug in a custom
ranking model? Implement a trait. Want a different dictionary format? Swap the
translator.

The deterministic core is safe Rust. The workspace forbids unsafe code by
default, with explicit ABI/FFI exceptions in `yune-rime-api` and `yune-cli`.

## Current Status

Yune is an active engine project.

- **Compatibility baseline:** Phase 1 is complete for its captured named
  target suites. Mandarin `luna_pinyin` is measured against upstream
  `rime/librime 1.17.0`; TypeDuck Cantonese profile behavior is measured
  against TypeDuck-HK/librime `v1.1.2` through the TypeDuck profile. Per the
  2026-07-05 D-31 amendment, canonical `jyut6ping3` candidate ordering,
  segmentation, fallback, and completion now use upstream `rime/librime
  1.17.0` plus pinned `rime/rime-cantonese`; M58 completed the fresh canonical
  capture at `f780410c`, including the user-specified `zijiguk` / `諮議局`
  case, which returns `諮議局` first and did not produce a canonical fix. The
  M55 expanded oracle sentence fixtures also surfaced known, recorded gaps.
  M59's current named acceptance closes canonical Lane A strict `13/13` over
  5,705 candidate positions, marked Cangjie strict `12/12` with `3 passed / 0
  ignored`, and the seven-input upstream Luna Lane B page- and position-exact,
  including the deployed 37/59 one-best-sentence-then-phrase page shape. This
  is exact parity for those captured lanes, not a universal all-Luna or
  all-schema ordering claim. `yune-web` has real in-browser validation
  (TypeDuck-Web); the
  TypeDuck-Windows backend has package/header, profile-ABI, and stock
  real-server IPC compatibility smoke through the named profile accessor,
  while the dedicated Windows Yune repository owns interactive TSF typing and
  visible candidate UI product work.
- **Current work:** milestones M38-M60 are complete, with M55 closed under a
  2026-07-04 corrective re-baseline: the benchmark now reads context after
  every keypress (the interactive shape), three pre-corrective closeout
  mechanisms were identified as measurement artifacts and reverted, and the
  standing regression gate (startup, session, eight Track A key rows, Track A
  peak memory, win rows locked `<1.00x`, TypeDuck Track B absolutes) is green
  twice on the honest metric and re-run green at M58 closeout. M58 also fixed
  current `yune-web` TypeDuck/profile reachability for `beingo` / `畀` and
  `zi` / `諮` by short-input profile-ranked paging, without first-page promotion.
  M59 is complete under the full D-47/D-48 contract. Final behavior source
  `443cc636` passes all 32 aggregate and all 160 individual signed-ratchet
  observations in the source-current follow-up packet, while `5fa986d8`
  reconciles the exact 60-asset shipped schema
  tree to 10 schema-asset dispositions, three configuration carriers, and nine
  executable validation rows. Fail-closed Emscripten, app/public builds, native
  recovery, and functional Playwright gates remain source-bound to closeout
  source `5fa986d8`; no browser rerun is claimed for the later native collector
  repair. All five WEB-04 requirements are complete. The
  independent closeout follow-up at `07845e02` makes manifest reconciliation
  bidirectional for the full tree, and `443cc636` fixes the shipping-source
  transformed-tie regression it exposed; the clean canonical recapture is
  again exact at 13/13 and all 5,705 positions without an exception.
  M60 formalizes that already-shipped reachability capability without changing
  engine behavior: the live registry retains `m59-reach03-v1`, declares
  `m60-reachability-v1`, contains exactly `reachabilityOptOuts: []`, and is
  checked against 17 production-derived translator tuples plus classified
  tracked schema roots. New product schemas remain blocking-open until their
  real deploy path is accepted. See the
  [M60 evidence packet](docs/reports/evidence/m60-schema-general-reachability-formalism/README.md).
  A later five-round Mac diagnostic at `0111cf47` has all 17 Track A medians
  and pooled worsts below `1.0x`; complete-input pages are exact on `16/17`,
  including 37/59. It is not source-matched to Windows `443cc636`, so no
  current platform-speed delta is claimed. The signed Windows baseline and
  ceilings remain unchanged.
- **Public demo:** `yune-web` is deployed at <https://yune-web.pages.dev>. The
  current `ef485b10` exact 47-key normal-typing canary passes at `43 ms` p95 /
  `44 ms` max with zero worker queue buildup; browser startup and footprint
  remain separate open deficits.
- **AI posture:** the AI layer exists but is default-off, local-only in the web
  harness, and outside the classic deterministic input path.

See [docs/roadmap.md](docs/roadmap.md) for the detailed milestone plan.

## Compatibility

Yune's compatibility is target-driven, not checklist-driven.

**Reference engines** (the "oracles" that define correct behavior):

- Default core oracle: upstream `rime/librime 1.17.0`
  (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`).
- Canonical Cantonese/Jyutping candidate oracle: upstream
  `rime/librime 1.17.0` plus pinned `rime/rime-cantonese`; M58 completed the
  provenance audit and reachability disposition, while the Yune-facing id
  split remains pending explicit user sign-off.
- TypeDuck profile oracle: TypeDuck-HK/librime `v1.1.2`
  (`74cb52b78fb2411137a7643f6c8bc6517acfde69`) for profile/display/comment
  behavior, multilingual dictionary lookup payloads, fork-only ABI/profile
  controls, and historical fixture-backed profile candidate guards. The
  preferred future TypeDuck multilingual profile id is `jyut6ping3_typeduck`,
  after M58's completed schema/profile blast-radius audit; no split landed, so
  it remains pending explicit user sign-off.

**Rules:**

- Preserve upstream-observable behavior for named targets.
- Isolate TypeDuck fork behavior behind the TypeDuck profile surface; do not
  use fork v1.1.2 ordering to define canonical `jyut6ping3` candidates.
- Add librime features only when a named target needs them.
- Keep expected bytes non-circular: always capture them from the relevant oracle,
  never derive them from Yune itself.

Default `rime_get_api()` remains upstream-shaped. TypeDuck fork-only ABI slots
are exposed exclusively through `rime_get_typeduck_profile_api()`.

## Performance

Performance is lane-specific. Native Track A compares Yune same-run with
pinned upstream librime and reads context after **every keypress**. Earlier
batch-shaped numbers are not comparable.

- **Final-M59 Windows observation:** the five-round follow-up at `443cc636`
  passes all `32/32` aggregate gate rows and `160/160` individual observations
  under unchanged ceilings, with all 17 Track A medians below `1.0x`. It is not
  a signed re-baseline. Current main contains later WEB-03 engine work through
  `7f758fba`, so the final-M59 packet remains source-bound rather than a claim
  about every later commit.
- **Signed Windows authority:** the M59 Increment-0 packet at `45775182` and
  the consolidated threshold registry are authoritative. The registry carries
  retained M55 corrective rows, four M59 injection-on re-derivations, and nine
  newly signed M59 rows. The final follow-up passes those already signed
  ceilings; it does not loosen or replace them.
- **Latest macOS diagnostic:** the five-round `0111cf47` run has every Track A
  median and pooled worst below `1.0x`; complete-input pages are exact on
  `16/17` rows, including 37/59. It is not source-matched to Windows
  `443cc636`, so it does not prove a platform speed delta.
- **Memory:** current Track A peaks are about `8.9x` same-run librime on
  Windows and `11.8x` same-run librime RSS on macOS. Browser footprint remains
  a separate deficit: the latest fair peer lane is `4.000x` on WASM memory and
  `3.471x` on unique encoded resources.
- **Web interaction:** clean source `ef485b10` passes the binding local
  8-scenario / 186-key 4x/4x release gate, its Cloudflare Pages deployment, and
  the source-pinned production canary. The deployed 47-key row is `43 ms` p95 /
  `44 ms` max with zero worker queue wait.
- **Track B:** the current Windows product guard is `16.900 us/key` median and
  the current Mac guard is `5.607 us/key` median, but the platform counters are
  not directly interchangeable and the lane has
  no directly interchangeable librime peer and supports no cross-engine claim.

Current dashboard:

- [All-platform performance results, visualizations, and bottleneck analysis](docs/reports/yune-vs-librime-performance.md)
- [M59 final evidence ledger](docs/reports/evidence/m59-canonical-jyutping-reachability-parity/README.md)
- [M60 reachability-formalism evidence](docs/reports/evidence/m60-schema-general-reachability-formalism/README.md)

## Quick Start

Prerequisites:

- Rust 1.76 or newer
- Node.js and npm (for the browser harness and TypeScript runtime)
- Emscripten (only if building the WASM artifact locally)

Build and test:

```bash
cargo build
cargo test --workspace
```

Feed keystrokes directly to the core engine:

```bash
cargo run -p yune-cli -- run "nihao "
```

Run against real RIME data through the full ABI path:

```bash
cargo run -p yune-cli -- frontend \
  --shared-data-dir ./path/to/rime-data \
  --user-data-dir ./tmp/yune-user \
  --schema luna_pinyin \
  --sequence "nihao "
```

Run the browser demo locally:

```bash
npm --prefix apps/yune-web install
npm --prefix apps/yune-web run build
npm --prefix apps/yune-web run start
```

For browser validation work, start with
[apps/yune-web/e2e/yune-browser-smoke.md](apps/yune-web/e2e/yune-browser-smoke.md).

## Quality Checks

Run these before merging significant changes:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix packages/yune-web-runtime test
npm --prefix packages/yune-web-runtime run build
```

Browser-visible claims need Playwright or equivalent real-browser evidence.

## Repository Layout

| Path | What's In It |
| --- | --- |
| `crates/yune-core` | The engine: dictionary lookup, spelling algebra, candidate ranking, filters, user dictionary, AI staging. |
| `crates/yune-rime-api` | C ABI adapter: exposes the engine through the supported librime-shaped default ABI and named profile surfaces. |
| `crates/yune-cli` | Developer CLI: feed it keystrokes, get JSON output for testing and debugging. |
| `packages/yune-web-runtime` | TypeScript wrapper for the WASM build. |
| `apps/yune-web` | Browser demo app — the public face of the project. |
| `docs` | Roadmap, architecture decisions, conventions, reports. |
| `fixtures` | Deterministic test fixtures (expected engine output for given inputs). |
| `scripts` | Build helpers, benchmarks, oracle-capture tooling. |

## Documentation

- [docs/conventions.md](docs/conventions.md) — architecture, stack, coding rules,
  testing conventions, ABI rules, integrations, and current risks.
- [docs/roadmap.md](docs/roadmap.md) — active roadmap and milestone gates.
- [docs/decisions.md](docs/decisions.md) — decision log and standing principles.
- [docs/requirements.md](docs/requirements.md) — requirement IDs and status.
- [docs/ledgers/fork-parity-ledger.md](docs/ledgers/fork-parity-ledger.md) —
  Cantoboard and TypeDuck fork deltas versus upstream.
- [docs/plans/](docs/plans/) — active, reference, and completed execution
  records.

## Non-Goals

Equally important as the goals — these are things Yune intentionally does not do:

- Bit-for-bit librime internals or full C++ plugin ABI parity.
- A broad librime feature checklist without a named target.
- Widening the default upstream `RimeApi` for TypeDuck-only behavior.
- Cloud inference as a hard dependency.
- Remote AI providers without explicit privacy and product gates.
- Claiming application/browser performance wins from native engine evidence.

## Contributing

Bug reports, feature proposals, and pull requests are welcome. For anything that
affects behavioral compatibility, include oracle-captured evidence (real RIME
output against the same input — expected values must not be derived from Yune
itself). Start with [docs/conventions.md](docs/conventions.md) for architecture
and coding rules.

## License

Original code is [MIT](LICENSE). Third-party schemas, dictionaries, fixtures,
generated data, and provenance materials keep their upstream licenses — see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
