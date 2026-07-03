# M55 Phase 2 Poet Storage Artifact Contract Slice

Date: 2026-07-03.

Verdict: **partial progress, not Phase 2 complete**.

This slice adds the safe Rust `YUNE-POET/1` compiled artifact contract in
`yune-core`:

- builder: `build_poet_bin`;
- validator/summary reader: `parse_poet_bin_summary`;
- explicit dictionary-checksum validation;
- section directory with fixed ids for sentence entries, entry text/code pools,
  vocabulary rows, first-code indexes, and character-code tables;
- wrong-version, truncated-artifact, and checksum-mismatch regression tests.

This does **not** yet complete Phase 2. Runtime lookup still uses the existing
heap-backed `UpstreamSentenceModel`, and deploy/mmap wiring in `yune-rime-api`
remains next. No memory-owner movement is claimed from this slice.

Commands run:

```powershell
cargo fmt
cargo test -p yune-core poet_bin
```

Focused result:

- `poet::storage::tests::poet_bin_summary_validates_versioned_sections` passed.
- `poet::storage::tests::poet_bin_rejects_wrong_version` passed.
- `poet::storage::tests::poet_bin_rejects_truncated_artifact` passed.
- `poet::storage::tests::poet_bin_rejects_checksum_mismatch` passed.

Next Phase 2 step:

1. Wire schema compile/deploy to write `<dictionary>.poet.bin`.
2. Add native mmap/owned-byte loading through the existing `schema_install.rs`
   compiled-storage seam.
3. Switch Luna poet reads to the byte-backed source while preserving candidate
   parity.
4. Run the Phase 2 parity, product-path, memory-owner, and full ratchet gates
   before marking any Phase 2 checkbox complete.
