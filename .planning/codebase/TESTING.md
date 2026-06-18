# Testing Patterns

**Analysis Date:** 2026-06-17

> Note on anchors: prefer the symbol/path names below over any line numbers, which drift as files change. Line numbers, where cited, were correct at the analysis date.

## Where this fits

Yune is a Rust engine plus a RIME-compatible C ABI (`crates/yune-rime-api`) consumed by **three** frontends, and the tests are organized around that boundary:

- **CLI surrogate** — `crates/yune-cli` drives the core engine and checks golden output fixtures.
- **TypeDuck-Web** — the browser path: the `yune_typeduck_*` WASM adapter exports (validated by native Rust ABI tests) plus the `@yune-ime/typeduck-runtime` TypeScript package (Vitest suite).
- **TypeDuck-Windows** — a native `rime.dll` consumer that depends on the fork-only `config_list_append_*` ABI entries.

The parity oracle for engine behavior is **upstream `github.com/rime/librime` plus the TypeDuck fork `github.com/TypeDuck-HK/librime` @ `v1.1.2`** (a frozen tagged release, not a local checkout). Current project direction is **web-first**: validate M9 TypeDuck-Web in a real browser before resuming the parked M10 TypeDuck-Windows native work. AI-native input is a separate later layer; some scaffolding (e.g. `MockAiRanker`) already appears in tests.

## Test Framework

**Runner:**
- Rust built-in test harness through Cargo.
- Config: workspace `Cargo.toml` plus per-crate manifests under `crates/yune-core/Cargo.toml`, `crates/yune-schema/Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, and `crates/yune-cli/Cargo.toml`.
- The TypeScript runtime uses **Vitest** (`packages/yune-typeduck-runtime/package.json`, `npm test` → `vitest run`).

**Assertion Library:**
- Standard Rust assertions: `assert_eq!`, `assert!`, `assert_ne!`, and explicit `panic!` in fixture-mismatch / parked-test handling.
- No third-party assertion, property-test, or snapshot-test crate is declared (only `serde_json` / `serde_yaml` as data parsers).

**Run Commands:**
```bash
cargo test --workspace                                   # Run all Rust tests
cargo test -p yune-rime-api session_api                  # Run a focused unit-test module by name
cargo test -p yune-rime-api --test frontend_client       # Frontend-style integration client
cargo test -p yune-rime-api --test typeduck_web          # TypeDuck-Web ABI/adapter contract
cargo test -p yune-core --test cantonese_parity          # v1.1.2 oracle parity (run from yune-core)
cargo clippy --workspace --all-targets -- -D warnings    # Lint all production and test targets
```
```bash
# TypeScript runtime (packages/yune-typeduck-runtime)
npm test                                                 # vitest run
```

**Lint gate:** the clippy command above is backed by `[workspace.lints]` in the root `Cargo.toml`:
`rust.unsafe_code = "forbid"` and `clippy.all = "warn"`, `clippy.pedantic = "warn"`. `-D warnings`
promotes those warnings to hard errors, so test code must satisfy pedantic clippy, and any necessary
`unsafe` must be re-permitted with an explicit crate-/block-level `allow` where the forbid would
otherwise block it.

## Test File Organization

**Location:**
- Unit tests are partly embedded in source files with `#[cfg(test)] mod tests`, for example `crates/yune-schema/src/lib.rs`, `crates/yune-cli/src/args.rs`, `crates/yune-cli/src/fixture.rs`, and `crates/yune-cli/src/transcript.rs`.
- `crates/yune-core` is modularized (`engine`, `filter`, `key`, `translator`, `dictionary`, `punctuation`, `state`, `userdb`, …). `lib.rs` is mostly a re-export facade but still carries a **large embedded `mod facade_tests`** (key-sequence parsing, CRC32 / dict-source checksums, compiled table/prism/reverse binary-metadata parsing). The trait-boundary / `Engine`-driven core tests moved out into a dedicated tree: `crates/yune-core/src/tests/{mod.rs,engine.rs,filter.rs,translator.rs}` (mounted via `#[cfg(test)] mod tests;` in `lib.rs`).
- `yune-rime-api` has a dedicated internal test tree under `crates/yune-rime-api/src/tests/`, mounted from `#[cfg(test)] mod tests;` in `crates/yune-rime-api/src/lib.rs`.
- Cargo integration tests (separate binaries) live under each crate's `tests/` directory — see Integration Tests below.
- CLI golden fixtures and host-trace goldens live outside the crates under `fixtures/`.

