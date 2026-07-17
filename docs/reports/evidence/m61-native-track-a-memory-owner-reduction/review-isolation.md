# M61 change-isolation and checker-quality review

## Verdict

**APPROVE.** I found no actionable change-isolation, checker-quality, evidence,
or scope finding in the frozen pre-review candidate tree
`a15303d5ecbf3eda73f31911aa9870e30356172a`.

## Frozen source and candidate envelope

- The restored source is commit `01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10`
  with tree `f1c36a0079d85628f5cbef140bd94288930cc2e8`. Its parent is the
  measured correction commit `91f5969688a3d2dba96a67d1cfe813c7ba4ee861`
  with tree `6626ed16d5e135fa477ca26e9786d11121c92b44`; the correction's parent,
  quality-repair commit `931c7c59d6d471c69b70dc0d2f082149665a4e68`,
  also has tree `f1c36a0079d85628f5cbef140bd94288930cc2e8`.
- I independently confirmed that both the measured correction and its revert
  touch exactly `crates/yune-core/src/poet/mod.rs`,
  `crates/yune-core/src/poet/storage.rs`, and
  `crates/yune-core/src/tests/poet.rs`; the restored tree is exactly the
  pre-correction tree.
- The external pre-review index writes the named frozen tree. Its declared
  52-path pre-review envelope is sorted, unique, path-safe, and exactly equals
  the `HEAD`-to-candidate tree delta. Every one of those 52 working-tree paths
  still matches the frozen tree byte-for-byte. The real Git index is empty and
  the recorded unrelated baseline is empty.
- The predeclared final envelope is exactly 54 paths, the new-path envelope is
  exactly 45 paths, and the evidence envelope is exactly 58 paths. The only
  permitted post-review delta is the packet manifest plus the two review
  receipts.

## Isolation and scope

- The test-repair tree `bf4ef0b8d7d234b248cc61e9a1c5ad6b57ee61af`
  changes only `crates/yune-core/tests/cantonese_parity.rs`. The combined test
  tree `6cb28424f7bcf5a535ac6173b651e9ba1b7bd160` additionally changes only
  `crates/yune-rime-api/src/tests/lifecycle_safety.rs`; the frozen candidate
  contains those exact two blobs. The first repair makes all-pages assertions
  consume complete candidate lists, while preserving the bounded-page absence
  assertion. The second aligns a documentation-contract test with the current
  process-global-service and cross-thread/no-parallel-progress wording.
- No production runtime, candidate ordering/ranking/selection/recomposition,
  C ABI, API table, export, schema/profile ID, browser/product, oracle fixture,
  signed threshold, default, or performance implementation changes are in the
  candidate. No compiled schema asset is changed.
- `.gitattributes` adds only an M61 evidence-directory rule disabling text
  normalization and scoping blank-end whitespace handling. Attribute probes
  confirmed that Rust, app, and other repository paths retain their existing
  behavior.
- M60 reachability opt-out bijection, expiry, registry, and updater behavior are
  not applicable to this closeout because none of their owning paths changed;
  the M55 registry hash is unchanged.

## Evidence and checker quality

- Both working-tree and frozen-tree manifest verification independently pass
  at exactly 55 listed files and 251801 bytes. The manifest is self-excluding
  and uses exact normalized relative paths, byte counts, and SHA-256 hashes.
- Packet lists are sorted, unique, normalized, and reject absolute,
  traversal, duplicate, missing, unlisted, gitlink, and hash/size-mismatch
  cases. The manifest unit suite passed seven tests; its Windows symlink test
  was skipped because the account lacked symlink-creation privilege. This is a
  non-blocking environmental caveat: both verifier modes contain explicit
  symlink rejection and the frozen Git tree has no such entry.
- The first normalized candidate's manifest mismatch is preserved as a red;
  the narrow M61-only `.gitattributes` correction produced the accepted tree.
  Current-document links pass for 6 documents / 205 local targets, evidence
  growth passes for 56 changed evidence files, privacy passes for 56 files
  against 6 forbidden literals, and both ordinary and isolated-index
  `git diff --check` pass. Setup-only BOM/UTF-16 failures and their scoped
  successful retries remain attributable rather than being rewritten.
- Source attribution binds every measured row to correction
  `91f5969688a3d2dba96a67d1cfe813c7ba4ee861`, tree
  `6626ed16d5e135fa477ca26e9786d11121c92b44`, the fixed binaries/receipt, and
  exact raw hashes. The independent reconciliation script is itself hashed and
  fails closed on source/tree, cadence, owner stability, aggregate completeness,
  and threshold violations.

## Measurement and disjoint recovery truth

- The packet accurately preserves the terminal no-go: the 25,096,192-byte
  median whole-process reduction has 18,724,242 explained bytes,
  `0.746098930` coverage, and a 6,371,950-byte residual, so both reconciliation
  thresholds fail. All 32 aggregate rows / 160 observations in each complete
  gate set pass, the 17/17 product receipts pass, and the unaccepted projection
  is clearly separated from acceptance evidence.
- Raw retry receipts support the non-duplicated closeout accounting of 1,184
  passed and 12 ignored. The literal workspace run remains exit 101; the
  bounded candidate-refresh test repair and the later lifecycle contract test
  repair are isolated in the two named test-only trees, and only their owning
  unreached slices were rerun. The packet does not misstate the literal broad
  command as green.

## Findings and caveats

Actionable findings: **none**. The only caveat is the privilege-caused symlink
unit-test skip described above; it does not weaken the candidate-tree or packet
membership proof.
