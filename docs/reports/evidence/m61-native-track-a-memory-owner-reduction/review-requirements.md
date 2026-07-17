# M61 Requirement And Evidence Review

Reviewer: independent requirement/evidence review  
Review date: 2026-07-16  
Verdict: **PASS — approved with no actionable findings**

## Frozen Review Subject

This review is bound to pre-review candidate tree
`a15303d5ecbf3eda73f31911aa9870e30356172a`.

The independently reconciled provenance chain is:

- measured correction commit
  `91f5969688a3d2dba96a67d1cfe813c7ba4ee861`, tree
  `6626ed16d5e135fa477ca26e9786d11121c92b44`;
- explicit revert and M61 base commit
  `01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10`, tree
  `f1c36a0079d85628f5cbef140bd94288930cc2e8`;
- frozen pre-review candidate tree
  `a15303d5ecbf3eda73f31911aa9870e30356172a`.

The correction changed exactly the three declared POET implementation/test
paths. The explicit revert restores the production tree exactly to the pushed
pre-correction quality-repair tree. The base-to-candidate no-rename path list
contains the same 52 paths as the preserved external path proof. The isolated
pre-review index writes the frozen candidate tree exactly, the real index is
empty, and `git diff --check` is clean.

## Requirement Findings

### Measurement and mandatory disposition

The source-bound raw records reproduce the following medians and worst rounds:

| Metric | Owned mode | Byte-backed mode |
| --- | ---: | ---: |
| private bytes, median | 108,482,560 | 83,386,368 |
| private bytes, worst | 108,978,176 | 84,475,904 |
| peak working set, median | 154,030,080 | 116,162,560 |
| peak working set, worst | 154,218,496 | 116,334,592 |
| eligible leaf bytes, median | 18,743,072 | 18,830 |

Independent arithmetic gives:

- private-byte delta: `25,096,192`;
- explained eligible-owner delta: `18,724,242`;
- attribution coverage: `0.746098930`;
- residual: `6,371,950` bytes;
- 20% residual bound: `5,019,238` bytes.

Coverage is below the required `0.80`, and residual exceeds both the fixed
5,000,000-byte bound and the 20% bound. The plan therefore requires disposition
D. The packet, D-50, roadmap, requirements, milestone history, and performance
dashboard consistently record that measured no-go, the explicit revert, and
the absence of a production-default memory claim.

### Correctness and source binding

The production-semantics aggregate evidence independently reproduces, for each
of the owned and byte-backed five-round modes, `32/32` aggregate rows and
`160/160` individual observations with no failures. The independently rerun
candidate-parity comparator reproduces `17/17`. Each diagnostic receipt binds
to measured source commit `91f5969688a3d2dba96a67d1cfe813c7ba4ee861`,
source tree `6626ed16d5e135fa477ca26e9786d11121c92b44`, and the declared DLL,
benchmark, model, and fixture hashes.

Eligible owner IDs are stable within each five-round mode with no duplicate
IDs. Every byte-backed round contains exactly the four declared `poet.*`
mapped owners classified as `poet_bin:byte_backed:mmap`, and no fifth owner.
The supplemental projection passes its diagnostic gate, but no supplemental
threshold is accepted or added to the signed registry. The signed registry is
unchanged and retains SHA-256
`e74e77b4dd5b253e0c2b5f4b12cc1e0279784d3c3fbf02006b5f8f18fccacdba`.

### Failure preservation and disjoint recovery

The exact broad runner preserves gates 1 through 19 as green and preserves the
literal `cargo test --workspace` gate as red. Its failing target recorded
`37/41` passing tests and four stale bounded-page comparisons. Retry 1 was
interrupted and correctly contributes no verdict. The named disjoint retries
then establish:

- the corrected Cantonese parity target: `41/41`;
- all other core integration targets: `69` passed, `8` ignored;
- the corrected API library target: `364` passed, `1` ignored;
- remaining API binaries/integrations: `114` passed, `3` ignored;
- the preserved broad prefix before the red target: `596` passed.

The non-duplicative recovery accounting is therefore `1,184` passed and `12`
ignored. It excludes partial passes from both red targets and excludes the
focused deployed-profile page test already contained in the complete
`yune_web` target. This supports compatibility recovery without rewriting or
concealing the measured broad red.

The only code changes in the frozen candidate are test-only corrections:

- `crates/yune-core/tests/cantonese_parity.rs` exhausts the real candidate
  stream before full-list fixture comparisons while retaining the bounded-page
  assertions that own initial-leader and target-absence behavior;
- `crates/yune-rime-api/src/tests/lifecycle_safety.rs` aligns test names and
  documentation assertions with the existing process-global service and
  cross-thread serialization contract.

Neither correction changes production runtime behavior, ABI layout or exports,
schema/profile IDs, candidate ordering or selection, browser behavior, package
behavior, or signed thresholds.

### Packet and documentation controls

The pre-review packet manifest verifies against the frozen tree with 55 listed
files and 251,801 bytes. Tracked measurement tables and verdict/provenance
records are byte-identical to their declared external raw sources. Current-doc
link validation passes for 6 current documents and 205 local targets;
evidence-growth validation passes for all 56 changed files; privacy validation
passes for all 56 files against the six forbidden literals.

The candidate's `.gitattributes` addition applies `-text` only below the M61
evidence directory. Attribute inspection confirms imported receipts are kept
byte-stable while ordinary Rust source retains the repository's normal text
handling. This narrowly resolves the discarded candidate's line-ending/tree
manifest mismatch without changing source policy elsewhere.

The seven terminal M61 requirement rows are closed consistently under
disposition D: baseline, attribution, branch selection, reduction outcome,
compatibility recovery, ratchet decision, and evidence closeout. D-47, D-48,
the M59 record, production runtime semantics, and the engine-support contract
remain authoritative and are not weakened.

## Proofs Checked

- exact correction, revert/base, and frozen candidate commit/tree identities;
- correction/revert path scope and exact production-tree restoration;
- isolated-index tree equality and exact 52-path no-rename delta;
- source-bound owner, process, reconciliation, verdict, and provenance hashes;
- independent owned-mode and byte-backed-mode aggregation reruns;
- independent `17/17` candidate-parity comparator rerun;
- signed-registry identity and absence of a newly accepted threshold;
- broad-run status stream, raw red log, retry ownership, and non-duplicate
  recovery accounting;
- both test-only diffs and their production/runtime inclusion boundaries;
- packet membership/hash verification against the frozen tree;
- current-document links, evidence growth, privacy, attributes, and
  `git diff --check`;
- roadmap, requirements, decisions, milestone ledger, performance dashboard,
  engine-support contract, M59 authority, and M61 plan consistency.

## Actionable Findings

None.

## Closeout Caveats

- The literal broad workspace test remains a preserved measured red; the
  packet claims only the exact successful prefix plus source-bound disjoint
  recovery, not a later all-workspace green rerun.
- The supplemental projection remains diagnostic and unaccepted. It must not
  be presented as a signed performance threshold or production-default win.
- Browser, deployment, packaging, benchmark expansion, and unrelated broad
  suites are outside M61's approved evidence boundary and are not inferred by
  this review.
- This verdict covers only pre-review tree
  `a15303d5ecbf3eda73f31911aa9870e30356172a`. After both reviews, the exact
  allowed candidate-tree delta is limited to `packet-manifest.csv`,
  `review-requirements.md`, and `review-isolation.md`; the manifest and final
  tree proofs must then be regenerated and verified before closeout.
- The containing closeout commit and post-push receipt necessarily remain
  post-review/post-push facts and are not claimed by this receipt.
