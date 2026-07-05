# M56 Phase 0 Undocumented Contract Gaps

Cross-check target: `docs/contracts/engine-support-contract.md` at `84a3c0dedcfed53887427f0d50af5366d5e61f78`.

## Gaps Driving Phase 1

| Gap | Evidence | Required disposition |
| --- | --- | --- |
| Staleness policy is only implicit in runtime storage wording. It does not explicitly say invalid/stale compiled artifacts must be loud and must not silently source-fallback. | `schema_install.rs::load_schema_dictionary_by_name` currently records `dictionary_source_fallback` for rejected compiled dictionaries when source YAML exists. | Add support-contract staleness rows and tests proving rebuild-or-error behavior. |
| Product-path cold/warm conformance is not a contract row. | Existing tests cover parity and smaller dictionary fixtures, but not a standing fresh-dir deploy/select/type conformance pair for `luna_pinyin` and TypeDuck `jyut6ping3`. | Add cold/warm tests and cite them in the contract/evidence. |
| Optional artifact handling is not split from product-path handling. | Poet is explicit opt-in via `YUNE_POET_BYTE_BACKED`; octagram `.gram` is external grammar input. | Contract should state artifact-specific stale/corrupt checks for optional/out-of-path artifacts. |

## Gaps Driving Phase 2

| Gap | Evidence | Required disposition |
| --- | --- | --- |
| Userdb lifecycle guarantee is not launch-contract explicit. | Contract has runtime storage rows but no learning, persistence, recovery, or sync rows. | Add user-data lifecycle rows with test pointers. |
| Userdb format and migration policy are not documented. | `*.userdb` and `*.userdb.txt` are text formats with metadata rows but no explicit version row. | Add a format-freeze test and contract row, or a named blocked migration row if old-format capture is needed. |
| Corrupt committed-store recovery is not documented. | Existing recovery test covers interrupted temp write, not a corrupt/truncated committed store. | Add test and contract row defining bounded data loss/no crash behavior. |
| Sync surface scope is not documented. | `userdb/sync.rs` is local snapshot merge/backup, not a cloud service. | Add local-only sync contract row and cite tests. |

## Gaps Driving Phase 3

| Gap | Evidence | Required disposition |
| --- | --- | --- |
| Crash/panic policy is not documented. | Root `Cargo.toml` has `panic = "abort"`; grep found no `catch_unwind` before M56. | Add release/dev panic policy rows and enforce with guards plus abuse tests. |
| Poison policy is not documented. | Production locks still use `.expect("... should not be poisoned")` across session/runtime/config/notification/module state. | Define recovery or fail-closed behavior and add a focused poison recovery test. |
| Threading promise is not documented. | Support contract has no threading rows; conventions only describe process-global singletons. | Add Yune-defined threading row before concurrent tests. |
| ABI entry-point guard coverage is not contract/test enforced. | `phase-0-inventory/abi-entry-ledger.csv` lists 162 `#[no_mangle] extern "C"` exports and all are currently unguarded. | Add guard module, apply to all exports, and add meta-test to keep the ledger synchronized. |
