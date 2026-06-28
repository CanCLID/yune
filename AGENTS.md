# Repository Guide

**Yune** is a Rust input-method engine that uses **librime as a compatibility
oracle** while building toward an AI-native input engine librime cannot provide.
It has a deterministic core (`yune-core`), a librime-shaped C ABI
(`yune-rime-api`), a CLI surrogate (`yune-cli`), a TypeScript browser runtime
(`packages/yune-web-runtime`), and the `yune-web` browser harness under
`apps/yune-web/`.

**Core value:** existing RIME schemas and frontends should behave predictably
through Yune, with every compatibility difference measured against the relevant
oracle before it is accepted.

**Goal shape - target-driven, not feature-complete:** the oracle is a behavioral
floor, not a feature checklist. Success means the named targets behave correctly
versus the oracle: upstream `luna_pinyin` and common-schema behavior against
upstream `rime/librime 1.17.0`, plus the TypeDuck `jyut6ping3` profile against
TypeDuck-HK/librime `v1.1.2`. Bit-for-bit librime feature parity is an explicit
non-goal. A librime feature is implemented only when a named target needs it.
See `decisions.md` D-24 (oracle precedence) and D-25 (target-driven scope).

## Current State

- **Phase 1 engine/basic oracle parity is complete for the named target set.**
  M0-M24 are complete, including M10 TypeDuck-Windows backend compatibility
  smoke through the named TypeDuck profile ABI and the M24 historical
  TypeDuck-Web-derived dogfooding/demo-hardening batch. Future `yune-web`
  dogfood reports should start a new scoped plan rather than reopening Phase 1.
- **Phase 2 is product/platform work.** The first Phase 2 planning artifact is
  `docs/plans/active/p2-win01-plan-typeduck-windows-next.md`, for a Yune-first
  TypeDuck-Windows product/frontend. Phase 2 work must not widen Yune's default
  upstream ABI.
- **M47 is the active engine milestone: iOS-budget native memory reduction.**
  Portable engine work to fit the single-active-schema native working set under an
  iOS keyboard-extension budget (steady ≤ 48 MB, peak ≤ 64 MB), measured on Windows
  via the lean `native_memory_probe`. Attribution-first: the measured baseline is
  `jyut6ping3_mobile` steady `~298 MB` / peak `~482 MB` loaded eagerly at
  `create_session`, with ~235 MB un-owned that Phase 0 must attribute before any
  reduction branch. Plan:
  `docs/plans/active/m47-plan-ios-budget-native-memory-reduction.md`; baseline:
  `docs/reports/ios-memory-budget.md`. On-device iOS proof is a later Phase 2 gate.
- **AI foundation exists.** M11 completed the core/CLI AI layer. M13 exposed it
  in the web harness now canonical as `yune-web`: default-off, local-only,
  second-pass `stage_ai` flow.
  Remote providers, richer contextual translation, and native frontend AI UX
  remain future product tracks.

## Canonical Docs - Read These

- **[docs/conventions.md](./docs/conventions.md) - start here.** The single
  reference for architecture, stack, repo structure, coding/testing
  conventions, C ABI rules, integrations, and current risks.
- [docs/roadmap.md](./docs/roadmap.md) - current dashboard, active sequence,
  scope boundaries, and M37 readiness gates.
- [docs/ledgers/milestone-history.md](./docs/ledgers/milestone-history.md) - completed milestone
  ledger and historical plan/evidence pointers.
- [docs/decisions.md](./docs/decisions.md) - the decision log (standing
  principles plus `D-*` entries).
- [docs/requirements.md](./docs/requirements.md) - requirement IDs and status.
- [docs/ledgers/fork-parity-ledger.md](./docs/ledgers/fork-parity-ledger.md) - source of truth
  for Cantoboard/TypeDuck fork improvements versus upstream `1.17.0`. Consult
  this before touching TypeDuck/Cantonese parity.
- [docs/plans/](./docs/plans) - active, reference, and completed execution records.
  Finished plans live under `docs/plans/completed/`.

## Key Constraints

