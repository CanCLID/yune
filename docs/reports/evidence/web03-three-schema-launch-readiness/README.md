# WEB-03 Three-Schema Launch Readiness Evidence

Date: 2026-06-27 local

Verdict: success for the WEB-03 launch compiled-asset contract. Tasks 2-5 are
complete: launch assets are regenerated, native diagnostics byte-back all three
public schemas, and fresh Emscripten/Playwright evidence shows the shipping
Jyutping launch/full browser rows peak and settle at `160.0 MiB`.

Scope boundary: this is a browser-harness/public-demo compiled-asset fix. It
does not claim a native-engine memory win, a broad product speed win, or a fair
browser memory-parity win versus My RIME. The synthetic `extras` attribution row
still reaches `893.1 MiB` because it intentionally withholds launch compiled
assets; it remains a negative control, not the shipped path.

## Evidence Files

- `task2-native-regeneration/workspace-rebuild-reports.csv`
- `task2-native-regeneration/workspace-rebuild-reports.json`
- `task2-native-regeneration/compiled-asset-inventory.csv`
- `task3-native-byte-backed/storage-diagnostics-all-schemas.json`
- `task3-native-byte-backed/storage-selected-all-schemas.csv`
- `task3-native-byte-backed/memory-owner-rows-all-schemas.csv`
- `task3-native-byte-backed/compiled-asset-inventory.csv`
- `visuals/web03-browser-wasm-memory.svg`
- `visuals/web03-browser-timing.svg`
- `../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/web03-after-byte-backed-assets/summary.csv`
- `../../../../apps/yune-web/e2e/results/yune-web-jyutping-memory-attribution/web03-after-byte-backed-assets/report.md`
- `../../../../apps/yune-web/e2e/results/web03-three-schema-launch-readiness/browser-attribution/web03-after-byte-backed-assets/summary.csv`
- `../../../../apps/yune-web/e2e/results/web03-three-schema-launch-readiness/browser-attribution/web03-after-byte-backed-assets/report.md`

## Regeneration

The ignored native regeneration test copied clean schema sources, excluded
committed `.bin` files, ran clean deploy tasks for the launch schemas, and
copied the regenerated assets into `apps/yune-web/public/schema`.

Required launch dictionaries rebuilt from source:

| Schema | Dictionary | Table | Prism | Reverse |
| --- | --- | --- | --- | --- |
| `luna_pinyin` | `luna_pinyin` | `Rebuilt` | `Rebuilt` | `Rebuilt` |
| `jyut6ping3_mobile` | `jyut6ping3_scolar` | `Rebuilt` | `Rebuilt` | `Rebuilt` |
| `jyut6ping3_mobile` | `luna_pinyin_yune_reverse` | `Rebuilt` | `Rebuilt` | `Rebuilt` |
| `jyut6ping3_mobile` | `jyut6ping3` | `Rebuilt` | `Rebuilt` | `Rebuilt` |
| `cangjie5` | `cangjie5` | `Rebuilt` | `Rebuilt` | `Rebuilt` |

No row used `ReusedPrebuilt`. A repeated import in the same clean workspace can
show `ReusedFresh` after a previous schema has already rebuilt it.

All launch prisms are `Rime::Prism/4.0`, including:

| Asset | Bytes | Header |
| --- | ---: | --- |
| `jyut6ping3_mobile.prism.bin` | 19,313,669 | `Rime::Prism/4.0` |
| `jyut6ping3_scolar.prism.bin` | 325 | `Rime::Prism/4.0` |
| `luna_pinyin_yune_reverse.prism.bin` | 1,513,837 | `Rime::Prism/4.0` |
| `cangjie5.prism.bin` | 1,430,557 | `Rime::Prism/4.0` |
| `luna_pinyin.prism.bin` | 1,641,885 | `Rime::Prism/4.0` |

## Byte-Backed Check

Native diagnostics pass for all three public-demo launch schemas:

| Schema | Input | Smoke top | Source fallback | Fallback rows | Selected storage |
| --- | --- | --- | --- | ---: | --- |
| `jyut6ping3_mobile` | `nei` | U+4F60 | `false` | 0 | `byte_backed` 15,248,382 B; `byte_backed` 4,640,555 B |
| `cangjie5` | `a` | U+65E5 | `false` | 0 | `byte_backed` 3,092,119 B |
| `luna_pinyin` | `ni` | U+4F60 | `false` | 0 | `byte_backed` 4,640,486 B |

The guard is behavioral: it asserts `source_fallback=false`, no fallback rows,
`selected_storage=byte_backed`, positive `byte_source_len`, and deterministic
smoke candidates for the launch schemas.

## Browser Remeasure

After installing and activating Emscripten through `emsdk`, the WEB-03 browser
remeasure rebuilt the WASM runtime, rebuilt the tracked app and public-demo
artifacts, and ran the Playwright memory/switching checks serially.

