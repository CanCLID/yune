# M57 macOS Track A Sentence-Model Parity Evidence

Status: complete on 2026-07-05. M57 is an engine-only parity and verification
repair. It does not make a new general performance claim.

## Summary

The macOS `rime_deployer`-compiled upstream `luna_pinyin.table.bin` is valid
but carries a different source/table checksum pair from the Windows-built
payload. Yune's target-scoped upstream Luna MARISA acceptance path recognized
the Windows checksum pair only, so the macOS compiled-table path fell back to
the wrong sentence-model construction shape.

The fix accepts both known upstream Luna MARISA checksum pairs behind the same
target gates: dictionary `luna_pinyin`, MARISA string table size `1,574,520`,
and the matching source/table checksum tuple. No C ABI field or default API
surface changed.

| Payload | Source checksum | Table checksum | Status |
| --- | --- | --- | --- |
| Windows upstream Luna MARISA | `0x16ad0e3e` | `0xb967cfef` | accepted before and after |
| macOS upstream Luna MARISA | `0xb3d4e98e` | `0x29d56c89` | accepted after M57 |

## Before And After

Focused macOS diagnostics over the same macOS-built Luna table:

| Counter | Before | After |
| --- | ---: | ---: |
| `stored_entries` | `498,564` | `498,564` |
| compact `all_codes()` count | unavailable in old evidence | `332,604` |
| compact expanded table entries | unavailable in old evidence | `513,353` |
| `translator.entries_by_code` | `191,984` | `0` shared compact storage |
| `compact_table.storage` | not selected | `498,564` |
| `poet.entries_by_code` | `191,984` | `513,353` |
| `poet.lookup_index` | `31,262` | `332,604` |
| `poet.abbreviation_vocabulary` | `421,966` | `11` |

Focused graph counters:

| Input | Before vocab / edges / abbr discovery | After vocab / edges / abbr discovery |
| --- | --- | --- |
| `ceshiyixiachangjushuruxingnengzenyang` | `9,741` / `10,211` / `0` | `168` / `401` / `0` |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `15,051` / `15,899` / `0` | `314` / `667` / `0` |
| `cszysmsrsd` | `9,020` / `9,090` / `0` | `151` / `11,156` / `9` |
| `zybfshmsru` | `6,993` / `7,070` / `0` | `89` / `8,833` / `9` |

Candidate snapshots for the two abbreviation rows now match local librime
`1.17.0` on both full macOS passes:

| Input | First page after M57 |
| --- | --- |
| `cszysmsrsd` | `重商主義什麼是認識到`, `重商主義`, `催生作用`, `產生爭議`, `測試資源` |
| `zybfshmsru` | `自有辦法什麼收入`, `自有辦法`, `重要部分`, `晝夜不分`, `主要部分` |

## Evidence Inventory

- `phase-0/current-main-focused/` - focused pre-fix macOS diagnostic rows,
  preserving the defective sentence-model owner shape and wrong abbreviation
  candidates.
- `phase-1/post-fix-focused/` - focused post-fix run over the same macOS table;
  compact enumeration and model owners are repaired.
- `full-pass-1/` and `full-pass-2/` - two complete macOS native verification
  passes from `scripts/benchmark-native-rime-inprocess-macos.sh`, each with
  Track A Yune, Track A librime `1.17.0`, Track B TypeDuck product guard lane,
  recomputed `summary-comparison.csv`, `claim-shape-check.csv`, and
  `macos-verdict.md`.

Both full passes report:

- `checksum_status=accepted_upstream_marisa_import_checksum`
- `stored_entries=498564`
- `compact_all_codes_count=332604`
- `compact_expanded_table_entries=513353`
- `compact_expansion_status=ok:expanded_minus_stored=14789`
- `poet.entries_by_code=513353`
- `poet.lookup_index=332604`
- `poet.abbreviation_vocabulary=11`

## Verification

Passed:

- `cargo fmt --check`
- `cargo build --release -p yune-rime-api`
- `cargo clippy -p yune-rime-api --bench native_inprocess_benchmark -- -D warnings`
- `cargo test -p yune-core compact_table_ -- --nocapture`
- `cargo test -p yune-rime-api schema_install::compiled_poet_checksum_tests --lib`
- `scripts/benchmark-native-rime-inprocess-macos.sh --output-root docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-1`
- `scripts/benchmark-native-rime-inprocess-macos.sh --output-root docs/reports/evidence/m57-macos-track-a-sentence-model-parity/full-pass-2`

Optional WEB-03 TypeDuck guard:

```bash
cargo test -p yune-rime-api --test yune_web web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion -- --exact
```

Not run as an engine guard because the local fixture setup is incomplete:
`apps/yune-web/public-demo/dist/schema` is absent, and the test panics before
Yune engine execution with `No such file or directory`.

## Closeout Read

M57 repairs the macOS verification bundle's comparability defect for the named
Track A rows. The bundle is now valid evidence for the repaired macOS sentence
model path, but it does not supersede the Windows M55 corrective standing gate
or establish a new broad Yune-vs-librime performance claim. Remaining known
Track A disclosures still stand: `n`, `zhongguo`, and both long-sentence first
pages can differ from librime for the previously recorded sentence/completion
ranking reasons.

The Windows M55 standing ratchet was not re-run as part of M57.
