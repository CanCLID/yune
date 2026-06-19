# Upstream Oracle Refresh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-establish upstream `rime/librime 1.17.0` as Yune's default core compatibility oracle, separate TypeDuck `v1.1.2` into an explicit compatibility profile, and land the first upstream parity slice without circular goldens.

**Architecture:** Treat the local upstream checkout as a build and capture cache only; the oracle identity is the GitHub repository, tag, and commit, not a machine-local path. Commit only provenance manifests, source-controlled fixtures, tests, and docs. The default `RimeApi` table follows upstream `1.17.0`; TypeDuck fork-only ABI and Cantonese/Jyutping behavior stays behind TypeDuck-profile labels until that profile is resumed.

**Tech Stack:** Rust workspace (`yune-core`, `yune-rime-api`, `yune-cli`), `serde_json`, librime C ABI headers, PowerShell, Git, CMake/MSVC/Boost for optional upstream local builds, TypeScript runtime verification for preserved TypeDuck-Web gates.

---

## Context

The M12 target is upstream `rime/librime` tag `1.17.0`, release commit
`33e78140250125871856cdc5b42ddc6a5fcd3cd4`. GitHub lists `librime 1.17.0`
as the latest release on 2026-06-19. The local checkout at
`C:\Users\laubonghaudoi\Documents\GitHub\librime` is useful for capture work;
it currently has:

- Remote: `https://github.com/rime/librime.git`
- `master` / local `latest`: `d71168e9e8c8392ed219dca011dbc76b80727d6c`
- Tag object `1.17.0`: `a52a3400f8b7679e839bc5fb8e6309a0fc4424da`
- Peeled `1.17.0` commit: `33e78140250125871856cdc5b42ddc6a5fcd3cd4`

TypeDuck `v1.1.2` remains a compatibility-profile oracle only:

- Repository: `https://github.com/TypeDuck-HK/librime`
- Tag: `v1.1.2`
- Commit: `74cb52b78fb2411137a7643f6c8bc6517acfde69`

The first concrete M12 parity slice is the standard upstream C ABI table. This
is deliberate: the current Yune table contains TypeDuck fork-only fields
(`start_quick`, `config_list_append_*`) in the default `RimeApi`, while upstream
`1.17.0` does not.

## Non-Goals

- Do not resume TypeDuck-Windows frontend E2E.
- Do not un-ignore the five TypeDuck `v1.1.2` Cantonese/Jyutping parity tests.
- Do not make TypeDuck fork-only behavior define core Yune behavior.
- Do not expose AI-native behavior through TypeDuck-Web or Windows.
- Do not commit local upstream build artifacts, DLLs, generated build folders, or
  machine-local absolute paths as canonical oracle identity.

## File Map

