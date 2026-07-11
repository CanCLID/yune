# M59 Increment 3b whole-input oracle capture

This packet closes the provenance prerequisite for the Double Pinyin and
Bopomofo transformed-algebra rows. The docs evidence directory contains no
Yune output and no binary payload. Four tiny upstream prism binaries live only
under the owning `yune-core` test-fixture directory so the production parser
can execute the external bytes. The expected strings below came from fresh upstream librime `1.17.0`
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

## Algebra-property mini-prisms

Four additional minimal schemas were deployed with the same pinned librime
binary to make spelling-property composition executable. Their small
upstream-generated prism bytes are intentionally checked in under
`crates/yune-core/tests/fixtures/upstream-1.17.0/`; no product-table binary is
copied into docs evidence. The exact source hashes, formulas, emitted prism
hashes, and independently decoded rows are recorded in
`crates/yune-core/tests/fixtures/upstream-1.17.0/m59-algebra-properties.json`.
The Python-standard-library decoder is
`scripts/decode-m59-algebra-prisms.py` at SHA-256
`3cc69624efe65d6ab518768d8556fb2d331ea2b2405d6e4120c051757300bdb7`.
It validates `Rime::Prism/4.0`, relative pointers, Darts exact matches,
descriptor packing and f32 bits, tips, and null-map identity semantics without
importing or invoking Yune's parser. Reproduce the observations with:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/replay-m59-algebra-oracle.ps1 -OracleBinaryRoot target/upstream-oracle/1.17.0/extract/dist -Workspace target/upstream-oracle/1.17.0/m59-algebra-properties-replay-fresh
```

The replay requires a previously absent disposable workspace. It binds the
pinned deployer and DLL, the manifest-registered fixture, the replay script,
and all nine checked-in source representations. Six repository text files
intentionally omit one capture-only final blank LF; that byte is restored only
when the resulting byte count and SHA-256 match the captured source. The script
also pins and checks the source mtime seconds embedded by librime in each
deployed schema, requires all four generated prism byte hashes to match the
checked-in artifacts, and invokes the decoder with `--verify` only. It never
rewrites the fixture or artifact observations.

`crates/yune-core/tests/upstream_algebra_properties.rs` first runs Yune's
production parser over those external bytes, then rebuilds the same observable
rows through Yune and compares both paths with the independent observations.

| Case | Formulas | Upstream observation | Emitted prism SHA-256 |
|---|---|---|---|
| Same-surface collision | `fuzz`, `abbrev`, ordinary `derive`, correction `derive`, all to `hx` | librime merges the four same-syllable rows into one normal descriptor | `b335c754e54ae5b30f713bc4a0cc29e853f51639eeed4541f8f57a9598992b88` |
| Property preservation | `abbrev`, fuzz `derive`, correction `derive`, then ordinary `xform/x$/z/` | `bz`, `cz`, and `dz` retain abbreviation, fuzz, and correction properties respectively | `66f07687794f39e22c2233832dbec2865f5712de914037c90055a767c5f01a86` |
| Cumulative credibility | `fuzz`, then two correction `derive` rules on one path | librime accumulates double and emits `di` credibility bits `0xC11E74AF`, one ULP above stepwise f32 | `7b1fa33c638b1a9a15071b3221927d0559d3f79cbe300fca4e52c0a2cc56906d` |
| Partial erase | `erase/a/` over `hao` | the partial match is inert: `hao` remains and `ho` is absent | `d0941d5b6950e278bcdbbc2b3c7677697696bd4f9e8e8dc43f4dc87db261ae9d` |

The semantic references are the pinned librime commit's
`src/rime/algo/spelling.{h,cc}`, `src/rime/algo/calculus.cc`,
`src/rime/dict/dict_compiler.cc`, `src/rime/dict/prism.cc`,
`src/rime/algo/syllabifier.cc`, and `src/rime/gear/script_translator.cc`.
They establish double-precision composition, type-aware correction merging,
all-or-nothing algebra loading into an identity prism after any invalid
definition, implicit null-map descriptors, and correction descriptors as
penalized exact aliases rather than default prefix-reachability edges.

## Deployed correction-spelling order

A fresh pinned-librime capture resolves the previously synthetic correction
subset. With equal-weight source rows `粗<TAB>cu<TAB>0` and
`錯<TAB>cuo<TAB>0`, algebra `derive/^cuo$/cu/correction`, and correction,
completion, and sentence generation all explicitly false, input `cu` returns
the complete terminal order `粗 (cu)`, then `錯 (cuo)`. The independently
decoded prism gives the second edge correction credibility bits `0xC0935D8E`.

The curated text-only fixture is
`crates/yune-core/tests/fixtures/upstream-1.17.0/m59-correction-spelling.json`
at SHA-256
`3c0bac5072f122d64398c0e51dc02e1d4edd17ee685f99cdee76a5ef83dc77da`.
It binds the raw all-pages capture SHA-256
`f5e1cf58cca162c03eadf71473b80376e440c044958ead3ca25e27e36565eee9`
and the verified disposable-capture manifest SHA-256
`4cb688f0624a7c19dd7a35b506aec0f30419f62a4eee0f93911d8caf7c6dcf48`.
No deployed table, prism, or oracle binary is copied into docs evidence. The
owning regression asserts the exact text/comment order and equal emitted
qualities across heap/source, owned compact, and byte-backed compact paths.

The preserved replay packet is the repo-relative
`increment-3b-transformed-algebra/correction-oracle-text/` directory. It keeps
the byte-identical raw capture (`f5e1...eee9`), provenance, independent prism
observation (`dbc875...e1a`), commands, source schema/dictionary/config,
capture/finalization scripts, decoder, probe source, non-empty logs, and the
external manifest records. Its
`repo-text-subset-manifest.json` at SHA-256
`18957375d362a391769658e2e10cbee9b54a16a10cd27740907d9b6e86a5d251`
records both origin and repository hashes for every retained file and names
every intentionally omitted binary, generated table/prism/reverse payload,
user database, built config, and zero-byte log. Verify the repository subset
with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-3b-transformed-algebra/correction-oracle-text/verify-repo-text-subset.ps1
```

