# Rejected d2499358 signed performance ratchet

The then-green five-run packet binds to clean production source `d2499358`, tree
`132e8d86bd58c3cb9296d2f2d340825c7a555710`, release DLL SHA-256
`4000869a4b3d2bee27d663e35cbe38cde7dd4c29b6ac6c7b76fdaf5d82e68238`,
and unchanged threshold SHA-256
`e74e77b4dd5b253e0c2b5f4b12cc1e0279784d3c3fbf02006b5f8f18fccacdba`.
It uses the full 17-input Track A set, one Track B product row, product deployment,
and `9/60/80` startup/session/key iterations.

Run 1 built the benchmark executable and emitted a source receipt. Runs 2-5
reused that exact executable and receipt. Aggregate provenance verifies:

- executable SHA-256
  `3bb2f230f27721d7c0b674e4a0ad56cb2c861a6c24a37b42c4a5542ca96cb14d`;
- receipt SHA-256
  `ad530ddaee57e2e4eaa7f3f2b56eb2ecfda09ad4474c163e0f7b40644374c395`;
- mode sequence `build,reuse,reuse,reuse,reuse`; and
- byte-identical candidate snapshot SHA-256
  `4fcb03525759a40e5f1d8756b5456815f8758c3882aadf5358d3c1ff6f43c463`.

[`gate-verdict.csv`](./gate-verdict.csv) passes all 32 aggregate rows and all 160
individual observations. Its SHA-256 is
`fcb468c08049a1aa4aee1dafa26357a399b1fbf11c5d2adf2a683bfcfc5dcba5`;
the provenance sidecar SHA-256 is
`148ceda85b4639299e9fb08fc8c135788c96e81b98a3cedf8c0623855425f3d7`.
No threshold changed. The packet was nevertheless rejected when independent
current-head review found an equal-weight bounded-prefix ordering inversion;
it is not Increment 4b acceptance evidence.

Each preserved run retains commands, effective invocation, environment,
external provenance, source-bound build receipt, summaries, threshold result,
product storage state, m37 metrics, memory-owner profile, candidate snapshots,
raw-lookup metrics, and run status. Binaries, deployment trees, sample-level
rows, startup traces, and build logs are intentionally excluded.

[`failed-attempts/`](../) records every gate or provenance stop;
none was cherry-picked out of the audit trail.