- Modify: `docs/plans/upstream-oracle-refresh.md` - this execution plan and live checklist.
- Create: `crates/yune-core/tests/fixtures/upstream-1.17.0/README.md` - human-readable upstream fixture provenance.
- Create: `crates/yune-core/tests/fixtures/upstream-1.17.0/oracle-manifest.json` - machine-readable upstream fixture provenance.
- Create: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/oracle-manifest.json` - machine-readable TypeDuck profile fixture provenance.
- Create: `crates/yune-core/tests/oracle_fixture_provenance.rs` - test that oracle fixture roots declare their provenance.
- Create: `docs/plans/m12-upstream-abi-audit.md` - slot-by-slot upstream vs TypeDuck vs current-Yune ABI audit.
- Modify: `crates/yune-rime-api/src/abi.rs` - make default `RimeApi` match upstream `1.17.0`.
- Modify: `crates/yune-rime-api/src/api_table.rs` - populate the upstream-shaped default API table.
- Modify: `crates/yune-rime-api/src/tests/abi.rs` - assert upstream `1.17.0` default slots.
- Modify: `crates/yune-rime-api/src/tests/config_api.rs` - keep direct tests for TypeDuck fork-only append helpers, but stop requiring them in default `rime_get_api()`.
- Create: `docs/plans/m12-coverage-audit.md` - classification of existing tests/docs as upstream-core, TypeDuck-profile, or deferred.
- Modify: `docs/requirements.md` - mark M12 requirements complete only after evidence lands.
- Modify: `docs/roadmap.md` - update M12 table states after implementation.
- Modify: `docs/CONVENTIONS.md` - keep any new fixture and ABI rules aligned with the implementation.
- Modify: `docs/decisions.md` - add a short follow-up note if the ABI split changes D-24 details.

## Task 1: Confirm Oracle Provenance And Worktree State

**Files:**
- Read: `docs/roadmap.md`
- Read: `docs/requirements.md`
- Read: `docs/CONVENTIONS.md`
- Read: `docs/decisions.md`
- Read: `docs/plans/upstream-oracle-refresh.md`
- Read: `C:\Users\laubonghaudoi\Documents\GitHub\librime\src\rime_api.h`
- Read: `C:\Users\laubonghaudoi\Documents\GitHub\librime\src\rime_levers_api.h`

- [ ] **Step 1: Record the current Yune worktree**

Run:

```powershell
git status --short --branch
```

Expected: the worker knows whether unrelated edits already exist. Do not revert
or stage unrelated edits.

- [ ] **Step 2: Verify upstream remote and release tag**

Run:

```powershell
git -C C:\Users\laubonghaudoi\Documents\GitHub\librime remote -v
git -C C:\Users\laubonghaudoi\Documents\GitHub\librime fetch --tags origin
git -C C:\Users\laubonghaudoi\Documents\GitHub\librime cat-file -p 1.17.0
git -C C:\Users\laubonghaudoi\Documents\GitHub\librime show --no-patch --format="%H%n%D%n%ci%n%s" 33e78140250125871856cdc5b42ddc6a5fcd3cd4
```

Expected:

```text
origin  https://github.com/rime/librime.git (fetch)
origin  https://github.com/rime/librime.git (push)
object 33e78140250125871856cdc5b42ddc6a5fcd3cd4
type commit
tag 1.17.0
33e78140250125871856cdc5b42ddc6a5fcd3cd4
tag: 1.17.0
chore(release): 1.17.0 :tada:
```

- [ ] **Step 3: Verify the current default ABI conflict**

Run:

```powershell
rg -n "start_quick|config_list_append|config_begin_list|get_prebuilt_data_dir|get_staging_dir" crates\yune-rime-api\src\abi.rs crates\yune-rime-api\src\api_table.rs crates\yune-rime-api\src\tests\abi.rs
```

Expected: matches in Yune's default `RimeApi` and ABI layout test. This confirms
M12 must address the TypeDuck fork-only fields before core upstream ABI parity
can be claimed.

- [ ] **Step 4: Commit no changes in this task**

This task is read-only. If it reveals a mismatch in the target commit or remote,
stop and update this plan before proceeding.

## Task 2: Add Machine-Readable Oracle Provenance

**Files:**
- Create: `crates/yune-core/tests/fixtures/upstream-1.17.0/README.md`
- Create: `crates/yune-core/tests/fixtures/upstream-1.17.0/oracle-manifest.json`
- Create: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/oracle-manifest.json`

- [ ] **Step 1: Create the upstream fixture directory**

Run:

```powershell
New-Item -ItemType Directory -Force crates\yune-core\tests\fixtures\upstream-1.17.0
```

Expected: the directory exists and contains no generated oracle bytes yet.

- [ ] **Step 2: Write the upstream README**

Write `crates/yune-core/tests/fixtures/upstream-1.17.0/README.md`:

```markdown
# Upstream librime 1.17.0 Oracle Fixtures

These fixtures are captured from upstream `rime/librime`, not from Yune and not
from the TypeDuck fork. Use them for core Yune compatibility behavior.

## Provenance

- Engine: `rime/librime`
- Engine tag: `1.17.0`
- Engine commit: `33e78140250125871856cdc5b42ddc6a5fcd3cd4`
- Tag object: `a52a3400f8b7679e839bc5fb8e6309a0fc4424da`
- Release URL: <https://github.com/rime/librime/releases/tag/1.17.0>
- Canonical repository: <https://github.com/rime/librime>
- Captured for: M12 upstream oracle refresh

## Capture Rules

- The local upstream checkout may be used as a build cache, but the local path is
  not part of fixture identity.
- Expected bytes must come from upstream librime, never from Yune.
- Every JSON fixture in this directory must include an `oracle` object with the
  engine, tag, commit, capture date, capture command, schema, and input sequence.
- If a case cannot be captured, keep the Yune test ignored with a `panic!()` body
  and document the exact command that would unblock it.
```

- [ ] **Step 3: Write the upstream manifest**

Write `crates/yune-core/tests/fixtures/upstream-1.17.0/oracle-manifest.json`:

