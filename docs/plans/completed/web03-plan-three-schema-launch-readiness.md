# WEB-03 Three-Schema Launch Readiness & Compiled-Asset Contract

> **Status:** Complete - **Track:** Web harness deploy + compiled-asset
> delivery - **Created:** 2026-06-27 - **Completed:** 2026-06-27 -
> **Type:** engine deploy fix + asset regeneration + browser remeasure
>
> Follows WEB-02 (`b216ca82`). Launch-ready means the public demo offers all
> three schemas selectable and working: upstream `rime-luna-pinyin`, upstream
> `rime-cangjie` (`cangjie5`), and multilingual `jyut6ping3`.

## Final Verdict

WEB-03 closes as a success for the launch compiled-asset contract.

- Engine fix `3ffd4b21` skips empty `custom_phrase` dictionary namespaces during
  deploy artifact requests, matching librime behavior and unblocking clean
  rebuilds for the web schemas.
- Asset commit `ef37bfe9` regenerates and ships current `Rime::Prism/4.0`
  launch assets for `jyut6ping3_mobile` including `jyut6ping3_scolar` and
  `luna_pinyin_yune_reverse`, `cangjie5`, and `luna_pinyin`.
- Native diagnostics prove `source_fallback=false`, zero fallback rows,
  `selected_storage=byte_backed`, and positive `byte_source_len` for all three
  public schemas.
- Fresh Emscripten/Playwright evidence records the public-demo
  `full-jyutping` browser row at `160.0 MiB` ready/peak/steady WASM with ready
  `1306 ms`, input-to-candidate `100 ms`, and commit `110 ms`.
- A post-closeout compact-path fix restores byte-backed Jyutping phrase
  composition (`ngogokdak -> 我覺得`) and visible prefix lookup rows for
  `zouhapci`; final follow-up gates pass full native `yune_web`,
  `cantonese_parity`, and a rebuilt public-demo browser smoke.
- The old `893.1 MiB` value remains only as the synthetic `extras` negative
  control that intentionally withholds launch compiled assets. It is not the
  shipped launch path.

Scope boundary: WEB-03 is a browser/public-demo compiled-asset fix. It does not
claim a native Track B memory win, a broad product speed win, or fair browser
memory parity versus My RIME.

## Root Cause

Yune deploy treated `table_translator@custom_phrase` with `dictionary: ''` as a
buildable dictionary. That emitted an empty dictionary-id artifact request, then
the clean/forced rebuild aborted before writing current compiled artifacts for
schemas that imported `custom_phrase`.

The stale browser assets were a consequence: public-demo Jyutping shipped old
`Rime::Prism/3.0` files, which the current prism parser rejected, forcing
source fallback and retaining large `translator.entries_by_code` maps.

The fix is engine-owned, but the launch readiness slice is web/asset-owned:
regenerate the assets, update the public schema manifest and worker asset lists,
and prove the rebuilt browser path uses byte-backed storage.

## Completed Tasks

### Task 1 - Land Engine Fix

Complete in `3ffd4b21`.

- Skip empty dictionary namespaces in `schema_dictionary_artifact_requests`.
- Add regression coverage:
  `empty_dictionary_namespace_yields_no_build_request`.
- Gates recorded by the engine-fix slice: `yune_web` 32/0,
  `cantonese_parity` 37/0, `upstream_luna_pinyin_parity` 12/0, and clippy
  clean.

### Task 2 - Regenerate + Ship Launch Assets

Complete in `ef37bfe9`.

- Clean forced regeneration rebuilt launch-schema compiled assets from source.
- Required launch dictionaries rebuilt rather than reused from stale prebuilt
  assets: `luna_pinyin`, `jyut6ping3`, `jyut6ping3_scolar`,
  `luna_pinyin_yune_reverse`, and `cangjie5`.
- All launch prisms are current `Rime::Prism/4.0`.
- `apps/yune-web/public/schema`, both schema manifests, the public-demo build,
  and the worker asset lists include the regenerated launch assets, including
  Cangjie `.table/.prism/.reverse.bin` payloads.

