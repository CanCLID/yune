# M51 Engine Support Contract And ABI Freeze Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Write and enforce the engine support contract for Yune's launch-ready engine surface: supported profiles, schema behavior targets, storage/runtime expectations, and ABI freeze rules.

**Architecture:** M51 is a contract-and-guard milestone after M50. It does not add product/frontends or widen ABI; it consolidates the rules already spread across `AGENTS.md`, `docs/conventions.md`, M10/M19 reference docs, requirements, and ABI tests into one support contract, then adds narrow tests/scripts that fail when the default upstream ABI, TypeDuck profile boundary, or `yune_web_*` export family drifts.

**Tech Stack:** Markdown docs, Rust ABI layout tests (`crates/yune-rime-api/src/tests/abi.rs`), package/header smoke tests, existing TypeDuck profile ABI tests, `scripts/yune-web-exports.txt`, `crates/yune-rime-api/src/web_runtime.rs`, `cargo fmt`, `cargo clippy`, and focused compatibility suites.

---

## Scope

In scope:

- Engine support contract for the repo, not platform frontend plans.
- Default upstream `rime_get_api()` expectations.
- TypeDuck profile ABI expectations through `rime_get_typeduck_profile_api()`.
- Yune-owned browser/WASM `snake_case` `yune_web_*` export expectations through
  `scripts/yune-web-exports.txt`.
- `RimeApi`, `RimeLeversApi`, `RimeCandidate`, and core lifecycle support promises.
- Supported named target set: upstream `luna_pinyin` / common-schema behavior against librime `1.17.0`, TypeDuck `jyut6ping3` profile behavior against TypeDuck-HK/librime `v1.1.2`, and compact byte-backed storage expectations.
- Explicit non-goals and unsupported librime features.

Out of scope:

- iOS/macOS packaging, Apple `phys_footprint`, keyboard-extension UX, or platform SDK work.
- Browser/public-demo performance, memory, UX, packaging, or deployment claims.
  The `yune_web_*` exported-function family is in scope only as an engine ABI
  contract.
- New RIME feature implementation unless a named target requires it.
- ABI widening.

## Files And Responsibilities

- Create: `docs/contracts/engine-support-contract.md`
  - Human-readable support contract and launch scope.
- Modify: `docs/conventions.md`
  - Link to the contract and keep the C ABI rules as the quick-reference version.
- Modify: `docs/requirements.md`
  - Add or update requirement IDs for the engine support contract and ABI freeze.
- Modify: `docs/roadmap.md`
  - Close M51 when complete and point future engine work at the contract.
- Modify if needed: `crates/yune-rime-api/src/tests/abi.rs`
  - Add missing layout/offset/data-size guards for default and TypeDuck profile ABI.
- Modify if needed: `crates/yune-rime-api/src/tests/config_api.rs`
  - Keep TypeDuck fork-only list append helpers profile-scoped.
- Modify if needed: `scripts/package-typeduck-windows.ps1`
  - Only if the existing package smoke does not prove the profile contract stated by the new document.
- Modify if needed: `scripts/yune-web-exports.txt`
  - Only if the written contract intentionally changes the browser/WASM export
    set; otherwise it is the allowlist to verify.
- Modify if needed: `crates/yune-rime-api/src/web_runtime.rs`
  - Only if a test exposes a mismatch between exported `yune_web_*` functions
    and the allowlist.
- Modify if needed: `crates/yune-rime-api/src/bin/yune_web_module.rs`
  - Keep the Emscripten linker anchor in sync with the export allowlist.
- Create: `docs/reports/evidence/m51-engine-support-contract-abi-freeze/`
  - Contract review notes, ABI gate outputs, and closeout evidence.

## Task 0: Baseline Contract Inventory

**Files:**

- Read: `AGENTS.md`
- Read: `docs/conventions.md`
- Read: `docs/requirements.md`
- Read: `docs/decisions.md`
- Read: `docs/plans/reference/m10-reference-typeduck-windows-contract.md`
- Read: `docs/plans/reference/m10-reference-typeduck-windows-native-build.md`
- Read: `docs/plans/reference/m19-reference-typeduck-profile-abi.md`
- Read: `scripts/yune-web-exports.txt`
- Read: `crates/yune-rime-api/src/web_runtime.rs`
- Read: `crates/yune-rime-api/src/bin/yune_web_module.rs`
- Create: `docs/reports/evidence/m51-engine-support-contract-abi-freeze/contract-inventory.md`

