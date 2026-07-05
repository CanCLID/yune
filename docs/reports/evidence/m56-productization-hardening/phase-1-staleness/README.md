# M56 Phase 1 Staleness And Cold/Warm Evidence

Date: 2026-07-04

## Disposition

- Compiled table/prism/reverse runtime load now fails closed when a present
  compiled artifact is corrupt or invalid. Source fallback remains allowed
  only when compiled artifacts are missing.
- Workspace deploy rebuilds corrupt/stale compiled table/prism/reverse
  artifacts through the real `workspace_update` path.
- Product cold/warm conformance is covered by
  `crates/yune-rime-api/tests/cold_start_conformance.rs` for `luna_pinyin`
  and TypeDuck `jyut6ping3`.
- Optional `.gram` loading has corrupt-payload real-path coverage in
  `resource_id::schema_octagram_loading_rejects_corrupt_gram_without_silent_grammar_success`.
- Poet artifact stale rejection is covered by the existing core test
  `poet::upstream_sentence_model_rejects_stale_poet_artifact_checksum`.
  M56 does not add committed product `*.poet.bin` payloads: poet storage
  remains optional and is not part of the default public schema payload.
- TypeDuck dictionary-lookup product assets now treat a normal translator table
  without rich TypeDuck lookup rows as stale for `dictionary_lookup_filter`
  consumers. This closes the `jyut6ping3_scolar.table.bin` order hazard where
  a desktop rebuild could otherwise be reused by the mobile comments-intact
  path while lacking the rich lookup bytes required by the TypeDuck fixtures.
- Deploy-skip audit: N/A at the engine ABI layer. Phase 0 found no
  engine-level skip-redeploy path; M41's skip incident lived in the
  web-harness worker.

## Commands Run

```powershell
cargo test -p yune-rime-api dictionary_data_ -- --nocapture
cargo test -p yune-rime-api workspace_update_rebuilds_source_dictionary_artifacts_and_reuses_fresh_outputs -- --nocapture
$env:YUNE_WEB03_EVIDENCE_DIR='docs\reports\evidence\m56-productization-hardening\phase-1-staleness\public-schema-regeneration'; $env:YUNE_WEB03_APPLY_ASSETS='1'; cargo test -p yune-rime-api --test yune_web web03_regenerates_public_schema_compiled_assets_from_clean_rebuild -- --ignored --nocapture
cargo test -p yune-rime-api --test cold_start_conformance -- --nocapture
cargo test -p yune-rime-api resource_id::schema_octagram_loading -- --nocapture
```

## Runtime Notes

- Final clean public-schema regeneration keeps the WEB-03 launch set only and
  wrote evidence under `public-schema-regeneration/task2-native-regeneration/`.
  The generated inventory contains no `*.poet.bin` payloads and no newly added
  product dictionaries beyond the pre-M56 launch assets.
- `cold_start_conformance` completed in 180.96s in the final workspace gate.
  The test is self-contained: each product path deploys once in an isolated
  shared/user temp root, then deploys a second time over the same temp output.
  The second deploy asserts `ReusedFresh` for the target table/prism/reverse
  artifacts and does not depend on committed repo `*.poet.bin` files.
