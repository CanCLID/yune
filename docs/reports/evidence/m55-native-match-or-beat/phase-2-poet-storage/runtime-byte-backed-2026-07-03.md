# M55 Phase 2 Runtime Byte-Backed Poet Slice - 2026-07-03

Verdict: progress only; Phase 2 is not closed.

## Scope

This slice wires the already-deployed `YUNE-POET/1` artifact into the runtime sentence model path:

- `yune-core` exposes `PoetByteSource` / `OwnedPoetBytes` and can construct `UpstreamSentenceModel` from a validated poet byte source.
- The runtime model serves `poet.entries_by_code`, `poet.vocabulary`, and `poet.abbreviation_vocabulary` by offset from artifact bytes instead of retained entry/vocabulary payload vectors.
- The remaining `poet.lookup_index` stays as the small guarded heap index over code ranges.
- `yune-rime-api` passes a validated compiled poet byte source into the upstream Luna sentence model when the compiled artifact is present.
- Native mmap-backed sources report `mmap_file_backed`; in-memory byte sources report guarded heap, matching the existing compact-table owner-class convention.

## Focused Evidence

Commands run:

```powershell
cargo fmt
cargo check -p yune-core -p yune-rime-api
cargo test -p yune-core upstream_sentence_model_
cargo test -p yune-core rebuild_plan_executor_writes_only_requested_artifacts
cargo test -p yune-core rime_dict_rebuild_plan_rebuilds_table_artifact_set_when_poet_missing_or_stale
cargo test -p yune-rime-api workspace_update_rebuilds_source_dictionary_artifacts_and_reuses_fresh_outputs
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
git diff --check
git diff --name-only -- "*.gram" "*.marisa"
git ls-files --others --exclude-standard -- "*.gram" "*.marisa"
```

Observed result:

- `cargo check -p yune-core -p yune-rime-api`: passed.
- `cargo test -p yune-core upstream_sentence_model_`: passed, `16` tests.
- `cargo test -p yune-core rebuild_plan_executor_writes_only_requested_artifacts`: passed, `1` test.
- `cargo test -p yune-core rime_dict_rebuild_plan_rebuilds_table_artifact_set_when_poet_missing_or_stale`: passed, `1` test.
- `cargo test -p yune-rime-api workspace_update_rebuilds_source_dictionary_artifacts_and_reuses_fresh_outputs`: passed, `1` test.
- `cargo fmt --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo test -p yune-core --test upstream_luna_pinyin_parity`: passed, `12` tests.
- `cargo test -p yune-core --test cantonese_parity`: passed, `37` tests.
- `git diff --check`: passed.
- Diff-scoped `*.gram` / `*.marisa` checks: no new or untracked model bytes in this slice.

New focused regressions:

- `upstream_sentence_model_reads_candidates_from_byte_backed_poet_artifact`
  - Builds a synthetic poet artifact with normal and abbreviation vocabulary.
  - Loads one model from heap entries and one model from a byte-backed poet source.
  - Asserts identical candidates for normal sentence input and abbreviation span input.
  - Asserts `poet.entries_by_code`, `poet.vocabulary`, and `poet.abbreviation_vocabulary` report `mmap_file_backed` for an mmap-style byte source, while `poet.lookup_index` remains `heap_owned_guarded`.
- `upstream_sentence_model_rejects_stale_poet_artifact_checksum`
  - Proves the runtime constructor rejects a poet artifact whose dictionary checksum does not match the expected compiled-table checksum.

## Remaining Phase 2 Gates

This slice does not satisfy the Phase 2 closeout by itself. Remaining gates:

- Run product-path CLI candidate byte comparisons for the 37-char row, 59-char row, and fixture sentences.
- Capture a real Luna `memory-owner-profile.csv` proving the large poet owners moved to mmap-backed storage with combined heap remnants below the Phase 2 target.
- Run `upstream_luna_pinyin_parity` and `cantonese_parity`.
- Run the full M55 ratchet gate and tighten the memory ceiling only after two green runs.
- Record any latency tradeoffs, especially on 37/59-char and win rows.
