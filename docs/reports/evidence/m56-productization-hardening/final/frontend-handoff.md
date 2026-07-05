# M56 Frontend Handoff

This note is for external frontend work that embeds Yune after M56.

## New Engine Guarantees

### Compiled Artifacts

- Missing compiled artifacts may rebuild from source where the deployment path
  owns source assets.
- Present corrupt, truncated, stale, or unsupported compiled artifacts fail
  loudly or rebuild on the real deployment path.
- Source fallback must not silently mask a present bad artifact.
- Cold and warm product-path conformance is pinned for `luna_pinyin` and
  TypeDuck `jyut6ping3`.
- The default `apps/yune-web/public/schema` payload remains the WEB-03 launch
  set. M56 does not ship optional `*.poet.bin` artifacts or newly generated
  product dictionaries in the public/default schema payload.
- Warm-start conformance is self-contained: tests deploy once in an isolated
  temp shared/user root, then deploy a second time over that temp output and
  assert table/prism/reverse reuse.

Primary tests:

- `cargo test -p yune-rime-api dictionary_data_ -- --nocapture`
- `cargo test -p yune-rime-api workspace_update_rebuilds_source_dictionary_artifacts_and_reuses_fresh_outputs -- --nocapture`
- `cargo test -p yune-rime-api --test cold_start_conformance -- --nocapture`
- `cargo test -p yune-rime-api resource_id::schema_octagram_loading -- --nocapture`
- `npm.cmd --prefix apps/yune-web run check:schema-manifest`
- `npm.cmd --prefix apps/yune-web run build:public`

### User Data

- Learned data survives session recreation and full RIME reinitialize over the
  same user directory.
- The current userdb file-store format is frozen by tests.
- Corrupt committed stores fail closed for learned data while ordinary table
  candidates remain usable.
- Partial peer sync failure reports failure and preserves valid local/peer
  merges.

Primary test:

- `cargo test -p yune-rime-api userdb:: -- --nocapture`

### ABI Crash Boundary

- Every discovered `#[no_mangle] extern "C"` export enters through the FFI
  guard.
- In unwind-capable test/dev builds, guarded panics return conservative failure
  values instead of unwinding across the C boundary.
- Release artifacts keep `panic = "abort"`; the release guarantee is the
  no-reachable-panic abuse suite, not cross-language unwinding.
- Process-global session/runtime state recovers poisoned locks for follow-up
  ordinary ABI lifecycle calls.
- Valid cross-thread session lookup is tolerated; M56 does not promise
  parallel progress or safe aliasing of caller-owned mutable objects across
  threads.
- Dangling non-null pointers remain caller undefined behavior.

Primary tests:

- `cargo test -p yune-rime-api --lib abi:: -- --nocapture`
- `cargo test -p yune-rime-api session_registry_poison_recovery_preserves_followup_happy_path -- --nocapture`
- `cargo test -p yune-rime-api --test abi_abuse -- --nocapture`

## Contract Pointers

- Support contract:
  `docs/contracts/engine-support-contract.md`
- Phase evidence:
  `docs/reports/evidence/m56-productization-hardening/`
- Completed plan:
  `docs/plans/completed/m56-plan-engine-productization-hardening.md`
