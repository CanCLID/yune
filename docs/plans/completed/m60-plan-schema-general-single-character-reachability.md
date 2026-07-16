# M60 Schema-General Reachability Capability Formalism

> **Milestone:** M60. **Status:** Complete (2026-07-15).
> **Track:** documentation and fail-closed static governance. **Created:**
> 2026-07-06; **rescoped:** 2026-07-07; **finalized:** 2026-07-15;
> **review-corrected:** 2026-07-15.

## Outcome

M60 turns M59's already-shipped schema-general reachability guarantee into a
durable capability contract. It makes future schema onboarding and any explicit
opt-out auditable and fail closed without changing engine behavior.

M60 closes when all six conditions hold:

1. one canonical contract describes the default-on invariant and the actual
   per-input relationship between the two reachability mechanisms;
2. the M59-founded acceptance coverage remains format-compatible with its M59
   gates and gains a separately versioned M60 formalism plus an empty current
   opt-out collection;
3. the manifest checker reconciles every affected shipped namespaced
   explicit-false tuple to a complete owner-approved opt-out record;
4. every tracked schema YAML belongs to a machine-classified root, and the
   updater creates a blocking open row for each new product schema asset without
   creating or implying an opt-out;
5. schema onboarding is documented from asset discovery through real-path
   acceptance; and
6. compact, source-bound evidence and two independent reviews close the
   milestone without reopening M59 or WEB03-11.

## Completion Record

M60 began from `b8cd897f9d6c3158d864bac9d2629482c45c7427` and was delivered
through the forward-only implementation sequence `9b55b9ba`, `c1f1f941`,
`78a9e38a`, and review fixes `e352fba4` and `c9b34774`. A concurrent writer
committed and pushed `9b55b9ba` before the
prescribed isolated pre-commit implementation tree was recorded. That proof
does not exist and was not backfilled. The user authorized forward-only
completion after disclosure. External incident and recovery receipts preserve
the deviation; all later commits have isolated-tree proofs, and the combined
implementation range remains exactly within the frozen 34-path M60 envelope.

The eight named gates passed. Two checker-launch setup failures and one
unsupported `git ls-files` preflight were preserved and retried narrowly. The
utility gate first completed red because Windows 8.3 and long user-profile paths
were compared lexically; `78a9e38a` fixed that packet-verifier defect, and only
the utility slice was rerun. The first isolation review then rejected preserved
tree `057656af` because the Python validators did not separately detect Windows
directory junctions. That tree and both discarded review results remain
external. `e352fba4` added junction checks plus both owning cases; the exact
utility slice then ran 16 tests with `OK`, both junction cases passed, and the
same two privilege-gated symbolic-link cases were skipped. A repeated isolation
review rejected tree `75ab7b95` because that implementation silently weakened
on Python before 3.12. `c9b34774` replaced the API dependency with Windows
reparse-tag detection that fails closed when metadata is unavailable and removed
the older-Python test skip. The same 16-test utility slice passed again with only
the two privilege-gated symbolic-link skips. The candidate was rebuilt and both
reviews were repeated. No M59 or
WEB03 evidence, runtime engine behavior, C ABI/API table/export, schema/profile
id or payload, browser UI, Windows product, ranking, selection, recomposition,
schema installation, performance threshold, benchmark, memory, or M61 behavior
changed.

The preserved pre-review tree, two independent reviews, exact three-path
post-review delta, final candidate tree, and closeout commit-tree equality are
the binding final proofs. The containing closeout SHA is recorded only in the
external post-push receipt to avoid self-reference.

## Authority And Boundaries

M59 owns the behavior. In particular:

- D-47 and M59-REACH-02 establish the conventional
  `translator/leading_syllable_reachability` default. The deployed
  `engine/translators` prescription code generalizes that rule to each
  dictionary-translator namespace, such as `script_translator@foo` → `foo`,
  unless its effective tracked setting explicitly says `false`;
- D-48 owns the existing candidate-order lanes and exceptions;
- final native behavior and performance authority remains `443cc636`;
- shipped registry/browser/package evidence remains bound to `5fa986d8`, with
  bidirectional manifest exactness added at `07845e02`; and
- historical M59 evidence is immutable and is linked, not rewritten.

