# M26 Task 0 Baseline

> **Status:** Captured before M26 implementation - **Milestone:** M26 (engine/runtime performance hardening) - **Captured:** 2026-06-22 - **Type:** evidence

## Commands

- `git status --short --branch --untracked-files=all`: `main` matched `origin/main`; the dirty files were the active M26 docs/plan files.
- `cargo bench -p yune-rime-api --bench frontend_baselines`: passed; Cargo printed the known `yune_rime_api` output filename collision warnings, but the benchmark exited 0.
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M25 DOGFOOD-03" --workers=1`: passed, 1/1 Playwright test.

## Native Synthetic Baseline

These rows are the pre-M26 synthetic benchmark harness, not large-real-asset evidence.

| benchmark | operations | fixture | data_size | total_ms | us_per_op |
| --- | ---: | --- | --- | ---: | ---: |
| `session_create_destroy` | 200 | `synthetic-basic-schema` | `sessions=200` | 3.328 | 16.637 |
| `per_key_simple_ascii_rime_process_key` | 600 | `default-echo-schema` | `keys=600 status/context/commit/free cycles=600` | 20.450 | 34.083 |
| `per_key_schema_loaded_lookup_rime_process_key` | 400 | `lookup-schema-table-dictionary` | `dictionary_entries=4 sessions=200 status/context/commit/free cycles=200` | 153.955 | 384.888 |
| `schema_deploy_dictionary_load` | 20 | `lookup-schema-table-dictionary` | `dictionary_entries=4 deploy_cycles=20` | 71.345 | 3567.225 |
| `userdb_learning_sync` | 80 | `learn-schema-synthetic-userdb` | `seeded_userdb_records=3 commits=80 syncs=80` | 739.479 | 9243.493 |

## Browser M25-Style Baseline

The focused M25 browser smoke still records worker/action timings rather than true keydown-to-paint timing. It rewrote `apps/yune-web/e2e/results/m25-dogfooding/M25-DOGFOOD-03/typing-latency-after.json`; that M25 artifact is intentionally not an M26 deliverable and should not be committed from this Task 0 run.

| field | value |
| --- | ---: |
| input | `hai` |
| processKeyActionCount | 6 |
| processKeyTypingCount | 5 |
| p95 totalMs | 63 |
| loadingIndicatorCount | 0 |

Action diagnostics from the M25-style smoke:

| action | queueWaitMs | workerRoundtripMs | workerMs | totalMs |
| --- | ---: | ---: | ---: | ---: |
| `processKey` | 0 | 27 | 26 | 27 |
| `processKey` | 0 | 1 | 0 | 2 |
| `processKey` | 0 | 11 | 10 | 11 |
| `processKey` | 0 | 0 | 0 | 0 |
| `processKey` | 0 | 9 | 9 | 9 |
| `processKey` | 0 | 0 | 0 | 0 |

Typing diagnostics from the M25-style smoke:

| input | totalMs |
| --- | ---: |
| `h` | 63 |
| `h` | 3 |
| `ha` | 14 |
| `ha` | 2 |
| `hai` | 12 |

## Limitations

- Native rows above use the existing 4-entry synthetic dictionary; Task 1 must add large-real-asset rows and median/p95/p99/max reporting.
- Browser rows above are existing worker/action timings; Task 2 must add keydown-to-paint or the closest browser-supported proxy and split worker queue, worker processing, roundtrip, response mapping, React state application, and candidate panel paint markers.
- No allocation/RSS data is available from the current synthetic harness.
