# M55 Phase 2 Product-Path Parity Slice - 2026-07-03

Verdict: product-path candidate parity is green; Phase 2 is still not closed.

## Scope

This slice covers the M55 Execution Rule 4 product-path comparison for the Phase 2 runtime byte-backed poet storage change. It compares the current runtime-byte-backed commit against the immediately prior deploy-validation commit:

- Before: `261f22f9` (`Wire M55 poet artifact deploy validation`)
- Current: `f236f6b4` (`Load M55 poet runtime from artifact bytes`)
- Schema: `luna_pinyin`
- Shared data: `apps/yune-web/public/schema`
- User data: fresh empty user-data directory per commit lane; the first CLI invocation deploys/rebuilds product artifacts, and later invocations reuse that lane's freshly built artifacts.

The documented command shape includes a trailing space. In these frontend transcripts, that trailing `space` commits and leaves no candidate menu, so the byte comparison uses the final composing event whose `context.input` equals the requested input. The evidence JSON records both the candidate event index and the trailing commit event index.

## Commands

Current lane:

```powershell
cargo build -p yune-cli
target\debug\yune-cli.exe frontend --shared-data-dir apps/yune-web/public/schema --user-data-dir <fresh-current-user-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Before lane:

```powershell
git worktree add --detach C:\yui-m55b-8e0fcb 261f22f9
cargo build -p yune-cli
target\debug\yune-cli.exe frontend --shared-data-dir apps/yune-web/public/schema --user-data-dir <fresh-before-user-dir> --schema luna_pinyin --sequence "<input> " --output json
```

Inputs:

- `ceshiyixiachangjushuruxingnengzenyang`
- `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`
- `zhongguo`
- `jianli`
- `biancheng`

## Result

All five rows match byte-for-byte for the candidate text list at the final composing event, and all trailing-space commits match. The product-path deploy also wrote `luna_pinyin.poet.bin` in both lanes.

| Input | Candidate event | Candidate count | Top candidate | Candidate list match | Trailing commit match |
| --- | ---: | ---: | --- | --- | --- |
| `ceshiyixiachangjushuruxingnengzenyang` | `36` | `5` | `測試一下長足輸入性能怎樣` | yes | yes |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `58` | `5` | `這個引擎其實應該支持超長橘子樹肉菜能用` | yes | yes |
| `zhongguo` | `7` | `4` | `中國大陸` | yes | yes |
| `jianli` | `5` | `5` | `建立` | yes | yes |
| `biancheng` | `8` | `5` | `變成` | yes | yes |

Poet artifact bytes:

- Before `261f22f9`: `53,029,606` bytes
- Current `f236f6b4`: `28,542,082` bytes

Machine-readable detail: `product-path-parity-2026-07-03.json`.

## Remaining Phase 2 Gates

This product-path slice does not satisfy Phase 2 closeout by itself. Remaining gates:

- Diagnostic native Luna memory-owner proof is recorded in `memory-owner-proof-2026-07-03.md`; the release full-ratchet `memory-owner-profile.csv` remains required before Phase 2 closeout.
- Run the full M55 ratchet gate and record any latency tradeoffs, especially on 37/59-char and win rows.
- Tighten the Track A memory ceiling only after the required green full-ratchet runs.
