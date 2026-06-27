# M46 Final Gates

Date: 2026-06-27

## Focused Implementation Gates

| Gate | Result |
| --- | --- |
| `cargo test -p yune-core memory_owner_rows_cover_m46` | Pass |
| `cargo test -p yune-rime-api --lib tests::session_api::m43_memory_owner_profile_exports_required_session_rows -- --exact` | Pass |
| `cargo build --release -p yune-rime-api` | Pass |
| Native benchmark into `phase-0-native/` | Pass |

## Browser Build And Evidence Gates

| Gate | Result |
| --- | --- |
| `npm.cmd --prefix apps/yune-web run typecheck` | Pass |
| `npm.cmd --prefix apps/yune-web run build` | Pass |
| `npm.cmd --prefix apps/yune-web run build:public` | Pass |
| `scripts/yune-web-wasm-build.sh` with local Emscripten env | Pass: export verification, JS glue fallback scan, and browser module smoke passed; `wasm-opt` post-optimization skipped by existing script validation guard |
| `YUNE_WEB_WASM_HEAP_BENCHMARK=1 ... --grep "YUNE WEB WASM HEAP" --workers=1` | Pass |
| `YUNE_WEB_WASM_ATTRIBUTION=1 ... --grep "YUNE WEB WASM ATTRIBUTION" --workers=1` | Pass |
| `YUNE_WEB_JYUTPING_MEMORY_ATTRIBUTION=1 ... --grep "M46 JYUTPING MEMORY" --workers=1` | Pass |

## Branch A Gates

| Gate | Result |
| --- | --- |
| `cargo test -p yune-rime-api select_schema_served_jyutping_mobile_routes_bare_grave_to_luna_reverse_lookup -- --nocapture` | Pass |
| `cargo test -p yune-rime-api select_schema_served_cangjie_routes_grave_jyutping_reverse_lookup -- --nocapture` | Pass |
| `YUNE_WEB_JYUTPING_MEMORY_ATTRIBUTION=1 YUNE_WEB_JYUTPING_MEMORY_EXPECT_SCHEMA_SWITCH_PASS=1 YUNE_WEB_JYUTPING_MEMORY_PHASE=branch-a-final-after-reverse-assets ... yune-web-jyutping-memory-attribution.spec.ts --workers=1` | Pass |
| Focused M22 reverse lookup browser gate | Pass: [`../../../../apps/yune-web/e2e/results/m46-branch-a-m22-reverse-after-schema-fix/`](../../../../apps/yune-web/e2e/results/m46-branch-a-m22-reverse-after-schema-fix/) |
| Branch A browser behavior set | Pass: [`../../../../apps/yune-web/e2e/results/m46-branch-a-behavior-gates-final/`](../../../../apps/yune-web/e2e/results/m46-branch-a-behavior-gates-final/) |

## Final Required Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | Pass |
| `cargo test -p yune-core --test cantonese_parity` | Pass: 37 passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test --workspace` | Pass |
| `git diff --check` | Pass |

## Verdict

M46 closes as `schema-switch-correctness-fixed-memory-unchanged` with
`measured-no-go-owner-unclassified`. Branch A fixed the product-affecting
schema-switch candidate loss, but native Track B remains `504,627,200 B` peak
and browser Jyutping remains `893.1 MiB`; this is not a full memory success.
