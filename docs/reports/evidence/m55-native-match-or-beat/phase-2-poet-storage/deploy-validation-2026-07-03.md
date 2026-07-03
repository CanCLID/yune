# M55 Phase 2 Poet Deploy Validation Slice

Date: 2026-07-03.

Verdict: **partial progress, not Phase 2 complete**.

This slice wires the `YUNE-POET/1` artifact into native dictionary deployment
far enough to make the artifact reproducible and stale-aware:

- `execute_rebuild_plan` now writes `<dictionary>.poet.bin` whenever the table
  artifact set is rebuilt.
- `rime_dict_rebuild_plan` treats the poet artifact as part of the table
  artifact set: a missing or checksum-stale poet artifact schedules a table
  artifact rebuild, even when `<dictionary>.table.bin` is otherwise fresh.
- Native deployment now probes `<dictionary>.poet.bin`, copies the prebuilt poet
  artifact with a prebuilt table, and passes poet checksum/availability into the
  rebuild planner.
- `schema_install.rs` validates any present compiled poet artifact against the
  table dictionary checksum and records a memory-probe marker with poet bytes and
  section counts.

Boundary:

- Runtime sentence lookup still uses the existing heap-backed
  `UpstreamSentenceModel`; this slice does not claim memory-owner movement.
- Missing poet artifacts are repaired by the deploy/rebuild path when source is
  available, and source-missing prebuilt reuse now requires a matching prebuilt
  poet artifact. Present corrupt/stale poet bytes reject the compiled path
  loudly. The final runtime consumption slice still needs to make byte-backed
  poet storage mandatory for the Track A compiled path.
- No public C ABI or browser/product surface changed.

Commands run:

```powershell
cargo fmt
cargo fmt --check
cargo test -p yune-core rime_dict_rebuild_plan
cargo test -p yune-core rebuild_plan_executor_writes_only_requested_artifacts
cargo test -p yune-core poet_bin
cargo test -p yune-rime-api poet_dict_file_checksum_from_path_reads_generated_artifact_header
cargo clippy -p yune-core -p yune-rime-api --all-targets -- -D warnings
```

Focused result:

- The rebuild planner reuses fresh artifacts only when table, poet, prism, and
  reverse metadata are all current.
- Missing/stale poet metadata forces a table artifact-set rebuild.
- The rebuild executor emits a parseable `<dictionary>.poet.bin` with the table
  artifact.
- The yune-rime-api deployment helper reads the checksum from generated poet
  bytes and rejects non-poet bytes.
- The poet artifact parser still rejects wrong version, truncation, and checksum
  mismatch.

Next Phase 2 step:

1. Split `UpstreamSentenceModel` reads behind byte-source accessors.
2. Load the deployed poet artifact through the existing compiled-storage seam.
3. Preserve the M42 abbreviation vocabulary path while removing retained heap
   copies for `poet.vocabulary` and `poet.entries_by_code`.
4. Run the Phase 2 parity, product-path, memory-owner, and full ratchet gates
   before marking any Phase 2 checkbox complete.
