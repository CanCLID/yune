# M59 Increment 3b whole-input oracle capture

This packet closes the provenance prerequisite for the Double Pinyin and
Bopomofo transformed-algebra rows. It contains no Yune output and no binary
payload. The expected strings below came from fresh upstream librime `1.17.0`
captures; they were not assembled from the older component fixtures.

| Schema | Whole input | Upstream preedit | Upstream top / preview | Full source dictionary exact term | `essay.txt` exact term | Fixture SHA-256 |
|---|---|---|---|---:|---:|---|
| `double_pinyin` | `hknivs` | `hao ni zhong` | `好逆鐘` | 0 | 0 | `33f373436769d0be0a719bafb6d0c2367e4295c4ed8f26ecda528adf043bf62d` |
| `bopomofo` | `cl3su3j06` | `ㄏㄠˇ ㄋㄧˇ ㄨㄢˊ` | `好你玩` | 0 | 0 | `3f563a940f5d0437b809307d6162e6e1f8ad63e3faf430e1641480fcce667dff` |

The Double Pinyin capture reached all 140 candidates and terminated on the
last page. The Bopomofo capture reached all five candidates and terminated on
its last page. `capture.whole_input_oracle_rows` records the complete pinned
source-dictionary and vocabulary exact-term scans that establish both tops are
composition results absent from the source lexicon. The curated provenance rows
also retain an external dictionary row for every oracle-top constituent
(`好`/`逆`/`鐘` and `好`/`你`/`玩`); they do not rely on the absent whole phrase.

## Pinned inputs

- librime: `1.17.0` / `33e78140250125871856cdc5b42ddc6a5fcd3cd4`
- `rime.dll`: `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`
- `rime_deployer.exe`: `3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071`
- capture script: `3275379dad406606555d0674ca73ba479ce177cadaf3bc486544c7e51494b8b5`
- probe source: `94f7deb7c3632a6c3c918536295b03d88aa8a80bbbbc9d8a26e896fb70bf07e7`
- `rime/rime-double-pinyin`: `01a13287cbd27819be1c34fa1ddc1b3643d5001b`, tree `a1c64a175f1d4f79938fa6da560a633933be7c2d`
- `rime/rime-bopomofo`: `6085c9a38a4a728047862b33d67eee18aa86f3b9`, tree `7c372ce307b3db4f9cd6f4b4e7b2921c077ab5a1`
- dependencies are recorded by exact commit and tree in each fixture; every
  repository was clean before capture and rechecked unchanged afterward.

The output files are
`crates/yune-core/tests/fixtures/upstream-1.17.0/double-pinyin-m59-whole-input.json`
and
`crates/yune-core/tests/fixtures/upstream-1.17.0/bopomofo-m59-whole-input.json`.
Each fixture embeds a fully resolved replay command including all dependency
repositories and expected commits. The commands actually entered for this
capture were:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/capture-upstream-schema.ps1 -OracleRoot target/upstream-oracle/1.17.0 -SchemaId double_pinyin -SchemaDataRepo rime/rime-double-pinyin -InputSequence hknivs -Output crates/yune-core/tests/fixtures/upstream-1.17.0/double-pinyin-m59-whole-input.json -SourceRowPolicy m59_transformed_algebra_whole_input_oracle -CaptureMode m59-whole-input -CaptureDate 2026-07-10

powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/capture-upstream-schema.ps1 -OracleRoot target/upstream-oracle/1.17.0 -SchemaId bopomofo -SchemaDataRepo rime/rime-bopomofo -InputSequence cl3su3j06 -Output crates/yune-core/tests/fixtures/upstream-1.17.0/bopomofo-m59-whole-input.json -SourceRowPolicy m59_transformed_algebra_whole_input_oracle -CaptureMode m59-whole-input -CaptureDate 2026-07-10
```

## Key and scenario semantics

`RimeProbe.Capture` sends each input character to
`RimeProcessKey(keycode, 0)`. For Bopomofo, the numeric characters in
`cl3su3j06` are therefore schema key events: `3`, `0`, and `6` participate in
the active tone/keymap algebra. They are not pre-decoded separators and are not
candidate-selection assertions.

The `m59-whole-input` scenario list is exactly `paging_first_input` and
`commit_first_input_space`. It deliberately excludes the old generic numeric
`2` selection scenario. In `m19-component` mode the historical scenario family
remains available, but Bopomofo now labels that action as a tone-key action.
That changes future M19 replay metadata intentionally; the historical component
fixtures remain byte-identical at their previously pinned hashes.

The hardened script fails before workspace recreation when the output exists,
the output is under the oracle cache, a binary hash differs, a schema source is
dirty, or a source commit differs. Final output uses create-new UTF-8 without a
BOM after binaries, repositories, script, and probe are rechecked unchanged.