**Naming:**
- Test functions use behavior descriptions in `snake_case`, such as `config_open_apis_load_runtime_yaml_files` in `crates/yune-rime-api/src/tests/config_api.rs` and `parses_rime_schema_subset` in `crates/yune-schema/src/lib.rs`.
- RIME compatibility tests often include `librime` in the name when matching external behavior (e.g. tests in `crates/yune-rime-api/src/tests/schema_processors.rs` and `schema_selection.rs`).
- The `yune-rime-api` internal modules track API / implementation areas: `abi.rs`, `candidate_api.rs`, `config_api.rs`, `context_status.rs`, `deployment.rs`, `dictionary_data.rs`, `distribution_schema_comparison.rs`, `levers.rs`, `lifecycle_safety.rs`, `resource_id.rs`, `runtime.rs`, `schema_api.rs`, `schema_processors.rs`, `schema_selection.rs`, `session_api.rs`, and `userdb.rs` under `crates/yune-rime-api/src/tests/` (mounted from `tests/mod.rs`).

**Structure:**
```text
crates/yune-core/src/lib.rs                       # facade + large embedded `mod facade_tests`
crates/yune-core/src/tests/{mod,engine,filter,translator}.rs   # Engine/trait-boundary core tests
crates/yune-core/tests/cantonese_parity.rs        # v1.1.2 oracle parity (integration)
crates/yune-core/tests/fixtures/typeduck-v1.1.2/  # frozen oracle goldens + README
crates/yune-schema/src/lib.rs                     # schema parser unit tests
crates/yune-rime-api/src/tests/mod.rs             # shared ABI test helpers
crates/yune-rime-api/src/tests/*.rs               # focused API and compatibility unit tests
crates/yune-rime-api/tests/*.rs                   # ABI/frontend integration binaries (see below)
crates/yune-rime-api/benches/frontend_baselines.rs # criterion-style bench (harness = false)
crates/yune-cli/src/*                             # small CLI unit tests
crates/yune-cli/tests/frontend_surrogate.rs       # CLI surrogate integration test
fixtures/*.json                                   # checked-in CLI output fixtures
fixtures/frontend-traces/*.json                   # sanitized host-trace goldens
packages/yune-typeduck-runtime/test/*.test.ts     # TypeScript Vitest suite
```

## Test Structure

**Suite Organization (CLI golden fixtures):**
```rust
#[cfg(test)]
mod tests {
    use super::{check_fixture, sequence_from_fixture};

    #[test]
    fn checked_in_fixtures_match_cli_output() {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = manifest_dir
            .parent()
            .and_then(Path::parent)
            .expect("CLI crate should live under workspace crates")
            .join("fixtures");
        // collect fixtures, sort them, and compare each fixture to generated output
    }
}
```

**Patterns:**
- Set up process-wide RIME state through `test_guard()` before ABI tests, as in `crates/yune-rime-api/src/tests/mod.rs`, `session_api.rs`, and `config_api.rs`.
- Use focused helper constructors for empty ABI structs, such as `empty_context`, `empty_status`, `empty_config`, `empty_traits`, and `empty_candidate_list_iterator` in `crates/yune-rime-api/src/tests/mod.rs`.
- Use direct assertions against user-visible state: commits, preedit, candidates, schema names, status bits, C strings, and file outputs.
- Use temporary directories with unique names (PID + nanos) for deployment/config tests, then remove them explicitly. See `unique_temp_dir` in `crates/yune-rime-api/src/tests/mod.rs` and in the integration tests.

## Parity / Oracle Tests

