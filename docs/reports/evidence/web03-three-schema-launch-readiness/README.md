# WEB-03 Three-Schema Launch Readiness Evidence

Date: 2026-06-27 local

Verdict: partial native/asset closeout. Tasks 2-4 are complete; Task 5 browser
remeasure is blocked because this machine has the `wasm32-unknown-emscripten`
Rust target but no `emcc`/`emar` on PATH and no `EMSDK` configured.

Do not claim the WEB-02 `893.1 MiB` browser high-water is fixed from this
evidence alone. This bundle proves regenerated launch assets and native
byte-backed storage, not browser linear-memory collapse.

## Evidence Files

- `task2-native-regeneration/workspace-rebuild-reports.csv`
- `task2-native-regeneration/workspace-rebuild-reports.json`
- `task2-native-regeneration/compiled-asset-inventory.csv`
- `task3-native-byte-backed/storage-diagnostics-all-schemas.json`
- `task3-native-byte-backed/storage-selected-all-schemas.csv`
- `task3-native-byte-backed/memory-owner-rows-all-schemas.csv`
- `task3-native-byte-backed/compiled-asset-inventory.csv`

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
```

Results:

- Regeneration guard: passed.
- Public-demo byte-backed guard: passed.
- Public-demo build: passed; pinned schema payload bytes `103,835,643`.
- `cargo fmt --check`: passed.
- `cantonese_parity`: 37 passed.
- `upstream_luna_pinyin_parity`: 12 passed.

`cargo test -p yune-rime-api --test yune_web` was attempted but exceeded the
five-minute command window and was stopped. The scoped WEB-03 tests above are
the recorded web-runtime evidence for this partial slice.

## Blocked Browser Work

The browser remeasure remains open:

- Build fresh WASM with `scripts/yune-web-wasm-build.sh` in an activated
  Emscripten environment.
- Rebuild the public demo.
- Run the WEB-02/WEB-03 Playwright memory attribution and schema-switch checks.
- Only then update final browser memory reports and claim whether the
  `893.1 MiB` high-water collapsed.
