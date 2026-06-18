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
- Captured on: 2026-06-18, Windows

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

The `comment` fields intentionally preserve the raw fork bytes as JSON escapes,
including leading `\f`, record separators `\r`, and multilingual dictionary
columns. Do not normalize these strings when using them as goldens.
