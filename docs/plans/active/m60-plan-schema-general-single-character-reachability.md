# M60 Schema-General Reachability Capability Formalism

> **Milestone:** M60. **Status:** Finalized — ready for execution; not started.
> **Track:** documentation and fail-closed static governance. **Created:**
> 2026-07-06; **rescoped:** 2026-07-07; **finalized:** 2026-07-15.

## Outcome

M60 turns M59's already-shipped schema-general reachability guarantee into a
durable capability contract. It makes future schema onboarding and any explicit
opt-out auditable and fail closed without changing engine behavior.

M60 closes when all six conditions hold:

1. one canonical contract describes the default-on invariant and the actual
   per-input relationship between the two reachability mechanisms;
2. the M59-founded acceptance coverage becomes an explicitly live, versioned
   registry with an empty current opt-out collection;
3. the manifest checker reconciles every affected shipped namespaced
   explicit-false tuple to a complete owner-approved opt-out record;
4. every tracked schema YAML belongs to a machine-classified root, and the
   updater creates a blocking open row for each new product schema asset without
   creating or implying an opt-out;
5. schema onboarding is documented from asset discovery through real-path
   acceptance; and
6. compact, source-bound evidence and two independent reviews close the
   milestone without reopening M59 or WEB03-11.

## Authority And Boundaries

M59 owns the behavior. In particular:

- D-47 and M59-REACH-02 make
  `<dictionary-translator namespace>/leading_syllable_reachability` default to
  `true` for every deployed schema (the conventional namespace is
  `translator`) unless a tracked setting explicitly says `false`;
- D-48 owns the existing candidate-order lanes and exceptions;
- final native behavior and performance authority remains `443cc636`;
- shipped registry/browser/package evidence remains bound to `5fa986d8`, with
  bidirectional manifest exactness added at `07845e02`; and
- historical M59 evidence is immutable and is linked, not rewritten.

The planning baseline is the pushed WEB03-11 closeout commit `10b119fa`; its
measured product source remains `ef485b10`. M60 neither reruns nor supersedes
that browser evidence. At kickoff, record the actual Yune base commit containing
this finalized plan and the exact tree state. The pre-existing staged
`.codex/config.toml` change is user-owned, remains excluded from every M60
commit, and must be reported as an unrelated dirty path rather than mislabeled
as a clean source tree.

Referenced oracle/profile pins remain:

