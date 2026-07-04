# M55 Final Current Product-Path CLI Check - 2026-07-04

Verdict: green. Current-code yune-cli frontend over `apps/yune-web/public/schema` matches the Phase 2 product-path parity candidate lists for all five rows.

Command shape:

```powershell
target\debug\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
```

The first row used a fresh temporary user-data directory and paid deploy/rebuild cost; later rows reused that directory. Expected candidate lists come from `phase-2-poet-storage/product-path-parity-2026-07-03.json`.

| Input | Candidate event | Candidate count | Top candidate | Candidate list match |
| --- | ---: | ---: | --- | --- |
| `ceshiyixiachangjushuruxingnengzenyang` | 36 | 5 | 測試一下長足輸入性能怎樣 | True |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | 58 | 5 | 這個引擎其實應該支持超長橘子樹肉菜能用 | True |
| `zhongguo` | 7 | 4 | 中國大陸 | True |
| `jianli` | 5 | 5 | 建立 | True |
| `biancheng` | 8 | 5 | 變成 | True |

Machine-readable detail: `product-path-current-cli-2026-07-04.json`.
