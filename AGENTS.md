# Repository Guide

**Yune** is a Rust input-method engine that uses **librime as a compatibility oracle** while building toward an AI-native input engine librime cannot provide. It has a deterministic core (`yune-core`), a librime-shaped C ABI (`yune-rime-api`), a CLI surrogate (`yune-cli`), and a TypeScript browser runtime (`packages/yune-typeduck-runtime`).

**Core value:** existing RIME schemas and frontends should behave predictably through Yune, with every compatibility difference measured against librime before it is accepted.

**Goal shape — target-driven, not feature-complete:** the oracle is a behavioral *floor*, not a feature checklist. Success means the *named* targets behave correctly versus the oracle — today the `luna_pinyin` core path (vs upstream `1.17.0`) and the TypeDuck `jyut6ping3` profile (vs `v1.1.2`) — broadening to common RIME schemas over time. The **AI-native layer librime cannot provide is the product north star**, not parity for its own sake; bit-for-bit librime feature parity is an explicit non-goal. A librime feature is implemented only when a named target needs it. See `decisions.md` D-24 (oracle precedence) and D-25 (target-driven scope).

## Canonical docs — read these

- **[docs/CONVENTIONS.md](docs/CONVENTIONS.md) — start here.** The single reference for architecture, stack, repo structure, coding & testing conventions, C ABI rules, integrations, and current risks.
- [docs/roadmap.md](docs/roadmap.md) — milestones and what's next (M14–M16 and FORK-PARITY-01..09 complete; preserve web gates, advance Track 2 M17–M19 upstream depth, and define named platform tracks such as iOS before implementation).
- [docs/decisions.md](docs/decisions.md) — the decision log (standing principles + `D-*` entries).
- [docs/requirements.md](docs/requirements.md) — requirement IDs and their status.
- [docs/fork-parity-ledger.md](docs/fork-parity-ledger.md) — **source of truth for every Cantoboard/TypeDuck fork improvement vs upstream `1.17.0`** (origin, category, Yune status: done / todo / deferred / non-goal). **Consult before touching TypeDuck/Cantonese parity** so a fork improvement is not silently lost or re-derived.
- [docs/plans/](docs/plans/) — per-stage execution plans (active at the top, finished ones under `plans/archive/`).

## Key constraints

- **Compatibility oracle:** upstream <https://github.com/rime/librime> latest stable is the default core oracle. The current pinned upstream target is `1.17.0` @ `33e78140250125871856cdc5b42ddc6a5fcd3cd4`. The TypeDuck fork <https://github.com/TypeDuck-HK/librime> @ `v1.1.2` / `74cb52b78fb2411137a7643f6c8bc6517acfde69` is profile-only for TypeDuck compatibility. These are referenced upstream/fork repositories, not local checkout paths.
- **Idiomatic Rust over a C++ clone:** preserve librime-*observable* behavior at the ABI boundary; keep internals clean, typed Rust.
- **Own each slice:** new behavior gets an owning module *and* owning tests; keep `lib.rs`/`main.rs` as facades.
- **C ABI:** `RimeApi` field order *is* the ABI - match upstream `rime_api.h` for core fields, and match the TypeDuck fork header only for explicit TypeDuck-profile fork-only slots. Never insert function-table entries mid-struct without oracle/header evidence.
- **Tests are oracle-driven and non-circular:** capture expected bytes from the oracle, run the real path, never derive the expected value from Yune itself. Uncaptured cases use `#[ignore = "blocked: …"]` with a `panic!()` body — no silent gaps.
- **Security:** runtime resource identifiers are logical IDs, not arbitrary filesystem paths.

## Codex workflow preference

- For non-trivial development work, use a sub-agent-driven workflow. Split work into bounded slices, dispatch repo-local custom agents when available, and keep the main thread focused on coordination, integration, and final verification.
- Keep the main/default model on the strongest available Codex model with highest reasoning (`gpt-5.5` with `model_reasoning_effort = "xhigh"` in this repo). Do not downgrade the main session just to use Spark quota.
- Use Spark (`gpt-5.3-codex-spark`) only for trivial or simple sub-agent slices, such as bounded file lookups, simple mechanical edits, straightforward test-name gathering, or low-risk narrow reviews. Spark agents must always use the highest reasoning setting (`model_reasoning_effort = "xhigh"`). Prefer the repo-local `spark-*` agents for those slices when available.
- Use the default strongest model for complex implementation, architecture, debugging, compatibility/oracle decisions, C ABI work, security-sensitive resource handling, broad reviews, and any task whose risk is unclear.
- Use parallel subagents for independent read-heavy work: codebase exploration, failing-test triage, compatibility/oracle investigation, and review passes. For write-heavy work, avoid parallel agents editing overlapping files; use one worker per slice and review before moving on.
- For substantial implementation, run two review passes before completion: first spec/requirement compliance, then code quality, ABI safety, and test coverage.
- Do not claim Spark quota was consumed unless the active model or subagent configuration is visible or confirmed in the current session.

## Quality gate

`cargo fmt` · `cargo clippy --workspace --all-targets -- -D warnings` · focused tests · `cargo test --workspace` when shared behavior changes. For the TypeScript package: `npm --prefix packages/yune-typeduck-runtime test` and `… run build`.

(The GSD planning system has been retired; planning/decisions/conventions now live under `docs/`, not `.planning/`.)
