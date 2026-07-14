# M59 source-current performance revalidation (2026-07-13)

This packet closes the post-review performance binding for final behavior commit
`443cc636862806e4f0dd1e12ab2e2e45f4189154`. It records five independent
17-input Track A plus one-row Track B rounds with product deployment enabled.
The executable median aggregator reports **32/32 rows pass** against the
owner-signed ceilings in `m55-thresholds.csv`.

## Source and environment binding

- source tree: `875a5d8705ff70d4765ca4dde87a941343f57d7a`
- source-content binding:
  `e5395f9fba892208f0c82a9604bd92185510fc73588b720d802a9967d8139477`
- measured Yune DLL:
  `f829a14033c4cad5e594e50349ee40f104686159404628343bd7673a9467f49b`
- pinned librime DLL:
  `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`
- pinned Windows librime/dependency archives:
  `7478c7caa4ff6b37de86daba1f7ce4a994a4f5ba24872a820fb2b3a9b01fed15` /
  `9ef5608d8a54ff52bbad7a9b4128de42b232f8e3dd1f5fd3bff42a0b1bacd7e8`
- clean LF schema sources: `rime-prelude@082425ea`,
  `rime-essay@48c7538f`, `rime-luna-pinyin@18a80335`, and
  `rime-stroke@3a4b0f40`
- provisioned upstream shared tree:
  `3801c4c83ba919e531b80ac27e2c06d116d08b19af2034fcb86e6e17ae1eecf6`
- provisioned upstream build tree:
  `7f8ce0b50e8acb3d5e66db55fb17879073e5be05a3a7cdc582745fe1e73bf39c`
- product schema tree:
  `0bc042c0ab09c732419cf6ba5ce008390e87894c7d374c0d1b44efeac10a9bf0`
- benchmark executable:
  `728e9e8b2600c5afe68379e585f2d714fad629a89275bcaedbef153465d38833`
- build receipt:
  `e2d2490bb3d02d7f8e3c969f6a10ed2f51c89609ab5e2ab10aae047494b1fe1c`

Round 1 built the native benchmark once. Rounds 2-5 reused that exact
executable and receipt. Every `run-status.txt` is `complete`; the aggregate is
reproducible from the six raw files consumed per run by
`scripts/aggregate-native-ratchet.py`. The command records are retained beside
those inputs. No DLL, executable, compiled schema, or other binary payload is
checked in.

## Long-input confirmation

| Input | Observations | Median | Ceiling | Verdict |
| --- | --- | ---: | ---: | --- |
| 37 characters (`ceshiyixiachangjushuruxingnengzenyang`) | `0.022, 0.022, 0.023, 0.023, 0.022` | `0.022x` | `2.339x` | pass |
| 59 characters (`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`) | `0.010, 0.010, 0.010, 0.010, 0.010` | `0.010x` | `1.748x` | pass |

The full 17-row Track A set, short keys (`n`, `ni`, `hao`), memory, startup,
session creation, and the Track B product row are all present in
`gate-verdict.csv`.

## Rejected environment diagnostic

An earlier attempt used a different precompiled upstream tree
(`shared=ba465db53332222393aeb8703f94e20a3fb0e33e2e4973484eb4d2b98b74a281`,
`build=17a5ed0cafeb085cf546cff4669f5662ff973f6cd3e6ab321fa9e0c11ba41855`).
It added a fourth upstream install during each session and made memory, startup,
and session creation red. Repeating that environment at pre-fix commit
`5879405c` produced the same three reds, so those runs were rejected as an
oracle-environment mismatch rather than attributed to `443cc636`. Their compact
text receipts are preserved under `rejected-wrong-oracle-diagnostic/` but are
not inputs to the accepted aggregate. The clean pinned repositories were
reprovisioned with LF source bytes before the five accepted rounds above.