The fetched M60 kickoff base is
`b8cd897f9d6c3158d864bac9d2629482c45c7427`; the WEB03-11 measured product
source remains `ef485b10`. M60 neither reruns nor supersedes that browser
evidence. At kickoff, local `main` and `origin/main` matched, the worktree and
real Git index were empty, and `.codex/config.toml` was clean. The two protected
UI paths named by the handoff were also clean; their kickoff SHA-256 values are
recorded externally, and they remain outside every M60 path list, candidate,
and commit. The real index must remain empty outside temporary, explicitly
path-limited M60 intent-to-add and commit operations.

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
`apps/yune-web/public/schema/`, currently the repository's only registered
product schema root. The engine-level default remains global. Test/oracle
fixtures and historical evidence are separate classified roots, not product
onboarding rows. Any future repository-owned product schema root must register
with the same checker before this governance claim can extend to it; an
unregistered root is a hard failure, not implicit coverage.

The live registry gains a machine-readable schema-root inventory. The
production checker discovers tracked schema files from
`git ls-files -z -- '*.schema.yaml'`, requires each path to match exactly one
registered root, and classifies that root as `product`, `test-fixture`, or
`historical-evidence`. It does not substitute a filesystem walk that could
silently include ignored files or miss tracked deletions. Product roots name
their manifest and acceptance registry. Fixture/evidence roots carry explicit
non-product dispositions. Unmatched or overlapping roots, product schema files
outside a registered root, symbolic-link entries, and execution outside a Git
worktree fail. Synthetic tests use an explicitly injected tracked-path list and
must prove each case.

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

The last point corrects stale prose in D-47, the fork-parity ledger, and the
completed M59 plan. M60 may add a clearly dated documentation correction to
those records, but it must not edit M59 evidence or present the correction as
new behavior.

## Requirements

The matching planned requirement IDs are M60-CONTRACT-01, M60-OPTOUT-01,
M60-AUDIT-01, M60-ONBOARD-01, M60-BOUNDARY-01, and M60-EVIDENCE-01.

### 1. Canonical capability contract

- [x] Add `docs/contracts/schema-general-reachability.md` as the single
      normative description of the translator-level default, explicit-false
      semantics, selection/recomposition scope, and per-input TypeDuck
      precedence.
- [x] Link it from the engine support contract and the relevant onboarding and
      parity documentation rather than duplicating its rules.
- [x] State that schema-id branches, input allowlists, promotion tables, baked
      oracle output, circular fixtures, per-schema `true` flags, and silent
      unsupported/N/A onboarding are prohibited.
- [x] Include an authority table that keeps final M59 native behavior and
      performance at `443cc636`, shipped registry/browser/package evidence at
      `5fa986d8` plus `07845e02`, and WEB03-11 measurement at `ef485b10`
      source-bound rather than projecting any lane onto M60.
- [x] State the exact covered translator arms and add a narrow regression for a
      prefix-fallback-enabled request that owns no proper prefix, so the
      corrected per-input prose is pinned to the real leading-single path. The
      upstream-table lazy bounded arm at `translator/mod.rs:5094` must be
      classified explicitly rather than silently generalized.
- [x] Correct the stale schema-wide precedence wording in
      `docs/ledgers/fork-parity-ledger.md` and add a dated correction to the
      completed M59 plan and D-47 while preserving their historical evidence
      claims.

### 2. Live registry and empty opt-out collection

- [x] Treat `apps/yune-web/schema-acceptance-coverage.json` as the live
      successor seeded by M59. Retain `version: m59-reach03-v1` because the M59
      checker, updater, and load-bearing Rust gate bind that coverage format;
      add and validate a separate
      `reachabilityFormalismVersion: m60-reachability-v1`. Leave the historical
      M59 packet unchanged.
- [x] Add the exact-root inventory and tracked-schema discovery contract above;
      no tracked schema YAML may sit outside one classified root.
- [x] Add `reachabilityOptOuts: []`. No current shipped schema has an approved
      opt-out, so no placeholder or synthetic production row is permitted.
- [x] Update `mechanismContract` to state the per-input precedence rule and link
      the canonical contract.

A future opt-out row has this exact logical shape. Any later schema-format
version is additive and must not weaken a field:

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

### Production-semantic audit architecture