- [ ] **Step 0.1: Record the current contract sources**

Create `contract-inventory.md` with:

```markdown
# M51 Contract Inventory

| Source | Contract facts used by M51 |
| --- | --- |
| `AGENTS.md` | default ABI is upstream-shaped; TypeDuck fork slots are profile-only |
| `docs/conventions.md` | C ABI rules, package/header behavior, data-flow boundaries |
| `docs/requirements.md` | existing ABI and host-validation requirements |
| `docs/decisions.md` | oracle precedence and target-driven scope |
| `docs/plans/reference/m10-reference-typeduck-windows-contract.md` | historical TypeDuck fork requirements and caveats |
| `docs/plans/reference/m19-reference-typeduck-profile-abi.md` | named TypeDuck profile accessor and slot order |
| `scripts/yune-web-exports.txt` | canonical 14-symbol `yune_web_*` browser/WASM export allowlist |
| `crates/yune-rime-api/src/web_runtime.rs` | implementation of exported `yune_web_*` functions |
| `crates/yune-rime-api/src/bin/yune_web_module.rs` | linker anchor keeping `yune_web_*` functions reachable for Emscripten |
```

Add direct line references or short excerpts only where needed; avoid long pasted blocks.

- [ ] **Step 0.2: Run current ABI gates**

Run:

```powershell
cargo test -p yune-rime-api abi
cargo test -p yune-rime-api config_api
cargo test -p yune-rime-api typeduck_windows_boundary
cargo test -p yune-rime-api --test yune_web yune_web_adapter_processes_keys_and_returns_json_state
```

If a selector does not match tests in the current tree, replace it with the narrow existing test command that covers the same ABI surface and record the replacement in `contract-inventory.md`.

- [ ] **Step 0.3: Commit inventory**

```powershell
git add -- docs/reports/evidence/m51-engine-support-contract-abi-freeze/contract-inventory.md
git commit -m "Inventory engine ABI support contract"
git push origin main
```

## Task 1: Write The Engine Support Contract

**Files:**

- Create: `docs/contracts/engine-support-contract.md`
- Modify: `docs/conventions.md`

- [ ] **Step 1.1: Create the contract document**

Create `docs/contracts/engine-support-contract.md` with these sections:

```markdown
# Engine Support Contract

Status: Active after M51 closeout.

## Supported Engine Targets

## Compatibility Oracles

## Default Upstream ABI Contract

## TypeDuck Profile ABI Contract

## Yune Web WASM ABI Contract

## Runtime Storage Contract

## Behavior And Performance Evidence Contract

## Unsupported Or Deferred Surfaces

## Change Process
```

- [ ] **Step 1.2: Fill Supported Engine Targets**

State exactly:

- upstream `luna_pinyin` and common-schema behavior targets are measured against upstream `rime/librime 1.17.0`;
- TypeDuck `jyut6ping3` profile behavior is measured against TypeDuck-HK/librime `v1.1.2`;
- broad librime feature parity is not a goal;
- new behavior needs a named target and oracle fixture.

- [ ] **Step 1.3: Fill ABI contract sections**

State exactly:

- default `rime_get_api()` returns an upstream-shaped `RimeApi`;
- `RimeApi` field order is ABI;
- `RimeCandidate` remains upstream-shaped unless a future named profile adds a separate opt-in surface;
- TypeDuck fork-only slots stay behind `rime_get_typeduck_profile_api()`;
- new TypeDuck profile slots require fresh TypeDuck fork-header evidence and tests.

- [ ] **Step 1.4: Fill the Yune Web WASM ABI contract section**

State exactly:

- `yune_web_*` is a Yune-owned browser/WASM ABI family, not the default RIME C
  ABI and not the TypeDuck profile ABI.
- The canonical exported-symbol allowlist is `scripts/yune-web-exports.txt`.
- The current allowlist contains exactly these 14 functions:
  `yune_web_init`, `yune_web_process_key`, `yune_web_select_candidate`,
  `yune_web_delete_candidate`, `yune_web_flip_page`, `yune_web_deploy`,
  `yune_web_customize`, `yune_web_set_option`, `yune_web_set_ai_enabled`,
  `yune_web_stage_ai`, `yune_web_cleanup`, `yune_web_response_json`,
  `yune_web_response_handled`, and `yune_web_free_response`.