```json
{
  "fixture_family": "upstream-core",
  "oracle": {
    "engine": "rime/librime",
    "engine_tag": "1.17.0",
    "engine_commit": "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
    "tag_object": "a52a3400f8b7679e839bc5fb8e6309a0fc4424da",
    "canonical_repository": "https://github.com/rime/librime",
    "release_url": "https://github.com/rime/librime/releases/tag/1.17.0"
  },
  "profile_only": false
}
```

- [ ] **Step 4: Write the TypeDuck manifest**

Write `crates/yune-core/tests/fixtures/typeduck-v1.1.2/oracle-manifest.json`:

```json
{
  "fixture_family": "typeduck-profile",
  "oracle": {
    "engine": "TypeDuck-HK/librime",
    "engine_tag": "v1.1.2",
    "engine_commit": "74cb52b78fb2411137a7643f6c8bc6517acfde69",
    "canonical_repository": "https://github.com/TypeDuck-HK/librime",
    "release_url": "https://github.com/TypeDuck-HK/librime/releases/tag/v1.1.2"
  },
  "profile_only": true
}
```

- [ ] **Step 5: Commit the provenance files**

Run:

```powershell
git add crates\yune-core\tests\fixtures\upstream-1.17.0\README.md crates\yune-core\tests\fixtures\upstream-1.17.0\oracle-manifest.json crates\yune-core\tests\fixtures\typeduck-v1.1.2\oracle-manifest.json
git commit -m "docs: add upstream oracle provenance"
```

Expected: only the three provenance files are staged and committed.

## Task 3: Add Fixture Provenance Guardrails

**Files:**
- Create: `crates/yune-core/tests/oracle_fixture_provenance.rs`

- [ ] **Step 1: Write the failing provenance test**

Create `crates/yune-core/tests/oracle_fixture_provenance.rs`:

```rust
use std::{fs, path::Path};

use serde_json::Value;

fn fixture_root(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn oracle_fixture_roots_have_machine_readable_provenance() {
    assert_manifest(
        "upstream-1.17.0",
        "upstream-core",
        "rime/librime",
        "1.17.0",
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
        false,
    );
    assert_manifest(
        "typeduck-v1.1.2",
        "typeduck-profile",
        "TypeDuck-HK/librime",
        "v1.1.2",
        "74cb52b78fb2411137a7643f6c8bc6517acfde69",
        true,
    );
}

fn assert_manifest(
    fixture_dir: &str,
    expected_family: &str,
    expected_engine: &str,
    expected_tag: &str,
    expected_commit: &str,
    expected_profile_only: bool,
) {
    let root = fixture_root(fixture_dir);
    assert!(
        root.join("README.md").is_file(),
        "{fixture_dir} must include a human-readable README.md"
    );

    let manifest_path = root.join("oracle-manifest.json");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", manifest_path.display()));
    let manifest: Value = serde_json::from_str(&manifest)
        .unwrap_or_else(|error| panic!("invalid JSON {}: {error}", manifest_path.display()));

    assert_eq!(manifest["fixture_family"], expected_family);
    assert_eq!(manifest["oracle"]["engine"], expected_engine);
    assert_eq!(manifest["oracle"]["engine_tag"], expected_tag);
    assert_eq!(manifest["oracle"]["engine_commit"], expected_commit);
    assert_eq!(manifest["profile_only"], expected_profile_only);
    assert!(
        manifest["oracle"]["canonical_repository"]
            .as_str()
            .is_some_and(|url| url.starts_with("https://github.com/")),
        "{fixture_dir} must identify a canonical GitHub oracle repository"
    );
}
```

- [ ] **Step 2: Run the focused test**

Run:

```powershell
cargo test -p yune-core --test oracle_fixture_provenance
```

Expected: PASS after Task 2 files exist.

- [ ] **Step 3: Commit the guardrail test**

Run:

```powershell
git add crates\yune-core\tests\oracle_fixture_provenance.rs
git commit -m "test: require oracle fixture provenance"
```

Expected: only the provenance test is staged and committed.

## Task 4: Build Or Document The Upstream Oracle

**Files:**
- Modify: `crates/yune-core/tests/fixtures/upstream-1.17.0/README.md`

- [ ] **Step 1: Move local librime to the pinned release**

Run:

```powershell
git -C C:\Users\laubonghaudoi\Documents\GitHub\librime checkout 1.17.0
git -C C:\Users\laubonghaudoi\Documents\GitHub\librime submodule update --init --recursive
```

