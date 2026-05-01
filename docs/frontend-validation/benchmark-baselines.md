# Frontend-Sensitive Benchmark Baselines

Phase 06 Plan 04 records ABI-observed frontend-sensitive baselines for `BENCH-01` and `BENCH-02` after the validation evidence from D-12 was available: native host lifecycle validation, TypeDuck-Web wrapper validation, and Squirrel/macOS plus Linux follow-up scoping. These are not direct `yune-core` microbenchmarks; the harness resolves `yune_rime_api::rime_get_api` and drives the `RimeApi` function table that frontend hosts observe.

## Scope and decisions

- `D-12`: benchmark categories were selected after Plans 06-01, 06-02, and 06-03 identified the frontend lifecycle surfaces to keep measurable.
- `D-13` / `BENCH-01`: covered session create/destroy, simple ASCII per-key `RimeProcessKey`, schema-loaded per-key lookup, schema deployment/dictionary loading, and userdb learning/sync.
- `D-14` / `BENCH-02`: used a dependency-free `std::time` harness with bounded operation counts, synthetic fixtures, logical schema/userdb IDs, and reproducible command metadata.

## How to run

```bash
/Users/trenton/.cargo/bin/cargo bench -p yune-rime-api --bench frontend_baselines
```

The bench target is declared in `crates/yune-rime-api/Cargo.toml` as:

```toml
[[bench]]
name = "frontend_baselines"
harness = false
```

## Run metadata

| Field | Value |
|---|---|
| Plan | 06-04 |
| Harness | `crates/yune-rime-api/benches/frontend_baselines.rs` |
| Benchmark layer | `rime_get_api` / `RimeApi` function table |
| Dependency policy | dependency-free; no Criterion or external frontend runtime |
| Build profile | Cargo `bench` profile, optimized |
| Rust toolchain | `rustc 1.95.0 (59807616e 2026-04-14)` |
| Platform | Darwin arm64 |
| Baseline commit | `d55e982` |
| External services | none |
| External frontend daemons/toolchains | none |
| User data | synthetic temp fixtures only |

## Baseline results

Command output from `/Users/trenton/.cargo/bin/cargo bench -p yune-rime-api --bench frontend_baselines`:

| benchmark | operations | fixture | data_size | total_ms | us_per_op |
|---|---:|---|---|---:|---:|
| session_create_destroy | 200 | synthetic-basic-schema | sessions=200 | 0.143 | 0.716 |
| per_key_simple_ascii_rime_process_key | 600 | default-echo-schema | keys=600 status/context/commit/free cycles=600 | 13.159 | 21.931 |
| per_key_schema_loaded_lookup_rime_process_key | 400 | lookup-schema-table-dictionary | dictionary_entries=4 sessions=200 status/context/commit/free cycles=200 | 125.187 | 312.967 |
| schema_deploy_dictionary_load | 20 | lookup-schema-table-dictionary | dictionary_entries=4 deploy_cycles=20 | 30.621 | 1531.042 |
| userdb_learning_sync | 80 | learn-schema-synthetic-userdb | seeded_userdb_records=3 commits=80 syncs=80 | 155.693 | 1946.159 |

## Scenario notes

### session_create_destroy

- Creates and destroys sessions through `RimeApi.create_session` and `RimeApi.destroy_session`.
- Uses a synthetic basic schema fixture.
- Covers frontend session lifecycle latency without direct engine calls.

### per_key_simple_ascii_rime_process_key

- Uses one session and repeatedly calls `RimeProcessKey` for simple ASCII keys.
- Each key iteration performs status, context, and commit reads where available and pairs successful reads with `free_status`, `free_context`, and `free_commit`.
- Measures frontend-observed key-processing plus read/free overhead.

### per_key_schema_loaded_lookup_rime_process_key

- Uses a synthetic schema-loaded table dictionary fixture with four entries.
- Each iteration creates a session, selects logical schema ID `lookup`, processes `b` and `a`, reads status/context/commit, frees ABI allocations, and destroys the session.
- Measures the schema-loaded lookup path required by `BENCH-01`.

### schema_deploy_dictionary_load

- Runs deployer initialization, workspace deploy, schema deploy for logical resource `lookup.schema.yaml`, session creation, schema selection, and destroy.
- Uses the same synthetic lookup dictionary fixture.
- Represents schema deployment and dictionary loading as frontend-observed ABI work.

### userdb_learning_sync

- Uses logical schema ID `learn` and synthetic userdb records only.
- Each iteration creates a session, selects the learning schema, processes `n` and `i`, commits composition to trigger learning, destroys the session, and runs `sync_user_data`.
- The fixture seeds three userdb records and keeps sync output inside temp fixture directories.

## Reproducibility and comparison guidance

- Compare future frontend or AI-native changes using the same command, profile, platform class, and operation counts before interpreting deltas.
- Treat the `std::time` harness as a coarse local baseline, not a statistical benchmarking suite. Run multiple times if investigating small changes.
- Do not compare debug-profile timings against these optimized Cargo `bench` results.
- Preserve the benchmark layer: future comparisons for `BENCH-01` and `BENCH-02` should continue to use `rime_get_api` / `RimeApi`, not direct `yune-core` shortcuts.
- If operation counts or fixtures change, record the new counts and fixture names alongside the old baseline rather than replacing the historical values silently.

## Sanitization

The committed baseline omits absolute temp paths, home-directory paths, raw pointers, process IDs, noisy timestamps, environment secrets, and personal user dictionary contents. Userdb benchmark data is synthetic and reported as counts plus fixture names, not raw personal dictionary material.