Parity against the frozen **TypeDuck-HK/librime v1.1.2** behavior is the core testing discipline. `crates/yune-core/tests/cantonese_parity.rs` validates Yune against the oracle fixture at `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json` (with a `README.md` alongside). The fixture records `oracle.engine` (`TypeDuck-HK/librime`), `engine_tag` (`v1.1.2`), `engine_commit`, `schema`, `module_list`, and per-input `selected_candidates` with their exact panel comment bytes (each comment begins with the TypeDuck panel marker `\u{000c}\r1,`).

Two disciplines that future parity tests MUST follow:

- **Non-circular.** `yune_dictionary_lookup_filter_emits_oracle_bytes_from_source_rows` reconstructs a dictionary YAML from the **raw TSV source rows**, runs `DictionaryLookupFilter::new(dictionary).apply(...)`, and asserts the produced comment equals the oracle comment. It never feeds the expected output back in as input. A separate `typeduck_v112_jyutping_oracle_fixture_is_locked` test locks the fixture's metadata and shape.
- **No silent gaps via `#[ignore]` with a documented blocker.** Parked / not-yet-capturable parity work is left as `#[ignore = "blocked: …"]` tests whose reason string names the blocker (e.g. `#[ignore = "blocked: capture v1.1.2 goldens for combine_candidates, show_full_code, and enable_sentence toggles before enabling"]`), and whose body `panic!`s so the test cannot silently pass. Do **not** delete a planned test — leave a reason-documented ignored stub so the gap is visible in `cargo test` output.

## Frontend / ABI Integration Tests

Integration tests are separate binaries under each crate's `tests/`:

- `crates/yune-rime-api/tests/frontend_client.rs` — drives the public API table through `rime_get_api`, C-compatible structs, and function pointers to approximate frontend usage.
- `crates/yune-rime-api/tests/typeduck_web.rs` — exercises the C/WASM adapter exports (`yune_typeduck_init` / `process_key` / `flip_page` / `select_candidate` / `delete_candidate` / `deploy` / `customize` / `cleanup`, plus `response_json` / `response_handled` / `free_response`), asserting the JSON response contract plus null-input and free behavior. Its reset path drives the engine through `rime_get_api()` function pointers (e.g. `cleanup_all_sessions`, `finalize`) rather than internals.
- `crates/yune-rime-api/tests/dynamic_loader.rs` — builds and `dlopen`s the shared library (`libyune_rime_api.so` / `.dylib` / `yune_rime_api.dll`) via `libloading` and resolves the exported `rime_get_api` symbol, exercising the table exactly as a frontend would. This is the strongest "drive through the real ABI" guarantee. `crates/yune-rime-api` declares `crate-type = ["rlib", "cdylib"]`, so the cdylib must stay loadable.
- `crates/yune-rime-api/tests/frontend_hosts.rs` (+ `frontend_hosts/{mod,native,native_frontends,typeduck_web}.rs`) — models browser-worker and native-host lifecycles through the Yune ABI and validates the host-trace goldens (see Fixtures).
- `crates/yune-cli/tests/frontend_surrogate.rs` — CLI surrogate integration path.

**Fork-only ABI contract (TypeDuck-Windows).** `crates/yune-rime-api/src/tests/config_api.rs` guards the fork-only list-append surface: `config_list_append_creates_and_extends_lists`, `config_list_append_scalar_variants_round_trip_through_accessors`, `config_list_append_rejects_invalid_and_non_list_targets`, and `rime_api_exposes_config_list_append_contract` (which `.expect()`s that `api.config_list_append_{string,bool,int,double}` are populated, with messages tying them to the TypeDuck-Windows requirement). The `RimeApi` struct field **order is the ABI** and must match the fork `rime_api.h`; reordering or removing entries will break the native consumer.

**TypeDuck-Web native fallback.** When the `wasm32-unknown-emscripten` Rust target or `emcc`/`emar` are unavailable, `scripts/typeduck-wasm-build.sh` deliberately runs `cargo test -p yune-rime-api --test typeduck_web` as the native fallback, so the WASM adapter contract is still validated without browser tooling (`FALLBACK_TEST` / `run_native_fallback`). Browser-level validation of M9 is the web-first goal beyond this fallback.