Expected: local librime is detached at `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.

- [ ] **Step 2: Build in a Visual Studio developer shell**

Run these commands from a Developer Command Prompt or a PowerShell session with
MSVC build tools on `PATH`:

```powershell
Set-Location C:\Users\laubonghaudoi\Documents\GitHub\librime
.\build.bat deps
.\build.bat test
```

Expected:

- `build.bat deps` completes without `Error: Boost not found!`.
- `build.bat test` completes with librime built and CTest passing.
- `rime.dll`, `rime_deployer.exe`, and `rime_api_console.exe` are discoverable
  under the local librime `build`, `dist`, or `bin` output tree.

- [ ] **Step 3: Record build evidence or a precise blocker**

If the build passes, append this to the upstream README:

```markdown
## Local Build Evidence

- Build host: Windows with MSVC developer environment
- Build commands:
  - `.\build.bat deps`
  - `.\build.bat test`
- Result: upstream `1.17.0` build and CTest completed successfully
- Required capture tools present: `rime.dll`, `rime_deployer.exe`, `rime_api_console.exe`
```

If the build cannot run, append this instead, replacing the example error with
the exact command and first failing diagnostic:

```markdown
## Local Build Blocker

- Blocked command: `.\build.bat deps`
- Blocking diagnostic: `Error: Boost not found! Please set BOOST_ROOT in env.bat.`
- Reproduction path: run the blocked command from a Visual Studio developer shell
  in a checkout of `rime/librime` tag `1.17.0`.
- M12 impact: runtime byte capture is blocked, but header-based ABI parity can
  continue from `src/rime_api.h` at commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
```

- [ ] **Step 4: Commit the build evidence**

Run:

```powershell
git add crates\yune-core\tests\fixtures\upstream-1.17.0\README.md
git commit -m "docs: record upstream oracle build status"
```

Expected: README-only commit. Do not commit upstream build outputs.

## Task 5: Audit And Refresh The Default C ABI

**Files:**
- Create: `docs/plans/m12-upstream-abi-audit.md`
- Modify: `crates/yune-rime-api/src/abi.rs`
- Modify: `crates/yune-rime-api/src/api_table.rs`
- Modify: `crates/yune-rime-api/src/tests/abi.rs`
- Modify: `crates/yune-rime-api/src/tests/config_api.rs`

- [ ] **Step 1: Create the ABI audit document**

Create `docs/plans/m12-upstream-abi-audit.md` with this table:

```markdown
# M12 Upstream ABI Audit

Upstream source: `rime/librime` tag `1.17.0`, commit
`33e78140250125871856cdc5b42ddc6a5fcd3cd4`, file `src/rime_api.h`.

