# Task 0 Target Selection And Data Policy

> **Status:** Complete - **Milestone:** M54 (native octagram grammar support) - **Updated:** 2026-07-01 - **Type:** evidence

## Verdict

Task 0 is complete. The lotem canonical oracle lane, RIME-LMDG validation lane, data licenses, model checksums, schema patch policy, contextual-translation decision, vendoring decision, and milestone numbering decision are pinned well enough to proceed to oracle capture.

M54 is an engine compatibility target because it covers the classic `poet` sentence/lattice grammar scoring path for named schemas. It is not an AI experiment, not a frontend/product request, and not a public performance milestone.

## Source Pins

| Source | Role | Pin |
| --- | --- | --- |
| `rime/librime` | default upstream engine oracle | tag `1.17.0` dereferenced to commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4` |
| `rime/rime-luna-pinyin` | canonical schema source for `luna_pinyin` | commit `18a80335c37522311f7cff02886cd81cec3b460a` |
| `rime/rime-prelude` | canonical schema dependency | commit `082425ea0684bca36474415d4a0e8db9b016487e` |
| `rime/rime-essay` | canonical vocabulary dependency | commit `48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed` |
| `rime/rime-stroke` | canonical schema dependency retained from existing upstream fixtures | commit `3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c` |
| `lotem/librime-octagram` | octagram plugin for oracle capture only | `HEAD` commit `dfcc15115788c828d9dd7b4bff68067d3ce2ffb8` |
| `lotem/rime-octagram-data` | canonical oracle config branch | `master` commit `8ceef1b42eb77e86501382a52e85c309c0f2f04c` |
| `lotem/rime-octagram-data` | canonical simplified model branch | `hans` commit `9c482c6660fa9e3268bd1d1a9341ef26aa90f94d` |
| `lotem/rime-octagram-data` | canonical traditional model branch | `hant` commit `bb8e1313552f0f27f2f968031dfaf4563e55d982` |
| `amzxyz/RIME-LMDG` | validation repo and license source | `HEAD` commit `f00fc2bf2b9b393f7a5c335a7067fa80a1f24f13`; tag `LTS` commit `c78463a521aee2681db6cd6424a75a9b413237a3` for LTS model release |

## License Verification

Verified from the current repository `LICENSE` files on 2026-07-01:

| Source | License | Evidence |
| --- | --- | --- |
| `lotem/rime-octagram-data` | LGPL-3.0 | `LICENSE` file SHA-256 `ea7d049c7705dc13afc202dd18e1827f3484f8212fd3fa7b82fc4a0c363432c9` |
| `amzxyz/RIME-LMDG` | CC-BY-4.0 | `LICENSE` file SHA-256 `cbd5af318286b74656f145dff5091907cc23d6d8c9ad09e0291ec2497792ba41` |
| `lotem/librime-octagram` | GPL-3.0 | `LICENSE` confirms GPL-3.0; plugin source is oracle/build reference only and must not be copied into Yune implementation |

## Model Checksums

Canonical lotem model branch `hans`:

| Model file | Bytes | SHA-256 |
| --- | ---: | --- |
| `zh-hans-t-essay-bgc.gram` | 4,114,432 | `eb33e589767c9ee0e262d4d8a9c040294fbfbb8ffaa4efe21c7338987c3d2924` |
| `zh-hans-t-essay-bgw.gram` | 10,404,864 | `11bbeebb84e321dc2617d5fe4c1b3c51945703ed779a12e12b8b0270e1baec79` |

Canonical lotem model branch `hant`:

| Model file | Bytes | SHA-256 |
| --- | ---: | --- |
| `zh-hant-t-essay-bgc.gram` | 4,075,520 | `3cc18e6c8d1ff9650f4b0be9681f414a0915eaa85a593df9e49ca7dc08dbe7fe` |
| `zh-hant-t-essay-bgw.gram` | 10,513,408 | `574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3` |

RIME-LMDG validation model release:

| Model file | Bytes | SHA-256 |
| --- | ---: | --- |
| `wanxiang-lts-zh-hans.gram` | 420,538,412 | `491d05ceb4a058b11f51ec4fe2cf83c092bfa042526f74ccc56977a209a2bd72` |
| `wanxiang-lts-zh-hant.gram` | 421,967,916 | `48085c1f87ca1a33ace42ffec13a3113f67606621586e25453e1a62ac55e1684` |

RIME-LMDG repository compare fixture noted but not selected as the default validation model:

| File | Bytes | SHA-256 |
| --- | ---: | --- |
| `rime-schema-compare/vendor/rime-frost_with_gram/zh-moqi.gram` | 7,339,052 | `7b9a593ce9f58bd538048cef11f825c3cd86307030552726d58785dd87d98ae9` |

## Schema Patch Decisions

Canonical oracle lane starts from upstream `luna_pinyin` and uses lotem traditional word-oriented grammar data, matching the checked-in traditional-output oracle fixtures:

```yaml
patch:
  __include: grammar:/hant
  translator/contextual_suggestions: false