Evidence:
[`../../reports/evidence/web03-three-schema-launch-readiness/task2-native-regeneration/`](../../reports/evidence/web03-three-schema-launch-readiness/task2-native-regeneration/).

### Task 3 - Regeneration Script + Byte-Backed Guard

Complete in `ef37bfe9`.

- Guard behavior asserts `source_fallback=false`, no fallback rows,
  `selected_storage=byte_backed`, positive `byte_source_len`, and deterministic
  smoke candidates for all public schemas.
- The Luna `.txt` vocabulary loader is part of the contract so regenerated
  tables preserve `essay.txt` weights.

Evidence:
[`../../reports/evidence/web03-three-schema-launch-readiness/task3-native-byte-backed/`](../../reports/evidence/web03-three-schema-launch-readiness/task3-native-byte-backed/).

### Task 4 - Cangjie Correctness

Complete in `ef37bfe9`.

The native guard covers the minimum deterministic smoke: `cangjie5` input `a`
returns U+65E5 first while using byte-backed selected storage.

### Task 5 - Browser Remeasure

Complete in the WEB-03 browser closeout.

Commands run after activating `emsdk`:

```powershell
cmd.exe /d /s /c 'call "C:\Users\laubonghaudoi\Documents\GitHub\emsdk\emsdk_env.bat" >NUL && "C:\Program Files\Git\bin\bash.exe" scripts/yune-web-wasm-build.sh'
node apps/yune-web/public-demo/build.mjs
npm --prefix apps/yune-web run build
$env:YUNE_WEB_JYUTPING_MEMORY_ATTRIBUTION='1'; $env:YUNE_WEB_JYUTPING_MEMORY_PHASE='web03-after-byte-backed-assets'; $env:YUNE_WEB_JYUTPING_MEMORY_EXPECT_SCHEMA_SWITCH_PASS='1'; npm --prefix apps/yune-web/e2e exec playwright -- test yune-web-jyutping-memory-attribution.spec.ts --config playwright.config.ts --workers=1
$env:YUNE_WEB_WASM_ATTRIBUTION='1'; $env:YUNE_WEB_WASM_ATTRIBUTION_RESULT_ROOT='web03-three-schema-launch-readiness/browser-attribution'; $env:YUNE_WEB_WASM_ATTRIBUTION_PHASE='web03-after-byte-backed-assets'; npm --prefix apps/yune-web/e2e exec playwright -- test yune-web-wasm-attribution.spec.ts --config playwright.config.ts --workers=1
```

Browser results:

| Row | Peak WASM | Steady WASM | Ready | Input-to-candidate | Commit |
| --- | ---: | ---: | ---: | ---: | ---: |
| public-demo `luna-core` | `64.0 MiB` | `64.0 MiB` | `989 ms` | `79 ms` | `122 ms` |
| public-demo `jyutping-core` | `160.0 MiB` | `160.0 MiB` | `1096 ms` | `80 ms` | `110 ms` |
| public-demo `jyutping-scolar` | `160.0 MiB` | `160.0 MiB` | `1289 ms` | `89 ms` | `117 ms` |
| public-demo `full-jyutping` | `160.0 MiB` | `160.0 MiB` | `1306 ms` | `100 ms` | `110 ms` |
| public-demo `extras` | `893.1 MiB` | `893.1 MiB` | `5178 ms` | `72 ms` | `117 ms` |

Schema-switch rows all pass and top out at `160.0 MiB`:

- clean Jyutping
- Cangjie -> Luna -> Jyutping
- Jyutping -> Luna -> Jyutping

Evidence:
[`../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/web03-after-byte-backed-assets/`](../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/web03-after-byte-backed-assets/),
[`../../../apps/yune-web/e2e/results/web03-three-schema-launch-readiness/browser-attribution/web03-after-byte-backed-assets/`](../../../apps/yune-web/e2e/results/web03-three-schema-launch-readiness/browser-attribution/web03-after-byte-backed-assets/).

### Task 6 - Phrase-Composition Follow-Up

Complete in the WEB-03 follow-up correctness slice.

