# WEB-04 Phase 0 Native Product-Path Evidence

Date: 2026-07-01.

Verdict: **no-go**. The dedicated `luna_pinyin_octagram` native product path
does not reproduce the fresh librime + octagram oracle ranking. Browser/UI and
asset-plumbing tasks remain blocked.

## Inputs

Pinned model:

- Source: `lotem/rime-octagram-data`
- License: `LGPL-3.0`
- Branch/commit: `hant`
  `bb8e1313552f0f27f2f968031dfaf4563e55d982`
- Model: `zh-hant-t-essay-bgw.gram`
- URL:
  `https://raw.githubusercontent.com/lotem/rime-octagram-data/bb8e1313552f0f27f2f968031dfaf4563e55d982/zh-hant-t-essay-bgw.gram`
- Size: `10513408` bytes
- SHA256:
  `574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3`

No third-party `.gram` bytes are committed. The verified model bytes live only
under ignored `target/` paths for this run.

## Temporary Native Layout

Temporary Yune shared data:
`target/web04-octagram-debug-harness/phase-0-native/shared`.

Important files:

- `default.yaml` listed both `luna_pinyin` and `luna_pinyin_octagram`.
- `luna_pinyin_octagram.schema.yaml` was a temp profile with
  `schema/schema_id: luna_pinyin_octagram`, `translator/contextual_suggestions:
  false`, and inline `grammar/language: zh-hant-t-essay-bgw`.
- `zh-hant-t-essay-bgw.gram` was copied into the temp shared-data root after
  checksum verification.
- There was no `grammar.yaml` in the temp shared-data root, so plain
  `luna_pinyin` could not consume a shared `grammar:/hant` node.

Plain Luna control shared data:
`target/web04-octagram-debug-harness/phase-0-native/shared-plain-control`.

That control root was copied from `apps/yune-web/public/schema`, did not include
the octagram profile, and did not include the `.gram` model.

## Commands

Build:

```powershell
cargo build -p yune-cli
```

Product-path command shape used for each WEB-04 input:

```powershell
target\debug\yune-cli.exe frontend `
  --shared-data-dir target\web04-octagram-debug-harness\phase-0-native\shared `
  --user-data-dir target\web04-octagram-debug-harness\phase-0-native\user\direct-luna_pinyin_octagram `
  --schema luna_pinyin_octagram `
  --sequence <input>
```

Equivalent `cargo run` shape:

```powershell
cargo run -p yune-cli -- frontend `
  --shared-data-dir target\web04-octagram-debug-harness\phase-0-native\shared `
  --user-data-dir target\web04-octagram-debug-harness\phase-0-native\user\direct-luna_pinyin_octagram `
  --schema luna_pinyin_octagram `
  --sequence <input>
```

Plain Luna negative-control command shape:

```powershell
target\debug\yune-cli.exe frontend `
  --shared-data-dir <shared-or-shared-plain-control> `
  --user-data-dir <fresh-user-dir> `
  --schema luna_pinyin `
  --sequence <input>
```

Fresh librime oracle base capture:

```powershell
powershell -ExecutionPolicy Bypass -File target\m54-native-octagram\capture-octagram-lotem.ps1 -RepoRoot .
```

The base M54 script was then reused without editing committed source: the
already-deployed fresh `octagram` and `null-grammar` lanes under
`target/m54-native-octagram/oracle-capture/lotem` were captured with
`scripts/oracle-rime-probe.cs` over the WEB-04 inputs:

```powershell
Add-Type -Path scripts\oracle-rime-probe.cs
[RimeProbe]::Capture($GrammarShared, $GrammarUser, $GrammarBuild,
  'luna_pinyin', [string[]]@('default','octagram'), $Inputs)
[RimeProbe]::Capture($NullShared, $NullUser, $NullBuild,
  'luna_pinyin', [string[]]@('default'), $Inputs)
```

Fresh oracle output bytes:

| Output | Bytes | SHA256 |
| --- | ---: | --- |
| `target/web04-octagram-debug-harness/phase-0-native/oracle-output/fresh-librime-lotem-octagram-web04-inputs.json` | 13710 | `289a57b32bde77c77a9bd77e1bdd0b57ec77e56c79e70a63da9e5f693885197d` |
| `target/web04-octagram-debug-harness/phase-0-native/oracle-output/fresh-librime-lotem-null-web04-inputs.json` | 13710 | `e96247e1f8a20ab17b1effc13f196843e828387091070f058296132469d22259` |
| `target/web04-octagram-debug-harness/phase-0-native/oracle-output/fresh-librime-lotem-comparison-web04-inputs.json` | 1344 | `cc6e6937230fea1eb1834da171aaa55eba4544c85483d37601c7b457d98beea1` |

## Result

The fresh librime oracle shows octagram-visible ranking changes for all four
WEB-04 rows. The deployed Yune dedicated profile does not match any of those
oracle tops:

| Input | Fresh librime octagram top | Yune `luna_pinyin_octagram` top | Match |
| --- | --- | --- | --- |
| `youhuiyong` | `優惠用` | `有會用` | no |
| `jintianhuiyi` | `今天會議` | `進提按會一` | no |
| `jintianwanshangyouhui` | `今天晚上又會` | `進提按玩上有會` | no |
| `gegeguojiayougegeguojiadeguoge` | `各個國家有各個國家的國歌` | `個個股哦及啊有個個股哦及啊的股哦個` | no |

Machine-readable detail is in `comparison.json`.

## Negative Control

Plain `luna_pinyin` remained stable with the octagram profile and model present.
The same plain Luna commands were run against:

- `target/web04-octagram-debug-harness/phase-0-native/shared`
- `target/web04-octagram-debug-harness/phase-0-native/shared-plain-control`

For `youhuiyong`, `jintianhuiyi`, `jintianwanshangyouhui`,
`gegeguojiayougegeguojiadeguoge`, `nihao`, and `zhongguo`, the plain Luna top-5
lists matched exactly between those two roots. The comparison file is:
`target/web04-octagram-debug-harness/phase-0-native/yune-cli-output/plain-negative-control-comparison.json`.

This proves the temporary dedicated profile/model did not turn on grammar for
plain `luna_pinyin`.

## Engine Observation

The no-go is consistent with the current native installer gate:

- `crates/yune-rime-api/src/schema_install.rs:457-475` attaches preset Luna
  vocabulary, octagram grammar, and the upstream sentence model only inside
  `is_upstream_luna_pinyin_profile`.
- `crates/yune-rime-api/src/schema_install.rs:857-868` returns true only for
  component `script_translator`, dictionary `luna_pinyin`, and
  `schema/schema_id == "luna_pinyin"`.

The WEB-04 prerequisite requires a dedicated `luna_pinyin_octagram` profile to
keep plain Luna default-off. That profile currently bypasses the native
upstream-Luna grammar path, so this is an engine/product-path defect for a
separate native milestone, not a browser harness task.

## Boundary Checks

- No browser implementation was started.
- No `apps/yune-web` asset plumbing was started.
- No public C ABI was changed.
- No third-party `.gram` bytes were committed.
- Task 1 and later WEB-04 work remain blocked by this native no-go.
