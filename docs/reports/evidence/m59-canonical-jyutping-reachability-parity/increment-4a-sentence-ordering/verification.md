# Increment 4a verification record

Implementation commit: `ca52ec427111e2ec36b2a80dfe7b25b6f2d3c456`.

The following completed successfully against that commit:

- `cargo fmt --all -- --check`;
- `cargo clippy -p yune-core -p yune-rime-api --all-targets -- -D warnings`;
- `cargo test -p yune-core --lib` — 412 passed;
- `cargo test -p yune-rime-api --lib` — 345 passed, 1 ignored;
- `cargo test -p yune-core --test cantonese_parity` — 41/41;
- `typeduck_windows_boundary` — 4/4;
- focused Luna, Cangjie, Double Pinyin, Bopomofo, API/web, paging, physical
  PageDown, profile-ranking, long-prefix, and source-current product controls;
- exact `upstream_script_policy_merges_phrase_sentence_and_partial_families_across_storage_paths`
  — 1 passed;
- exact `upstream_sentence_model_reads_candidates_from_byte_backed_poet_artifact`
  — 1 passed; and
- exact `dictionary_data_ignores_compiled_poet_artifact_until_explicitly_enabled`
  — 1 passed; and
- `python -B -m unittest -v scripts.tests.test_m59_evidence_tools.ResidualClassifierTests`
  — 13 passed; and
- `python -B -m unittest -v scripts.tests.test_m59_evidence_tools`
  — 74 passed.

The focused browser/API controls passed in isolated processes. A full 50-test
`yune_web` process was also attempted, but timed out after 15 minutes under
cache pressure; the full browser suite is therefore not claimed green here.
It remains a final M59-GATES-01 obligation.

The signed performance aggregation command in
[`performance-ratchet/README.md`](./performance-ratchet/README.md) exits `0` with
32/32 aggregate rows passing. This increment packet is not a substitute for the
full native release/WASM/browser/package gates or five final rounds from M59's
final behavior commit.