The JavaScript checker must not grow an independent RIME configuration
compiler. M60 adds a narrow, read-only Rust audit path that reuses the
production deployment/config-compiler semantics:

- add an internal audit module plus the tooling-only binary
  `crates/yune-rime-api/src/bin/yune-schema-reachability-audit.rs`;
- any library entry point needed by that binary is `#[doc(hidden)]`, has no
  `extern "C"`/`no_mangle` export, and is not added to either Rime API table;
- reuse the production include, patch, custom-overlay, nested/slash-key, and
  translator-prescription resolution rather than copying those rules into the
  binary or JavaScript;
- instrument that same resolution to emit deterministic JSON tuples containing
  `schemaAsset`, `schemaId`, translator component, namespace, `configPath`,
  effective boolean, `settingAsset` (`null` only for an inherited default with
  no literal setting), ordered directive/source trace, and hashes of every
  contributing tracked asset; and
- fail when a directive inside the dependency closure of
  `engine/translators` or a derived
  `<namespace>/leading_syllable_reachability` path is unresolved or unsupported.
  Unrelated configuration branches need not become a full compiler target.

`apps/yune-web/scripts/check-schema-asset-manifest.mjs` invokes this binary
through a fixed `cargo run --locked -q -p yune-rime-api --release --bin
yune-schema-reachability-audit -- ...` command, reusing the product release
build profile, and consumes its stdout. The production entry point rejects a
missing tool, nonzero exit, malformed output, asset-hash mismatch, or
caller-supplied replacement JSON. JavaScript owns the registry/root/path/
bijection policy; Rust owns effective deployment semantics and setting-source
attribution. This is an explicitly allowed tooling-only Rust touchpoint, not an
engine-behavior or C-ABI change.

The audit recognizes exactly the booleans deployment accepts: YAML booleans and
quoted `"true"`/`"false"` strings case-insensitively, including `"TRUE"` and
`"False"`. It must not silently invent broader `yes`/`no`/`on`/`off` semantics.

A shared synthetic fixture tree and declarative expected tuples cover
`script_translator@foo`, root and nested `__include`, mapping/string/sequence
`__patch`, custom-overlay precedence, optional references, multiple setting
sources, case-insensitive quoted false, and unresolved references. The expected
tuples are authored from the fixture source, not generated by either system
under test. A new narrow Rust gate drives the real deploy/task path over that
tree and compares runtime-effective behavior plus the audit trace to those
tuples; the Node test consumes the same Rust audit output. No JavaScript-authored
merge result is allowed to certify JavaScript/Rust equivalence.

The reconciliation key is (`settingAsset`, `configPath`, `schemaAsset`). If one
shared carrier setting affects several installable schema assets, it requires
one owner-approved row for each affected `schemaAsset`. The complete affected
set is the finite set of accepted product-schema rows whose Rust-emitted source
trace contains that `settingAsset`/`configPath`. Every deployed dictionary-
translator prescription in every product schema must resolve to exactly one
audit tuple keyed by schema asset, component, and namespace. A shared/global
false setting is prohibited when that finite affected-schema set cannot be
derived from the registered roots, carriers, and resolved traces.

### 3. Mandatory fail-closed checker

- [x] Extend `apps/yune-web/scripts/check-schema-asset-manifest.mjs`; the static
      audit is mandatory, not optional. Refactor the current import-side-effect
      script into an exported validator plus a thin production entry point; the
      entry point must always run the Rust extractor and the validator.
- [x] Consume every Rust-derived deployed dictionary-translator namespace from
      the shipped schema tree and declared configuration carriers, then scan its
      effective namespaced `leading_syllable_reachability` setting.
- [x] Require exactly one complete opt-out row for each affected shipped
      (`settingAsset`, `configPath`, `schemaAsset`) false-setting tuple and
      exactly one tracked false tuple for each opt-out row.
- [x] Resolve `schemaAsset`, `settingAsset`, `schemaId`, `acceptanceId`, evidence
      path, and approval decision against tracked current files and an accepted
      real-path row.
- [x] Reject duplicate ids or exact reconciliation-key triplets, missing or
      malformed fields, unknown or unmanifested assets, open/unresolved
      acceptance rows, orphaned records, stale records after asset removal,
      non-40-hex commits, unsafe or missing repo-relative paths, empty
      surfaces/triggers, invalid dates, and expired `reviewBy` dates.
