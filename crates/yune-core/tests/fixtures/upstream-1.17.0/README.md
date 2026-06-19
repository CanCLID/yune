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
- Captured for: M12 upstream oracle refresh

## Capture Rules

- The local upstream checkout may be used as a build cache, but the local path is
  not part of fixture identity.
- Expected bytes must come from upstream librime, never from Yune.
- Every JSON fixture in this directory must include an `oracle` object with the
  engine, tag, commit, capture date, capture command, schema, and input sequence.
- If a case cannot be captured, keep the Yune test ignored with a `panic!()` body
  and document the exact command that would unblock it.
