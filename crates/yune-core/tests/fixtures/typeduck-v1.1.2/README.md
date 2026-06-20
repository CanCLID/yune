# TypeDuck v1.1.2 Oracle Fixtures

These fixtures were captured from the real TypeDuck fork so later Yune behavior
changes do not guess candidate comment bytes.

## Provenance

- Engine: `TypeDuck-HK/librime` tag `v1.1.2`
- Engine commit: `74cb52b78fb2411137a7643f6c8bc6517acfde69`
- Engine archive: `rime-TypeDuck-v1.1.2-Windows-msvc-x64.7z`
- Dependency archive: `rime-deps-TypeDuck-v1.1.2-Windows-msvc-x64.7z`
- Dictionary lookup plugin: `TypeDuck-HK/rime-dictionary-lookup-filter`
- Plugin commit: `3e4605c4fae99f068df2edb85aaeab5a97752795`
- Schema: `TypeDuck-HK/schema`
- Schema commit: `1bed1ae6a0ab48055f073774d7dfd152a171c548`
- Captured on: 2026-06-18 and 2026-06-19, Windows

## Capture Notes

The schema archive was expanded into a scratch shared-data directory under
`target/typeduck-oracle/v1.1.2/`, then compiled with:

```powershell
rime_deployer.exe --build <user_data_dir> <shared_data_dir> <user_data_dir>\build
```

The capture caller loaded `rime.dll` with modules:

```text
default
dictionary_lookup
```

It selected `jyut6ping3_mobile`, sent the ASCII input sequences in
`jyut6ping3-mobile-comments.json`, and recorded schema identity, composition
state, menu metadata, and selected candidate records from the returned
`RimeContext`.

`jyut6ping3-fork-parity-01-real-dictionary-fuzzy.json` is a focused
2026-06-19 capture for FORK-PARITY-01. It uses deployed `jyut6ping3_mobile`
with the production-sized `translator.dictionary: jyut6ping3` table and
`dictionary_lookup_filter.dictionary: jyut6ping3_scolar` lookup table, both
127,144 source rows in the TypeDuck schema data. The single input `m` locks the
real fork behavior where the preserved Cantonese spelling algebra keeps both
the direct `m4` row and the fuzzy `ng5` row visible.

`jyut6ping3-fork-parity-02-prefer-user-phrase.json` is a focused 2026-06-19
capture for FORK-PARITY-02. It loads the real TypeDuck levers module and
imports three user dictionary rows: a low-commit equal-code row
(`YUNELOW	nei5	1`), a high-commit equal-code row
(`YUNEHIGH	nei5	100000000`), and a multi-syllable full-code row
(`YUNELONG	nei5 hou2	1`). The fixture locks the fork behavior where user
phrases are visible, equal-code rows are not preferred by code length alone, and
the multi-syllable user phrase ranks after the full system phrase but before
shorter competing rows.

`jyut6ping3-fork-parity-06-letter-to-tone.json` is a focused 2026-06-19 capture
for FORK-PARITY-06. It uses deployed `jyut6ping3_mobile` and locks the
TypeDuck `letter_to_tone`/`tone_to_letter` behavior where complete tone-letter
inputs such as `neivv` and `neiqq` display numeric Jyutping preedit (`nei4`,
`nei6`) while `RimeGetInput` still returns the raw ASCII letters. The `neix`
and `neiq` rows intentionally prove that partial or unmatched inputs can remain
raw in the composition preedit.

`jyut6ping3-m21-sentence-composition.json` is a focused 2026-06-20 capture for
M21-GAP-01. It locks the TypeDuck v1.1.2 `jyut6ping3_mobile` sentence
composition surface for `loengnincin`, `leoicijyu`, `ngohaigo`, and three
analogous cross-boundary dictionary inputs. The fixture is the hard oracle for
this gap; the deployed `typeduck.hk/web` product remains a feel target only.

The `comment` fields intentionally preserve the raw fork bytes as JSON escapes,
including leading `\f`, record separators `\r`, and multilingual dictionary
columns. Do not normalize these strings when using them as goldens.

`reverse-lookup-prompt.json` was captured from the same local v1.1.2 binary with
a scratch schema that has an affix reverse-lookup prompt named `HR6 粵語`, one
lookup row (`火	huo`) and two target rows (`火	ho`, `火	huo`). It locks the
fork's `"; "` multi-pronunciation joiner and prompt/preedit bytes for the HR-6
parity slice.