- [x] Keep the current `reachabilityOptOuts` collection empty and green.
- [x] Extend `check-schema-asset-manifest.test.mjs` with one fully synthetic
      valid future opt-out and negative cases for every rejection class above.
      Synthetic fixtures prove the format without creating a current opt-out.
      They must call the exported production validator with temporary roots, an
      injected tracked-path list, the real Rust extractor, and a fixed UTC/as-of
      date; no copied test-only validator or merge implementation is accepted.
      The thin production entry point must call that same validator with
      `git ls-files` discovery and the real UTC clock. Include explicit-false
      with no opt-out row, opt-out with no false tuple, namespaced translator,
      shared-carrier, include, patch-precedence, multiple-source,
      case-insensitive-string, unresolved-merge, and expired-row cases.

### 4. Fail-closed onboarding and updater

- [x] Add the normative `## Schema onboarding` procedure to
      `docs/contracts/schema-general-reachability.md` and link it from
      `docs/conventions.md`. Document this sequence: add the schema asset;
      regenerate the manifest;
      run the updater; classify shipped/runtime/dependency/nonshipped ownership;
      resolve the automatically created blocking `status: open` row; attach
      D-24/D-31-correct oracle or owner-spec provenance; add the narrow deploy-
      path test; then mark the row accepted and run the checker.
- [x] Preserve the updater's automatic open-row behavior for every new schema
      asset. It must preserve the M59 coverage version, M60 formalism version,
      root inventory, and opt-out collection, and never synthesize, approve, or
      suggest an opt-out.
- [x] Extract an injectable reconciliation helper from the production updater
      and require the production updater entry point to call it. Exercise that
      exact helper and the end-to-end production entry point from
      `apps/yune-web/scripts/update-schema-asset-manifest.test.mjs`. Prove with
      a temporary tree that onboarding a new schema creates a blocking open row
      and cannot pass as unsupported/N/A, while an existing formalism/root/
      opt-out block survives byte-for-byte except for deliberate canonical JSON
      formatting.
- [x] Keep shipped schemas, dependency-only assets, runtime mirrors, canonical
      `rime-cantonese` validation, and mandatory nonshipped validation rows
      explicitly distinct.

### 5. Documentation and closeout

- [x] Update the support contract, requirements, decisions, roadmap, top-level
      README, milestone history, and parity ledger only for the formalism
      actually delivered.
- [x] Add `scripts/check-current-doc-links.py` and
      `scripts/tests/test_current_doc_links.py`; the production utility must
      reject missing, escaping, or symbolic-link local targets and consume an
      explicit touched-current-doc list.
- [x] Add `scripts/verify-packet-manifest.py` and
      `scripts/tests/test_packet_manifest.py`; enforce exact bidirectional
      packet membership, byte sizes, SHA-256 values, safe paths, and no
      symbolic links.
- [x] Move this plan to `docs/plans/completed/` only in the final closeout
      commit after implementation, gates, reviews, and compact evidence pass.

This planning-finalization series separately corrected the pre-existing WEB03
requirement status to `11/11` complete and reconciled the three WEB-02 plus
eleven WEB-03 traceability rows. That factual housekeeping is excluded from M60
execution and evidence claims.

## Execution Sequence

1. **Kickoff and provenance.** Fetch `origin/main`; record the exact base SHA,
   branch/upstream relation, tracked status, empty real index, protected UI
   hashes, timestamps, tool versions, and the fixed oracle/profile pins. Stop
   if any unexplained change overlaps M60 files.
2. **Contract and enforcement commit.** Add the contract, separate formalism
   version, empty registry collection, Rust audit tool, checker/updater changes,
   static and narrow real-path tests, and required doc corrections. Tooling-only
   Rust code and tests are explicitly in scope; runtime behavior, C ABI, API
   tables, exports, schemas, and thresholds are not. Use the isolated-tree,
   intent-to-add, and path-limited procedure below for this commit as well as
   closeout. Commit only explicit M60 paths directly on `main`; no branch or
   registered worktree is planned.
3. **Implementation gates.** Run only the exact behavior/static checks below.
   Write complete raw output to a create-new external root such as
   `$HOME/yune-m60-schema-reachability-formalism/<source-sha>/`.