| Field | Upstream 1.17.0 slot | Current Yune slot before M12 | M12 classification | Action |
|---|---:|---:|---|---|
| `start_maintenance` | 4 | 4 | upstream core | keep |
| `start_quick` | absent | 5 | TypeDuck profile | remove from default `RimeApi`; keep implementation only as parked profile code |
| `is_maintenance_mode` | 5 | 6 | upstream core | move to upstream slot 5 |
| `config_begin_map` | 44 | 46 | upstream core | move to upstream slot 44 |
| `config_begin_list` | 68 | 45 | upstream core | move to upstream slot 68 |
| `config_list_append_bool` | absent | 72 | TypeDuck profile | remove from default `RimeApi`; keep direct helper tests |
| `config_list_append_int` | absent | 73 | TypeDuck profile | remove from default `RimeApi`; keep direct helper tests |
| `config_list_append_double` | absent | 74 | TypeDuck profile | remove from default `RimeApi`; keep direct helper tests |
| `config_list_append_string` | absent | 75 | TypeDuck profile | remove from default `RimeApi`; keep direct helper tests |
| `get_input` | 69 | 76 | upstream core | move to upstream slot 69 |
| `get_prebuilt_data_dir` | 80 | 55 | upstream core | move to upstream slot 80 |
| `get_staging_dir` | 81 | 56 | upstream core | move to upstream slot 81 |
| `change_page` | 97 | 102 | upstream core | move to upstream slot 97 |
| function slot count | 98 | 103 | upstream core | default `RimeApi` exposes 98 function slots |
```

- [ ] **Step 2: Write the failing upstream ABI test**

In `crates/yune-rime-api/src/tests/abi.rs`, change
`rime_api_function_table_layout_matches_librime_header` so it asserts these
upstream `1.17.0` slots:

```rust
assert_api_slot!(setup, 0);
assert_api_slot!(set_notification_handler, 1);
assert_api_slot!(initialize, 2);
assert_api_slot!(finalize, 3);
assert_api_slot!(start_maintenance, 4);
assert_api_slot!(is_maintenance_mode, 5);
assert_api_slot!(join_maintenance_thread, 6);
assert_api_slot!(deployer_initialize, 7);
assert_api_slot!(prebuild, 8);
assert_api_slot!(deploy, 9);
assert_api_slot!(deploy_schema, 10);
assert_api_slot!(deploy_config_file, 11);
assert_api_slot!(sync_user_data, 12);
assert_api_slot!(create_session, 13);
assert_api_slot!(find_session, 14);
assert_api_slot!(destroy_session, 15);
assert_api_slot!(cleanup_stale_sessions, 16);
assert_api_slot!(cleanup_all_sessions, 17);
assert_api_slot!(process_key, 18);
assert_api_slot!(commit_composition, 19);
assert_api_slot!(clear_composition, 20);
assert_api_slot!(get_commit, 21);
assert_api_slot!(free_commit, 22);
assert_api_slot!(get_context, 23);
assert_api_slot!(free_context, 24);
assert_api_slot!(get_status, 25);
assert_api_slot!(free_status, 26);
assert_api_slot!(set_option, 27);
assert_api_slot!(get_option, 28);
assert_api_slot!(set_property, 29);
assert_api_slot!(get_property, 30);
assert_api_slot!(get_schema_list, 31);
assert_api_slot!(free_schema_list, 32);
assert_api_slot!(get_current_schema, 33);
assert_api_slot!(select_schema, 34);
assert_api_slot!(schema_open, 35);
assert_api_slot!(config_open, 36);
assert_api_slot!(config_close, 37);
assert_api_slot!(config_get_bool, 38);
assert_api_slot!(config_get_int, 39);
assert_api_slot!(config_get_double, 40);
assert_api_slot!(config_get_string, 41);
assert_api_slot!(config_get_cstring, 42);
assert_api_slot!(config_update_signature, 43);
assert_api_slot!(config_begin_map, 44);
assert_api_slot!(config_next, 45);
assert_api_slot!(config_end, 46);
assert_api_slot!(simulate_key_sequence, 47);
assert_api_slot!(register_module, 48);
assert_api_slot!(find_module, 49);
assert_api_slot!(run_task, 50);
assert_api_slot!(get_shared_data_dir, 51);
assert_api_slot!(get_user_data_dir, 52);
assert_api_slot!(get_sync_dir, 53);
assert_api_slot!(get_user_id, 54);
assert_api_slot!(get_user_data_sync_dir, 55);
assert_api_slot!(config_init, 56);
assert_api_slot!(config_load_string, 57);
assert_api_slot!(config_set_bool, 58);
assert_api_slot!(config_set_int, 59);
assert_api_slot!(config_set_double, 60);
assert_api_slot!(config_set_string, 61);
assert_api_slot!(config_get_item, 62);
assert_api_slot!(config_set_item, 63);
assert_api_slot!(config_clear, 64);
assert_api_slot!(config_create_list, 65);
assert_api_slot!(config_create_map, 66);
assert_api_slot!(config_list_size, 67);
assert_api_slot!(config_begin_list, 68);
assert_api_slot!(get_input, 69);
assert_api_slot!(get_caret_pos, 70);
assert_api_slot!(select_candidate, 71);
assert_api_slot!(get_version, 72);
assert_api_slot!(set_caret_pos, 73);
assert_api_slot!(select_candidate_on_current_page, 74);
assert_api_slot!(candidate_list_begin, 75);
assert_api_slot!(candidate_list_next, 76);
assert_api_slot!(candidate_list_end, 77);
assert_api_slot!(user_config_open, 78);
assert_api_slot!(candidate_list_from_index, 79);
assert_api_slot!(get_prebuilt_data_dir, 80);
assert_api_slot!(get_staging_dir, 81);
assert_api_slot!(commit_proto, 82);
assert_api_slot!(context_proto, 83);
assert_api_slot!(status_proto, 84);
assert_api_slot!(get_state_label, 85);
assert_api_slot!(delete_candidate, 86);
assert_api_slot!(delete_candidate_on_current_page, 87);
assert_api_slot!(get_state_label_abbreviated, 88);
assert_api_slot!(set_input, 89);
assert_api_slot!(get_shared_data_dir_s, 90);
assert_api_slot!(get_user_data_dir_s, 91);
assert_api_slot!(get_prebuilt_data_dir_s, 92);
assert_api_slot!(get_staging_dir_s, 93);
assert_api_slot!(get_sync_dir_s, 94);
assert_api_slot!(highlight_candidate, 95);
assert_api_slot!(highlight_candidate_on_current_page, 96);
assert_api_slot!(change_page, 97);
assert_eq!(
    std::mem::size_of::<RimeApi>(),
    align_up(table_start + fn_size * 98, fn_align)
);
```

Note: the macro's slot index starts at `setup`, not at `data_size`. The upstream
header has 98 function-pointer fields after `data_size`.

- [ ] **Step 3: Run the focused test and confirm it fails**

Run:

```powershell
cargo test -p yune-rime-api abi::rime_api_function_table_layout_matches_librime_header
```

Expected: FAIL because the current default table still contains TypeDuck slots.

- [ ] **Step 4: Update `RimeApi` and `api_table.rs` to upstream order**

Implement the minimum change:

- Remove `start_quick` from default `RimeApi`.
- Remove `config_list_append_bool`, `config_list_append_int`,
  `config_list_append_double`, and `config_list_append_string` from default
  `RimeApi`.
- Place `config_begin_map`, `config_next`, and `config_end` immediately after
  `config_update_signature`.
- Place `config_begin_list` immediately after `config_list_size`.
- Place `get_prebuilt_data_dir` and `get_staging_dir` after
  `candidate_list_from_index`, matching upstream `1.17.0`.
- Keep the exported helper implementations `RimeStartQuick` and
  `RimeConfigListAppend*` in their existing modules only if they still compile
  and are covered by direct tests; they are parked TypeDuck-profile code, not
  default upstream API fields.

- [ ] **Step 5: Update config append tests**

In `crates/yune-rime-api/src/tests/config_api.rs`, keep these direct helper
tests because they validate parked TypeDuck-profile implementation behavior:

- `config_list_append_creates_and_extends_lists`
- `config_list_append_scalar_variants_round_trip_through_accessors`
- `config_list_append_rejects_invalid_and_non_list_targets`

Delete or rewrite `rime_api_exposes_config_list_append_contract` so no default
`rime_get_api()` test requires TypeDuck-only fields. Add this replacement:

```rust
#[test]
fn default_rime_api_exposes_upstream_config_list_contract() {
    let _guard = test_guard();
    let api = unsafe { &*rime_get_api() };

    assert!(
        api.config_list_size.is_some(),
        "upstream RimeApi exposes config_list_size"
    );
    assert!(
        api.config_begin_list.is_some(),
        "upstream RimeApi exposes config_begin_list"
    );
}
```

- [ ] **Step 6: Run focused ABI and config tests**

Run:

```powershell
cargo test -p yune-rime-api abi::rime_api_function_table_layout_matches_librime_header
cargo test -p yune-rime-api config_api::default_rime_api_exposes_upstream_config_list_contract
cargo test -p yune-rime-api config_api::config_list_append_creates_and_extends_lists
cargo test -p yune-rime-api config_api::config_list_append_scalar_variants_round_trip_through_accessors
cargo test -p yune-rime-api config_api::config_list_append_rejects_invalid_and_non_list_targets
```

Expected: all PASS.

- [ ] **Step 7: Commit the ABI refresh**

Run:

```powershell
git add docs\plans\m12-upstream-abi-audit.md crates\yune-rime-api\src\abi.rs crates\yune-rime-api\src\api_table.rs crates\yune-rime-api\src\tests\abi.rs crates\yune-rime-api\src\tests\config_api.rs
git commit -m "fix: align default rime api with upstream oracle"
```

Expected: only ABI refresh files are committed.

## Task 6: Audit Existing TypeDuck Assumptions

**Files:**
- Create: `docs/plans/m12-coverage-audit.md`
- Modify: affected comments/test names only when the audit finds unlabeled profile behavior.

- [ ] **Step 1: Generate the candidate list**

Run:

```powershell
rg -n "TypeDuck|typeduck|v1\.1\.2|start_quick|config_list_append|dictionary_lookup|librime" crates docs scripts packages
```

Expected: a review list covering TypeDuck fixtures, TypeDuck-Web tests, ABI tests,
config append tests, docs, and packaging scripts.

- [ ] **Step 2: Create the coverage audit document**

Create `docs/plans/m12-coverage-audit.md`:

```markdown
# M12 Coverage Audit

