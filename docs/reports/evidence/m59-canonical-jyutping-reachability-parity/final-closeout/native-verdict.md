# Native verdict

Verdict: **PASS with targeted recovery from a recorded host-memory abort.**

| Surface | Verdict |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| Retained workspace-test prefix | CLI 34/34; frontend surrogate 5/5; core library 528/528; canonical provenance 2/2 |
| Interrupted target | host allocation abort during `cantonese_parity`; no assertion failure |
| `cantonese_parity` recovery | 41/41 PASS, single-threaded |
| Remaining core integration targets | all executed tests PASS; eight declared ignores |
| `cargo test -p yune-rime-api` | 476 executed PASS; four declared ignores |
| Doc-tests | PASS |
| Release build | PASS |

The recovery is intentionally non-duplicative: successful `5fa986d8`
targets from the broad run were not rerun. The normalized interrupted receipt
and all recovery receipts are in `logs/native/`.