4. **Assemble the pre-review candidate tree.** Curate the compact packet with a
   pre-review manifest covering every then-present packet file but without the
   two not-yet-created review receipts, update current docs, and prepare the plan
   move to `completed/`. Record the distinct implementation commit in the
   packet. Build the isolated candidate index described below and preserve its
   tree as `m60-pre-review-tree.txt`; never overwrite it with a later hash.
5. **Independent review 1 — requirement/evidence compliance.** Verify M59,
   D-47/D-48, runtime code, contract, registry, and evidence boundaries agree.
6. **Independent review 2 — change isolation and checker quality.** Verify the
   bijection, negative cases, path safety, expiry behavior, updater behavior,
   documentation accuracy, pre-review candidate isolation, and absence of
   runtime/ABI/performance changes.
7. **Record reviews and build the final candidate.** Each receipt names the
   preserved pre-review tree. Add only
   `review-requirements.md` and `review-isolation.md`, regenerate
   `packet-manifest.csv`, and rebuild the isolated index as
   `m60-final-candidate-tree.txt`. Prove the complete pre-review-to-final tree
   diff contains exactly those three paths; any other change invalidates the
   reviews and returns to step 4. Then run the link, packet-manifest,
   evidence-growth, and diff checks against the final candidate. Preserve
   failures and rerun only the affected slice under an explicit retry name.
8. **Closeout commit and push.** Commit only M60 paths, prove the resulting
   `HEAD^{tree}` equals the final candidate tree, push `main`, re-run the
   range/final-tree checks against the committed closeout, and verify remote
   source identity. The reviews bind the pre-review tree; the exact three-path
   delta proof binds it to the final tree. Record the containing closeout SHA
   only in an external post-push receipt and the handoff; do not create a
   self-referential follow-up commit. Confirm the real index is empty and both
   protected UI files remain byte-identical to kickoff and unstaged.

## Load-Bearing Verification

```sh
npm --prefix apps/yune-web run check:schema-manifest
node --test apps/yune-web/scripts/check-schema-asset-manifest.test.mjs
node --test apps/yune-web/scripts/update-schema-asset-manifest.test.mjs
cargo test -p yune-rime-api --test yune_web m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false
cargo test -p yune-rime-api --test yune_web m59_manifest_plain_jyut6ping3_real_deploy_default_on_and_explicit_false
cargo test -p yune-rime-api --test yune_web m60_namespaced_reachability_audit_matches_real_deploy
cargo test -p yune-core prefix_fallback_without_owned_prefix_keeps_leading_syllable_reachability
python3 -m unittest scripts/tests/test_current_doc_links.py scripts/tests/test_packet_manifest.py
```

The utility tests reject missing/unsafe/traversing/symbolic-link-or-junction local Markdown
targets and missing touched-path inputs. Packet tests enforce exact
bidirectional membership and reject duplicate/missing rows, traversal or
symbolic links or junctions, and byte-size or SHA-256 mismatches.

Before implementation, create and retain these newline-delimited external
lists in `LC_ALL=C` sorted, unique order; empty, duplicate, absolute, escaping,
unmatched, or stale entries fail:

- `$OUT/m60-implementation-paths.txt`: every path in the implementation commit;
- `$OUT/m60-implementation-new-paths.txt`: its paths not known to Git at the
  implementation base;
- `$OUT/m60-paths.txt`: every final M60 path, including deletions, new files,
  and the two review receipts;
- `$OUT/m60-pre-review-paths.txt`: the final list minus the two not-yet-created
  review receipts; the pre-review `packet-manifest.csv` remains included;
- `$OUT/m60-new-paths.txt`: the subset not known to Git at the closeout base;
- `$OUT/touched-current-docs.txt`: current Markdown files from the allowlist,
  excluding deleted/historical-only files;
- `$OUT/m60-evidence-paths.txt`: every curated M60 packet path; and
- `$OUT/post-review-allowed-paths.txt`: exactly
  `packet-manifest.csv`, `review-isolation.md`, and
  `review-requirements.md` under the M60 evidence root.