- upstream `rime/librime 1.17.0`:
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4`;
- canonical `rime/rime-cantonese`:
  `c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0`, tree
  `eb193fb80675ffa60df3c32bf24afa7d7f68617a`; and
- TypeDuck-HK/librime profile oracle `v1.1.2`:
  `74cb52b78fb2411137a7643f6c8bc6517acfde69`.

Those pins identify provenance only; M60 performs no new oracle capture or
benchmark. D-24 and D-31 continue to decide which oracle owns a claim.

D-49 keeps Windows TSF, UI, installer/update, diagnostics, evidence, and
scheduling in [`CanCLID/yune-windows`](https://github.com/CanCLID/yune-windows).
M60 makes no Windows product change. M61 is not allocated or implied by this
plan; any M61 scope requires a separate owner decision outside this plan.

The governed shipped-product root is
`apps/yune-web/public/schema/`, currently the repository's only non-fixture
schema root. The engine-level default remains global. Test/oracle fixtures are
not product onboarding rows. Any future repository-owned product schema root
must register with the same checker before this governance claim can extend to
it; an unregistered root is a hard failure, not implicit coverage.

The live registry gains a machine-readable schema-root inventory. The checker
discovers every tracked `*.schema.yaml`, requires it to match exactly one
registered root, and classifies that root as `product`, `test-fixture`, or
`historical-evidence`. Product roots name their manifest and acceptance
registry. Fixture/evidence roots carry explicit non-product dispositions.
Unmatched or overlapping roots, product schema files outside a registered root,
and symbolic-link entries fail. Synthetic tests must prove each case.

## Runtime Truth To Formalize

The two mechanisms remain distinct:

- `prefix_fallback` is TypeDuck-profile compatibility machinery and is not
  inherited by new schemas;
- `leading_syllable_reachability` is the upstream-shaped, schema-general
  default inherited by every new schema; and
- precedence is **per input**, not schema-wide: `prefix_fallback` is
  authoritative only when that request owns a deployed proper prefix
  (`prefix_fallback_owned`); otherwise the independent leading-syllable path
  remains available.

The last point corrects stale prose in the fork-parity ledger and completed M59
plan. M60 may add a clearly dated documentation correction to those records,
but it must not edit M59 evidence or present the correction as new behavior.

## Requirements

The matching planned requirement IDs are M60-CONTRACT-01, M60-OPTOUT-01,
M60-AUDIT-01, M60-ONBOARD-01, M60-BOUNDARY-01, and M60-EVIDENCE-01.

### 1. Canonical capability contract

- [ ] Add `docs/contracts/schema-general-reachability.md` as the single
      normative description of the translator-level default, explicit-false
      semantics, selection/recomposition scope, and per-input TypeDuck
      precedence.
- [ ] Link it from the engine support contract and the relevant onboarding and
      parity documentation rather than duplicating its rules.
- [ ] State that schema-id branches, input allowlists, promotion tables, baked
      oracle output, circular fixtures, per-schema `true` flags, and silent
      unsupported/N/A onboarding are prohibited.
- [ ] Correct the stale schema-wide precedence wording in
      `docs/ledgers/fork-parity-ledger.md` and add a dated correction to the
      completed M59 plan and D-47 while preserving their historical evidence
      claims.

### 2. Live registry and empty opt-out collection

- [ ] Treat `apps/yune-web/schema-acceptance-coverage.json` as the live
      successor seeded by M59, advance its version to an M60 formalism version,
      and leave the historical M59 packet unchanged.
- [ ] Add the exact-root inventory and tracked-schema discovery contract above;
      no tracked schema YAML may sit outside one classified root.
- [ ] Add `reachabilityOptOuts: []`. No current shipped schema has an approved
      opt-out, so no placeholder or synthetic production row is permitted.
- [ ] Update `mechanismContract` to state the per-input precedence rule and link
      the canonical contract.

A future opt-out row has this exact logical shape; the implementation may add a
schema version but must not weaken any field:

```json
{
  "optOutId": "stable-unique-id",
  "schemaAsset": "exact/path/to/schema.schema.yaml",
  "settingAsset": "exact/path/containing-the-false-setting.yaml",
  "schemaId": "logical_schema_id",
  "configPath": "exact_namespace/leading_syllable_reachability",
  "source": {
    "repository": "https://example.invalid/owner/schema",
    "commit": "40-hex-source-commit"
  },
  "owner": "named-owner",
  "reason": "specific product or oracle reason",
  "affectedSurfaces": ["named-real-path-surface"],
  "evidence": {
    "kind": "owner-spec",
    "path": "tracked/repo-relative/evidence-or-contract-path",
    "sourceCommit": "40-hex-yune-source-commit"
  },
  "acceptanceId": "accepted-real-path-row",
  "approval": {
    "decisionId": "D-nn",
    "approver": "named-owner",
    "approvedOn": "YYYY-MM-DD"
  },
  "review": {
    "triggers": ["specific-revisit-trigger"],
    "reviewBy": "YYYY-MM-DD"
  }
}
```

`schemaAsset` disambiguates repeated logical ids and deployed mirrors;
`settingAsset` identifies the tracked literal setting. An opt-out is a separate
owner decision that narrows D-47. It cannot be inferred from test-only temporary
configuration, generated output, missing evidence, or a dependency-only row.
The only allowed evidence kinds are `oracle` and `owner-spec`.

The checker derives each dictionary-translator namespace from
`engine/translators` exactly as deployment does, including prescriptions such
as `script_translator@foo`, and stores the resulting effective path such as
`foo/leading_syllable_reachability`. It parses effective YAML configuration;
it must recognize nested mappings, slash-path keys, patch/custom overlays, and
quoted or unquoted booleans rather than relying on a literal-text regex.

Production-parser fixtures also cover `__include` inheritance, `__patch`
override precedence, multiple setting-source attribution, and unresolved
include/patch failure. Any RIME merge construct the audit cannot resolve fails
closed; it is never treated as absence of an opt-out.

The reconciliation key is (`settingAsset`, `configPath`, `schemaAsset`). If one
shared carrier setting affects several installable schema assets, it requires
one owner-approved row for each affected `schemaAsset`. A shared/global false
setting whose complete affected-schema set cannot be enumerated is prohibited.

### 3. Mandatory fail-closed checker

- [ ] Extend `apps/yune-web/scripts/check-schema-asset-manifest.mjs`; the static
      audit is mandatory, not optional.
- [ ] Derive every deployed dictionary-translator namespace from the shipped
      schema tree and declared configuration carriers, then scan its effective
      namespaced `leading_syllable_reachability` setting.
- [ ] Require exactly one complete opt-out row for each affected shipped
      (`settingAsset`, `configPath`, `schemaAsset`) false-setting tuple and
      exactly one tracked false tuple for each opt-out row.
- [ ] Resolve `schemaAsset`, `settingAsset`, `schemaId`, `acceptanceId`, evidence
      path, and approval decision against tracked current files and an accepted
      real-path row.
- [ ] Reject duplicate ids or asset/config pairs, missing or malformed fields,
      unknown or unmanifested assets, open/unresolved acceptance rows, orphaned
      records, stale records after asset removal, non-40-hex commits, unsafe or
      missing repo-relative paths, empty surfaces/triggers, invalid dates, and
      expired `reviewBy` dates.
- [ ] Keep the current `reachabilityOptOuts` collection empty and green.
- [ ] Extend `check-schema-asset-manifest.test.mjs` with one fully synthetic
      valid future opt-out and negative cases for every rejection class above.
      Synthetic fixtures prove the format without creating a current opt-out.
      They must call the exported production parser/validator with temporary
      roots and a fixed injected UTC/as-of date; no copied test-only validator
      is accepted. Include namespaced translator, shared-carrier, include,
      patch-precedence, multiple-source, and unresolved-merge cases.

### 4. Fail-closed onboarding and updater

- [ ] Document this sequence: add the schema asset; regenerate the manifest;
      run the updater; classify shipped/runtime/dependency/nonshipped ownership;
      resolve the automatically created blocking `status: open` row; attach
      D-24/D-31-correct oracle or owner-spec provenance; add the narrow deploy-
      path test; then mark the row accepted and run the checker.
- [ ] Preserve the updater's automatic open-row behavior for every new schema
      asset. It must preserve the opt-out collection and never synthesize,
      approve, or suggest an opt-out.
- [ ] Extract an injectable reconciliation helper from the production updater
      and exercise that exact helper from
      `apps/yune-web/scripts/update-schema-asset-manifest.test.mjs`. Prove with
      a temporary tree that onboarding a new schema creates a blocking open row
      and cannot pass as unsupported/N/A.
- [ ] Keep shipped schemas, dependency-only assets, runtime mirrors, canonical
      `rime-cantonese` validation, and mandatory nonshipped validation rows
      explicitly distinct.

### 5. Documentation and closeout

- [ ] Update the support contract, requirements, decisions, roadmap, top-level
      README, milestone history, and parity ledger only for the formalism
      actually delivered.
- [ ] Move this plan to `docs/plans/completed/` only in the final closeout
      commit after implementation, gates, reviews, and compact evidence pass.

This planning-finalization change separately corrected the pre-existing WEB03
requirement-count summary to `11/11` complete. That factual housekeeping is
already done and is excluded from M60 execution and evidence claims.

## Execution Sequence

1. **Kickoff and provenance.** Fetch `origin/main`; record the exact base SHA,
   branch/upstream relation, tracked status, the excluded staged config path,
   timestamps, tool versions, and the fixed oracle/profile pins. Stop if any
   other unexplained change overlaps M60 files.
2. **Contract and enforcement commit.** Add the contract, versioned empty
   registry, checker/updater changes, static tests, and narrowly required doc
   corrections. Commit only explicit M60 paths directly on `main`; no branch or
   registered worktree is planned.
3. **Implementation gates.** Run only the exact behavior/static checks below.
   Write complete raw output to a create-new external root such as
   `$HOME/yune-m60-schema-reachability-formalism/<source-sha>/`.
4. **Assemble the final candidate tree.** Curate the compact packet, update
   current docs, and prepare the plan move to `completed/`. Record the distinct
   implementation commit in the packet. The tracked packet does not attempt to
   contain either its own candidate-tree hash or closeout commit SHA. Build the
   isolated candidate index described below before either review.
5. **Independent review 1 — requirement/evidence compliance.** Verify M59,
   D-47/D-48, runtime code, contract, registry, and evidence boundaries agree.
6. **Independent review 2 — change isolation and checker quality.** Verify the
   bijection, negative cases, path safety, expiry behavior, updater behavior,
   documentation accuracy, final candidate-tree isolation, and absence of
   engine/ABI/performance changes.
7. **Record reviews and run final-tree gates.** Add both review receipts, then
   run the link, packet-manifest, evidence-growth, and diff checks against the
   complete isolated candidate. Preserve failures and rerun only the affected
   slice under an explicit retry name.
8. **Closeout commit and push.** Commit only M60 paths, prove the resulting
   `HEAD^{tree}` equals the reviewed candidate tree, push `main`, re-run the
   range/final-tree checks against the committed closeout, and verify remote
   source identity. Record the containing closeout SHA only in an external
   post-push receipt and the handoff; do not create a self-referential follow-up
   commit. Confirm the unrelated staged config remains staged and uncommitted.

## Load-Bearing Verification

```sh
npm --prefix apps/yune-web run check:schema-manifest
node --test apps/yune-web/scripts/check-schema-asset-manifest.test.mjs
node --test apps/yune-web/scripts/update-schema-asset-manifest.test.mjs
cargo test -p yune-rime-api --test yune_web m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false
cargo test -p yune-rime-api --test yune_web m59_manifest_plain_jyut6ping3_real_deploy_default_on_and_explicit_false
python3 -m unittest scripts/tests/test_current_doc_links.py scripts/tests/test_packet_manifest.py
```

The utility tests reject missing/unsafe/traversing/symbolic-link local Markdown
targets and missing touched-path inputs. Packet tests enforce exact
bidirectional membership and reject duplicate/missing rows, traversal or
symbolic links, and byte-size or SHA-256 mismatches.

Create `$OUT/m60-paths.txt` as the explicit M60 commit allowlist. Build the
candidate index from `HEAD`, not from the real index that already stages the
user-owned config. Build it before both reviews, then rebuild it after adding
the review receipts and before the final-tree checks:

```sh
rm -f "$OUT/m60.index"
GIT_INDEX_FILE="$OUT/m60.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m60.index" git add -A --pathspec-from-file="$OUT/m60-paths.txt"
GIT_INDEX_FILE="$OUT/m60.index" git write-tree > "$OUT/m60-candidate-tree.txt"
```

M60 adds and owns two small repository-wide documentation/evidence utilities so
the final checks are reproducible rather than ad hoc. Only after the isolated
index has been rebuilt for the complete packet and review receipts, run:

```sh
python3 scripts/check-current-doc-links.py --paths-from "$OUT/touched-current-docs.txt"
python3 scripts/verify-packet-manifest.py docs/reports/evidence/m60-schema-general-reachability-formalism/packet-manifest.csv
python3 scripts/check-evidence-growth.py --repo-root . --paths-from "$OUT/m60-evidence-paths.txt"
GIT_INDEX_FILE="$OUT/m60.index" git diff --cached --check
```

Use this isolated index for both reviews and the candidate diff check. Commit
the same allowlisted paths with `git commit --only`, then require
`git rev-parse HEAD^{tree}` to equal `m60-candidate-tree.txt`. This proves the
reviewed tree is the committed M60 tree while leaving `.codex/config.toml`
staged in the real index.

After the closeout commit, repeat the last three applicable checks against the
committed tree using `--compare-base <m60-base> --treeish HEAD` for the evidence
guard, verify the packet manifest from `HEAD`, and run
`git diff --check <m60-base>..HEAD`. Zero broken current-document links, packet
hash mismatches, evidence-policy violations, or whitespace errors are accepted.

The two narrow Rust tests are load-bearing because they prevent a paper-only
formalism from drifting away from the real default-on/explicit-false deploy
path. Do not run M59 performance, full Rust, WASM, browser, Cloudflare,
packaging, memory, or latency suites. If M60 unexpectedly requires production
Rust, browser, ABI, schema payload, or threshold changes, stop and request a
new scope decision rather than expanding this milestone.

## Evidence Contract

Keep raw attempts outside the repository. Curate only text receipts under
`docs/reports/evidence/m60-schema-general-reachability-formalism/`:

- `README.md` with verdict and exact source boundary;
- `provenance.txt` and `commands.md`;
- schema-manifest checker and Node negative-test summaries;
- the two narrow deploy-path test summaries;
- onboarding/open-row test, link-audit, evidence-growth, and diff-check output;
- requirement/evidence review and change-isolation/checker review;
- `packet-manifest.csv` with byte size and SHA-256 for every packet file except
  itself.

The external packet additionally retains the M60-only candidate-tree hash and
the post-push receipt naming the containing closeout SHA and proving remote
equality. Neither is inserted into the tracked packet, which avoids impossible
self-reference.

No raw metrics, binaries, compiled schemas, WASM, screenshots, benchmark data,
or copied M59 artifacts belong in this packet. The packet must remain below the
repository's 10 MiB curated-evidence cap.

## Failure And Retry Policy

- A setup failure before a gate completes may be corrected and retried under an
  explicit `retry-N-<reason>` directory.
- A completed red checker or test run is preserved externally with its source,
  command, and disposition. After a fix, rerun only the affected owning check;
  do not erase or rename the red as a setup failure.
- A discovered runtime behavior, candidate-order, performance, ABI, browser, or
  packaging discrepancy stops M60 for diagnosis. It is not repaired or waived
  as documentation work.
- No M59 threshold, signed ceiling, oracle fixture, accepted candidate list,
  or historical evidence is changed or re-baselined.
- An incomplete, expired, orphaned, or unsupported/N/A opt-out row is a hard
  failure. There is no waiver inside M60.

## Non-Goals

- No reachability, ranking, selection, recomposition, schema-install, or worker
  behavior change.
- No new schema, product profile, schema-id rename, userdb migration, C ABI, UI,
  Cloudflare configuration, or Windows product work.
- No schema-specific `true` flag or adapter.
- No current opt-out and no pre-approval of a future opt-out.
- No new performance claim, ceiling, benchmark, exception, or M59 reproof.
- No unification of `prefix_fallback` and
  `leading_syllable_reachability`.
- No M61 scope or schedule.