The memory remeasure at `d4d84203` was valid, but the regenerated byte-backed
Jyutping path initially failed full native `yune_web`: `ngogokdak` did not
compose `我覺得`, and the `zouhapci` visible lookup smoke lost required first-page
dictionary rows. The compact path now resolves non-correction prism aliases for
sentence substrings and prefix fallback probes, then orders prefix fallback by
longest consumed prefix, dictionary weight, and stable emission order.

Final follow-up gates:

- `cargo test -p yune-rime-api --test yune_web`: 33 passed, 0 failed, 2 ignored.
- `cargo test -p yune-core --test cantonese_parity`: 37 passed.
- WEB-03 byte-backed guard asserts `ngogokdak -> 我覺得`.
- Browser smoke against rebuilt public demo passes `ngogokdak -> 我覺得` and
  `zouhapci` visible dictionary rows.

Evidence:
[`../../reports/evidence/web03-three-schema-launch-readiness/phrase-composition-regression-fix/final-gates.md`](../../reports/evidence/web03-three-schema-launch-readiness/phrase-composition-regression-fix/final-gates.md),
[`../../../apps/yune-web/e2e/results/web03-three-schema-launch-readiness/phrase-composition-regression-fix/browser-phrase-smoke/`](../../../apps/yune-web/e2e/results/web03-three-schema-launch-readiness/phrase-composition-regression-fix/browser-phrase-smoke/).

### Task 7 - Long-Input Latency Follow-Up

Complete in the WEB-03 latency follow-up.

The phrase-composition repair restored correctness but over-broadened
compact-path sentence/prefix fallback expansion. A live deployed probe on
2026-06-28 reproduced the regression while memory stayed fixed at `160.0 MiB`:
the 28-character Jyutping row reached `3764 ms` and the 52-character row
reached `1518 ms` exact keydown-to-paint.

The follow-up bounds hidden work while keeping the compact-path alias behavior:

- sentence alias expansion asks the prism only for the codes the sentence path
  can consume;
- sentence spans cap collected candidates per span;
- prefix fallback sorts a small capped pending set instead of every matching
  dictionary row.

Rebuilt local public-demo evidence records `130 ms` for
`sihaacoenggeoisyujapgecukdou` and `74 ms` for
`taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng`, with ready/peak WASM
memory still `160.0 MiB`.

Evidence:
[`../../../apps/yune-web/e2e/results/web03-latency-regression-fix/local-browser-latency/`](../../../apps/yune-web/e2e/results/web03-latency-regression-fix/local-browser-latency/).

## Closeout Updates

Updated:

- [`../../roadmap.md`](../../roadmap.md)
- [`../../requirements.md`](../../requirements.md)
- [`../../decisions.md`](../../decisions.md)
- [`../../ledgers/milestone-history.md`](../../ledgers/milestone-history.md)
- [`../../reports/yune-vs-librime-performance.md`](../../reports/yune-vs-librime-performance.md)
- [`../../reports/yune-vs-librime-root-cause-analysis.md`](../../reports/yune-vs-librime-root-cause-analysis.md)
- [`../../reports/evidence/web02-jyutping-wasm-memory-attribution/README.md`](../../reports/evidence/web02-jyutping-wasm-memory-attribution/README.md)
- [`../../reports/evidence/web03-three-schema-launch-readiness/README.md`](../../reports/evidence/web03-three-schema-launch-readiness/README.md)
- [`../../../apps/yune-web/e2e/results/web03-latency-regression-fix/local-browser-latency/`](../../../apps/yune-web/e2e/results/web03-latency-regression-fix/local-browser-latency/)

## Remaining Measured Blockers

- The fair browser memory comparison still belongs to the `luna_pinyin` lane:
  Yune is `160.0 MiB` versus My RIME `16.0 MiB`.
- Native Track B memory remains the M46 result: around `504 MB` peak and mostly
  unclassified process memory. WEB-03 does not alter that native result.
- The WEB-03 `extras` attribution row remains `893.1 MiB` when launch compiled
  assets are intentionally withheld; it is a negative control.
