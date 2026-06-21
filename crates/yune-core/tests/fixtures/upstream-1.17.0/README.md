# Upstream librime 1.17.0 Oracle Fixtures

These fixtures are captured from upstream `rime/librime`, not from Yune and not
from the TypeDuck fork. Use them for core Yune compatibility behavior.

## Provenance

- Engine: `rime/librime`
- Engine tag: `1.17.0`
- Engine commit: `33e78140250125871856cdc5b42ddc6a5fcd3cd4`
- Tag object: `a52a3400f8b7679e839bc5fb8e6309a0fc4424da`
- Release URL: <https://github.com/rime/librime/releases/tag/1.17.0>
- Canonical repository: <https://github.com/rime/librime>
- Captured for: M12 upstream oracle refresh; M17 upstream `luna_pinyin`
  sentence/lattice closeout; M18 deployment/processor depth; M19 breadth schemas

## Capture Rules

- The local upstream checkout may be used as a build cache, but the local path is
  not part of fixture identity.
- Prefer the official upstream release binary for behavioral byte capture when
  available. The local source build is a reproducibility cross-check, not the
  primary behavioral oracle.
- Expected bytes must come from upstream librime, never from Yune.
- Every JSON fixture in this directory must include an `oracle` object with the
  engine, tag, commit, capture date, capture command, schema, and input sequence.
- If a case cannot be captured, keep the Yune test ignored with a `panic!()` body
  and document the exact command that would unblock it.

## Adding A New Upstream Schema

1. Clone or update the schema-data repository under
   `target/upstream-oracle/1.17.0/schema-src/` and record its Git commit in the
   fixture.
2. Capture through the generalized wrapper, for example:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-schema.ps1 -OracleRoot target/upstream-oracle/1.17.0 -SchemaId double_pinyin -SchemaDataRepo rime/rime-double-pinyin -Output crates/yune-core/tests/fixtures/upstream-1.17.0/double-pinyin-basic.json
```

3. Keep the fixture provenance non-circular: `oracle.engine`,
   `oracle.engine_tag`, `oracle.engine_commit`, `oracle.release_url`,
   `oracle.capture_date`, `oracle.capture_command`, `capture.schema_data`,
   `capture.schema_data_commit`, `capture.dependency_repositories`, and
   `capture.source_row_policy` must all be present.
4. Add a per-schema branch in `oracle_fixture_provenance.rs`, then add an owning
   parity test that drives Yune's parser/algebra/translator/Engine path against
   the captured bytes. Unsupported sentence/language-model cases stay as
   ignored tests with `panic!()` bodies.

## Captured Fixtures

### `luna-pinyin-basic.json`

- Schema: `luna_pinyin`
- Upstream schema data: `rime/rime-luna-pinyin`
- Schema-data dependencies: `rime/rime-prelude`, `rime/rime-essay`, and
  `rime/rime-stroke`
- Inputs: `ni`, `hao`, `zhong`, `guo`, `zhongguo`
- Source-row policy: `curated_oracle_winners`

### `luna-pinyin-selection.json`

- Input: `ni`
- Source-row policy: `all_rows_for_exact_code_plus_relevant_essay_rows`
- Includes every exact-code `ni` row from `luna_pinyin.dict.yaml`.
- Includes relevant `essay.txt` rows for every in-scope candidate so Yune cannot
  accidentally rank page-one candidates with default or zero essay weights.

### `luna-pinyin-actions.json`

- Scenarios: first page, next page, previous page, numeric selection, and Space
  commit for `ni`.
- Source-row policy: `action_sequence_oracle_snapshots`.
- Yune-side tests must use the `Engine` key path because menu state and commits
  are part of the behavior.

### `luna-pinyin-reverse-lookup.json`

- Scenarios: stroke reverse lookup prefixes `` `h ``, `` `hs ``, and a no-result
  prefix.
- Source-row policy: `curated_reverse_lookup_rows`.
- Includes stroke dictionary rows, stroke essay rows, and luna_pinyin comment
  rows used by the reverse lookup assertions.

### `luna-pinyin-punctuation.json`

- Scenarios: ordinary punctuation commit, `/fh` symbol candidates, and an
  unmatched symbol path.
- Source-row policy: `curated_symbols_from_pinned_prelude`.
- Includes the exact punctuation/symbol entries consumed by the Yune test.

### `luna-pinyin-options.json`

- Scenarios: `zh_hans` off/on for phrase and single-code inputs,
  `ascii_punct`, and `full_shape`.
- Source-row policy: `option_action_sequence_oracle_snapshots`.
- Active Yune coverage is limited to supported paths; M17/M18 later closed the
  formerly blocked sentence/lattice and processor-only gaps with separate
  fixtures.

### `luna-pinyin-sentence.json`

- Source-row policy: `m17_upstream_luna_sentence_language_model`.
- Inputs: `zhongguo`, `nihao`, `woshi`, `tiantian`, and `renmin`.
- Captures upstream `luna_pinyin` first-page sentence candidates from the pinned
  1.17.0 release binary.
- Carries source dictionary rows for every tested code plus the in-scope
  `essay.txt` rows needed to reconstruct candidate weights.
- Records `grammar_model: null` and `grammar_fallback_penalty:
  -13.815510557964274`, matching upstream `grammar.h`'s null-grammar branch.

### `luna-pinyin-lattice.json`

- Source-row policy: `m17_upstream_luna_sentence_lattice`.
- Scenario: `zhongguo` page 1, Page_Down page 2, and Page_Up page 1 again.
- Captures the full first two pages needed to prove the M17 lattice behavior,
  including the highlighted page-2 commit preview (`中谷o`) for the partial-code
  sentence candidate.
- Uses the same null-grammar provenance as `luna-pinyin-sentence.json`.
- Capture command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-m17-poet.ps1 -OracleRoot target/upstream-oracle/1.17.0
```

