# M59 Increment 2 informational checkpoint

This checkpoint is informational under the owner-locked full Path A. It does
not retain a rescope option and does not move any work out of M59. The next
engine increments remain 3a and 3b, followed by the serialized 4a/4b protocol.

## Transformed-algebra repro state

The planned whole-input rows `hknivs` and `cl3su3j06` do **not** occur in any
tracked fixture or test yet. The existing pinned captures prove only component
inputs:

| Fixture | Pinned schema source | Captured component rows | Current owning suite |
|---|---|---|---|
| `double-pinyin-basic.json` (`2f17053131d73028f315229fe7f22df226fc4b67f3b224e19ce99ed2bf864d24`) | `rime/rime-double-pinyin@01a13287cbd27819be1c34fa1ddc1b3643d5001b` | `hk -> 好`, `ni -> 你`, `vs -> 中` | 3 passed / 1 sentence-lattice ignored |
| `bopomofo-basic.json` (`3288e14306c3fc1cfe53e10f0bb743afa02e514d3bafe652f544e423f1047c70`) | `rime/rime-bopomofo@6085c9a38a4a728047862b33d67eee18aa86f3b9` | `cl3 -> 好`, `su3 -> 你`, `j06 -> 玩` | 3 passed / 3 ignored |

Component rows do not authorize concatenated expected strings. In particular,
neither `好你中` nor `好你玩` is asserted here. Increment 3b must first create
fresh whole-input oracle captures with the pinned capture script (SHA-256
`b23e07a8626243fb6e08477f48b0efbf22f0118c82799cbfe4eb01b6ecf0ca96`):

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File scripts/capture-upstream-schema.ps1 `
  -OracleRoot target/upstream-oracle/1.17.0 `
  -SchemaId double_pinyin `
  -SchemaDataRepo rime/rime-double-pinyin `
  -InputSequence hknivs `
  -Output target/m59-i3b-algebra/double-pinyin-hknivs.json

powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File scripts/capture-upstream-schema.ps1 `
  -OracleRoot target/upstream-oracle/1.17.0 `
  -SchemaId bopomofo `
  -SchemaDataRepo rime/rime-bopomofo `
  -InputSequence cl3su3j06 `
  -Output target/m59-i3b-algebra/bopomofo-cl3su3j06.json
```

These direct-construction fixtures are diagnostic inputs, not substitutes for
the required default-on/explicit-false deploy-path matrix.

## Expanded CJ-1 fixture

The current authoritative fixture is
`crates/yune-core/tests/fixtures/upstream-1.17.0/cangjie5-composition.json`
(66,584 bytes, SHA-256
`24408c3b2b83db516ae1382d2ba743b41ead50c7c026aee2837a01137c7ecbcf`).
It contains 12 complete all-page cases. The three owner rows are upstream
candidate zero (`hwmvsqtt -> 粵拼`, `ebcnyripm -> 測試`, and
`takohaeosk -> 莫伯洢`); `hdaetcu -> 香港` is the whole-input control.

The fresh owning suite remains 2 passed / 1 ignored. The executable exact
diagnostic remains 4/12 passed and 8/12 failed, with no exception. CJ-1 is
narrowly identified as upstream-lane segmentation scoring: Yune chooses the
eight-root `h|w|m|v|s|q|t|t` path (`竹田一女尸手廿廿`) instead of
`hwmvs|qtt -> 粵拼`. Separate product-path evidence already proves composition,
so this is not classified as a missing composition capability.

## OpenCC variant inventory

`opencc-same-code-inventory.csv` is the complete deterministic checkpoint
inventory for the pinned canonical source. It was produced by
`scripts/inventory-opencc-same-code.ps1` (SHA-256
`5dc88c282a7dd8dec6d4c43a5be08e90d6b6b7276c383d5decd704e0c7b363f7`)
against clean
`rime/rime-cantonese@c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0`
(tree `eb193fb80675ffa60df3c32bf24afa7d7f68617a`) and
`HKVariantsFull.txt` SHA-256
`145b561c68a697d5f2197da0c091caf4a0e9457f0a4c56cdf2ae7ad4b8ff8cc2`.
The generator derives its complete table surface from pinned
`jyut6ping3.dict.yaml` SHA-256
`4301001fb7bb52d5d1a9c032c519ac18ba50677e926e01006e34a48788385efa`
(`jyut6ping3.chars`, `words`, `phrase`, `lettered`, and `maps`), scans 473,867
dictionary data rows, 65 ordered OpenCC mappings, and 131 unique output texts,
and produces 83 exact-code sibling groups. Commit, tree, manifest hash/imports,
and OpenCC hash are embedded in every CSV row. All three expected pins are
mandatory; output aliases to the mapping, manifest, or imported dictionaries
are rejected; replacement uses a same-directory temporary file. A second
generation was byte-identical (CSV SHA-256
`01522f437038a3591d3a3b92cbdace2cced1b1e9076e566ca40662c736afcaf1`).

Exact replay command:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass `
  -File scripts/inventory-opencc-same-code.ps1 `
  -DictionaryRoot target/upstream-oracle/1.17.0/schema-src/rime-cantonese `
  -Output target/m59-i2-opencc-replay.csv `
  -ExpectedDictionaryCommit c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0 `
  -ExpectedDictionaryTree eb193fb80675ffa60df3c32bf24afa7d7f68617a `
  -ExpectedOpenCcSha256 145b561c68a697d5f2197da0c091caf4a0e9457f0a4c56cdf2ae7ad4b8ff8cc2
```

Representative groups include `bai3: 秘 祕`, `bei3: 秘 祕`,
`zung2/zung3: 粽 糉 糭`, `toi4: 台 臺`, and `wai6: 為 爲`. Current Yune
parses only the first OpenCC output and mutates a candidate in place before
deduplication; the concrete D-48 symptom is oracle `秘@8, 祕@9` versus Yune
retaining only `秘`. The inventory prices 4c but does not itself prove librime
`ConvertWord` ordering, stable deduplication, or partial-segmentation semantics;
those still require fresh oracle fixtures in 4c.

## Relative effort and risk

No repository evidence supports calendar estimates, so this checkpoint uses
relative size rather than invented day counts.

| Increment | Relative effort / risk | Evidence-based reason |
|---|---|---|
| 4a sentence/phrase ordering | Very high; current long pole | `being`, `beingo`, `beixngoxx`, and `zijiguk` diverge at index 0; `mgoi` diverges at index 1. Multiple sentence paths and model/storage activation must be reconciled while TypeDuck stays isolated. |
| 4b abbreviation/segmentation | Very high; performance-critical | Current synthesis is restricted to exactly two toned syllables with initial `m`; graph bounds are also explicit. The exact gaps are large (`n` 455 vs 1309, `nri` 0 vs 1309, `ngohaig` 46 vs 2050). The signed `n` ceiling is 3.006 and the fresh worst is 2.888, only about 3.9% headroom, so the declared owner stop is realistic. |
| 4c OpenCC variants | Medium implementation breadth; high acceptance/data risk | The conversion code is localized, but the general simplifier/uniquifier chain and 83 canonical same-code groups are affected. Fresh whole-word and partial-segmentation oracle evidence is mandatory. |

Path A continues unchanged. 4a and 4b remain Fable-blocking after landing, and
4b remains performance-blocking before landing if any signed short-key row is
red.
