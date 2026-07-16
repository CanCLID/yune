# M61 Candidate-Parity Isolation And Tooling Review

**Verdict:** APPROVE — no findings.

**Preserved pre-review candidate tree:**
`70b8d6a81795c0e4349bbc5e9c7d29d4fd6e06bc`.

**Base:** commit `1c405855427033464f8f720eae31e6d2d6c34fc3`, tree
`b879a6b0c342f58925717cc488058d7cb31ab682`.

The external isolated index and bytewise-sorted manifest reconstruct exactly
the preserved thirteen-path candidate. The complete fifteen-path union is
those thirteen paths plus only this receipt and
`candidate-parity-review-requirements.md`. The real index remained empty and
all diff checks passed.

No threshold/registry, POET owner/storage/default, memory optimization,
Cargo/API/C-ABI/export, profile/schema ID, or unrelated evidence path is in
the candidate. The sole runtime edit is the authorized class predicate: stale
unpruned raw-identity choices are excluded while transformed choices and live
pruned edges remain intact.

The exact frozen input order is identical in the comparator, both wrappers,
and aggregator. The comparator fails closed on missing, extra, duplicate,
non-contiguous, malformed, I/O, or output-collision cases; a shape-valid
mismatch exits separately; PASS requires `17/17`. It parses and hashes the
same captured input bytes. PowerShell and macOS both validate exact receipt
keys, tool/snapshot/input/output hashes, source tree, and all oracle identities
before completion, then recheck packet immutability. The aggregator requires
uniform five-round source/oracle/tool/output identity, validates parity and
detail semantics, deterministically replays the comparator, retains atomic
path-safe publication/invalidation, and requires the full Track B lane.

Negative coverage includes the preserved `16/17` red, structural row faults,
unusable output paths, input-byte mutation, macOS post-compare mutation,
hash-consistent parity/detail contradictions, cross-round drift, output
collision, incomplete runs, and publication failure.

Independent green checks:

- candidate parity 10/10, M59 evidence tools 84/84, M61 supplemental 9/9,
  M61 native-mode contract 11/11, and native benchmark tooling 19/19;
- stale-identity structural regression 1/1, transformed-overlap control 1/1,
  and deployed pinned fixture 1/1;
- PowerShell AST parse, macOS wrapper `bash -n`, isolated tree/path proofs,
  changed-file formatting checks, and diff checks.

A non-owning broad formatting probe reported only pre-existing M60 formatting
drift outside the thirteen-path candidate. It is not a correction finding and
no unrelated path was changed.
