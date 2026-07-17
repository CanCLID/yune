# M61 Phase 0B Requirement And Evidence Review

**Verdict:** APPROVE - no findings.

**Independent reviewer:** `/root/m61_trackb_plan_amendment`.

**Preserved pre-review candidate tree:**
`5df83f5adc9c02dac09bafb74561c99e39cdf046`.

**Plan blob:** `7a3d949aad74536a1badd9db5a3d737cf9762769`.

**Base:** commit `f18b0df2d0149bc2a28cd9bd2c075c34030b5568`, tree
`e4ba5201eab8b8fd8cb24ae14dd49a8c9959aa10`.

The active M61 plan is the sole pre-review path. The preserved blocker receipt
has SHA-256
`bbb0a2649c3243c2680edc7d8e259a531a36e8150d508094bab6b843fd13ea4f`.
Owned runs 1-2 are `32/32`; run 3 is `31/32`; every round retains candidate
parity `17/17`. The 60 run-3 observations reproduce median private bytes
`32,727,040 B`, above the unchanged `32,084,378 B` ceiling by `642,662 B`.

The two Track B `compact_table.syllabary_codes` rows are stable at
`4,850,892 B` across `134,628` codes; the corresponding Track A row is only
`11,573 B` across 424 codes. One UTF-8 buffer plus monotonic `u32` offsets has
sufficient structural leverage, while the plan correctly requires fresh
process evidence rather than inferring private-byte movement from owner bytes.

The six-path boundary covers representation and owner accounting, prism indexed
and iterated access, prism construction, translator/reverse-graph consumers,
and schema installation. It preserves exact source order, duplicates, empty and
non-ASCII strings, artifact formats, runtime behavior, Track A, ABI, thresholds,
cadence, and evidence tooling. The exact owning test and stop-if-insufficient
rule fail closed.

The correction must be the direct child of this amendment commit. Acceptance
requires a new detached clone, complete `17 + 1` preflight, and five wholly new
`32/32` owned rounds. A corrected-source red exhausts the exception and requires
a separate exact revert, push, and remote-equality proof while retaining the
correction, revert, and red evidence. No `f18b0df2` round is reusable.

The unchanged M55 registry and supplemental ratchet remain bound by SHA-256
`e74e77b4dd5b253e0c2b5f4b12cc1e0279784d3c3fbf02006b5f8f18fccacdba`
and `d52d064f410df36c1c22dd5523430062563a17bb9f2f63253b607d211badefd7`.
Existing M61 requirement and roadmap rows already cover this prerequisite, so
no `docs/requirements.md` or `docs/roadmap.md` change is required.
