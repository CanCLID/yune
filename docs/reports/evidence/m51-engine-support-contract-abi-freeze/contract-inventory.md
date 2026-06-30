# M51 Contract Inventory

Scope: contract and ABI guard work only. This inventory does not make browser
performance, browser memory, product, package, deployment, platform frontend, or
iOS-device claims.

## Contract Sources

| Source | Contract facts used by M51 |
| --- | --- |
| `AGENTS.md:3-20` | Yune is a librime-compatible Rust engine with `yune-core`, `yune-rime-api`, `yune-cli`, `packages/yune-web-runtime`, and `apps/yune-web`; success is target-driven, not full librime feature parity. |
| `AGENTS.md:72-84` | Upstream `rime/librime 1.17.0` is the default oracle; TypeDuck-HK/librime `v1.1.2` is profile-only; `RimeApi` field order is ABI; TypeDuck fork-only slots stay behind explicit profile accessors such as `rime_get_typeduck_profile_api()`. |
| `AGENTS.md:91-98` | Browser-visible claims require browser evidence, and future TypeDuck-Windows work must not widen default `rime_get_api()`. |
| `docs/conventions.md:24-59` | `yune-rime-api` is the only compatibility surface; CLI, yune-web, and TypeDuck-Windows all consume ABI/export surfaces; default `rime_get_api()` follows upstream 1.17.0 while TypeDuck fork slots are profile-only. |
| `docs/conventions.md:197-200` | There are two export families: librime-shaped `RimePascalCase` and Yune-owned `snake_case` `yune_web_*`; `scripts/yune-web-exports.txt` is the allowlist for the 14 browser/WASM exports. |
| `docs/conventions.md:276-280` | librime is validation-only with no runtime link; yune-web exports 14 functions; TypeDuck-Windows package smoke keeps default `rime_get_api()` upstream-shaped and verifies profile behavior through the named TypeDuck profile. |
| `docs/conventions.md:292-302` | Default core and default `RimeApi` follow upstream 1.17.0; TypeDuck-Windows work is complete as a compatibility profile; profile isolation and the core/ABI boundary remain live guardrails. |
| `docs/decisions.md:13-27` | Librime is the compatibility oracle, not an implementation template; upstream-first oracle sequencing and target-driven scope are standing decisions. |
| `docs/decisions.md:113-121` | Emscripten C ABI exports are the browser build contract; one active process-global service per WASM instance is the current host contract. |
| `docs/requirements.md:21-24` | Existing ABI requirements cover frontend/native validation, struct/lifetime/session regression coverage, resource ID safety, and deterministic process-wide runtime behavior. |
| `docs/requirements.md:111,124` | TypeDuck-specific ABI/comment/frontend behavior is profile-only; `config_list_append_{string,bool,int,double}` exists through `rime_get_typeduck_profile_api()` and not default `rime_get_api()`. |
| `docs/requirements.md:234-245` | Browser/demo controls and inspector work preserved `RimeApi`, `RimeCandidate`, and ABI layout; UI convenience did not add default ABI/export changes. |
| `docs/plans/reference/m10-reference-typeduck-windows-contract.md:15-26` | M10 is complete as a TypeDuck compatibility profile; historical tasks must not be reinterpreted as instructions to add fork-only slots to default `RimeApi`. |
| `docs/plans/reference/m10-reference-typeduck-windows-contract.md:246-264` | TypeDuck profile comment payloads are carried through `RimeCandidate.comment`; the gap was semantics, not transport. |
| `docs/plans/reference/m10-reference-typeduck-windows-native-build.md:18-30` | The TypeDuck fork header is not safe as Yune's default header; Yune keeps upstream-shaped `RimeCandidate`, upstream `rime_get_api()`, and no default `start_quick` or list-append slots. |
| `docs/plans/reference/m10-reference-typeduck-windows-native-build.md:42-44,70-86` | The package uses `rime_typeduck_profile_api.h`, rejects fork-shaped default headers, and smokes packaged profile slots through `rime_get_typeduck_profile_api()`. |
| `docs/plans/reference/m19-reference-typeduck-profile-abi.md:5-15` | Default `rime_get_api()` remains upstream-sized; the TypeDuck profile accessor returns a larger opt-in table whose first bytes are the upstream prefix. |
| `docs/plans/reference/m19-reference-typeduck-profile-abi.md:19-40` | The TypeDuck profile delta is the four list-append slots in `bool`, `int`, `double`, `string` order; tests verify default upstream size, profile size, append slots, and append behavior. |
| `scripts/yune-web-exports.txt:1-14` | Canonical current `yune_web_*` allowlist contains exactly 14 symbols. |
| `crates/yune-rime-api/src/web_runtime.rs:51-479` | Implements the 14 `#[no_mangle] extern "C"` `yune_web_*` functions. |
| `crates/yune-rime-api/src/bin/yune_web_module.rs:10-39` | Linker anchor references all 14 `yune_web_*` functions so the Emscripten module keeps them reachable. |
| `packages/yune-web-runtime/src/module.ts:14-29,60-74` | TypeScript runtime binds the same 14 `yune_web_*` names and signatures. |

## Baseline ABI Gates

| Command | Result | Notes |
| --- | --- | --- |
| `cargo test -p yune-rime-api abi` | pass | 2 unit tests, 2 frontend-host ABI tests, and 4 TypeDuck boundary ABI tests matched the selector; all passed. |
| `cargo test -p yune-rime-api config_api` | pass | 22 selected tests passed, including `default_rime_api_exposes_upstream_config_list_contract` and TypeDuck append helper behavior. |
| `cargo test -p yune-rime-api typeduck_windows_boundary` | pass with zero selected | Selector is stale for the current tree; it selected 0 tests. |
| `cargo test -p yune-rime-api --test typeduck_windows_boundary` | pass | Replacement for the stale selector; 4 integration tests passed. |
| `cargo test -p yune-rime-api --test yune_web yune_web_adapter_processes_keys_and_returns_json_state` | pass | 1 selected yune-web adapter test passed. |

## Baseline Decision

M51 can proceed from a clean contract baseline. The only Task 0 adjustment is
that `typeduck_windows_boundary` must be run as the integration-test target
(`--test typeduck_windows_boundary`) rather than as a plain name selector.
