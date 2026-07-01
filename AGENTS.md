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
- **M47 (iOS-budget native memory reduction) is complete for its portable scope.**
  Phase 0 + RED-01…RED-08 byte-backed the native footprint (table, prism, *and*
  rich comment/lookup payloads served from mmap'd compiled storage like
  librime/Cantoboard), taking the comments-intact `jyut6ping3_mobile` keyboard
  profile from `~298 MB` to `~67 MB` working set / `~22 MB` private (the iOS-dirty
  proxy, under the 48 MB target) with the full multilingual TypeDuck dictionary
  retained and parity-clean. **All numbers are Windows proxies, not iOS
  `phys_footprint` — "iOS budget proven" is not claimed.** On-device measurement
  remains a far-future platform validation gate, and RED-09/10/11 remain optional
  future engine-optimization candidates; none is a newly opened numbered
  milestone. Completed plan:
  `docs/plans/completed/m47-plan-ios-budget-native-memory-reduction.md`; report:
  `docs/reports/ios-memory-budget.md`. Lean probe: `crates/yune-rime-api/tests/native_memory_probe.rs`.
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

## Verification Discipline

Treat any "done / gates passed / verified" report — from a prior session,
another agent, or a handoff — as an **unverified claim, not a fact.**
Independently re-run the load-bearing check before you accept it or build on it.
This repeatedly caught real defects that a trusted handoff would have shipped:

- **Run the exact named gate, not a subset.** A closeout claimed the quality
  gate passed but never ran `cargo clippy --workspace --all-targets -- -D
  warnings`, and that gate was failing. Lint-only errors do not fail `cargo
  test`/`cargo build`, so a partial "fmt + tests passed" check silently misses
  them. If you claim the gate passed, run the exact commands (fmt, the broad
  clippy `-D warnings`, the relevant tests) and list them — do not paraphrase
  "verification passed."
- **An oracle test must execute the real path over external bytes.** A
  byte-parity bug shipped green because its "oracle" test compared one
  checked-in file to another, and the other grammar tests round-tripped through
  Yune's own encoder/builder/reader. Self-consistency is not oracle parity. A
  real oracle test runs Yune's production path over externally-captured oracle
  bytes and byte-compares (see the oracle-driven/non-circular rule in Key
  Constraints); a manifest/provenance assertion is not a behavior test.
- **Verify public/docs claims against evidence across ALL claim kinds.** README
  and top-level claims (performance, compatibility, oracle precedence,
  frontend-validation, ABI, licensing) drifted ahead of the measured evidence
  across several closeouts, and a claim audit scoped to one kind (e.g.
  performance) missed the others. Check every claim kind against the evidence
  and the engine support contract at closeout, not just the one in focus.

When reviewing another agent's work, re-run the specific gate or reproduce the
exact behavior a finding hinges on rather than trusting the summary. The
two-review-pass rule above is only as good as the independent checks behind it.

The GSD planning system has been retired. Planning, decisions, conventions, and
requirements now live under `docs/`, not `.planning/`.
