# M60 Schema-General Reachability Capability Formalism

> **Milestone:** M60. **Status:** Draft, ready for owner scheduling after M59. **Track:**
> documentation/static-audit follow-up. **Created:** 2026-07-06; **rescoped:**
> 2026-07-07; **normalized:** 2026-07-13.

## Binding Boundary

M59 owns and implements the behavior. It makes arbitrary non-lexicon
single-character composition default-on for every schema, with zero per-schema
enablement work, and records real-path shipped-schema acceptance. M60 must not
reimplement, repair, widen, or re-baseline that behavior.

M60 is formalism-only:

- document the already-shipped capability and its default-on invariant;
- define an auditable explicit opt-out record with owner, reason, evidence, and
  review date;
- make onboarding documentation and static checks point to M59's registry;
- preserve the distinct TypeDuck `prefix_fallback` and schema-general
  `leading_syllable_reachability` mechanisms and their precedence;
- record provenance for any future explicit opt-out.

An unsupported/N/A classification is not an escape hatch. A newly added schema
inherits the M59 default and enters the M59 acceptance registry as a blocking
open row until its real-path acceptance evidence is supplied.

## Preconditions

- [x] M59 is fully gated, archived, and recorded complete; M60 execution may
      begin only after the shared closeout commit is pushed.
- [ ] Read the final M59 evidence ledger, D-47/D-48, the engine support
      contract, and `apps/yune-web/schema-acceptance-coverage.json`.
- [ ] Confirm no M59 behavior, performance, browser, packaging, or evidence
      blocker is being relabeled as M60 paperwork.

## Deliverables

### 1. Capability contract

- [ ] Add a short contract section naming the default-on invariant, the
      translator-level owner, explicit-false semantics, selection/recomposition
      behavior, and TypeDuck profile precedence.
- [ ] State that schema ids, input allowlists, baked oracle output, promotion
      tables, and silent onboarding failure are prohibited.
- [ ] Link the canonical M59 acceptance registry and final evidence rather than
      duplicating its schema inventory.

### 2. Explicit opt-out record

- [ ] Define one machine-readable opt-out shape with: logical schema id,
      schema-source commit, owner, reason, affected surface, oracle/owner-spec
      evidence, approval, review trigger, and expiry or review date.
- [ ] Require explicit owner sign-off before an opt-out can be accepted.
- [ ] Reject an opt-out row that has missing provenance, an untracked schema,
      or no corresponding real-path acceptance disposition.
- [ ] Do not add a placeholder opt-out for any current schema.

### 3. Onboarding and static audit

- [ ] Document the workflow for adding a schema asset: update the schema tree,
      run the manifest updater, resolve the automatically created open
      acceptance row, add oracle/owner-spec provenance, and run the narrow
      owning checker/test.
- [ ] Add or tighten a static audit only if needed to verify the documentation
      contract. It may validate registry/opt-out structure and links; it must
      not duplicate M59 runtime behavior or add schema-specific enablement.
- [ ] Keep canonical `rime-cantonese`, shipped/product schemas, dependency-only
      assets, and mandatory nonshipped validation rows explicitly distinct.

### 4. Evidence and closeout

- [ ] Preserve text-only commands, checker output, link audit, and two
      independent documentation/spec reviews under a compact M60 evidence root.
- [ ] Update requirements, decisions, roadmap, support contract, README, and
      milestone history only for the formalism actually delivered.
- [ ] Move this plan to `completed/` only after the narrow static/link gates are
      green and the evidence packet is committed and pushed.

## Verification

Use the smallest owning checks. This is a documentation/static-audit milestone:
do not rerun M59 performance, full Rust, WASM, browser, or packaging suites
unless M60 unexpectedly changes their owning code (which requires an explicit
scope decision first).

Minimum expected checks:

- the schema manifest/acceptance checker;
- any new opt-out/static-audit unit test;
- Markdown link/path inspection for touched documents;
- `git diff --check`.

## Non-Goals

- No reachability implementation or ranking change.
- No schema-specific true flag or adapter.
- No new schema, product profile, schema-id rename, userdb migration, C ABI, or
  browser surface.
- No new performance ceiling, benchmark claim, or M59 exception.
- No unifying `prefix_fallback` with `leading_syllable_reachability`.
- No weakening or reopening D-47/D-48.
