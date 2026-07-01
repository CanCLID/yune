# Engine Support Contract

Status: Active since M51; unchanged by M52 (performance guardrails only, no ABI/export/storage boundary change); re-verified against the code by the M53 release-readiness audit; updated by M54 to add named native octagram-compatible grammar support without changing the public C ABI.

This contract defines Yune's launch-facing engine support boundary. It is a
contract for engine behavior, storage, ABI shape, and evidence lanes; it is not
a product, platform frontend, package, deployment, browser performance, browser
memory, or iOS-device claim.

## Supported Engine Targets

Yune supports named targets, not full librime feature parity.

- Upstream `luna_pinyin` and common-schema behavior targets are measured
  against upstream `rime/librime 1.17.0`.
- TypeDuck `jyut6ping3` profile behavior is measured against
  TypeDuck-HK/librime `v1.1.2`.
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
TypeDuck-HK/librime `v1.1.2` is a profile-only oracle for TypeDuck
compatibility. If the upstream oracle and TypeDuck fork disagree, upstream
behavior wins for default core behavior unless the behavior is explicitly
installed behind a named TypeDuck profile test, fixture, adapter, or ABI note.

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