The external full manifest was verified before import at SHA-256
`4cb688f0624a7c19dd7a35b506aec0f30419f62a4eee0f93911d8caf7c6dcf48`
over 36 artifacts. Its repository copy is deliberately named
`external-artifact-sha256.normalized.csv`: `.gitattributes` normalizes that CSV
from BOM/CRLF to LF, so its repository hash differs and it is not represented
as a locally self-verifying copy. The original manifest hash record and
verification JSON are retained beside it; the subset verifier is authoritative
for the checked-in text packet. No oracle DLL/EXE or generated binary payload
is checked in.

**Explicit 4b residual:** this increment does not claim general property parity
for heap/source `ExpandedSpellingCode` collisions. Normal-correction versus
fuzzy collision provenance is fixed and covered here, and the prism writer,
compiled/runtime prism path, and deployed leading index use pinned
`SpellingProperties::Update` semantics. The pre-existing fuzzy-versus-
abbreviation heap collision still lacks full spelling-type precedence; exact
abbreviation metadata/graph closure belongs to serialized increment 4b and must
be tested there before the heap path can claim general collision parity.

The partial-erase deployment uses librime's identity-prism optimization: its
raw spelling-map offset is null, so the accessor supplies the default
`syllable_id == spelling_id` descriptor. The independent decoder and Yune's
production parser both execute that representation. Yune's writer now emits
the same null-map form for a pure identity deployment and writes canonical
syllable ids into Darts leaves (including unsorted syllabaries). The runtime
retains external Darts bytes in the byte source and models the null map as
constant-size identity metadata; it does not allocate one descriptor vector
per spelling.

The tracked product `stroke.table.bin` / `stroke.prism.bin` regression loads
all 157,000 spellings and proves the null-map owner, reachability seed, and
reachability index remain zero-allocation before and after successful
37-character and 59-character callers plus an 84-character authoritative miss.
Those are focused direct-traversal boundary controls, not substitutes for the
fresh signed Track A performance rounds. The previously attempted eager seed
took more than four minutes of CPU on this fixture and was discarded; the
direct common-prefix regression completes in under one second on the same
machine.