| Path | Coverage | Classification | M12 action |
|---|---|---|---|
| `crates/yune-rime-api/src/tests/abi.rs` | Default `RimeApi` layout | upstream core | Refresh to upstream `1.17.0` slot order |
| `crates/yune-rime-api/src/tests/config_api.rs` | Direct `RimeConfigListAppend*` helpers | TypeDuck profile implementation | Keep direct helper tests; remove default API-table requirement |
| `crates/yune-core/tests/cantonese_parity.rs` | TypeDuck v1.1.2 Cantonese/Jyutping comments and ignored blockers | TypeDuck profile | Keep profile-only; do not un-ignore without genuine TypeDuck goldens |
| `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` | Captured TypeDuck fixtures | TypeDuck profile | Add manifest; keep under `typeduck-v1.1.2/` |
| `crates/yune-rime-api/src/tests/schema_selection.rs` | Reverse lookup prompt bytes from TypeDuck v1.1.2 | TypeDuck profile | Keep explicit TypeDuck name in test and fixture |
| `crates/yune-rime-api/tests/typeduck_web.rs` | TypeDuck-Web adapter and real-assets behavior | TypeDuck-Web profile gate | Keep green; do not convert to upstream core |
| `scripts/package-typeduck-windows.ps1` | Native TypeDuck-Windows package smoke | parked TypeDuck-Windows profile | Do not run as an M12 gate; update only when TypeDuck profile resumes |
| `docs/plans/yune-windows-contract-implementation-plan.md` | TypeDuck-Windows plan | parked TypeDuck-Windows profile | Keep parked/reference banner |
| `docs/plans/yune-windows-native-build.md` | TypeDuck-Windows packaging | parked TypeDuck-Windows profile | Keep parked/reference banner |
```

- [ ] **Step 3: Rename or comment any unlabeled profile tests**

If the audit finds a test whose name says only `librime` but whose expected bytes
come from TypeDuck `v1.1.2`, rename it or add a comment containing
`TypeDuck profile oracle: v1.1.2`. Do not change behavior in this step.

- [ ] **Step 4: Run focused TypeDuck profile gates**

Run:

```powershell
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
```

Expected:

- `cantonese_parity`: active tests pass and the five documented TypeDuck blockers
  remain ignored.
- `typeduck_web`: PASS, preserving the web profile path.

- [ ] **Step 5: Commit the audit**

Run:

```powershell
git add docs\plans\m12-coverage-audit.md crates\yune-core\tests\cantonese_parity.rs crates\yune-rime-api\src\tests\schema_selection.rs crates\yune-rime-api\tests\typeduck_web.rs scripts\package-typeduck-windows.ps1 docs\plans\yune-windows-contract-implementation-plan.md docs\plans\yune-windows-native-build.md
git diff --cached --name-only
git commit -m "docs: audit TypeDuck profile coverage for M12"
```

Expected: staged paths are limited to audit docs and any naming/comment fixes
that directly label TypeDuck-profile behavior.

## Task 7: Update Milestone Docs After Evidence Lands

**Files:**
- Modify: `docs/requirements.md`
- Modify: `docs/roadmap.md`
- Modify: `docs/CONVENTIONS.md`
- Modify: `docs/decisions.md`
- Modify: `AGENTS.md` only if the repo guide's one-line oracle wording is stale.

- [ ] **Step 1: Mark M12 requirements complete only when their evidence exists**

Update `docs/requirements.md`:

- `UPSTREAM-ORACLE-01`: checked only after upstream README and manifest land.
- `UPSTREAM-ORACLE-02`: checked only after both upstream and TypeDuck manifest
  roots exist and the provenance test passes.
- `UPSTREAM-AUDIT-01`: checked only after `docs/plans/m12-coverage-audit.md`
  lands.
- `TYPEDUCK-PROFILE-01`: checked only after TypeDuck tests/docs/scripts are
  labeled profile-only and default upstream ABI no longer depends on fork-only
  fields.

- [ ] **Step 2: Update roadmap M12 table**

Update `docs/roadmap.md` M12 rows from `Pending` to the actual state:

- `Pin upstream oracle`: `Done` when Task 2 and Task 3 pass.
- `Fixture naming policy`: `Done` when manifest guard test passes.
- `TypeDuck assumption audit`: `Done` when Task 6 lands.
- `First upstream parity slice`: `Done` when Task 5 lands.

- [ ] **Step 3: Update conventions and decisions**

Make the smallest needed edits:

- `docs/CONVENTIONS.md`: default `RimeApi` means upstream `1.17.0`; TypeDuck
  fork-only fields require an explicit TypeDuck profile surface.
- `docs/decisions.md`: append a D-24 note that M12's first parity slice refreshed
  the default API table to upstream `1.17.0`.

- [ ] **Step 4: Run docs consistency scans**

Run:

```powershell
rg -n --glob "!docs/plans/upstream-oracle-refresh.md" "TypeDuck.*default oracle|v1\.1\.2.*default core|M10.*active|upstream.*start_quick.*default|upstream.*config_list_append.*default|start_quick.*upstream core|config_list_append.*upstream core" AGENTS.md docs crates
rg -n --glob "!docs/plans/upstream-oracle-refresh.md" "upstream-1\.17\.0|typeduck-v1\.1\.2|UPSTREAM-ORACLE|UPSTREAM-AUDIT|TYPEDUCK-PROFILE" AGENTS.md docs crates
```

Expected:

- First scan returns no false claims. If it finds intentional profile references,
  reword them so the profile label is explicit.
- Second scan shows the upstream and TypeDuck fixture labels in the expected docs
  and tests.

- [ ] **Step 5: Commit milestone docs**

Run:

```powershell
git add AGENTS.md docs\requirements.md docs\roadmap.md docs\CONVENTIONS.md docs\decisions.md
git diff --cached --name-only
git commit -m "docs: close M12 oracle refresh requirements"
```

Expected: staged paths are docs only.

## Task 8: Full Verification

**Files:**
- Read-only verification over the whole workspace.

- [ ] **Step 1: Format Rust**

Run:

```powershell
cargo fmt
```

Expected: command exits 0.

- [ ] **Step 2: Run focused M12 tests**

Run:

```powershell
cargo test -p yune-core --test oracle_fixture_provenance
cargo test -p yune-rime-api abi::rime_api_function_table_layout_matches_librime_header
cargo test -p yune-rime-api config_api::default_rime_api_exposes_upstream_config_list_contract
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
```

Expected: all non-ignored tests PASS; existing TypeDuck blocker tests remain
ignored with `panic!()` bodies.

- [ ] **Step 3: Run workspace tests and lints**

Run each command separately in PowerShell:

```powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix packages/yune-typeduck-runtime test
npm --prefix packages/yune-typeduck-runtime run build
git diff --check
```

Expected: all commands exit 0. `git diff --check` may warn about CRLF-to-LF
normalization only if Git reports it outside whitespace-error status; whitespace
errors must be fixed.

- [ ] **Step 4: Final review pass**

Review the final diff for:

- No committed generated upstream build artifacts.
- No local machine path used as canonical oracle identity.
- No expected bytes generated from Yune.
- No TypeDuck fork-only field required by default upstream `rime_get_api()`.
- TypeDuck-Web tests still green.
- M12 requirements and roadmap table match actual evidence.

- [ ] **Step 5: Final commit if verification changed formatting**

If `cargo fmt` changed M12 Rust files, stage only those files and commit:

```powershell
git add crates\yune-core\tests\oracle_fixture_provenance.rs crates\yune-rime-api\src\abi.rs crates\yune-rime-api\src\api_table.rs crates\yune-rime-api\src\tests\abi.rs crates\yune-rime-api\src\tests\config_api.rs
git commit -m "style: format M12 oracle refresh changes"
```

Expected: no unrelated files are staged.

## Completion Criteria

M12 is complete when all of the following are true:

- Upstream `rime/librime 1.17.0` provenance is checked in under
  `upstream-1.17.0/`.
- TypeDuck `v1.1.2` provenance is checked in under `typeduck-v1.1.2/` and marked
  profile-only.
- A provenance guard test prevents unlabeled oracle fixture roots.
- The default `RimeApi` table matches upstream `1.17.0` slot order and function
  count.
- TypeDuck fork-only ABI helpers are not required by default `rime_get_api()`.
- Existing TypeDuck behavior is audited and labeled as profile-only.
- M12 docs and requirements reflect the evidence that actually landed.
- `cargo fmt`, focused M12 tests, `cargo test --workspace`,
  `cargo clippy --workspace --all-targets -- -D warnings`, TypeScript runtime
  test/build, and `git diff --check` pass.
