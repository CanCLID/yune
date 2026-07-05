# Engine Support Contract

Status: Active since M51; unchanged by M52 (performance guardrails only, no ABI/export/storage boundary change); re-verified against the code by the M53 release-readiness audit; updated by M54 to add named native octagram-compatible grammar support without changing the public C ABI; updated by M56 to add productization hardening policies for compiled-artifact staleness, user-data lifecycle, ABI crash behavior, threading, poison recovery, and release panic strategy without widening the ABI.

This contract defines Yune's launch-facing engine support boundary. It is a
contract for engine behavior, storage, ABI shape, and evidence lanes; it is not
a product, platform frontend, package, deployment, browser performance, browser
memory, or iOS-device claim.

## Supported Engine Targets

Yune supports named targets, not full librime feature parity.

- Upstream `luna_pinyin` and common-schema behavior targets are measured
  against upstream `rime/librime 1.17.0`.
- Canonical Cantonese/Jyutping candidate ordering, segmentation, fallback, and
  completion are measured against upstream `rime/librime 1.17.0` plus pinned
  `rime/rime-cantonese`; M58 owns the final Yune-facing id/provenance split.
- TypeDuck multilingual/profile behavior is measured against
  TypeDuck-HK/librime `v1.1.2` under current shipped profile ids until a
  signed-off schema split. `jyut6ping3_typeduck` is the preferred future
  TypeDuck profile id.
- Broad librime feature parity is not a goal.
- New behavior needs a named target and an oracle fixture before it can become
  required behavior.

## Native Octagram Grammar Support

M54 supports octagram-compatible `.gram` scoring only as a Yune-native
`Grammar` provider for the named upstream `luna_pinyin` octagram target. The
canonical oracle lane is upstream `rime/librime 1.17.0` plus
`lotem/librime-octagram` and pinned `lotem/rime-octagram-data`; the
`amzxyz/RIME-LMDG` lane is pinned real-world validation evidence.

Support rules:

- schemas without `grammar/language` keep the existing null-grammar behavior;
- `grammar/language` is a logical runtime data ID that resolves to
  `<language>.gram`, not an arbitrary filesystem path;
- full third-party `.gram` models are external oracle or validation inputs
  unless a future plan explicitly accepts size, license, and attribution costs;
- contextual translation remains deferred;
- this support does not implement the librime C++ plugin ABI, plugin lifecycle,
  dynamic module loading, Lua, predict, proto, or frontend/platform contracts.

## Compatibility Oracles

The default oracle for core Yune behavior is upstream `rime/librime 1.17.0`.
Canonical `jyut6ping3` candidate behavior uses that upstream engine with pinned
`rime/rime-cantonese`. TypeDuck-HK/librime `v1.1.2` is a profile-only oracle
for TypeDuck compatibility: multilingual comments, dictionary lookup payloads,
display/profile behavior, fork-only ABI/profile controls, and grandfathered
profile candidate guards from historical M14-M28 fixtures. If the upstream
oracle and TypeDuck fork disagree on candidate ordering, segmentation,
fallback, or completion, new canonical `jyut6ping3` claims follow upstream
unless a later explicit decision scopes a TypeDuck-profile-only candidate claim.

The oracle repositories are source references. Yune does not link or call
librime at runtime.

## Default Upstream ABI Contract

`rime_get_api()` returns an upstream-shaped `RimeApi` table. `RimeApi` field
order is ABI because native frontends read the function table by struct-pointer
offset.

Default upstream ABI rules:

- Default `RimeApi` fields match the supported upstream `rime_api.h` shape.
- `RimeCandidate` remains upstream-shaped: `text`, `comment`, `reserved`.
- TypeDuck fork-only slots are not exposed by default `rime_get_api()`.
- `RimeLeversApi` remains covered by upstream-shaped layout expectations.
- New default `RimeApi` fields, reordered fields, or `RimeCandidate` widening
  require a new named upstream target, header evidence, layout tests, and a
  roadmap/requirement update.

## TypeDuck And Yune Windows Profile ABI Contract

TypeDuck fork-only ABI support is opt-in. The named TypeDuck profile accessor
is:

```c
rime_get_typeduck_profile_api()
```

Yune Windows packaging exposes the same current profile table through the
Windows/profile accessor:

```c
rime_get_yune_windows_profile_api()
```

`rime_get_yune_windows_profile_api()` is a parallel profile accessor for the
current Windows package/header lane; it does not widen default `rime_get_api()`.
Both profile tables start with the upstream Yune `RimeApi` prefix and advertise
a larger `data_size`. The current profile delta is the fork-only list-append
family in this order:

- `config_list_append_bool`
- `config_list_append_int`
- `config_list_append_double`
- `config_list_append_string`

These slots must stay behind the named profile accessors. New profile slots
require fresh fork/header evidence, a named profile contract update,
package/header evidence when packaging is affected, and focused tests.

## Yune Web WASM ABI Contract

`yune_web_*` is a Yune-owned browser/WASM ABI family. It is not the default RIME
C ABI and not the TypeDuck profile ABI.

The canonical exported-symbol allowlist is `scripts/yune-web-exports.txt`. The
current allowlist contains exactly these 14 functions:

- `yune_web_init`
- `yune_web_process_key`
- `yune_web_select_candidate`
- `yune_web_delete_candidate`
- `yune_web_flip_page`
- `yune_web_deploy`
- `yune_web_customize`
- `yune_web_set_option`
- `yune_web_set_ai_enabled`
- `yune_web_stage_ai`
- `yune_web_cleanup`
- `yune_web_response_json`
- `yune_web_response_handled`
- `yune_web_free_response`