## TypeScript Runtime Tests

`packages/yune-typeduck-runtime/` is the published `@yune-ime/typeduck-runtime` browser runtime. It has a **Vitest** suite (`npm test` → `vitest run`; devDependencies `typescript`, `vitest`). Tests live under `test/` (`response.test.ts`, `typeduck.test.ts`, `filesystem.test.ts`, `keys.test.ts`) and use **hand-written fakes** (`test/fake-filesystem.ts`, `test/fake-module.ts`) for the WASM module and filesystem — mirroring the Rust hand-written-fakes philosophy.

## Cross-platform Test Hygiene

- **Poison-tolerant locks.** Process-wide ABI test locks recover from a poisoned mutex with `.unwrap_or_else(PoisonError::into_inner)` so a panic in one `#[test]` does not cascade into spurious failures elsewhere. See `test_guard()` and `notification_events_lock()` in `crates/yune-rime-api/src/tests/mod.rs`. (Note: `dynamic_loader.rs` uses its own simpler `expect`-based guard.)
- **Shape-based time assertions.** The librime signature `modified_time` is asserted by **shape** via `assert_librime_ctime_shape` — five whitespace-separated fields: weekday, month, day `1..=31`, `HH:MM:SS`, year — not by exact value, because it is generated from `ctime(3)` on unix and an internal `format_ctime_utc` civil-date formatter on non-unix / emscripten (`crates/yune-rime-api/src/lib.rs`). New time-dependent assertions should match shape, not literal output.

## Mocking

**Framework:** hand-written fakes and fixtures (no mocking framework).

**Patterns:**
```rust
// crates/yune-core/src/tests/engine.rs
struct CommentTranslator;

impl Translator for CommentTranslator {
    fn name(&self) -> &'static str {
        "comment_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        if input != "ni" {
            return Vec::new();
        }
        vec![
            Candidate { text: "你".to_owned(), comment: "first-comment".to_owned(),
                        source: CandidateSource::Table, quality: 1.0 },
            Candidate { text: "呢".to_owned(), comment: "second-comment".to_owned(),
                        source: CandidateSource::Table, quality: 1.0 },
        ]
    }
}
```

**What to Mock:**
- Translator/ranker/filter behavior at trait boundaries (from `crates/yune-core`), using local structs like `CommentTranslator` in `crates/yune-core/src/tests/engine.rs` plus production helper types like `StaticTableTranslator`. AI-ranker scaffolding uses `MockAiRanker`.
- Frontend modules with `extern "C"` function pointers, in `crates/yune-rime-api/src/tests/mod.rs` and `crates/yune-rime-api/tests/frontend_client.rs`.
- Runtime config and schema data by writing YAML into unique temp directories, in `crates/yune-rime-api/src/tests/config_api.rs`, `deployment.rs`, and `schema_selection.rs`.

**What NOT to Mock:**
- Do not mock the ABI function table when testing frontend behavior. Reach the engine through `rime_get_api()` / the `RimeApi` function pointers, as in `crates/yune-rime-api/tests/frontend_client.rs`, `typeduck_web.rs`, and `dynamic_loader.rs` (which resolves `rime_get_api` from the built cdylib). Never reach through private internals.
- Do not bypass `Engine` for core behavior tests. Use `Engine`, `StaticTableTranslator`, `PunctuationTranslator`, and key-sequence helpers (re-exported from `crates/yune-core` — defined in `engine.rs`, `translator/`, `punctuation.rs`, `key.rs`) and `crates/yune-cli/src/sample_core.rs`.
- Do not replace checked-in CLI fixtures with ad hoc inline expectations when exercising the fixture contract in `crates/yune-cli/src/fixture.rs`.

## Fixtures and Factories