```

The included `grammar:/hant` config maps to:

```yaml
grammar:
  language: zh-hant-t-essay-bgw
translator/max_homophones: 7
translator/max_homographs: 7
```

Rationale: existing upstream `luna_pinyin` fixtures use traditional output bytes, and the captured M54 oracle rows compare traditional candidate text. The `hant` lane produced observable octagram-dependent movement and is the selected canonical lane for committed M54 fixtures.

RIME-LMDG validation lane uses the documented LTS traditional model with contextual suggestions disabled:

```yaml
patch:
  __include: octagram

octagram:
  __patch:
    grammar:
      language: wanxiang-lts-zh-hant
      collocation_max_length: 6
      collocation_min_length: 3
      collocation_penalty: -14
      non_collocation_penalty: -6
      weak_collocation_penalty: -100
    translator/contextual_suggestions: false
    translator/max_homophones: 8
```

## Contextual Translation Decision

M54 keeps contextual translation out of scope. Oracle capture must use same-session composing candidate behavior and must set `translator/contextual_suggestions: false` for both external lanes. If a future target needs after-commit contextual suggestions, that needs a separate plan and oracle evidence.

## Vendoring Decision

- Do not vendor full lotem `.gram` files into routine Yune fixtures. They are external canonical oracle inputs pinned by branch commit and SHA-256. Commit oracle output bytes and fixture manifests instead.
- Do not vendor RIME-LMDG LTS `.gram` files. The selected validation assets are about 420 MB each and are CC-BY-4.0 with attribution obligations. Pin release URL/digest and commit output evidence instead.
- Yune-owned unit tests should use generated synthetic tiny `.gram` data for parser/scoring tests. Synthetic fixtures must be generated from Yune code or checked-in Yune-owned bytes, not copied from third-party `.gram` models.

## M54 Numbering

The live roadmap had a tentative M54 native Track A memory-research candidate and no active numbered engine milestone. M54 is adopted for octagram because it is now the concrete named compatibility target. Track A memory research moved to the tentative M55 candidate during closeout.

## No-Go Review

No Task 0 no-go fired:

- The lotem canonical oracle stack is pinned and reproducible from public commits and model checksums.
- The RIME-LMDG validation stack is pinned from the `LTS` release asset digests and repository license.
- Model-data licenses were verified from current `LICENSE` files.
- The vendoring policy avoids committing large third-party model files.
- The implementation path remains clean-room and native-Rust; plugin source is oracle/build reference only.
- Public C ABI changes remain out of scope.

## Commands And Sources

Representative commands run from the repo root:

```powershell
git ls-remote https://github.com/rime/librime.git "refs/tags/1.17.0^{}"
git ls-remote https://github.com/lotem/librime-octagram.git HEAD
git ls-remote --heads https://github.com/lotem/rime-octagram-data.git
git clone --depth 1 --branch hans https://github.com/lotem/rime-octagram-data.git target\m54-native-octagram\external\rime-octagram-data-hans
git clone --depth 1 --branch hant https://github.com/lotem/rime-octagram-data.git target\m54-native-octagram\external\rime-octagram-data-hant
git ls-remote https://github.com/amzxyz/RIME-LMDG.git HEAD
Invoke-RestMethod -Uri https://api.github.com/repos/amzxyz/RIME-LMDG/releases/tags/LTS -Headers @{ 'User-Agent' = 'Codex-M54' }
Get-FileHash -Algorithm SHA256 target\m54-native-octagram\external\rime-octagram-data-hans\*.gram
Get-FileHash -Algorithm SHA256 target\m54-native-octagram\external\rime-octagram-data-hant\*.gram
```
