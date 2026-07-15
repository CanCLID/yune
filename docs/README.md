# Docs Index

Start with [`conventions.md`](./conventions.md) for repository conventions, then use [`roadmap.md`](./roadmap.md) for current sequencing and readiness gates.

## Current Entry Points

- [`conventions.md`](./conventions.md) - architecture, stack, repo structure, coding/testing conventions, integrations, risks, and planning-doc rules.
- [`roadmap.md`](./roadmap.md) - current dashboard, active sequence, scope boundaries, and milestone readiness/closeout state.
- [`requirements.md`](./requirements.md) - requirement IDs and milestone status.
- [`decisions.md`](./decisions.md) - standing principles and decision log.

## Ledgers

- [`ledgers/milestone-history.md`](./ledgers/milestone-history.md) - completed milestone outcomes and historical evidence pointers.
- [`ledgers/fork-parity-ledger.md`](./ledgers/fork-parity-ledger.md) - Cantoboard/TypeDuck fork improvements versus upstream `1.17.0`.

## Plans

- [`plans/active/`](./plans/active) - current or planned work that can still be executed.
- [`plans/reference/`](./plans/reference) - standing designs and compatibility contracts that are not active execution plans.
- [`plans/completed/`](./plans/completed) - finished, superseded, or historical execution records.

## Supporting Material

- [`references/`](./references) - stable non-plan reference material, such as frontend/backend contracts.
- [`provenance/`](./provenance) - source/fork provenance records.
- [`reports/`](./reports) - performance reports and evidence indexes.
- [`reports/yune-vs-librime-performance.md`](./reports/yune-vs-librime-performance.md)
  - single current dashboard for all-platform benchmark results,
    visualizations, evidence boundaries, and bottleneck analysis.
- [`reports/history/README.md`](./reports/history/README.md)
  - superseded performance reports and source-bound analysis retained for
    audit history.
- [`reports/evidence/m59-current-source-macos-20260714/`](./reports/evidence/m59-current-source-macos-20260714/)
  - reviewed five-round `0111cf47` Mac diagnostic with the complete 17-row and
    Track B tables, fixed-binary/provenance audit, portable report, Fable review
    resolution, and normalized manifest for the full external raw packet.
- [`reports/evidence/m59-final-source-macos-20260713/`](./reports/evidence/m59-final-source-macos-20260713/)
  - historical source-matched five-round `5879405c` Mac diagnostic with a disclosed
    transient-output deviation, fixed-binary audit, behavior gates, and
    validated source-bound report; no `443cc636` or current-main Mac rerun.

## Placement Rules

- Keep the `docs/` root for canonical entry points only.
- Put current work in `plans/active/`; move it to `plans/completed/` when closed.
- Put long-lived design or contract material in `plans/reference/` or `references/`, not in an archive folder.
- Put source-history material in `provenance/`.
- Do not add a generic `archive/` directory; choose the category that explains why the document is being kept.