**Test Data (inline dictionary YAML example):**
```rust
const SAMPLE_DICT: &str = r#"
---
name: sample
version: "0.1"
sort: by_weight
...

你	ni	10
好	hao	10
你好	ni hao	100
"#;
```

**Location:**
- CLI JSON fixtures: `fixtures/sample-nihao.json`, `fixtures/sample-composing.json`, `fixtures/sample-backspace.json`, `fixtures/sample-punctuation.json`.
- Host-trace goldens: `fixtures/frontend-traces/native-host-lifecycle.json`, `squirrel-lifecycle.json`, `typeduck-web-basic.json` — sanitized (no raw pointers) and validated by `crates/yune-rime-api/tests/frontend_hosts.rs` (e.g. `frontend_host_trace_fixture_contract_is_sanitized`, `typeduck_web_basic_fixture_is_sanitized_and_matches_trace_contract`).
- v1.1.2 oracle goldens: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json` (+ `README.md`).
- Shared RIME API test factories: `crates/yune-rime-api/src/tests/mod.rs`.
- Inline YAML fixtures are common in `crates/yune-rime-api/src/tests/config_api.rs`, `deployment.rs`, `schema_processors.rs`, and `schema_selection.rs`.
- Core dictionary and key-sequence test data is largely inline in `crates/yune-core/src/lib.rs` (`mod facade_tests`).

## Coverage

**Requirements:** None enforced by tooling. No coverage command or threshold is configured.

**View Coverage:**
```bash
Not configured
```

## Test Types

**Unit Tests:**
- Core parser, dictionary, translator, filter, ranker, key handling, schema, and CLI behavior via `#[test]` functions in `crates/yune-core` (`mod facade_tests` plus `src/tests/`), `crates/yune-schema/src/lib.rs`, and `crates/yune-cli/src/*.rs`.
- `yune-rime-api` API surfaces via the focused modules under `crates/yune-rime-api/src/tests/`.

**Integration Tests:**
- The `crates/yune-rime-api/tests/*` binaries and `crates/yune-cli/tests/frontend_surrogate.rs` drive public APIs through the ABI table, the WASM adapter exports, and (via `dynamic_loader.rs`) the loaded cdylib.
- `crates/yune-core/tests/cantonese_parity.rs` validates engine output against the v1.1.2 oracle.
- Config/deployment/schema tests write temp runtime directories and YAML files to exercise file-backed behavior.

**Benchmarks:**
- `crates/yune-rime-api/benches/frontend_baselines.rs` is a non-test bench target (`[[bench]] frontend_baselines`, `harness = false`).

**E2E Tests:**
- No automated OS input-method E2E harness. Browser E2E for TypeDuck-Web is the web-first M9 goal (validate in a real browser); until then the `typeduck_web` ABI tests + native fallback and the Vitest runtime suite are the safety net.
- CLI fixture checks in `crates/yune-cli/src/fixture.rs` remain the closest repo-local end-to-end path for core-backed input-sequence output.

## Common Patterns

**Async Testing:**
```rust
Not used. Tests are synchronous Rust `#[test]` functions (no async runtime in any manifest).
```

**Error Testing:**
```rust
let error = Schema::parse_rime_yaml(
    r#"
schema:
  name: Missing ID
"#,
)
.expect_err("schema without schema_id should fail");

assert_eq!(
    error.to_string(),
    "missing required RIME schema field: schema.schema_id"
);
```

- For C ABI error paths, assert `FALSE`, null pointers, or unchanged output state, as in `crates/yune-rime-api/src/tests/session_api.rs`, `config_api.rs`, and `crates/yune-rime-api/tests/frontend_client.rs`.
- For parser errors, prefer `expect_err` plus exact user-facing error text when the message is part of the contract, as in `crates/yune-schema/src/lib.rs`.
- For fixture mismatches, return a detailed `Err(String)` from `check_fixture` in `crates/yune-cli/src/fixture.rs` and panic only at the test boundary.

---

*Last reviewed: 2026-06-17 — refreshed for the TypeDuck-Web (M9) and TypeDuck-Windows (M10) work; current direction is web-first.*
