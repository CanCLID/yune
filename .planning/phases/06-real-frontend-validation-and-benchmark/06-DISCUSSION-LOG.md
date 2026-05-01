# Phase 6: Real Frontend Validation And Benchmark - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-01
**Phase:** 06-real-frontend-validation-and-benchmark
**Areas discussed:** Validation target order, frontend lifecycle coverage, TypeDuck-Web integration shape, benchmark baselines, AI-native readiness gate

---

## Validation Target Order

| Option | Description | Selected |
|--------|-------------|----------|
| Native loader first, TypeDuck-Web second, macOS third, Linux scoped later | Builds from existing cdylib loader coverage, then validates a real browser app wrapper, then native IME lifecycle paths. | ✓ |
| TypeDuck-Web first | Starts from a real app frontend immediately, but risks confusing browser/WASM behavior with native IME behavior. | |
| Native frontends only | Prioritizes Squirrel/ibus/fcitx but misses the compact TypeDuck-Web wrapper opportunity. | |

**User's choice:** Auto-selected recommended default under `--auto`.
**Notes:** The user explicitly asked whether TypeDuck-Web can be used; the decision is yes, as an additive real-application target that does not replace native validation.

---

## Frontend Lifecycle Coverage

| Option | Description | Selected |
|--------|-------------|----------|
| Trace-driven host-shaped lifecycle coverage | Cover setup/init/deploy/schema/session/key/context/status/commit/teardown and capture call traces for mismatches. | ✓ |
| Broad frontend emulation | Try to imitate complete frontend behavior inside tests, increasing fragility and scope. | |
| Only extend existing unit tests | Keeps scope small but may miss frontend lifecycle sequencing issues. | |

**User's choice:** Auto-selected recommended default under `--auto`.
**Notes:** Existing `dynamic_loader.rs` already exercises the cdylib and API table; Phase 6 should extend this into fuller host-shaped validation.

---

## TypeDuck-Web Integration Shape

| Option | Description | Selected |
|--------|-------------|----------|
| Map TypeDuck-Web wrapper calls and capture runnable reproduction, minimized fixture, or documented blocker | Uses TypeDuck-Web as a real app-shaped frontend without vendoring or overcommitting to browser/WASM constraints. | ✓ |
| Vendor TypeDuck-Web into the repo | Could make reproduction self-contained but adds dependency and maintenance scope. | |
| Treat TypeDuck-Web as documentation only | Avoids setup cost but misses real wrapper validation value. | |

**User's choice:** Auto-selected recommended default under `--auto`.
**Notes:** TypeDuck-Web exercises `rime_get_api`, setup, initialize, notification handling, deploy/maintenance, session creation, key simulation, context/commit reads, candidate operations, customization, and IDBFS persistence.

---

## Benchmark Baselines

| Option | Description | Selected |
|--------|-------------|----------|
| Add minimal reproducible frontend-sensitive baselines after validation paths identify hot surfaces | Covers session lifecycle, per-key processing, deployment/dictionary loading, and userdb learning/sync without premature infrastructure. | ✓ |
| Build a large benchmark suite first | Could be comprehensive but risks measuring the wrong surfaces before frontend validation. | |
| Defer benchmarks entirely | Simplifies validation but fails Phase 6 benchmark requirements. | |

**User's choice:** Auto-selected recommended default under `--auto`.
**Notes:** Benchmarks should be comparable for future frontend and AI-native changes.

---

## AI-Native Readiness Gate

| Option | Description | Selected |
|--------|-------------|----------|
| End Phase 6 with a go/no-go recommendation based on lifecycle validation and benchmarks | Keeps AI-native work deferred until compatibility foundation is tested through frontend lifecycles. | ✓ |
| Start AI-native design in parallel | Faster product exploration but violates the current phase boundary. | |
| Block AI-native until every native frontend works | Too strict for a milestone gate; documented blockers can be enough for no-go/go-with-caveats. | |

**User's choice:** Auto-selected recommended default under `--auto`.
**Notes:** The readiness decision should be evidence-based, not a design exercise for AI features.

---

## Claude's Discretion

- Exact lifecycle trace format.
- Exact benchmark harness organization.
- Whether frontend-observed gaps become integration tests, minimized fixtures, or notes first.

## Deferred Ideas

- AI-native provider/ranker/context/memory/privacy implementation.
- Full librime C++ plugin ABI compatibility.
- New graphical end-user frontend.