Adding, renaming, or removing any exported `yune_web_*` function requires
updating `scripts/yune-web-exports.txt`, the Emscripten linker anchor in
`crates/yune-rime-api/src/bin/yune_web_module.rs`, TypeScript runtime calls, and
focused tests.

M51 documents this ABI family only. It makes no browser performance, browser
memory, UX, package, or deployment claim.

## Runtime Storage Contract

Launch profiles rely on compact runtime storage remaining byte-backed or
mmap-backed where that storage was adopted to satisfy memory and launch
readiness:

- compact table storage stays byte-backed where required;
- prism storage stays byte-backed where required;
- lookup/comment payloads stay byte-backed or storage-backed where required by
  TypeDuck profile memory and comment behavior;
- source fallback is a measured blocker, not an acceptable launch default.

Retained heap indexes are allowed only with owner evidence proving they are
small enough for the target. A retained prefix/vocabulary index must not be
introduced silently as a compatibility or latency shortcut.

## Productization Hardening Contract

M56 defines product-facing hardening behavior for external frontends without
adding ABI fields or changing named happy-path behavior.

### Compiled Artifact Staleness

Compiled artifacts are valid only when their magic/version/shape checks pass.
For the named product paths (`luna_pinyin`, canonical Jyutping once M58 pins its
schema identity, and the TypeDuck profile lane under current or future signed-off
ids), cold and warm deploy/select/type conformance tests must keep output pinned
to the relevant oracle fixtures for that lane.

Policy:

- missing compiled artifacts may rebuild from source where the deployment path
  owns source assets;
- present but corrupt, truncated, stale, or unsupported compiled artifacts must
  fail loudly or rebuild on the real deployment path;
- source fallback must not silently mask a present bad artifact;
- optional poet byte-backed storage is controlled by `YUNE_POET_BYTE_BACKED=1`
  and is not part of the default product payload; a missing or stale
  `*.poet.bin` must not force default table/prism/reverse rebuilds when that
  opt-in is disabled;
- optional artifact kinds outside the named product paths, such as `.gram`, get
  artifact-specific stale-injection tests rather than fictional product-path
  coverage.

### User Data Lifecycle

User-data behavior is supported as Yune-defined product behavior unless an
oracle fixture explicitly pins librime behavior for a row.

Policy:

- learning effects must survive session recreation and full RIME reinitialize
  over the same user directory;
- the current file-store format is frozen by tests; any future format change
  needs a migration test and an updated contract row;
- corrupt committed user stores fail closed for learned data while preserving
  ordinary table candidates;
- backup/restore and partial sync failure must preserve local usability and
  report failed peers rather than silently treating a partial merge as fully
  successful.

### ABI Crash, Threading, And Poison Recovery

Every discovered `#[no_mangle] extern "C"` export in `yune-rime-api` enters
through the standardized FFI guard. In unwind-capable test/dev builds, a panic
inside an ABI entry point returns the slot's conservative failure value
(`FALSE`, `0`, or null pointer/string-slice) instead of unwinding across the C
boundary. Arbitrary dangling non-null pointers remain caller undefined
behavior; no C ABI guard can make those safe.

The threading promise is intentionally narrow: process-global runtime/session
state is internally synchronized, and cross-thread calls using valid session
ids are tolerated without panicking, but M56 does not promise parallel progress,
wait-free behavior, or safe aliasing of caller-owned mutable objects across
threads.

If the process-global session registry mutex is poisoned in an unwind build,
the public ABI path recovers the poisoned guard and must keep a follow-up
ordinary lifecycle call usable. This is a recovery policy for Yune's own
process state, not a guarantee that an interrupted caller-owned operation was
semantically completed. Exhaustive poisoning of every process-global registry
and runtime-path lock remains future hardening unless a later frontend exposes
a concrete product risk.

### Release Panic Strategy

The workspace release profile remains:

```toml
[profile.release]
panic = "abort"
```

Release `cdylib` artifacts therefore still abort if a panic is reachable. M56's
release guarantee is the no-reachable-panic abuse suite plus guarded
debug/test behavior, not cross-language unwinding in shipped builds. Flipping a
future release artifact to unwind requires separate binary-size, latency, WASM,
and product evidence plus a contract update.

## Behavior And Performance Evidence Contract

Native, browser, product, and platform claims must stay in separate evidence
lanes.

- Native engine claims require native engine evidence.
- Browser runtime or harness claims require browser evidence.
- TypeDuck product/frontend claims require product/frontend evidence.
- Platform claims require evidence from that platform.
- Windows private/working-set proxies are not Apple `phys_footprint`.

Measured blockers remain blockers until fresh evidence closes them. A report may
attribute a blocker without claiming a reduction.

## Unsupported Or Deferred Surfaces

The following surfaces are unsupported or deferred unless a future named plan
adds a target, oracle, ABI contract, and evidence:

- full librime C++ plugin ABI compatibility;
- learned `.gram` / octagram grammar beyond the named native M54 target;
- contextual translation;
- remote AI providers in the classic deterministic path;
- default ABI widening for TypeDuck fork convenience;
- platform frontend packaging or keyboard-extension contracts;
- browser performance, memory, UX, package, or deployment claims from
  `yune_web_*` ABI documentation alone.

## Change Process

Changing a support boundary requires:

1. Name the target, oracle, and owner module.
2. Capture or cite the oracle/header evidence before implementation.
3. Update this contract and the relevant requirement or roadmap row.
4. Add or update focused layout, profile, export-list, or behavior tests.
5. Keep evidence lanes separate in reports and closeout docs.