### `m18-luna-pinyin-prism.json` / `m18-luna-pinyin-prism.bin`

- Source-row policy: `upstream_deployer_compiled_prism_artifact`.
- Captures the upstream `luna_pinyin.prism.bin` generated by the pinned 1.17.0
  deployer and checked in as binary evidence for a real Darts double-array
  section.
- Used by M18 tests to prove Yune parses upstream prism Darts sections and to
  lock non-circular metadata/lookup evidence.

### `m18-punctuation-processor.json`

- Schema: curated inline `m18_punct`.
- Source-row policy: `curated_processor_schema_literal`.
- Scenarios: `ascii_punct` no-op, direct `{commit: ...}` punctuation,
  scalar confirm-unique preview, pair preview alternation, and list candidate
  cycling.
- Capture command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-m18-punctuation.ps1 -OracleRoot target/upstream-oracle/1.17.0
```

- Capture command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-luna-pinyin.ps1 -OracleRoot target/upstream-oracle/1.17.0 -Output crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json
```

The active Yune check is:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test oracle_fixture_provenance
```

### `double-pinyin-basic.json`

- Schema: `double_pinyin`
- Upstream schema data: `rime/rime-double-pinyin`
- Schema-data dependencies: `rime/rime-prelude`, `rime/rime-essay`,
  `rime/rime-luna-pinyin`, and `rime/rime-stroke`
- Inputs: `ni`, `hk`, `vs`, `go`
- Source-row policy: `m19_double_pinyin_curated_shuangpin_algebra`

### `cangjie5-basic.json`

- Schema: `cangjie5`
- Upstream schema data: `rime/rime-cangjie`
- Schema-data dependencies: `rime/rime-prelude`, `rime/rime-essay`, and
  `rime/rime-luna-pinyin`
- Inputs: `a`, `am`, `amd`
- Source-row policy: `m19_cangjie5_curated_table_codes`

### `bopomofo-basic.json`

- Schema: `bopomofo`
- Upstream schema data: `rime/rime-bopomofo`
- Schema-data dependencies: `rime/rime-prelude`, `rime/rime-essay`,
  `rime/rime-terra-pinyin`, and `rime/rime-stroke`
- Inputs: `su3`, `cl3`, `j06`, `w/4`
- Source-row policy: `m19_bopomofo_curated_zhuyin_algebra`

## Oracle Binary Evidence

- Release assets:
  - `rime-33e7814-Windows-msvc-x64.7z`
  - `rime-deps-33e7814-Windows-msvc-x64.7z`
- Local cache: `target/upstream-oracle/1.17.0/` (not source-controlled)
- Required capture tools verified in the extracted release:
  - `dist/lib/rime.dll`
  - `dist/bin/rime_deployer.exe`
  - `dist/include/rime_api.h`
- Header check: extracted `dist/include/rime_api.h` has the same Git blob hash
  as upstream `src/rime_api.h` at `33e78140250125871856cdc5b42ddc6a5fcd3cd4`
  (`2fccde0fb83ead04d0a12ef834c3770d64dff211`).

## Local Source Build Evidence

- Build host: Windows with MSVC developer environment.
- Local checkout: `rime/librime` at `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- Build commands:
  - `.\build.bat deps`
  - `.\build.bat test`
- Result: upstream `1.17.0` build completed and CTest reported `100% tests
  passed, 0 tests failed out of 1`.
- Required local tools present after the source build:
  - `dist/lib/rime.dll`
  - `dist/bin/rime_deployer.exe`