Schema-switch evidence:

| Scenario | Verdict | Max observed WASM | Worker action errors |
| --- | --- | ---: | ---: |
| `clean-jyutping` | pass | `160.0 MiB` | 0 |
| `schema-switch` | pass | `160.0 MiB` | 0 |
| `jyutping-luna-jyutping` | pass | `160.0 MiB` | 0 |

Attribution evidence:

| Public-demo row | Ready | Input-to-candidate | Commit | Peak WASM | Steady WASM |
| --- | ---: | ---: | ---: | ---: | ---: |
| `luna-core` | `989 ms` | `79 ms` | `122 ms` | `64.0 MiB` | `64.0 MiB` |
| `jyutping-core` | `1096 ms` | `80 ms` | `110 ms` | `160.0 MiB` | `160.0 MiB` |
| `jyutping-scolar` | `1289 ms` | `89 ms` | `117 ms` | `160.0 MiB` | `160.0 MiB` |
| `reverse-lookup` | `3432 ms` | `71 ms` | `106 ms` | `160.0 MiB` | `160.0 MiB` |
| `opencc` | `1541 ms` | `87 ms` | `108 ms` | `160.0 MiB` | `160.0 MiB` |
| `full-jyutping` | `1306 ms` | `100 ms` | `110 ms` | `160.0 MiB` | `160.0 MiB` |
| `extras` | `5178 ms` | `72 ms` | `117 ms` | `893.1 MiB` | `893.1 MiB` |

`extras` is the negative-control row from the attribution harness: it requests
no launch compiled assets and therefore still exercises the old source-fallback
high-water shape.

## Verification

Commands run:

```powershell
$env:YUNE_WEB03_EVIDENCE_DIR='docs/reports/evidence/web03-three-schema-launch-readiness'; $env:YUNE_WEB03_APPLY_ASSETS='1'; cargo test -p yune-rime-api --test yune_web web03_regenerates_public_schema_compiled_assets_from_clean_rebuild -- --ignored --exact
node apps/yune-web/scripts/update-schema-asset-manifest.mjs
node apps/yune-web/public-demo/build.mjs
$env:YUNE_WEB03_EVIDENCE_DIR='docs/reports/evidence/web03-three-schema-launch-readiness'; cargo test -p yune-rime-api --test yune_web web03_public_demo_launch_schemas_byte_back_compiled_assets -- --exact
cargo fmt --check
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-core --test upstream_luna_pinyin_parity
cmd.exe /d /s /c 'call "C:\Users\laubonghaudoi\Documents\GitHub\emsdk\emsdk_env.bat" >NUL && "C:\Program Files\Git\bin\bash.exe" scripts/yune-web-wasm-build.sh'
node apps/yune-web/public-demo/build.mjs
npm --prefix apps/yune-web run build
$env:YUNE_WEB_JYUTPING_MEMORY_ATTRIBUTION='1'; $env:YUNE_WEB_JYUTPING_MEMORY_PHASE='web03-after-byte-backed-assets'; $env:YUNE_WEB_JYUTPING_MEMORY_EXPECT_SCHEMA_SWITCH_PASS='1'; npm --prefix apps/yune-web/e2e exec playwright -- test yune-web-jyutping-memory-attribution.spec.ts --config playwright.config.ts --workers=1
$env:YUNE_WEB_WASM_ATTRIBUTION='1'; $env:YUNE_WEB_WASM_ATTRIBUTION_RESULT_ROOT='web03-three-schema-launch-readiness/browser-attribution'; $env:YUNE_WEB_WASM_ATTRIBUTION_PHASE='web03-after-byte-backed-assets'; npm --prefix apps/yune-web/e2e exec playwright -- test yune-web-wasm-attribution.spec.ts --config playwright.config.ts --workers=1
```

Results:

- Regeneration guard: passed.
- Public-demo byte-backed guard: passed.
- Public-demo build: passed; pinned schema payload bytes `103,835,643`.
- `cargo fmt --check`: passed.
- `cantonese_parity`: 37 passed.
- `upstream_luna_pinyin_parity`: 12 passed.
- Emscripten WASM build: passed with `emcc`/`emar` from
  `C:\Users\laubonghaudoi\Documents\GitHub\emsdk`.
- Public-demo build: passed; pinned schema payload bytes `103,835,643`.
- Tracked app build: passed.
- Browser schema-switch memory check: passed; max observed WASM `160.0 MiB`.
- Browser attribution check: passed; public-demo `full-jyutping` peak and
  steady `160.0 MiB`.

`cargo test -p yune-rime-api --test yune_web` was attempted but exceeded the
five-minute command window and was stopped. The scoped WEB-03 tests above are
the recorded web-runtime evidence for this slice.
