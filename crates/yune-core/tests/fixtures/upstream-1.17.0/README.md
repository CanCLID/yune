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
  sentence/lattice closeout; M18 deployment/processor depth; M19 breadth schemas;
  M55 Phase 3R sentence fixture expansion; M59 complete Luna Lane B
  text/order/position and partial-selection composition; M59 Cangjie
  candidate-zero target derivation and exact-order diagnostics

## Capture Rules

- The local upstream checkout may be used as a build cache, but the local path is
  not part of fixture identity.
- Prefer the official upstream release binary for behavioral byte capture when
  available. The local source build is a reproducibility cross-check, not the
  primary behavioral oracle.
- Expected bytes must come from upstream librime, never from Yune.
- Every JSON fixture must record the engine, tag, commit, capture date, capture
  command, schema, input sequence, and source-row policy in its own standard
  `oracle`/`capture` fields or, for a purpose-built capture with a corresponding
  `oracle-manifest.json` entry, jointly across the fixture and that entry.
  Purpose-built captures may use an equivalent nested shape, but their owning
  provenance test must pin the exact binary, source, dependency, query-tree, and
  policy identities.
- If a case cannot be captured, keep the Yune test ignored with a `panic!()` body
  and document the exact command that would unblock it.

## Adding A New Upstream Schema

1. Clone or update the schema-data repository under
   `target/upstream-oracle/1.17.0/schema-src/` and record its Git commit in the
   fixture.
2. Capture through the generalized wrapper, for example:

```powershell
$captureDate = Get-Date -Format yyyy-MM-dd
$captureOutput = "target/upstream-schema-capture-$([guid]::NewGuid().ToString('N')).json"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/capture-upstream-schema.ps1 -OracleRoot target/upstream-oracle/1.17.0 -SchemaId double_pinyin -SchemaDataRepo rime/rime-double-pinyin -Output $captureOutput -CaptureDate $captureDate
```

The output must be a fresh path outside `OracleRoot`. Review that capture, then
import it separately; the wrapper never overwrites a tracked fixture.

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

### `m59-luna-leading-single-composition.json`

- Schema: `luna_pinyin`
- Inputs: `moboyi`, `boyi`, `yi`, `zhonggao`, `zhongguo`, `gao`, `guo`
- Complete candidate counts: `225`, `297`, `841`, `117`, `125`, `164`, `366`
- Source-row policy:
  `m59_lane_b_complete_order_and_partial_selection_composition`
- Captures every candidate text and position across every page, plus the
  `moboyi` -> `莫伯洢` partial-selection composition chain used by M59 D-48.
- The disposable deploy binds clean pinned schema repositories, upstream binary
  hashes, and complete shared/build tree hashes. Fixed staged mtimes make
  same-input recaptures byte-identical without excluding deployed bytes.
- The manifest recapture command writes create-new output under ignored
  `target/`; review its byte/diff result and import it separately. The capture
  tool intentionally refuses to overwrite this tracked fixture.

### `cangjie5-composition.json`

- Schema: upstream `cangjie5` over pinned `rime/rime-cangjie`.
- Inputs: `hwmvsqtt`, `ebcnyripm`, `takohaeosk`, `hwmvs`, `qtt`, `ebcn`,
  `yripm`, `tak`, `oha`, `eosk`, `hdaetcu`, and `lyk`.
- Source-row policy:
  `d48_cangjie5_exact_code_cohorts_for_char_by_char_composition`.
- The raw capture records complete all-page order plus clean binary, tool,
  schema/dependency commit and Git-tree identities. The curator derives owner
  targets from captured candidate zero, verifies their ASCII U+ declarations,
  links the atomic candidates, and embeds the complete pinned source cohorts.
- The tracked fixture is byte-identical to the reviewed curated oracle in
  `docs/reports/evidence/m59-cangjie5-order-parity/increment-1-executable-evidence/`.
  The packet preserves the untouched raw oracle, Yune capture, strict exact
  diff, same-path replay, and the still-red CJ-1 disposition.
- The manifest recapture command writes raw and curated create-new outputs under
  ignored `target/`; review them first and import the curated bytes separately.

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

### `luna-pinyin-sentence-expanded.json`

- Source-row policy: `m55_phase3r_luna_sentence_expansion`.
- Scenarios: 11 oracle-captured sentence snapshots covering 3-5 syllable
  phrases, mixed-length sentences, `shijian`/`beijing`
  completion-over-bareword rows, and the 37/59-character M55 benchmark inputs.
