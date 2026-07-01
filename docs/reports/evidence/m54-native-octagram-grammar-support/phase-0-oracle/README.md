# M54 Phase 0 Oracle Capture

Date: 2026-07-01.

This phase captured reproducible librime + octagram oracle outputs for M54.
The checked-in output fixtures are:

- `crates/yune-core/tests/fixtures/upstream-octagram/lotem-luna-pinyin-octagram.json`
- `crates/yune-core/tests/fixtures/upstream-octagram/rime-lmdg-luna-pinyin-validation.json`
- `crates/yune-core/tests/fixtures/upstream-octagram/synthetic-rear-boundary-oracle.json`
- `crates/yune-core/tests/fixtures/upstream-octagram/oracle-manifest.json`

## Oracle Build

The oracle binary is a source build of `rime/librime` `1.17.0`
(`33e78140250125871856cdc5b42ddc6a5fcd3cd4`) with
`lotem/librime-octagram` (`dfcc15115788c828d9dd7b4bff68067d3ce2ffb8`)
merged as a built-in module. Windows external DLL plugin loading is not used.

Build inputs:

- Visual Studio Community 2022 toolchain, x64.
- CMake `4.3.4` and Ninja `1.13.0` installed into
  `target/m54-native-octagram/venv`.
- Boost `1.89.0`, archive SHA256
  `85a33fa22621b4f314f8e85e1a5e2a9363d22e4f4992925d4bb3bc631b5a0c7a`.
- Official librime dependency asset
  `rime-deps-33e7814-Windows-msvc-x64.7z`.

Build outputs:

- `rime.dll` SHA256
  `c972d2a09b516176dfd62c57a293cd3ae57d06267f891e8c9331f85b6c761c38`.
- `rime_deployer.exe` SHA256
  `b24904f215c455d22ee089a65124885e18d562a534d594a997ff523708ce4034`.

## Captured Lanes

Canonical lane:

- Engine/schema: upstream `luna_pinyin`.
- Model source: `lotem/rime-octagram-data`, LGPL-3.0.
- Model: `zh-hant-t-essay-bgw.gram`, branch `hant` commit
  `bb8e1313552f0f27f2f968031dfaf4563e55d982`, SHA256
  `574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3`.
- Schema patch: `__include: grammar:/hant` and
  `translator/contextual_suggestions: false`.

Validation lane:

- Engine/schema: upstream `luna_pinyin`.
- Model source: `amzxyz/RIME-LMDG`, CC-BY-4.0.
- Model: `wanxiang-lts-zh-hant.gram`, LTS release, SHA256
  `48085c1f87ca1a33ace42ffec13a3113f67606621586e25453e1a62ac55e1684`.
- Schema patch: `grammar/language: wanxiang-lts-zh-hant`,
  `collocation_max_length: 6`, `collocation_min_length: 3`,
  `collocation_penalty: -14`, `non_collocation_penalty: -6`,
  `weak_collocation_penalty: -100`, and
  `translator/contextual_suggestions: false`.

## Observed Behavior

The canonical lotem lane includes unchanged negative controls (`nihao`,
`zhongguo`, `woxiangqubeijing`) and octagram-dependent rows:

- short: `youhuiyong`
- medium: `jintianhuiyi`, `jintianwanshangyouhui`
- long: `gegeguojiayougegeguojiadeguoge`

The RIME-LMDG lane validates the same grammar path with a larger real-world
model and records additional changed rows such as `jintianyouhui` and
`wanshanghuiyi`.

## Data Policy

No full lotem or RIME-LMDG `.gram` model file is checked in. The repository
stores oracle output bytes, URLs, commits/releases, checksums, licenses, and
attribution/provenance notes only. The only checked-in `.gram` bytes are the
tiny Yune-owned synthetic rear-boundary fixture used to make the OCTA-1 parity
regression executable.
