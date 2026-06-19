# M12 Coverage Audit

> **Status:** Finished - **Milestone:** M12 (Upstream Oracle Refresh) - **Closed:** 2026-06-19 - **Type:** coverage audit reference

M12 makes upstream `rime/librime 1.17.0` the default core oracle. TypeDuck
`v1.1.2` remains a compatibility-profile oracle for TypeDuck-Web and parked
TypeDuck-Windows work only. The default `rime_get_api()` table must not require
TypeDuck fork-only slots such as `start_quick` or `config_list_append_*`.

| Path | Existing coverage | Classification | M12 action |
|---|---|---|---|
| `crates/yune-rime-api/src/tests/abi.rs` | Default `RimeApi` slot layout and table size | upstream core | Refreshed to upstream `rime/librime 1.17.0`; default ABI parity is upstream-first. |
| `crates/yune-rime-api/src/tests/config_api.rs` | Direct `RimeConfigListAppend*` helper behavior | TypeDuck profile | Keep as TypeDuck-profile implementation-only tests; default `rime_get_api()` no longer exposes `config_list_append_*`. |
| `crates/yune-core/tests/cantonese_parity.rs` | Active Cantonese/Jyutping oracle parity slices | TypeDuck profile | Keep labeled as TypeDuck `v1.1.2` profile behavior; do not treat as upstream core proof. |
| `crates/yune-core/tests/cantonese_parity.rs` ignored tests | Uncaptured fork-only Cantonese/Jyutping cases | deferred | Remain `#[ignore = "blocked: ..."]` until genuine TypeDuck `v1.1.2` goldens are captured. |
| `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` | Captured TypeDuck fixture data and manifest | TypeDuck profile | Keep under the explicit `typeduck-v1.1.2/` provenance directory. |
| `crates/yune-core/tests/fixtures/upstream-1.17.0/` | Upstream fixture provenance README and manifest | upstream core provenance | Use as the default core oracle provenance root for M12 and later upstream-first slices. |
| `crates/yune-rime-api/src/tests/schema_selection.rs` reverse lookup prompt fixture | Reverse lookup prompt and schema prompt bytes captured from TypeDuck `v1.1.2` | TypeDuck profile | Keep explicit TypeDuck profile labeling; do not promote to upstream core unless upstream goldens are captured. |
| `crates/yune-rime-api/tests/typeduck_web.rs` | TypeDuck-Web adapter, runtime response, real-assets fallback gate | TypeDuck-Web profile gate | Keep green as the web compatibility-profile gate; do not convert to an upstream core ABI requirement. |
| `scripts/package-typeduck-windows.ps1` | Native TypeDuck-Windows package path and historical slot smoke | parked TypeDuck-Windows profile | Fail fast during M12; re-enable only after a named TypeDuck profile ABI surface exists and slot smoke is re-derived from TypeDuck-HK/librime `v1.1.2` `rime_api.h`. |
| `docs/plans/m10-reference-typeduck-windows-contract.md` | TypeDuck-Windows contract and historical execution notes | parked TypeDuck-Windows profile | Keep as parked reference; older default-table assumptions are not current M12 gates. |
| `docs/plans/m10-reference-typeduck-windows-native-build.md` | Native package reproduction notes and archived smoke evidence | parked TypeDuck-Windows profile | Keep as parked reference; package smoke is historical and not valid against default upstream `RimeApi`. |

## Closeout Notes

- `start_quick` is not an upstream `1.17.0` default ABI field and is not exported
  as a flat default symbol in M12.
- `config_list_append_*` helpers remain useful TypeDuck-profile implementation
  code, but they are not part of the default upstream `RimeApi`.
- TypeDuck-Windows packaging is blocked until Yune has an explicit named
  TypeDuck-profile ABI surface and fresh slot evidence from the TypeDuck fork
  header.
