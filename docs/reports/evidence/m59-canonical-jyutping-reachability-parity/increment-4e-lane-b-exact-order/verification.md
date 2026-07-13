# Verification

Production source: `5879405c7b0f76af4dca7382f00b3e0605386f2c`

Source tree: `bbba5a0c1edf46385077f3ebd91afeb2ebcedf7e`

Pinned librime: `33e78140250125871856cdc5b42ddc6a5fcd3cd4`

## Focused Rust and product gates

- `cargo fmt --check`: pass.
- `cargo clippy -p yune-core -p yune-rime-api --all-targets -- -D warnings`: pass.
- `cargo test -p yune-core --lib`: `528 passed / 0 failed`.
- `cargo test -p yune-rime-api --lib`: `364 passed / 0 failed / 1 ignored`.
- `m59_lane_b_product_matches_complete_pinned_librime_order`: `1/1`.
- `m59_luna_long_sentence_page_order_matches_pinned_oracle_on_byte_backed_product`: `1/1`.
- `m59_correction_oracle_source_and_compiled_deploy_paths_match_complete_order`: `1/1`.
- Equal-weight two-Script producer prescription regression: `1/1`.
- Weighted/initial-quality normalized merge regression: `1/1`.
- User-only Script outer-merge regression: `1/1`.
- Owned/byte-backed normalized merge-quality regression: `1/1`.
- `python -m unittest scripts.tests.test_m59_web04_native_runner`: `2/2`.
- PowerShell runner parser: `0` errors.
- `npm.cmd --prefix apps/yune-web run check:schema-manifest`: `59` assets verified.
- `git diff --check`: pass before production commit.

## Native WEB-04

Verdict: `8 passed / 0 failed` across four plain/null controls and four Octagram
rows. Source was clean and exact; provenance mode was `source-built-release`.

- Release CLI SHA-256: `d738aae187addcaeb53857082d9f3cabc9afacafe5cb180a62bd4775eec8f7d8`.
- Release DLL SHA-256: `b7eb08519a140586acf94fb00949862179f54570905bee793f6c4a1b9eef55f7`.
- Oracle fixture SHA-256: `885ca3e611aa819e52733dfcd03f9eba5e233c78cd02e37e26cb2fdd77e09882`.
- Grammar model SHA-256: `574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3`.
- Verdict SHA-256: `6140ebc834e5a8bfc7a1430afa31db0ee20f9db394b15683486f5acfdd588f70`.

## Five-round signed ratchet

Verdict: `32/32` aggregate rows pass, `160/160` individual observations pass.

- Threshold file SHA-256: `e74e77b4dd5b253e0c2b5f4b12cc1e0279784d3c3fbf02006b5f8f18fccacdba`.
- Benchmark executable SHA-256: `c85fe3fe63d10c4e6bcd467a961c9e60d2740701b895327ae65b5a6493add541`.
- Run-1 build receipt SHA-256: `e98edd924fd5af396adc056c95ee6b9846c460b366ec7da5b7c91544136cb3d0`.
- Aggregate verdict SHA-256: `b3c68da83ac08914a48fed346fcbb7d3176a04a14f3e530283dddc3189fda7cf`.
- Aggregate provenance SHA-256: `0f1c776e8386ac88cdf7517bb3584bcc566eb2febc48fbd72ca24f528a0a0242`.
- Mode sequence: `build,reuse,reuse,reuse,reuse`.
- Product deployment: enabled in every run.
- Inputs/iterations/environment/source: identical in all five runs.

The pre-declared short-key owner stop was not triggered.