- Adding, renaming, or removing any exported `yune_web_*` function requires
  updating `scripts/yune-web-exports.txt`, the Emscripten linker anchor in
  `crates/yune-rime-api/src/bin/yune_web_module.rs`, TypeScript runtime calls,
  and focused tests.
- M51 documents this ABI family only; it makes no browser performance, memory,
  UX, package, or deployment claim.

- [ ] **Step 1.5: Fill runtime storage and evidence sections**

State exactly:

- compact table, prism, and lookup/comment payloads should stay byte-backed/mmap-backed where launch profiles rely on them;
- source fallback is a measured blocker, not an acceptable launch default;
- native, browser, product, and platform claims must stay separate;
- Windows private/working-set proxies are not Apple `phys_footprint`.

- [ ] **Step 1.6: Link the contract from conventions**

Add one paragraph near the C ABI rules in `docs/conventions.md`:

```markdown
The detailed launch-facing engine contract lives in
[`docs/contracts/engine-support-contract.md`](./contracts/engine-support-contract.md).
Keep this section as the quick C ABI rule reference; update the contract when a
support boundary changes.
```

- [ ] **Step 1.7: Commit the contract draft**

```powershell
git add -- docs/contracts/engine-support-contract.md docs/conventions.md
git commit -m "Document engine support contract"
git push origin main
```

## Task 2: Freeze ABI Expectations With Tests

**Files:**

- Modify if needed: `crates/yune-rime-api/src/tests/abi.rs`
- Modify if needed: `crates/yune-rime-api/src/tests/config_api.rs`
- Modify if needed: `crates/yune-rime-api/src/api_table.rs`
- Create: `docs/reports/evidence/m51-engine-support-contract-abi-freeze/abi-freeze.md`

- [ ] **Step 2.1: Compare the written contract to existing tests**

Check whether tests already assert:

- `RimeApi.data_size`;
- every default upstream slot offset that frontends use;
- `RimeLeversApi.data_size`;
- `RimeCandidate` size and field offsets;
- TypeDuck profile accessor data size and fork-only slots;
- absence of fork-only slots from default upstream `rime_get_api()`.
- `scripts/yune-web-exports.txt` matches the exported `yune_web_*` functions
  documented in the contract and kept alive by `yune_web_module.rs`.

Record the result in `abi-freeze.md`.

- [ ] **Step 2.2: Add missing guard tests only**

If a listed expectation is missing, add a direct layout, function-table, or
export-list test. Use existing `field_offset` / `assert_api_slot!` helpers in
`crates/yune-rime-api/src/tests/abi.rs` for RIME ABI checks rather than
introducing a new ABI framework. For `yune_web_*`, prefer a small test or script
that compares `scripts/yune-web-exports.txt` against the exported functions
named in `web_runtime.rs` and the linker anchor in `yune_web_module.rs`.

Do not change ABI implementation unless a test exposes a real mismatch with the current contract.

- [ ] **Step 2.3: Run focused ABI tests**

Run:

```powershell
cargo test -p yune-rime-api abi
cargo test -p yune-rime-api config_api
cargo test -p yune-rime-api --test yune_web yune_web_adapter_processes_keys_and_returns_json_state
```

Expected: pass.

- [ ] **Step 2.4: Commit ABI guards**

```powershell
git add -- crates/yune-rime-api/src/tests/abi.rs crates/yune-rime-api/src/tests/config_api.rs scripts/yune-web-exports.txt crates/yune-rime-api/src/web_runtime.rs crates/yune-rime-api/src/bin/yune_web_module.rs docs/reports/evidence/m51-engine-support-contract-abi-freeze/abi-freeze.md
git commit -m "Freeze engine ABI expectations"
git push origin main
```

If no test changes are needed, commit only `abi-freeze.md` with a message that says the existing guards already cover the contract.

## Task 3: Align Requirements And Roadmap

**Files:**

- Modify: `docs/requirements.md`
- Modify: `docs/roadmap.md`
- Create: `docs/reports/evidence/m51-engine-support-contract-abi-freeze/docs-sync.md`

