# M54 Upstream Octagram Fixtures

These fixtures were captured on 2026-07-01 for M54 native
octagram-compatible grammar support.

They store oracle output bytes and provenance only. The third-party `.gram`
models are not checked in; each fixture records the model URL, commit or
release, byte size, SHA256, and license.

## Files

- `lotem-luna-pinyin-octagram.json` - canonical oracle lane: upstream
  `luna_pinyin` with `lotem/librime-octagram` and
  `lotem/rime-octagram-data` `zh-hant-t-essay-bgw.gram`.
- `rime-lmdg-luna-pinyin-validation.json` - real-world validation lane:
  upstream `luna_pinyin` with the RIME-LMDG LTS Hant model and README
  octagram parameters.
- `oracle-manifest.json` - fixture hashes and no-vendoring notes.

Both lanes force `translator/contextual_suggestions: false`; M54 validates the
classic `poet`/grammar path, not contextual translation.
