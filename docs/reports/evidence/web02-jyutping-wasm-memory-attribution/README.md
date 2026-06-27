# WEB-02 Jyutping WASM Memory Attribution Evidence

Date: 2026-06-27

Verdict: Phase 0 success. The public-demo Jyutping browser path is now
classified as source fallback, not byte-backed storage.

## Root Cause

The shipped public-demo Jyutping prism assets are `Rime::Prism/3.0`:

- `jyut6ping3_mobile.prism.bin`: `242,728 B`, `Rime::Prism/3.0`
- `jyut6ping3_scolar.prism.bin`: `2,343,228 B`, `Rime::Prism/3.0`

Yune's compiled-prism parser accepts current `Rime::Prism/4.0` payloads for
the compact/byte-backed path. The public-demo ABI path therefore records:

- `source_fallback=true`
- reason: `prism parse failed: UnsupportedVersion`
- selected storage: `owned_heap`
- `byte_source_len=0`
- retained owner estimate: `529,602,374 B`

The retained heap owner is engine memory at runtime, but the cause is the
web/public-demo artifact contract: the shipped Jyutping compiled assets are not
the current deploy artifacts that M46 validated natively.

## Evidence Files

- [`task0-web-abi-public-demo/storage-diagnostics.json`](./task0-web-abi-public-demo/storage-diagnostics.json)
  - full inspector storage diagnostics emitted by the web ABI response path.
- [`task0-web-abi-public-demo/storage-selected.csv`](./task0-web-abi-public-demo/storage-selected.csv)
  - live selected storage rows.
- [`task0-web-abi-public-demo/memory-owner-rows.csv`](./task0-web-abi-public-demo/memory-owner-rows.csv)
  - live retained owner estimates.
- [`task0-web-abi-public-demo/compiled-asset-inventory.csv`](./task0-web-abi-public-demo/compiled-asset-inventory.csv)
  - shipped shared assets and post-deploy `user/build` inventory.
- [`visuals/web02-public-demo-storage-owner.svg`](./visuals/web02-public-demo-storage-owner.svg)
  - visual summary of retained source fallback bytes versus the browser
    high-water.

## Verification

```powershell
cargo test -p yune-rime-api --test yune_web yune_web_adapter_storage_diagnostics_reports_live_jyutping_storage -- --exact
$env:YUNE_WEB02_EVIDENCE_DIR='C:\Users\laubonghaudoi\Documents\GitHub\yune\docs\reports\evidence\web02-jyutping-wasm-memory-attribution\task0-web-abi-public-demo'
cargo test -p yune-rime-api --test yune_web web02_public_demo_storage_diagnostics_exports_owner_rows -- --ignored --exact --nocapture
```

## Follow-Up Branch

Do not run another payload-family or `INITIAL_MEMORY` branch from this evidence.
The next reduction branch should fix the browser/public-demo compiled-asset
contract so Jyutping selects byte-backed/current-format storage, then rerun the
browser high-water measurement. If the fixed path remains high, only then move
to allocator high-water or streamed-deploy work.
