# M27 Startup Path Reconciliation

> **Status:** Captured before M27 optimization - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

## Native Benchmark Path

- `run_real_startup_runtime_ready` copies real TypeDuck-Web schema/build assets before starting its timer, then times `setup_fixture`, `create_session`, `select_schema`, `destroy_session`, and runtime reset. It does not time `write_real_schema_assets`.
- `run_real_deploy_cache_hit` copies assets and initializes the fixture once, then times repeated `deployer_initialize`, `deploy`, and `deploy_schema` calls only. It does not time the later RIME session init/schema-selection path.
- M26 native `startup_real_jyut6ping3_mobile_runtime_ready` therefore maps to the browser's post-asset RIME init/schema-selection wait more closely than `deploy_real_jyut6ping3_mobile_cache_hit`.

## Browser Evidence From M26

- Fresh startup records `assets:load:finish` at `125ms`, `schema:select:start` at `125ms`, and `schema:select:finish` / `runtime:initialized` at `10712ms`.
- Reload startup records `assets:load:finish` at `104ms`, `schema:select:start` at `104ms`, and `schema:select:finish` / `runtime:initialized` at `10420ms`.
- Fresh browser persistence diagnostics record `schema:deploy:start` at `2026-06-22T06:11:48.651Z`, `deploy:cache-miss` at `2026-06-22T06:11:48.687Z`, and `schema:deploy:finish` at `2026-06-22T06:11:48.737Z`, followed by `rime:init:start` at `2026-06-22T06:11:48.738Z` and `rime:init:finish` at `2026-06-22T06:11:59.202Z`.
- Reload browser persistence diagnostics record `schema:deploy:start` at `2026-06-22T06:11:59.750Z`, `deploy:cache-miss` at `2026-06-22T06:11:59.782Z`, and `schema:deploy:finish` at `2026-06-22T06:11:59.830Z`, followed by `rime:init:start` at `2026-06-22T06:11:59.830Z` and `rime:init:finish` at `2026-06-22T06:12:10.031Z`.

## Reconciliation Result

- Browser-paid path: mixed, but the expensive path is post-deploy RIME init/schema selection over preloaded deployed assets.
- Native benchmark row that matches browser path: `startup_real_jyut6ping3_mobile_runtime_ready`.
- Native benchmark row that must not drive browser optimization: `deploy_real_jyut6ping3_mobile_cache_hit`.
- Evidence for fresh browser path: deploy takes about `86ms`, while post-deploy `rime:init` takes about `10464ms` and aligns with the `schema:select`/`runtime:init` browser interval.
- Evidence for reload browser path: deploy takes about `80ms`, while post-deploy `rime:init` takes about `10201ms` and aligns with the `schema:select`/`runtime:init` browser interval.
- Startup owners to split next: `setup`, `initialize`, `create_session`, `select_schema_total`, `schema_config_load`, `processor_install`, `translator_install`, `compiled_table_load`, `compiled_prism_load`, `source_dictionary_parse_if_any`, `translator_index_build`, `filter_install`, `userdb_open_or_sync`, `destroy_session`, `teardown_or_finalize`.
