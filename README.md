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
  while interactive TSF typing and visible candidate UI remain Phase 2
  product/frontend work.
- **Current work:** milestones M38-M59 are complete, with M55 closed under a
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
- **Public demo:** `yune-web` is deployed at <https://yune-web.pages.dev>. It's
  a Yune engine demo, not a claim that browser-level performance is solved.
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

The standing native Windows gate is intentionally measured same-run against
upstream `rime/librime 1.17.0`, with the context read after
**every keypress** (the shape real frontends have; earlier batch-shaped
numbers are not comparable and the pre-corrective M55 closeout numbers were
measurement artifacts — see
[the corrective record](docs/reports/evidence/m55-native-match-or-beat/corrective-2026-07-04/README.md)).
This is not a claim that every benchmark row ranks identically to librime or
that browser performance is solved.

![Yune vs librime native latency ratios across all input dimensions](docs/reports/evidence/dashboard-visuals-2026-07-04/native-track-a-latency-ratios.svg)

Signed Windows Track A same-run ratios (M55 corrective gate run D, 2026-07-04;
lower is better) remain the gate:

- **Faster than librime:** startup `0.895x` and session `0.864x` (run-noisy),
  `zhongguo` `0.255x`, `cszysmsrsd` `0.381x`, and `zybfshmsru` `0.564x` —
  the three win rows are locked `<1.00x` in the standing gate.
- **Slower than librime, bounded and guarded:** the short keys `hao` `1.574x`
  (+9 us), `ni` `2.433x` (+25 us), `n` `2.636x` (+34 us) — tens of
  microseconds, imperceptible while typing — and the sentence rows: 37-char
  `1.913x` (+273 us) and 59-char `1.528x` (+352 us), both improved ~35% from
  the pre-M55 `3.05x`/`2.25x` by real graph-work reductions.
- **Memory:** the shipping default keeps the Luna poet payload on the heap
  (Track A peak `185.7 MB` vs librime peer `13.5 MB`) because the latency
  ceilings bind. M55's historical `YUNE-POET/2` byte-backed opt-in
  (`YUNE_POET_BYTE_BACKED=1`) measured `113.2 MB` with identical candidates
  but cost `4.6x`/`3.2x` on the sentence rows. M59's sentence/phrase index
  extends the current artifact to `YUNE-POET/3`; `/2` is deliberately rejected
  and rebuilt, so the M55 numbers remain historical rather than being silently
  attributed to `/3`. M59's `/3` path now supplies the named exact Lane B and
  37/59 page-shape behavior and passes the unchanged signed performance gate;
  no new byte-backed poet memory win is claimed from that behavior result. The
  browser `luna_pinyin` gap
  (`64 MiB` vs My RIME `16 MiB`) is a separate lane. The Jyutping product path is **not** a like-for-like
  comparison (TypeDuck multilingual dictionary); M47 byte-backed the shipping
  keyboard profile to about `67 MB` working set / `22 MB` private.
- Track B TypeDuck-profile rows and browser startup are separate evidence lanes,
  not upstream-librime native comparisons. The M58 closeout Track B guard proof is
  the final-pass ratchet (`335.823 us` on the 50+ key product row, ceiling
  `347.975 us`), and it remains TypeDuck/profile product-path evidence rather
  than canonical `rime-cantonese` candidate-oracle evidence.

The historical source-bound Mac packet at `afb7079b` is diagnostic, not a new
gate or current-main proof. At that source Yune wins 6/17 aggregate Track A rows
and loses 11. Its 37/59 aggregate ratios are
`0.399x`/`0.205x`, but candidate-text-different prefixes dominate those wins;
the text-matched sensitivity is `1.420x`/`1.204x`. `n`/`zh` use
`8.682x`/`4.092x` librime's instructions. Allocator and platform effects are
partial, and exact cross-platform attribution remains open.

Current reports:

- [docs/reports/yune-vs-librime-performance.md](docs/reports/yune-vs-librime-performance.md)
- [docs/reports/yune-vs-librime-root-cause-analysis.md](docs/reports/yune-vs-librime-root-cause-analysis.md)
- [M59 post-fix macOS root-cause packet](docs/reports/evidence/m59-post-fix-root-cause-20260711/)
- [M59 final evidence ledger](docs/reports/evidence/m59-canonical-jyutping-reachability-parity/README.md)

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
