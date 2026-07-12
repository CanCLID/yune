# Preserved Increment 4b performance attempts

These attempts are evidence, not accepted runs:

- `e97811a5-eager-materialization/`: the first exact behavior implementation
  eagerly admitted the complete abbreviation family on the typing path. The
  signed short-key checkpoint failed (`n 4.412/3.006`, `ni 2.963/2.666`,
  `hao 1.905/1.844`) together with other latency rows. The owner chose the
  retained-ceiling lazy/page-bounded path; no ceiling was changed.
- `d7adc253-retained-lookup-cache-memory/`: three complete rounds proved the
  first bounded implementation retained both lookup indexes. Track B session
  working set was `68,820,992`, `69,005,312`, and `69,066,752` bytes against
  `66,872,115`; private bytes were `33,095,680`, `35,430,400`, and
  `35,344,384` against `32,084,378`. This caused the compact row-start index fix.
- `8c75f548-per-run-harness-nondeterminism/`: five complete rounds were
  behavior/performance green, but each independently linked benchmark harness
  had a different SHA-256. The aggregate packet was rejected on provenance.
- `8c75f548-same-path-linker-nondeterminism/`: three further complete green
  rounds built through one stable path and still produced three different
  executable hashes, proving path stability did not make Windows linking
  byte-deterministic.
- `ea8656c3-green-ratchet-clippy-gate-red/`: the new source-bound one-build/four-
  reuse protocol produced a green 32-row packet, but the literal workspace
  Clippy gate found two type-complexity errors and one Rust-1.76 MSRV violation.
  That packet is rejected; the behavior-preserving release-gate repair landed
  at `4bed300e`.
- `4bed300e-green-ratchet-internal-review-red/`: the repaired source produced a
  source-bound five-round 32/32 packet, but independent review found that two
  bounded-collector replacement paths could reverse equal-weight source order.
  The packet is rejected even though its five named class-4 rows stayed green.
  `eb117c53` fixes both saturated-worst and later-better-duplicate replacement,
  adds owned and byte-backed bounded-versus-complete regressions, and received
  behavior and five-round reruns.
- `eb117c53-web03-release-gate/`: that source-bound rerun was green on all 32
  aggregate rows and all 160 individual observations, but the exact workspace
  gate exposed `442,856` prefix-fallback views against the WEB-03 `5,000`
  ceiling on the first long Jyutping row. The packet is rejected even though
  its candidate and signed-ratchet results were green. `d2499358` bounds the
  oversized uncached family generically, but its first rerun was later rejected
  by current-head order review.
- `d2499358-current-head-review/`: all 32 aggregate rows and 160 observations
  were green, but independent review found an equal-weight bounded-prefix
  inversion. `c5d954e2` restores librime's per-chunk current-head order and
  `d508e05b` keeps bounded/complete prefix qualities field-identical; only the
  regenerated d508 packet is accepted.

The preserved subtrees contain no executable, DLL, compiled table, or deployed
schema payload. Green measurements with invalid provenance or a red release
gate are not accepted by substitution.