Before the implementation commit, build an isolated candidate from its two
implementation lists, write `m60-implementation-tree.txt`, add intent-to-add
entries only for its new-path subset, commit with `git commit --only
--pathspec-from-file`, require `HEAD^{tree}` to equal the preserved tree, and
require the real index to remain empty. The detailed
closeout commands below are normative for that operation with the corresponding
implementation filenames substituted; do not use a less isolated first-commit
shortcut.

```sh
rm -f "$OUT/m60-implementation.index"
GIT_INDEX_FILE="$OUT/m60-implementation.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m60-implementation.index" git add -A \
  --pathspec-from-file="$OUT/m60-implementation-paths.txt"
GIT_INDEX_FILE="$OUT/m60-implementation.index" git write-tree \
  > "$OUT/m60-implementation-tree.txt"
test -z "$(git diff --cached --name-only)"
LC_ALL=C sort -u "$OUT/m60-implementation-paths.txt" \
  > "$OUT/m60-implementation-paths.sorted"
LC_ALL=C sort -u "$OUT/m60-implementation-new-paths.txt" \
  > "$OUT/m60-implementation-new-paths.sorted"
cmp "$OUT/m60-implementation-paths.txt" \
  "$OUT/m60-implementation-paths.sorted"
cmp "$OUT/m60-implementation-new-paths.txt" \
  "$OUT/m60-implementation-new-paths.sorted"
comm -23 "$OUT/m60-implementation-new-paths.sorted" \
  "$OUT/m60-implementation-paths.sorted" \
  > "$OUT/m60-implementation-new-paths-not-allowed.txt"
test ! -s "$OUT/m60-implementation-new-paths-not-allowed.txt"
test -z "$(git diff --name-only --diff-filter=A)"
git add --intent-to-add \
  --pathspec-from-file="$OUT/m60-implementation-new-paths.txt"
git diff --name-only --diff-filter=A | LC_ALL=C sort \
  > "$OUT/m60-implementation-intent-actual.txt"
cmp "$OUT/m60-implementation-new-paths.sorted" \
  "$OUT/m60-implementation-intent-actual.txt"
git commit --only \
  --pathspec-from-file="$OUT/m60-implementation-paths.txt" \
  -m "Implement M60 reachability formalism"
test "$(git rev-parse HEAD^{tree})" = "$(cat "$OUT/m60-implementation-tree.txt")"
test -z "$(git diff --cached --name-only)"
test -z "$(git diff --name-only --diff-filter=A)"
```

Build the candidate index from `HEAD`, never from the real index. Before both
reviews, preserve the first tree:

```sh
rm -f "$OUT/m60.index"
GIT_INDEX_FILE="$OUT/m60.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m60.index" git add -A --pathspec-from-file="$OUT/m60-pre-review-paths.txt"
GIT_INDEX_FILE="$OUT/m60.index" git write-tree > "$OUT/m60-pre-review-tree.txt"
```

After the two receipts are copied from the external review output, regenerate
`packet-manifest.csv` so its exact membership includes them, rebuild the index,
and prove the only tree changes since review are the two receipts and manifest:

```sh
rm -f "$OUT/m60.index"
GIT_INDEX_FILE="$OUT/m60.index" git read-tree HEAD
GIT_INDEX_FILE="$OUT/m60.index" git add -A --pathspec-from-file="$OUT/m60-paths.txt"
GIT_INDEX_FILE="$OUT/m60.index" git write-tree > "$OUT/m60-final-candidate-tree.txt"
git diff-tree --no-commit-id --name-only -r \
  "$(cat "$OUT/m60-pre-review-tree.txt")" \
  "$(cat "$OUT/m60-final-candidate-tree.txt")" \
  | LC_ALL=C sort > "$OUT/post-review-actual-paths.txt"
LC_ALL=C sort "$OUT/post-review-allowed-paths.txt" > "$OUT/post-review-expected-paths.txt"
cmp "$OUT/post-review-expected-paths.txt" "$OUT/post-review-actual-paths.txt"
```

Only after that proof, run the final checks:

```sh
python3 scripts/check-current-doc-links.py --paths-from "$OUT/touched-current-docs.txt"
python3 scripts/verify-packet-manifest.py docs/reports/evidence/m60-schema-general-reachability-formalism/packet-manifest.csv
python3 scripts/check-evidence-growth.py --repo-root . --paths-from "$OUT/m60-evidence-paths.txt"
GIT_INDEX_FILE="$OUT/m60.index" git diff --cached --check
```

