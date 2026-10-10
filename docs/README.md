# Docs Index

The documents here describe what is true about the system. Planned and
in-progress work lives in [GitHub issues](https://github.com/CanCLID/yune/issues);
the pinned issue labelled `roadmap` is the entry point.

- [`conventions.md`](./conventions.md) - start here: architecture, stack, repo
  structure, coding and testing conventions, C ABI rules, integrations, risks,
  and where planning, history, and evidence live.
- [`contracts/engine-support-contract.md`](./contracts/engine-support-contract.md)
  - supported targets, oracle precedence, ABI boundaries, evidence lanes, and
  the standing requirements.
- [`contracts/schema-general-reachability.md`](./contracts/schema-general-reachability.md)
  - the default-on single-character reachability contract and its registry.
- [`contracts/typeduck-profile-abi.md`](./contracts/typeduck-profile-abi.md) -
  the opt-in TypeDuck-profile ABI surface.
- [`contracts/fork-parity.md`](./contracts/fork-parity.md) - Cantoboard and
  TypeDuck fork improvements versus upstream `1.17.0`.
- [`decisions.md`](./decisions.md) - standing principles plus the decisions
  that still bind code.
- [`history.md`](./history.md) - one row per completed milestone with its
  closing commit and former plan path.
- [`performance.md`](./performance.md) - the current performance scorecard.
- [`provenance/`](./provenance) - source and fork provenance records.
- [`assets/`](./assets) - charts referenced by the pages above.

## Placement Rules

- Keep the `docs/` root for canonical entry points only.
- Do not add plan, roadmap, requirement, or ledger files; open an issue instead.
- Raw evidence stays outside Git. A compact receipt that a test needs is a
  fixture next to that test. Historical packets removed in the 2026-10
  consolidation remain in git history before the consolidation commit.
- Long-lived contract material goes in `contracts/`; source history goes in
  `provenance/`.
