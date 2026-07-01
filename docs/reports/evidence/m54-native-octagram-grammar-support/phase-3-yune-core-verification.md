# M54 Yune Core Verification

Date: 2026-07-01.

This evidence verifies the M54 implementation against the committed lotem
canonical oracle fixture and the committed RIME-LMDG validation fixture without
vendoring third-party `.gram` models. The machine-readable committed summary is
`phase-3-yune-core-verification.json`.

## Inputs

- Oracle fixture:
  `crates/yune-core/tests/fixtures/upstream-octagram/lotem-luna-pinyin-octagram.json`
- Dictionary/schema source root:
  `target/m54-native-octagram/external/schema-src`
- External grammar model:
  `target/m54-native-octagram/external/rime-octagram-data-hant/zh-hant-t-essay-bgw.gram`
- Model provenance:
  `lotem/rime-octagram-data`, `hant` commit
  `bb8e1313552f0f27f2f968031dfaf4563e55d982`, SHA256
  `574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3`

## Method

Temporary ignored harness:
`target/m54-native-octagram/yune-core-verify-harness`.

Command shape:

```powershell
cargo run --manifest-path target\m54-native-octagram\yune-core-verify-harness\Cargo.toml -- `
  target\m54-native-octagram\external\schema-src `
  target\m54-native-octagram\external\rime-octagram-data-hant\zh-hant-t-essay-bgw.gram `
  crates\yune-core\tests\fixtures\upstream-octagram\lotem-luna-pinyin-octagram.json `
  target\m54-native-octagram\yune-core-verify-lotem-integrated.json `
  nihao zhongguo youhuiyong jintianhuiyi jintianwanshangyouhui `
  gegeguojiayougegeguojiadeguoge woxiangqubeijing
```

The harness loaded the pinned `luna_pinyin.dict.yaml`, pinned `essay.txt`
preset vocabulary, parsed the external `.gram` through Yune's
`OctagramGrammar`, attached it to `StaticTableTranslator`, and compared the top
candidate for each committed oracle case.

Output report SHA256:
`18521c346272a998f816011fd891bfa1275eeae48b008f24465c6acfb3ed74e7`.

## Result

| Input | Oracle top | Yune top | Result |
| --- | --- | --- | --- |
| `nihao` | `你好` | `你好` | match |
| `zhongguo` | `中國` | `中國` | match |
| `youhuiyong` | `優惠用` | `優惠用` | match |
| `jintianhuiyi` | `今天會議` | `今天會議` | match |
| `jintianwanshangyouhui` | `今天晚上又會` | `今天晚上又會` | match |
| `gegeguojiayougegeguojiadeguoge` | `各個國家有各個國家的國歌` | `各個國家有各個國家的國歌` | match |
| `woxiangqubeijing` | `我想去北京` | `我想去北京` | match |

Notes:

- The report intentionally compares the accepted top candidate, not full first
  page order. Yune still has known table/menu ordering differences outside M54.
- The fix that closed `youhuiyong` and `jintianwanshangyouhui` was to prevent
  octagram-enabled normal phrase derivation from using zero-weight character
  codes such as `戲 hui 0%`; null-grammar behavior remains unchanged.

## RIME-LMDG Validation Lane

Additional ignored harness input:
`target/m54-native-octagram/external/RIME-LMDG-release/wanxiang-lts-zh-hant.gram`.

Model provenance:
`amzxyz/RIME-LMDG`, LTS release commit
`c78463a521aee2681db6cd6424a75a9b413237a3`, SHA256
`48085c1f87ca1a33ace42ffec13a3113f67606621586e25453e1a62ac55e1684`.

Command shape:

```powershell
cargo run --manifest-path target\m54-native-octagram\yune-core-verify-harness\Cargo.toml -- `
  target\m54-native-octagram\external\schema-src `
  target\m54-native-octagram\external\RIME-LMDG-release\wanxiang-lts-zh-hant.gram `
  crates\yune-core\tests\fixtures\upstream-octagram\rime-lmdg-luna-pinyin-validation.json `
  target\m54-native-octagram\yune-core-verify-rime-lmdg-integrated.json `
  nihao zhongguo youhuiyong jintianhuiyi jintianyouhui `
  jintianwanshangyouhui wanshanghuiyi gegeguojiayougegeguojiadeguoge `
  woxiangqubeijing
```

Output report SHA256:
`5e4b1daa1141be5d8b0634a87b0ba5d82db88ed4e472723710825e8898c854c3`.

| Input | Oracle top | Yune top | Result |
| --- | --- | --- | --- |
| `nihao` | `你好` | `你好` | match |
| `zhongguo` | `中國` | `中國` | match |
| `youhuiyong` | `又會用` | `又會用` | match |
| `jintianhuiyi` | `今天會議` | `今天會議` | match |
| `jintianyouhui` | `今天又會` | `今天又會` | match |
| `jintianwanshangyouhui` | `今天晚上又會` | `今天晚上又會` | match |
| `wanshanghuiyi` | `晚上會議` | `晚上會議` | match |
| `gegeguojiayougegeguojiadeguoge` | `各個國家有各個國家的國歌` | `各個國家有各個國家的國歌` | match |
| `woxiangqubeijing` | `我想去北京` | `我想去北京` | match |