`git commit --only` cannot name a path that is untracked in the real index even
when it exists in the isolated candidate. Immediately before closeout, confirm
the real index is empty, then add intent-to-add entries only for
`$OUT/m60-new-paths.txt` and perform the path-limited commit:

```sh
test -z "$(git diff --cached --name-only)"
LC_ALL=C sort -u "$OUT/m60-paths.txt" > "$OUT/m60-paths.sorted"
LC_ALL=C sort -u "$OUT/m60-new-paths.txt" > "$OUT/m60-new-paths.sorted"
cmp "$OUT/m60-paths.txt" "$OUT/m60-paths.sorted"
cmp "$OUT/m60-new-paths.txt" "$OUT/m60-new-paths.sorted"
comm -23 "$OUT/m60-new-paths.sorted" "$OUT/m60-paths.sorted" \
  > "$OUT/m60-new-paths-not-allowed.txt"
test ! -s "$OUT/m60-new-paths-not-allowed.txt"
test -z "$(git diff --name-only --diff-filter=A)"
git add --intent-to-add --pathspec-from-file="$OUT/m60-new-paths.txt"
git diff --name-only --diff-filter=A | LC_ALL=C sort \
  > "$OUT/m60-intent-actual.txt"
cmp "$OUT/m60-new-paths.sorted" "$OUT/m60-intent-actual.txt"
git commit --only --pathspec-from-file="$OUT/m60-paths.txt" -m "Close M60 reachability formalism"
test "$(git rev-parse HEAD^{tree})" = "$(cat "$OUT/m60-final-candidate-tree.txt")"
test -z "$(git diff --cached --name-only)"
test -z "$(git diff --name-only --diff-filter=A)"
```

The pre-review tree, final tree, exact three-path delta, and commit-tree equality
are distinct proofs. Do not call the final tree itself reviewed.

After the closeout commit, repeat the last three applicable checks against the
committed tree using `--compare-base <m60-base> --treeish HEAD` for the evidence
guard, verify the packet manifest from `HEAD`, and run
`git diff --check <m60-base>..HEAD`. Zero broken current-document links, packet
hash mismatches, evidence-policy violations, or whitespace errors are accepted.

The four narrow Rust tests are load-bearing because they prevent a paper-only
formalism from drifting away from the real default-on/explicit-false deploy
path, bind the Rust audit output to deployment, and lock the corrected per-input
precedence. The planned Rust audit module, tooling binary, and focused tests are
in scope. Do not run M59 performance, full Rust, WASM, browser, Cloudflare,
packaging, memory, or latency suites. If M60 requires a runtime behavior change,
C ABI/API-table/export change, schema payload change, or threshold change, stop
and request a new scope decision rather than expanding this milestone.

## Evidence Contract

Keep raw attempts outside the repository. Curate only text receipts under
`docs/reports/evidence/m60-schema-general-reachability-formalism/`:

- `README.md` with verdict and exact source boundary;
- `provenance.txt` and `commands.md`;
- schema-manifest checker, Rust extractor, and Node negative-test summaries;
- the four narrow Rust test summaries;
- onboarding/open-row test, link-audit, evidence-growth, and diff-check output;
- `review-requirements.md` and `review-isolation.md`, each naming the preserved
  pre-review tree;
- `packet-manifest.csv` with byte size and SHA-256 for every packet file except
  itself.

The external packet additionally retains the distinct pre-review and final
candidate-tree hashes, the exact three-path delta proof, and the post-push
receipt naming the containing closeout SHA and proving remote equality. None is
inserted into the tracked packet, which avoids impossible self-reference.

No raw metrics, binaries, compiled schemas, WASM, screenshots, benchmark data,
or copied M59 artifacts belong in this packet. The packet must remain below the
repository's 10 MiB curated-evidence cap.

## Failure And Retry Policy

- A setup failure before a gate completes may be corrected and retried under an
  explicit `retry-N-<reason>` directory.
- An allowlist path that is stale, unmatched, outside the repository, or absent
  from the expected new-path subset is a setup failure before candidate
  measurement. Preserve the failed command and retry explicitly after fixing
  the list; never weaken path matching.
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
