# M55 Phase 2 Ratchet No-Go - 2026-07-03

Verdict: Phase 2 is not closed. Do not start Phase 3 or Phase 4 from this
state without an explicit follow-up decision.

## What Changed

The first full Phase 2 ratchet attempt exposed that the Track A benchmark flow
was not actually loading the Yune-generated poet artifact. The benchmark copied
the upstream librime oracle `user/build` tree into the Track A run root, so
`luna_pinyin.poet.bin` was absent and the release memory-owner profile still
reported the poet payload owners as heap-owned.

This follow-up changed the benchmark path and runtime checksum handling:

- `scripts/benchmark-native-rime-inprocess.ps1` now performs an untimed Track A
  deploy-prep process, captures the generated `luna_pinyin.poet.bin`, restores
  the original oracle build artifacts, and copies only the poet artifact back
  before timed sampling.
- `native_inprocess_benchmark` supports `--deploy-only`, so deploy prep does
  not contaminate timed benchmark samples or peak memory measurements.
- The deploy helper runs the schema-specific deploy/update path instead of the
  broad deploy entry point.
- `schema_install.rs` accepts the known upstream Luna marisa compact-table
  checksum pairing when validating the generated poet artifact. The poet bytes
  are keyed to the source dictionary checksum, while the imported upstream
  marisa table metadata carries the marisa table checksum.
- The poet runtime path avoids one per-row character-vector allocation and
  adds a transient per-graph character-code cache. This is a temporary
  access-path reduction, not a retained runtime heap index.

Focused checksum tests were added for the known upstream Luna marisa pairing
and the normal compiled-poet checksum path.

## Evidence

Primary run:

```text
docs/reports/evidence/m55-native-match-or-beat/phase-2-poet-storage/ratchet-gate-1/
```

Exact gate command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-2-poet-storage\ratchet-gate-1 `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

The gate exited non-zero. `threshold-check.csv` records:

| Row | Observed | Ceiling | Result |
| --- | ---: | ---: | --- |
| `ceshiyixiachangjushuruxingnengzenyang` | `6.289x` | `3.267x` | fail |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `4.333x` | `2.447x` | fail |
| Track B product long row | `378.449 us` | `375.253 us` | fail |

The same run proves the Phase 2 memory owner movement:

| Owner | Byte class | Mapped bytes |
| --- | --- | ---: |
| `poet.entries_by_code` | `mmap_file_backed` | `3048137` |
| `poet.vocabulary` | `mmap_file_backed` | `25492848` |
| `poet.abbreviation_vocabulary` | `mmap_file_backed` | `713` |
| `poet.lookup_index` | `heap_owned_guarded` | `0` |

Track A process peak high-water was `110198784` bytes, below the current
`198000000` byte ceiling. The product path reported
`selected_storage=rsmarisa_byte_backed`,
`checksum_status=accepted_upstream_marisa_import_checksum`, and
`table_format=rime_marisa_string_table:1574520`.

## Interpretation

Phase 2 successfully moved the named poet payload owners out of retained heap,
but the byte-backed access path still breaches the ratchet on the 37-character
and 59-character Luna rows. The small Track B product latency miss is also a
release-gate failure, even though it is only about `0.85%` over the ceiling.

Because the full ratchet is red, there is no Phase 2 ceiling tightening and no
M55 closeout claim. The next M55 decision is whether to continue access-path
work inside Phase 2, close Phase 2 partial/no-go, or explicitly rescope the
program before later phases.

## Verification Run For This Slice

```powershell
cargo fmt
cargo test -p yune-rime-api compiled_poet_checksum
cargo test -p yune-core upstream_sentence_model_ -- --test-threads=1
cargo check -p yune-core -p yune-rime-api
cargo clippy -p yune-core -p yune-rime-api --all-targets -- -D warnings
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\phase-2-poet-storage\ratchet-gate-1 `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

Follow-up verification after recording this no-go also ran:

```powershell
cargo fmt --check
git diff --check
git diff --name-only -- "*.gram" "*.marisa"
git ls-files --others --exclude-standard -- "*.gram" "*.marisa"
```

`cargo fmt --check`, the focused tests, `cargo check`, targeted clippy, and
`git diff --check` passed. No new or untracked `*.gram` / `*.marisa` bytes were
present. The full ratchet command is expected to fail for this no-go evidence.
