# M61 Phase 0A Isolation, Oracle, And Threshold Review

**Verdict:** APPROVE — no findings.

**Preserved pre-review candidate tree:**
`f8bbe2be5d1525e240dff7977bf3a86c43279927`.

**Base:** commit `7805882d93428db0a3791b0631290ab319b524f0`, tree
`c198d23fc6777ad8b119e30552980243a6acdbb6`.

The isolated plan candidate reconstructs exactly, the active plan is the sole
pre-review delta, `git diff --check` is clean, and the real index is empty. The
amendment preserves immutable lineage through `91b8991c`, `6a1cbed7`, and
`7805882d`; none of those pre-correction SHAs may be an accepted measurement
source.

The amendment and correction procedures fail closed on sorted path lists,
preserved pre-review trees, receipt-only post-review deltas, isolated indexes,
commit-tree equality, direct-main push, and remote equality. The comparator
requires both engines, the frozen 17 inputs, exact page-zero shape and field
equality, deterministic outputs, nonzero exit on any mismatch, wrapper
fail-closure, and five-round aggregation of hashed PASS receipts with uniform
tool/output/source/oracle identity.

The pinned librime commit/tree, official DLL, shared tree, freshly generated
build tree `446c90b2f4ffd76b4ec1f4ecca4f534c986e72e3d8803c6998926d0b1cebbf17`,
product schema tree, Yune DLL, and benchmark executable match the preserved
preflight. The fresh build identity is correctly distinguished from M59's
historical generated tree: deployment regenerates schema `__build_info`
timestamps while pinned source, official DLL, and shared source remain fixed.

The authorized correction is isolated from memory optimization, storage
defaults, thresholds, ranking weights, ABI/API/exports, schema/profile IDs,
browser/product work, and Track B. The exact `17 + 1` inputs, `9/60/80` cadence,
M55 registry, supplemental memory ratchet, fixed-binary rules, and failure/retry
policy are unchanged. The preserved `16/17` result remains a correctness red;
another red blocks M61 without waiver or disposition-D reclassification.