- Generated by `scripts/capture-upstream-luna-pinyin.ps1` from
  `target/upstream-oracle/1.17.0/luna-pinyin-scenarios.json`; the default
  scenario set in that script contains the Phase 3R rows and tested-code
  metadata.
- Carries source dictionary rows for each listed syllable plus the full input
  code, `essay.txt` rows for the in-scope source/candidate terms, and a separate
  11-row sentence-support slice captured from pinned `rime/rime-essay`
  `48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed`. The mechanics test consumes only
  these checked-in fixture bytes; it does not consult a deployed product asset.
- Capture command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-luna-pinyin.ps1 -OracleRoot target/upstream-oracle/1.17.0 -Output crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json -SentenceExpandedOutput crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json
```

### `m18-luna-pinyin-prism.json` / `m18-luna-pinyin-prism.bin`

- Source-row policy: `upstream_deployer_compiled_prism_artifact`.
- Captures the upstream `luna_pinyin.prism.bin` generated by the pinned 1.17.0
  deployer and checked in as binary evidence for a real Darts double-array
  section.
- Used by M18 tests to prove Yune parses upstream prism Darts sections and to
  lock non-circular metadata/lookup evidence.

### `m59-librime-log-weight/`

- Source-row policy: `m59_librime_compiled_log_weight_and_script_encoder_boundary`.
- Purpose-built Luna subset compiled and executed by pinned librime
  `33e78140250125871856cdc5b42ddc6a5fcd3cd4` on 2026-07-11.
- Retains the exact 4,744-byte upstream Marisa `.table.bin` as lowercase hex,
  its source rows and hashes, the oracle page for `zhegeyinqing`, and exact-5%
  `changju`/below-boundary `changzhu` controls.
- The fixture distinguishes librime's natural-log compiled table weights from
  source-linear weights. In particular, 蓋/`ge` at `0.09%` remains a serialized
  word entry but must be excluded from ScriptEncoder phrase expansion and must
  not synthesize `遮蓋`; 足/`ju` at exactly `5%` must still expand `長足`.
- The owning test decodes the captured table bytes, runs Yune's real compact
  table and default-owned sentence-model path, and compares the whole first
  page with the librime capture.

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

### M59 transformed-algebra whole-input captures

- `double-pinyin-m59-whole-input.json` captures `hknivs` as one uninterrupted
  key sequence. The pinned upstream top candidate and commit preview are
  `好逆鐘`, with preedit `hao ni zhong`.
- `bopomofo-m59-whole-input.json` captures `cl3su3j06` as one uninterrupted
  key sequence. The pinned upstream top candidate and commit preview are
  `好你玩`, with preedit `ㄏㄠˇ ㄋㄧˇ ㄨㄢˊ`.
- Both tops have exact-term count zero in the complete pinned source dictionary
  and `essay.txt`; the fixtures record those scans under
  `capture.whole_input_oracle_rows`. They are oracle composition results, not
  strings inferred from the older component rows and not Yune-produced values.
  The curated source rows include every Unicode-scalar constituent of each
  oracle top, so the composition has external character-row provenance even
  though the complete phrase is absent.
- `m59-whole-input` mode sends every character directly through
  `RimeProcessKey(keycode, 0)`. In Bopomofo, `3`, `0`, and `6` therefore remain
  schema key events used by the tone/keymap algebra. The M59 scenario list is
  deliberately limited to paging and Space commit; it does not call a numeric
  tone key a candidate-selection action.
- The hardened script requires an explicit capture date, the expected librime
  DLL/deployer hashes, exact clean schema/dependency commits, and a new output
  path outside the oracle cache. The complete effective command is embedded in
  each fixture and names that fixture's repository-relative path.

The script's `m19-component` mode retains the historical component scenario
family for future diagnostic recaptures, but its Bopomofo numeric-`2` action is
now truthfully named as a tone-key action. That label correction intentionally
changes future M19 Bopomofo replay metadata. The historical component fixture
bytes remain untouched: `double-pinyin-basic.json` is
`2f17053131d73028f315229fe7f22df226fc4b67f3b224e19ce99ed2bf864d24`
and `bopomofo-basic.json` is
`3288e14306c3fc1cfe53e10f0bb743afa02e514d3bafe652f544e423f1047c70`.
Their original generalized-capture script was pinned at
`b23e07a8626243fb6e08477f48b0efbf22f0118c82799cbfe4eb01b6ecf0ca96`;
the M59 whole-input fixtures pin the hardened script separately.

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
