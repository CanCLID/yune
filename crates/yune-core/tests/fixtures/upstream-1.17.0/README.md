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
- Prefer the official upstream release binary for behavioral byte capture when
  available. The local source build is a reproducibility cross-check, not the
  primary behavioral oracle.
- Expected bytes must come from upstream librime, never from Yune.
- Every JSON fixture in this directory must include an `oracle` object with the
  engine, tag, commit, capture date, capture command, schema, and input sequence.
- If a case cannot be captured, keep the Yune test ignored with a `panic!()` body
  and document the exact command that would unblock it.

## Captured Fixtures

### `luna-pinyin-basic.json`

- Schema: `luna_pinyin`
- Upstream schema data: `rime/rime-luna-pinyin`
- Schema-data dependencies: `rime/rime-prelude`, `rime/rime-essay`, and
  `rime/rime-stroke`
- Inputs: `ni`, `hao`, `zhong`, `guo`, `zhongguo`
- Capture command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-luna-pinyin.ps1 -OracleRoot target/upstream-oracle/1.17.0 -Output crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json
```

The active Yune check is:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity
```

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
