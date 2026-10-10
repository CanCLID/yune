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
upstream `rime/librime 1.17.0`; canonical `jyut6ping3` candidate ordering,
segmentation, fallback, and completion against upstream `rime/librime 1.17.0`
plus pinned `rime/rime-cantonese`; and TypeDuck multilingual/comment/profile
behavior against TypeDuck-HK/librime `v1.1.2` as a profile lane with
grandfathered fixture-backed candidate guards. `jyut6ping3_typeduck` is the
preferred future TypeDuck profile id. M58's blast-radius audit is complete, but
the schema-id direction still requires explicit user sign-off before
implementation.
Bit-for-bit librime feature parity is an explicit non-goal. A librime feature is
implemented only when a named target needs it. See `decisions.md` D-24 (oracle
precedence), D-25 (target-driven scope), and D-31 (canonical Jyutping versus
TypeDuck profile split).

## Current State

- **Phase 1 engine/basic oracle parity is complete for the named target set**
  (M0-M24), including TypeDuck-Windows backend compatibility smoke through the
  named TypeDuck profile ABI and the TypeDuck-Web-derived dogfooding batches.
- **Windows product/platform work is externally owned.** The dedicated Windows
  Yune repository owns the TypeDuck-Windows frontend/product schedule. This
  repository owns only the engine/package contract, the named profile API,
  oracle fixtures, and separately reviewed cross-repository engine proposals.
  Windows product needs must not widen Yune's default upstream ABI.
- **Canonical reachability and exact-order closure (M59/M60) is complete.**
  Lane A strict `13/13` across 5,705 captured positions, Lane B exact for all
  seven captured inputs, Cangjie `12/12`, deployed 37/59 Luna page-shape
  parity, schema-general default-on single-character reachability for every
  shipped schema, and a source-current `443cc636` performance ratchet of
  `32/32` aggregate rows. The live registry has exactly `reachabilityOptOuts: []`.
- **Native memory reduction (M61) closed as a measured no-go.** The byte-backed
  correction was reverted; no production-default memory reduction is claimed.
  Any new native-memory attempt needs fresh source-bound owner attribution.
- **iOS-budget native memory work (M47) is complete for its portable scope.**
  The comments-intact `jyut6ping3_mobile` keyboard profile went from about
  `298 MB` to about `67 MB` working set / `22 MB` private, measured as Windows
  proxies, not iOS `phys_footprint`; on-device validation is a planned issue.
  Lean probe: `crates/yune-rime-api/tests/native_memory_probe.rs`.
- **AI foundation exists.** The core/CLI AI layer (M11) and its default-off,
  local-only, second-pass `stage_ai` exposure in `yune-web` (M13) are done.
  Remote providers and native frontend AI UX remain future product tracks.

Completed milestones, with closing commits and former plan paths, are listed in
[docs/history.md](./docs/history.md). Current numbers are in
[docs/performance.md](./docs/performance.md).

## Planning Happens In GitHub Issues

Planned and in-progress work lives in
[GitHub issues](https://github.com/CanCLID/yune/issues). The pinned issue
labelled `roadmap` is the parent of every planned item; children carry
"blocked by" relationships. Do not create plan files under `docs/`. Issues
are public: state the user-visible outcome in the title, gloss internal
jargon on first use, and keep host-specific paths and private data out.

Every issue carries three kinds of metadata, and they do not overlap:

- **Type** (Bug, Feature, Task): what it is. The issue forms preset it.
- **Fields** (Priority: Urgent/High/Medium/Low; Effort: High/Medium/Low): how
  it ranks. Set them in the Fields panel when the issue is opened or triaged.
- **Labels**: the area (`engine`, `web`, `abi`, `platform`, `documentation`)
  plus the kind of work (`perf`, `memory`, `parity`, `ux`, `test`,
  `decision-needed`, `roadmap`). There is no `bug` or `enhancement` label;
  Type covers that.

Pull request descriptions carry the closeout: the exact verification commands
run and their results, and `Closes #N` for the issue.

## Canonical Docs - Read These

- **[docs/conventions.md](./docs/conventions.md) - start here.** The single
  reference for architecture, stack, repo structure, coding/testing
  conventions, C ABI rules, integrations, and current risks.
- [docs/contracts/engine-support-contract.md](./docs/contracts/engine-support-contract.md)
  - supported targets, oracle precedence, ABI boundaries, evidence lanes, and
  the standing requirements.
- [docs/decisions.md](./docs/decisions.md) - standing principles plus the
  decisions that still bind code.
- [docs/contracts/fork-parity.md](./docs/contracts/fork-parity.md) - source of
  truth for Cantoboard/TypeDuck fork improvements versus upstream `1.17.0`.
  Consult this before touching TypeDuck/Cantonese parity.
- [docs/history.md](./docs/history.md) - completed milestone ledger with
  closing commits; full plan text is in git history.
- [docs/performance.md](./docs/performance.md) - the current performance
  scorecard and bottleneck disposition.

## Key Constraints

- **Compatibility oracle:** upstream <https://github.com/rime/librime>
  `1.17.0` at `33e78140250125871856cdc5b42ddc6a5fcd3cd4` is the default core
  oracle. Canonical Cantonese/Jyutping candidate behavior uses that upstream
  engine with pinned <https://github.com/rime/rime-cantonese>. TypeDuck-HK/librime `v1.1.2` at
  `74cb52b78fb2411137a7643f6c8bc6517acfde69` is profile-only for TypeDuck
  multilingual/comment/profile behavior and historical fixture-backed profile
  candidate guards. The preferred future TypeDuck profile id is
  `jyut6ping3_typeduck`; M58's schema/profile blast-radius audit is complete,
  while explicit user sign-off remains pending. These are referenced
  upstream/fork repositories, not local checkout paths.
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
  profile smoke. The dedicated Windows Yune repository owns subsequent TSF and
  product/frontend work; do not use it as a reason to widen default
  `rime_get_api()` here.

## Codex Workflow Preference

- For non-trivial development work, use a sub-agent-driven workflow. Split work
  into bounded slices, dispatch repo-local custom agents when available, and
  keep the main thread focused on coordination, integration, and final
  verification.
- Keep the main/default model and every subagent on the strongest available
  Codex model at its highest supported reasoning level. This repo pins
  `gpt-5.6-sol` with `model_reasoning_effort = "ultra"` for the main session and
  the repo-local `default`, `worker`, `explorer`, and `yune-reviewer` agents.
- The repo-local agent definitions intentionally override Codex's built-in
  `default`, `worker`, and `explorer` roles so delegated work is not
  automatically routed to a cheaper model. Use the role that fits the slice;
  use `yune-reviewer` for independent review passes.
- There is no automatic fallback. If Sol Ultra is unavailable on the current
  account or surface, explicitly override both the main default and every
  repo-local agent definition to the strongest available model at its highest
  supported reasoning level, and state the fallback. Do not silently downgrade
  work for latency or quota.
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
- Minimize duplicate verification. Reuse a source-current successful gate for
  later evidence or docs-only commits, and do not rerun an entire broad suite
  merely because its log needs to be packaged. If a broad gate stops for an
  environmental reason after earlier phases passed, preserve that result and
  rerun only the failed or still-unproved slice unless the release contract
  explicitly requires a fresh end-to-end rerun.
- Before starting any expensive gate, state why it is load-bearing for the
  current change or milestone. Do not run speculative, overlapping, or
  reassurance-only suites while an owning gate already covers the same claim.
- Do not claim Sol Ultra was used unless the active runtime or agent thread
  reports it; checked-in configuration proves intent, not execution.

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
