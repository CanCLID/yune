# WEB03-11 Browser Input-Latency Hard Stop

Status: candidate evidence; Cloudflare acceptance remains pending.

This directory preserves the first complete release-profile receipt after the
WEB03-11 engine and browser-guard correctives. The receipt is intentionally not
called the final Cloudflare verdict: the Git-integrated deployment for the same
source commit returned a generic failed check, while its authenticated build log
was unavailable to this session. A later source-current Cloudflare PASS and a
post-deploy production canary are still required before WEB03-11 can close.

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
