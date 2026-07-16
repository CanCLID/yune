# Engine Support Contract

Status: Active since M51; unchanged by M52 (performance guardrails only, no ABI/export/storage boundary change); re-verified against the code by the M53 release-readiness audit; updated by M54 to add named native octagram-compatible grammar support without changing the public C ABI; updated by M56 to add productization hardening policies for compiled-artifact staleness, user-data lifecycle, ABI crash behavior, threading, poison recovery, and release panic strategy without widening the ABI; clarified by M59 Increment 4a for upstream Luna ScriptTranslation page shape and compiled-weight domains, by Increment 4b for bounded abbreviation production and shared-cache lifecycle, by Increment 4c for ordered one-to-many OpenCC conversion, by Increment 4d for the explicit upstream-table Cangjie validation policy, and by Increment 4e for exact Luna complete-list ordering, again without ABI widening.

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

## Per-Lane Ranking-Parity Policy (D-48, owner FINAL 2026-07-08)

What "parity" requires differs by lane:

- **Order-parity REQUIRED (page/prefix-exact text AND order through the captured
  range, options mirrored — NOT ordered-subsequence):** canonical
  `rime-cantonese` (Lane A), upstream `luna_pinyin` (Lane B), and `cangjie5`.
  Every `order-only` divergence is classified and returned as a **disposition
  table for owner sign-off** — no self-disposition. Dispositions: **fix**, or a
  **named owner-signed exception** citing the owning feature (e.g. an injected
  single-char family beyond oracle depth). `cangjie5` order rows are onboarded by
  first capturing librime `cangjie5`; rows librime cannot compose become
  owner-provenance beyond-oracle first-candidate pins, scoping cangjie order
  parity to the librime-comparable range.
- **Reachability + comment/profile parity ONLY (no ranking-parity requirement):**
  the TypeDuck profile lane (reaffirms product decision #1 and D-31). Grandfathered
  fixture-backed candidate guards (e.g. `beingo` vs v1.1.2) are frozen regression
  pins — not extended, not deleted; multilingual comments stay oracle-backed
  against TypeDuck `v1.1.2`.

M59 Increment 4c closes Lane A's complete D-48 text/order contract at strict
`13/13` across all pages and 5,705 candidates, including global/page position,
page shape, termination, preedit, and commit preview. No exception or
beyond-oracle-depth row is consumed; the historical 4a class-3 disposition is
unused by this final capture. The named Hong Kong OpenCC chain emits ordered
one-to-many exact whole-word conversions, deduplicates each owned expansion
family stably, and uses only default variants for partial segmentation.
Candidate comments are not part of Lane A's D-48 text/order acceptance and
retain 854 disclosed non-gating differences; canonical comment byte identity
is not claimed. TypeDuck comment/profile parity remains its separate profile
contract.

M59 Increment 4d closes the marked upstream-Cangjie lane at strict all-page
`12/12`, including `tak 30/30`; the owning suite is `3 passed / 0 ignored`, and
the separately captured unmarked control remains exact `12/12`. M59 Increment
4e closes all seven captured upstream Luna inputs (`moboyi`, `boyi`, `yi`,
`zhonggao`, `zhongguo`, `gao`, `guo`) for candidate text, page shape, page-local
and global position, and terminal-page state over tracked byte-backed assets.
All three D-48 lanes are complete without a final-capture exception.

For the captured 37- and 59-character upstream Luna page-zero rows, the required
shape is one best full-span sentence followed by the independent oracle phrase
stream. That is a target-specific capture, not a claim that every Luna input
always has exactly one full-span candidate. Increment 4e proves this page shape
alongside, but separately from, its seven-input Lane B complete-list contract.

## Schema-General Reachability Acceptance

M59 makes `leading_syllable_reachability` default-on for every schema while
retaining request-local TypeDuck-profile `prefix_fallback` precedence. The
canonical [schema-general reachability contract](schema-general-reachability.md)
defines that precedence: correction-bearing requests bypass the ownership
probe, while an eligible request sets `prefix_fallback_owned` only when it
admits a deployed normal proper prefix; an eligible `NoPrefix` result remains
unowned. Existing path-local correction guards then decide whether the
independent leading-syllable path can run. Commit `5fa986d8`
binds that contract to the shipped surface: both manifests cover the exact
60-asset schema tree; the registry dispositions 10 schema assets and three
configuration carriers; and nine executable validation rows link the fixed
eight-row deploy matrix plus the direct selectable tracked `jyut6ping3` path.
New schema assets enter the registry as blocking open rows. M60 retains the
M59 capability version `m59-reach03-v1`, adds formalism version
`m60-reachability-v1`, and permits no current opt-out. This is an
acceptance/audit rule, not a schema-id gate or ABI surface.

M59 closes this contract with final native evidence at `443cc636`; fail-closed
Emscripten, tracked-app/public-package, and functional-browser evidence remains
bound to closeout source `5fa986d8`. All five WEB-04
requirements are complete: the browser proves all four Octagram ranking rows,
the plain-Luna default-off control, and missing-model fail-closed behavior. The
historical WEB-05 same-WASM comparison remains bound to its original binary and was not
rebaselined as a cross-binary M59 gate.

### Upstream Luna Compiled-Weight Semantics

The owned/default Luna sentence model may be reconstructed from source weights
or compiled librime tables, but those domains remain explicit:

- source dictionary/essay weights are raw weights;
- compiled `.table.bin` entry weights are natural logarithms of the source
  weights and are consumed directly by log-domain graph scoring;
- a compiled log value must not be summed as a raw weight or logged again;
- ScriptEncoder's pronunciation-share cutoff is inclusive at 5%, including
  when the source value is represented by a rounded compiled `f32` log.

These are internal target-behavior constraints. They change neither the public
C ABI nor any signed performance threshold.

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

### Shared Dictionary Cache Lifecycle

Fingerprint-keyed immutable dictionary translators and byte-backed
lookup-record indexes may survive `RimeFinalize` across same-root
reinitialization. `RimeFinalize` ends sessions and clears ephemeral
input/page-window caches, but does not invalidate those immutable same-root
entries.

`RimeCleanupAllSessions`, dictionary-root changes, and workspace updates clear
sessions and the full shared dictionary/lookup caches before a path change or
filesystem mutation. Setup, deployment, and workspace mutation remain
externally serialized with session use; internal synchronization does not
authorize concurrent mutation.

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