- **Compatibility oracle:** upstream <https://github.com/rime/librime>
  `1.17.0` at `33e78140250125871856cdc5b42ddc6a5fcd3cd4` is the default core
  oracle. TypeDuck-HK/librime `v1.1.2` at
  `74cb52b78fb2411137a7643f6c8bc6517acfde69` is profile-only for TypeDuck
  compatibility. These are referenced upstream/fork repositories, not local
  checkout paths.
- **Idiomatic Rust over a C++ clone:** preserve librime-observable behavior at
  the ABI boundary; keep internals clean, typed Rust.
- **Own each slice:** new behavior gets an owning module and owning tests; keep
  `lib.rs`/`main.rs` as facades.
- **C ABI:** `RimeApi` field order is the ABI. Match upstream `rime_api.h` for
  core/default fields, and expose TypeDuck fork-only slots only through explicit
  TypeDuck-profile surfaces such as `rime_get_typeduck_profile_api()`.
- **Tests are oracle-driven and non-circular:** capture expected bytes from the
  oracle, run the real path, never derive expected values from Yune itself.
  Uncaptured cases use `#[ignore = "blocked: ..."]` with a `panic!()` body - no
  silent gaps.
- **Security:** runtime resource identifiers are logical IDs, not arbitrary
  filesystem paths.
- **yune-web / TypeDuck-Web provenance:** `apps/yune-web/source/` is the local
  upstream-derived app checkout. The committed Yune-owned state is the patch under
  `apps/yune-web/patches/`, the `yune-integration/` bridge, E2E
  tests, and recorded evidence. Browser-visible claims require Playwright or
  equivalent real-browser evidence.
- **TypeDuck-Windows:** M10 proves Yune can satisfy the existing native backend
  profile smoke. Future Windows work is Phase 2 product/frontend work; do not
  use it as a reason to widen default `rime_get_api()`.

## Codex Workflow Preference

- For non-trivial development work, use a sub-agent-driven workflow. Split work
  into bounded slices, dispatch repo-local custom agents when available, and
  keep the main thread focused on coordination, integration, and final
  verification.
- Keep the main/default model on the strongest available Codex model with
  highest reasoning (`gpt-5.5` with `model_reasoning_effort = "xhigh"` in this
  repo). Do not downgrade the main session just to use Spark quota.
- Use Spark (`gpt-5.3-codex-spark`) only for trivial or simple sub-agent slices,
  such as bounded file lookups, simple mechanical edits, straightforward
  test-name gathering, or low-risk narrow reviews. Spark agents must always use
  the highest reasoning setting (`model_reasoning_effort = "xhigh"`). Prefer the
  repo-local `spark-*` agents for those slices when available.
- Use the default strongest model for complex implementation, architecture,
  debugging, compatibility/oracle decisions, C ABI work, security-sensitive
  resource handling, broad reviews, and any task whose risk is unclear.
- Use parallel subagents for independent read-heavy work: codebase exploration,
  failing-test triage, compatibility/oracle investigation, and review passes.
  For write-heavy work, avoid parallel agents editing overlapping files; use one
  worker per slice and review before moving on.
- For substantial implementation, run two review passes before completion:
  first spec/requirement compliance, then code quality, ABI safety, and test
  coverage.
- Do not run tests, typecheck, lint, browser smoke, or full quality gates by
  default. Run verification only when the task or touched code path requires it,
  when the user asks for it, when you need evidence for a claim, or when risk is
  high enough that skipping it would make the handoff unreliable. For docs-only
  or narrow mechanical edits, prefer targeted inspection/link checks over broad
  test suites.
- Do not claim Spark quota was consumed unless the active model or subagent
  configuration is visible or confirmed in the current session.

## Quality Gate

These are release/milestone gates and should be run when the work actually needs
that level of verification; they are not automatic for every task.

Rust:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

TypeScript runtime:

```powershell
npm --prefix packages/yune-web-runtime test
npm --prefix packages/yune-web-runtime run build
```

yune-web browser work must also follow the current plan or archived M24
baseline and `apps/yune-web/e2e/yune-browser-smoke.md`; preserve the
real-browser evidence gate for user-visible claims.

The GSD planning system has been retired. Planning, decisions, conventions, and
requirements now live under `docs/`, not `.planning/`.
