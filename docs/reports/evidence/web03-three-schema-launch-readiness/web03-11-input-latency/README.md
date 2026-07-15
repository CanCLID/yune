# WEB03-11 Browser Input-Latency Hard Stop

Status: complete at `ef485b10`.

## Final closeout boundary

WEB03-11 closed on 2026-07-15 at clean source
`ef485b102b3a5e75359e547008b47ed89eb89c7e`. The exact Cloudflare build
entrypoint passed from a disposable detached clone, the Git-integrated Pages
deployment `e4ad5c7b-4084-47f7-abe7-e2a034c443ef` succeeded, production
`build-info.json` named that exact clean source with Rust `1.96.1`, Emscripten
`4.0.23`, and Node `22.16.0`, and the source-pinned production canary passed.

The binding local 4x/4x lane recorded all 8 scenarios, all 186 keys, and all
178 cadence gaps on time. The deployed-origin lane also recorded 8/8,
186/186, and 178/178; it is a post-deploy canary with worker amplification
disabled, not a replacement for the binding loopback release profile. The
independent deployed 47-key Jyutping canary measured `43 ms` p95, `44 ms`
maximum, and `0 ms` maximum worker queue wait, with all 46 gaps on time.

Compact receipts are [`closeout-ef485b10.json`](./closeout-ef485b10.json),
[`local-release-gate-ef485b10.csv`](./local-release-gate-ef485b10.csv), and
[`deployed-canary-ef485b10.csv`](./deployed-canary-ef485b10.csv). Full JSON,
commands, and logs remain external at `$HOME/yune-web03-11-closeout-ef485b10`.
The compact packet hashes are pinned in
[`manifest-ef485b10.sha256`](./manifest-ef485b10.sha256).
The first deployed invocation passed but retained only its log because the
local-run output variable was used; the preserved, explicitly named
`deployed-canary-receipt-capture` repeated the same unchanged source/policy and
also passed. No measured red was discarded or retried.

The directly preceding clean `981ab059` Cloudflare measurement also remains a
red, preserved externally at `$HOME/yune-web03-11-closeout-981ab059`. Its exact
47-key normal row passed, but the binding `jyutping-short` row recorded only
1/2 on-time gaps (including a `395.4 ms` delay), the next row stopped at 27/28
diagnostics, and the aggregate timed out incomplete/fail. That source was not
retried. `ef485b10` is a corrective new source that removes hidden public-path
diagnostics polling; it does not cherry-pick the earlier run.

The still earlier `cce99c3c` red below also remains historical evidence and is
not rewritten by this closeout.

## Historical `cce99c3c` candidate boundary

The remainder of this document preserves the first complete release-profile
receipt after the WEB03-11 engine and browser-guard correctives. At that
historical boundary it was intentionally not called the final Cloudflare
verdict: the Git-integrated deployment for the same source commit returned a
generic failed check, while its authenticated build log was unavailable to that
session. The final `ef485b10` boundary above now supplies the later pass and
production canary without rewriting this history.

## Source and artifact identity

- Yune source commit: `cce99c3cb3e198c019fb75e8e8839b80550d5a5c`
- Source tree state: clean
- Pinned Emscripten SDK: `4.0.23`
- Emscripten release commit: `aaa43392544d695232b70eda706d751f18980c2a`
- emsdk repository commit: `db04e88298d9916fc51fcd3743045ca3eb695127`
- Rust: `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Node used by the package build: `v24.16.0`
- Schema payload: `132572773` bytes
- Schema manifest SHA-256:
  `5565dfbaab79ce2d5309418338c2ed446138f3958a0023bceebcfd4c6311be86`
- WASM SHA-256:
  `7180ef62ac1ea3c95e81f7031c116db8e32323864842c3bc1a8e15c6c1bbbfdc`
- Public-artifact manifest SHA-256:
  `b8f3c60baad01382bb9629f44beacee4773f7b3739d03f070cd94ae07083a784`
- Receipt SHA-256:
  `1e757e8a9909737673ff9a3c36d5495f1c98d695f7b1b6857df24434b36c8259`

No WASM, schema, worker, or other binary payload is copied into evidence.

## Binding local replay

The exact Cloudflare build entrypoint was replayed from a clean tracked tree
with the pinned toolchain and Playwright browser cache. The output directory was
outside the repository.

```powershell
& 'C:\Program Files\Git\bin\bash.exe' -lc '
set -euo pipefail
export YUNE_WEB_EMSDK_DIR=/c/tmp/yune-web03-emsdk
export PLAYWRIGHT_BROWSERS_PATH=/c/tmp/yune-web03-ms-playwright
export YUNE_WEB_LATENCY_OUTPUT_DIR=/c/tmp/yune-web03-final-local-gate
emcc(){ emcc.bat "$@"; }
export -f emcc
emar(){ emar.bat "$@"; }
export -f emar
bash apps/yune-web/public-demo/cloudflare-pages-build.sh
'
```

Verdict: PASS in 160 seconds. The immutable release profile was applicable and
valid with no diagnostic overrides: 4x Chromium main-thread throttling, 4x
loopback worker service amplification, 250 ms cadence, 186 verified keys, 178
valid cadence gaps, six visible candidates after every key, p95 `<= 750 ms`,
and maximum `<= 1000 ms`.

| Scenario | Input length | Median | p95 | Maximum | First candidate |
| --- | ---: | ---: | ---: | ---: | --- |
| `jyutping-short` | 3 | 65 ms | 169 ms | 169 ms | `係` |
| `jyutping-historical-long-1` | 28 | 41 ms | 67 ms | 128 ms | `時下場據輸入嘅速度` |
| `jyutping-historical-long-2` | 52 | 40 ms | 49 ms | 62 ms | `睇下如果打好場嘅句子個性能會點樣` |
| `typeduck-learned-userdb-prefix` | 3 | 48 ms | 99 ms | 99 ms | `我` |
| `luna-short` | 3 | 45 ms | 46 ms | 46 ms | `好` |
| `luna-37` | 37 | 47 ms | 171 ms | 196 ms | `測試一下長句輸入性能怎樣` |
| `luna-59` | 59 | 66 ms | 312 ms | 341 ms | `這個引擎其實應該支持超長句子輸入才能用` |
| `cangjie-short` | 1 | 46 ms | 46 ms | 46 ms | `日` |

The full unmodified receipt is
[`local-release-gate-cce99c3c.json`](./local-release-gate-cce99c3c.json).

## Cloudflare disposition at this boundary

The Git-integrated Pages deployment for `cce99c3c` did not publish:

- Deployment ID: `983487ea-1197-4081-ba2a-33b3ce3fe588`
- GitHub check-run ID: `87034949347`
- Check conclusion: `failure`
- Public check detail: `Build failed.`
- Authenticated build log: not available in this session

The public site consequently remained on an older package whose
`build-info.json` did not include `sourceCommit`. This red is preserved and is
not waived or converted into a release PASS by the local replay.
