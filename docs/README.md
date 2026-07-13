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

## Placement Rules

- Keep the `docs/` root for canonical entry points only.
- Put current work in `plans/active/`; move it to `plans/completed/` when closed.
- Put long-lived design or contract material in `plans/reference/` or `references/`, not in an archive folder.
- Put source-history material in `provenance/`.
- Do not add a generic `archive/` directory; choose the category that explains why the document is being kept.