- [ ] **Step 3.1: Add requirement IDs**

Add requirement entries using the existing requirement style:

- `M51-CONTRACT-01`: engine support contract exists and is linked from conventions/roadmap.
- `M51-ABI-01`: default upstream ABI layout remains locked by tests.
- `M51-ABI-02`: TypeDuck fork-only ABI remains profile-scoped.
- `M51-ABI-03`: `yune_web_*` exported-symbol ABI remains synchronized with
  `scripts/yune-web-exports.txt`.
- `M51-EVIDENCE-01`: native/browser/product/platform claims must cite their own evidence lanes.

- [ ] **Step 3.2: Update the roadmap**

Update `docs/roadmap.md` so M51 is marked complete only after:

- the contract exists;
- ABI tests pass;
- broad clippy remains green after M50 Task 0;
- M50 latency rows may still close as measured partial and do not block M51
  unless the M50 work changes an ABI or support boundary;
- no platform/frontend work is claimed.

- [ ] **Step 3.3: Write docs sync evidence**

Create `docs-sync.md` with:

```markdown
# M51 Docs Sync

Updated:

- `docs/contracts/engine-support-contract.md`
- `docs/conventions.md`
- `docs/requirements.md`
- `docs/roadmap.md`

No platform frontend, browser performance, browser memory, packaging, or
iOS-device claim is made. The only browser-related scope is the `yune_web_*`
engine ABI/export-list contract.
```

- [ ] **Step 3.4: Commit docs sync**

```powershell
git add -- docs/requirements.md docs/roadmap.md docs/reports/evidence/m51-engine-support-contract-abi-freeze/docs-sync.md
git commit -m "Align roadmap with engine support contract"
git push origin main
```

## Task 4: Final Gate And Closeout

**Files:**

- Modify: `docs/ledgers/milestone-history.md`
- Move: `docs/plans/active/m51-plan-engine-support-contract-abi-freeze.md`
  to `docs/plans/completed/m51-plan-engine-support-contract-abi-freeze.md`
- Create: `docs/reports/evidence/m51-engine-support-contract-abi-freeze/final-gates.md`

- [ ] **Step 4.1: Run final gates**

Required:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-rime-api abi
cargo test -p yune-rime-api config_api
cargo test -p yune-rime-api --test yune_web yune_web_adapter_processes_keys_and_returns_json_state
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
```

Run `cargo test --workspace` if any ABI implementation or shared runtime behavior changed.

- [ ] **Step 4.2: Record final gates**

Create `final-gates.md`:

```markdown
# M51 Final Gates

| Gate | Result | Notes |
| --- | --- | --- |
| `cargo fmt --check` | record actual result | record any stderr summary |
| `cargo clippy --workspace --all-targets -- -D warnings` | record actual result | record any stderr summary |
| `cargo test -p yune-rime-api abi` | record actual result | record selected test count |
| `cargo test -p yune-rime-api config_api` | record actual result | record selected test count |
| `cargo test -p yune-rime-api --test yune_web yune_web_adapter_processes_keys_and_returns_json_state` | record actual result | record selected test count |
| `cargo test -p yune-core --test upstream_luna_pinyin_parity` | record actual result | record selected test count |
| `cargo test -p yune-core --test cantonese_parity` | record actual result | record selected test count |

Verdict: success/partial.
```

- [ ] **Step 4.3: Close with correct verdict**

Success requires:

- contract document created and linked;
- ABI expectations either already covered or newly covered by tests;
- TypeDuck fork-only slots remain profile-scoped;
- `yune_web_*` exported symbols remain synchronized with
  `scripts/yune-web-exports.txt`;
- no default ABI widening;
- final gates pass.

Partial if any test or packaging smoke remains blocked; record the measured blocker.

- [ ] **Step 4.4: Commit closeout**

```powershell
git add -- docs/ledgers/milestone-history.md docs/plans/completed/m51-plan-engine-support-contract-abi-freeze.md docs/reports/evidence/m51-engine-support-contract-abi-freeze/final-gates.md
git commit -m "Close M51 engine support contract"
git push origin main
```

If M51 remains partial, keep this plan under `docs/plans/active/` and commit the partial evidence without moving the plan.
