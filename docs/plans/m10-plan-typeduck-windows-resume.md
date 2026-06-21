# M10 TypeDuck-Windows Native Backend Resume Implementation Plan

> **Status:** Blocked at T2 - **Milestone:** M10 (TypeDuck-Windows native backend) - **Updated:** 2026-06-21 - **Type:** execution plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Yune a verified native backend for the real TypeDuck-Windows frontend through the named TypeDuck profile ABI, with native package/header smoke, a packaged host-loader lifecycle gate, and documented frontend smoke before M10 is marked complete.

**Architecture:** M10 is a platform-integration milestone, not a new default-core milestone. The default `rime_get_api()` table and upstream `RimeCandidate` layout must remain upstream `rime/librime 1.17.0` shaped, while TypeDuck-Windows integration must opt into the TypeDuck-profile ABI surface added by M19. The decisive work is proving the native Windows package, header surface, loader handshake, candidate-struct layout, deployer settings path, candidate comments, and frontend behavior against a pinned TypeDuck-Windows checkout.

**Tech Stack:** Rust `yune-core` / `yune-rime-api`, RIME C ABI, MSVC Windows dynamic library packaging, PowerShell package/smoke scripts, TypeDuck-Windows/weasel C++ frontend, TypeDuck-HK/librime `v1.1.2` oracle fixtures.

---

## Entry Criteria

Do not start implementation until all entry checks pass.

- [ ] `origin/main` contains the completed prior roadmap goals that M10 depends on: M19 TypeDuck profile ABI and any M22 browser/product work that was queued before M10. M17 is not a functional M10 prerequisite; only wait for it if the current roadmap sequence explicitly says M10 must run after M17.
- [ ] The working tree is clean, or all unrelated changes are identified and left untouched.
- [ ] The executor has read:
  - `AGENTS.md`
  - `docs/CONVENTIONS.md`
  - `docs/roadmap.md`
  - `docs/typeduck-windows-backend-requirements.md`
  - `docs/plans/m10-reference-typeduck-windows-contract.md`
  - `docs/plans/m10-reference-typeduck-windows-native-build.md`
  - `docs/plans/m19-reference-typeduck-profile-abi.md`
  - `docs/fork-parity-ledger.md`
- [ ] Work directly on `main` and push directly to `origin/main` unless a branch or PR is explicitly requested by the user.

Verification commands:

```powershell
git fetch origin
git status --short --branch
git log --oneline -n 12
```

Expected result: the current branch is `main`, it is in sync with `origin/main`, and the preceding milestones are present in the visible history or current roadmap.

## Current State

- M10 is currently parked in `docs/roadmap.md`.
- `docs/typeduck-windows-backend-requirements.md` is the engine-side graduation contract.
- M19 added `rime_get_typeduck_profile_api()` and an owning test at `crates/yune-rime-api/tests/typeduck_profile_abi_surface.rs`.
- The default `rime_get_api()` table remains upstream-sized and must stay that way.
- `scripts/package-typeduck-windows.ps1` now builds/packages the Windows DLL and runs the packaged TypeDuck-profile smoke.
- Existing TypeDuck v1.1.2 fixtures live under `crates/yune-core/tests/fixtures/typeduck-v1.1.2/`.
- `crates/yune-core/tests/cantonese_parity.rs` currently has active tests for the TypeDuck behavior surfaces that used to be parked or ignored. M10 should verify them before adding new engine work.
- The pinned TypeDuck-Windows checkout is still under `target/typeduck-windows-e2e/TypeDuck-Windows` at `f3ffcfe3b6a3018b1c3c9d256a6f0d587a2d2e27`; the local shell still lacks `msbuild.exe`, `devenv.exe`, `cmake.exe`, `nuget.exe`, and `nmake.exe`, so T1/T3 remain blocked.

## Completion Rule

M10 completion is tiered. A worker must report the highest tier reached; do not collapse these into one vague "E2E" claim.

- **T0 - ABI/header decision:** the TypeDuck-Windows checkout, TypeDuck v1.1.2 headers, and Yune headers have been compared for `RimeApi`, `RimeCandidate`, and `RimeCandidateListIterator` layout; the chosen handshake is documented.
- **T1 - package/link:** Yune produces a native TypeDuck-profile Windows package with headers, and TypeDuck-Windows builds/links against it.
- **T2 - packaged host-loader lifecycle:** a host-shaped dynamic-loader harness loads the packaged `dist/lib/rime.dll`, obtains the chosen TypeDuck profile accessor, and drives the full RIME lifecycle plus the deployer-shaped config path.
- **T3 - real frontend smoke:** the real TypeDuck-Windows frontend is exercised with the Yune package. T3 may be a documented manual interactive IME smoke on a Windows host; full automation is desirable but not required for M10 if T1 and T2 are green.

M10 is complete only at **T1 + T2 + documented T3**. If package smoke passes but TypeDuck-Windows cannot build, the result is T0/T2 progress and M10 remains blocked. If T1 and T2 pass but the only missing piece is fully automated TSF interaction, record that as an automation deferral, not a failure, after manual T3 evidence is documented.

## Files And Ownership

Expected files to modify during execution:

- `scripts/package-typeduck-windows.ps1` - re-enable package production and smoke against the TypeDuck profile surface.
- `crates/yune-rime-api/src/api_table.rs` - only if the TypeDuck profile ABI needs an additional proven slot such as `start_quick`; never widen the default table.
- `crates/yune-rime-api/src/lib.rs` - only for explicit export plumbing needed by the TypeDuck profile package.
- `crates/yune-rime-api/tests/typeduck_profile_abi_surface.rs` - profile ABI contract tests.
- `crates/yune-rime-api/tests/dynamic_loader.rs` and `crates/yune-rime-api/tests/frontend_hosts/` - reuse or extend the native host-loader lifecycle for packaged DLL smoke.
- `crates/yune-rime-api/tests/` - add a native package/profile smoke integration test if the script smoke needs Rust-side support.
- `crates/yune-core/tests/cantonese_parity.rs` - only if a real TypeDuck-Windows-required behavior gap remains after running the current suite.
- `docs/typeduck-windows-backend-requirements.md` - graduation checklist and target evidence.
- `docs/roadmap.md` - M10 status and next-step removal when complete or blocker text when still blocked.
- `docs/requirements.md` - requirement status updates for Windows/profile/package/frontend gates.
- `docs/fork-parity-ledger.md` - only if a new TypeDuck fork-only ABI slot or behavior is added.
- `docs/plans/m10-reference-typeduck-windows-native-build.md` - updated package command, layout, smoke result, and blocker details.
- `docs/plans/m10-plan-typeduck-windows-resume.md` - progress notes and final closeout evidence.

Files that must stay unchanged unless new oracle/header evidence proves otherwise:

- `crates/yune-rime-api/src/abi.rs` default `RimeApi` layout.
- Default `rime_get_api()` behavior.
- Default upstream `RimeCandidate` and `RimeCandidateListIterator` layout.

## Non-Goals

- Do not make TypeDuck fork-only behavior part of the default upstream ABI.
- Do not add `start_quick` because it exists in fork history; add it only if current TypeDuck-Windows code or headers require it for this integration.
- Do not assume the engine swap is zero-frontend-change. If TypeDuck-Windows currently calls only `rime_get_api()`, a small loader patch to call `rime_get_typeduck_profile_api()` for a Yune package is the preferred honest path.
- Do not ship a package-only fork-shaped `rime_get_api()` unless the user explicitly accepts the D-24 tension and the package is impossible to confuse with the normal upstream-shaped Yune build.
- Do not ignore a TypeDuck fork-shaped `RimeCandidate` layout. If the frontend is compiled against `text/comment/quality/reserved`, either rebuild/patch the frontend against an upstream-shaped header or design an explicitly isolated TypeDuck-profile candidate ABI; do not widen the default Yune candidate struct.
- Do not treat TypeDuck-Web tests as TypeDuck-Windows frontend E2E.
- Do not port TypeDuck-Windows UI code into Yune.
- Do not weaken existing oracle tests or replace captured oracle bytes with Yune-derived expected output.
- Do not call `scripts/package-typeduck-windows.ps1 -SkipSmoke` a valid M10 gate.

---

## Work Items

### WI-0 - Workspace And Roadmap Guard

**Purpose:** prevent M10 from starting on top of unfinished prerequisite work or mixed unrelated edits.

**Files:**

- Read: `docs/roadmap.md`
- Read: `docs/typeduck-windows-backend-requirements.md`
- Read: `docs/plans/m19-reference-typeduck-profile-abi.md`

Steps:

- [ ] Confirm current branch and sync state.

  ```powershell
  git fetch origin
  git status --short --branch
  ```

  Expected: `## main...origin/main` with no unrelated modified files.

- [ ] Confirm the profile accessor exists in code and tests.

  ```powershell
  rg -n "rime_get_typeduck_profile_api|RimeTypeDuckProfileApi|config_list_append" crates/yune-rime-api docs/plans/m19-reference-typeduck-profile-abi.md
  ```

  Expected: hits in `api_table.rs`, `lib.rs`, `typeduck_profile_abi_surface.rs`, and the M19 reference doc.

- [ ] Confirm `scripts/package-typeduck-windows.ps1` is still parked before editing it.

  ```powershell
  Select-String -Path scripts/package-typeduck-windows.ps1 -Pattern "packaging is parked|rime_get_typeduck_profile_api|config_list_append"
  ```

  Expected: the script still fails fast, or if another worker already re-enabled it, the script has profile-smoke coverage and this plan should be updated before continuing.

- [ ] If prerequisites are not complete, stop and update `docs/roadmap.md` with the exact remaining prerequisite. Do not partially implement M10.

Acceptance:

- No source changes.
- A short note is added to this plan's progress section if M10 is blocked before implementation starts.

### WI-1 - Re-Establish The Real TypeDuck-Windows Surface

**Purpose:** identify the current frontend, toolchain, and exact ABI expectations before changing Yune.

**Files:**

- Read: `target/typeduck-windows-e2e/TypeDuck-Windows/` if present.
- Optional external checkout: `target/typeduck-windows-e2e/TypeDuck-Windows`.
- Modify: `docs/plans/m10-reference-typeduck-windows-native-build.md`.
- Modify: `docs/typeduck-windows-backend-requirements.md`.

Steps:

- [ ] Locate or create the TypeDuck-Windows checkout used for E2E.

  ```powershell
  $tdw = "target\typeduck-windows-e2e\TypeDuck-Windows"
  $tdwCommit = "f3ffcfe3b6a3018b1c3c9d256a6f0d587a2d2e27"
  if (Test-Path $tdw) {
      git -C $tdw rev-parse HEAD
      git -C $tdw status --short --branch
  } else {
      git clone https://github.com/TypeDuck-HK/TypeDuck-Windows $tdw
      git -C $tdw checkout --detach $tdwCommit
      git -C $tdw rev-parse HEAD
  }
  ```

  Expected: a concrete commit hash is recorded in `docs/plans/m10-reference-typeduck-windows-native-build.md`. If the existing checkout is dirty or at a different commit, clone a second disposable checkout under `target\typeduck-windows-e2e\TypeDuck-Windows-$tdwCommit` instead of resetting user or generated changes in place.

- [ ] Check the Windows build tools from the same shell that will run E2E.

  ```powershell
  where.exe msbuild
  where.exe devenv
  where.exe cmake
  where.exe nuget
  where.exe nmake
  ```

  Expected: every available tool path is recorded. Missing tools are recorded by name.

- [ ] Inspect the current frontend/deployer ABI calls.

  ```powershell
  rg -n "rime_get_api|rime_get_typeduck_profile_api|config_list_append|start_quick|RimeApi|RimeCandidate|RimeCandidateListIterator|comment|quality|DISABLE_COMPLETION_VALUE|enable_completion|enable_word_completion" target\typeduck-windows-e2e\TypeDuck-Windows
  ```

  Expected: the plan records whether TypeDuck-Windows calls only `rime_get_api()`, whether it has a loader layer that can call a different accessor, whether it uses `start_quick`, whether it compiles against a fork-shaped `RimeCandidate` with `double quality`, where `RimeCandidate.comment` is consumed, and which completion option name the settings/deployer path patches.

- [ ] Inspect TypeDuck v1.1.2 and packaged headers for `RimeCandidate` layout.

  ```powershell
  $headerRoots = @(
      "target\typeduck-oracle\v1.1.2\extract\dist\include",
      "target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\include",
      "target\typeduck-windows-e2e\TypeDuck-Windows"
  )
  foreach ($root in $headerRoots) {
      if (Test-Path $root) {
          Get-ChildItem -Path $root -Recurse -Filter rime_api.h |
              ForEach-Object {
                  Write-Host "== $($_.FullName)"
                  Select-String -Path $_.FullName -Pattern "RimeCandidate|RimeCandidateListIterator|quality|reserved|config_list_append|start_quick" -Context 2,4
              }
      }
  }
  ```

  Expected: `docs/plans/m10-reference-typeduck-windows-native-build.md` records the exact `RimeCandidate` and `RimeCandidateListIterator` field order/size expectation for the TypeDuck fork header and for the header TypeDuck-Windows will compile against. If the TypeDuck fork header includes `double quality`, record that as an ABI mismatch with Yune's upstream three-pointer `RimeCandidate`.

- [ ] Record the build entry points.

  ```powershell
  Get-ChildItem -Path target\typeduck-windows-e2e\TypeDuck-Windows -Recurse -Include *.sln,*.vcxproj,*.bat,*.ps1,README*,INSTALL* |
      Select-Object -ExpandProperty FullName
  ```

  Expected: the exact build command candidates are listed in `docs/plans/m10-reference-typeduck-windows-native-build.md`.

Acceptance:

- `docs/plans/m10-reference-typeduck-windows-native-build.md` names the TypeDuck-Windows commit, available tools, missing tools, build entry points, ABI call sites, candidate-layout/header findings, and completion-option patch findings.
- `docs/typeduck-windows-backend-requirements.md` no longer relies on the stale 2026-06-19 checkout note without saying whether it was refreshed.
- If the checkout or toolchain is unavailable, M10 is marked blocked with exact commands and missing prerequisites.

### WI-2 - Decide And Prove The TypeDuck Profile ABI And Candidate Layout Handshake

**Purpose:** close the gap between "Yune exposes a named profile accessor" and "TypeDuck-Windows actually obtains ABI structs and function tables it can safely read."

**Files:**

- Modify: `docs/plans/m10-reference-typeduck-windows-native-build.md`.
- Modify: `docs/typeduck-windows-backend-requirements.md`.
- Modify: `crates/yune-rime-api/tests/typeduck_profile_abi_surface.rs`.
- Modify: `crates/yune-rime-api/src/api_table.rs` only if a proven profile-only slot or package-only alias is required.
- Modify: `crates/yune-rime-api/src/lib.rs` only if export plumbing is required.

Handshake choices:

- **Preferred choice:** patch or configure TypeDuck-Windows to call `rime_get_typeduck_profile_api()` when using a Yune package, while ordinary Yune consumers continue calling `rime_get_api()`. This is a small frontend loader change, but it preserves D-24 and keeps fork-only slots out of the default Yune ABI.
- **Last-resort fallback:** if current TypeDuck-Windows cannot be changed to call the named accessor, create a TypeDuck-specific package mode that exports a fork-shaped `rime_get_api()` from the packaged artifact only. This is a real D-24 footgun: it must be impossible to confuse with the normal Yune build, must require an explicit package flag/name, and must have tests proving the default repo build remains upstream-sized.
- **Candidate-layout decision:** if TypeDuck-Windows is compiled against a TypeDuck fork header where `RimeCandidate` is `text/comment/quality/reserved`, Yune's upstream three-pointer `RimeCandidate` is not layout-compatible. The preferred fix is to rebuild or patch the TypeDuck-Windows integration headers/code so it consumes the upstream-shaped candidate layout and uses Yune's internal candidate quality only through non-ABI debug surfaces. If the frontend truly requires fork-shaped candidate structs, stop and design an explicitly isolated TypeDuck-profile candidate ABI before packaging; do not change the default Yune layout.

Steps:

- [ ] Use WI-1 evidence to choose one handshake and write the choice into `docs/plans/m10-reference-typeduck-windows-native-build.md`.

- [ ] Use WI-1 header evidence to choose one candidate-layout strategy and write it into `docs/plans/m10-reference-typeduck-windows-native-build.md`.

  Required decision text:

  - "TypeDuck-Windows is built against an upstream-shaped `RimeCandidate`" and why that is true; or
  - "TypeDuck-Windows is patched/rebuilt to consume an upstream-shaped `RimeCandidate` for Yune packages" and where that patch lives; or
  - "TypeDuck-Windows requires a fork-shaped `RimeCandidate`; M10 is blocked pending a profile-only candidate ABI design."

- [ ] Strengthen `typeduck_profile_abi_surface.rs` so it proves three contracts:

  ```powershell
  cargo test -p yune-rime-api --test typeduck_profile_abi_surface
  ```

  Required assertions:

  - `rime_get_api()` returns the upstream-sized default table.
  - `rime_get_typeduck_profile_api()` returns a larger table with `config_list_append_bool`, `config_list_append_int`, `config_list_append_double`, and `config_list_append_string`.
  - Appending through the profile table creates a missing list and extends an existing list.

- [ ] Keep the default layout test green.

  ```powershell
  cargo test -p yune-rime-api rime_frontend_struct_layout_matches_librime_header
  ```

  Expected: Yune's default `RimeCandidate` remains three pointers and `RimeCandidateListIterator` embeds that upstream-shaped candidate by value.

- [ ] Audit `start_quick` from current TypeDuck-Windows source.

  ```powershell
  rg -n "start_quick|RimeStartQuick" target\typeduck-windows-e2e\TypeDuck-Windows docs crates
  ```

  If only docs/fork history mention it, record "not required by current M10 evidence" and do not implement it. If current frontend code calls it, add it only to the TypeDuck profile surface and add an owning test proving default `rime_get_api()` stays unchanged.

- [ ] If using the package-only fallback, add an explicit test name that contains `typeduck_package_default_api_alias` and run it only under the package feature/profile. The normal `cargo test -p yune-rime-api --test typeduck_profile_abi_surface` must still prove no default alias is active.

Acceptance:

- There is a documented handshake choice.
- There is a documented candidate-layout choice.
- The chosen handshake is covered by tests.
- Default upstream ABI remains unchanged.
- If a fork-shaped `RimeCandidate` is required, M10 is blocked before package production unless an isolated profile-only candidate ABI has been designed, tested, and accepted.
- Any `start_quick` decision is evidence-backed.

### WI-3 - Re-Enable Native Package Production And Profile Smoke

**Purpose:** turn `scripts/package-typeduck-windows.ps1` from a parked script into a real gated package builder for the TypeDuck profile surface.

**Files:**

- Modify: `scripts/package-typeduck-windows.ps1`.
- Modify: `docs/plans/m10-reference-typeduck-windows-native-build.md`.
- Modify: `docs/typeduck-windows-backend-requirements.md`.
- Optional create: `crates/yune-rime-api/tests/typeduck_windows_package_smoke.rs` if Rust-side dynamic loading is cleaner than PowerShell-only smoke.

Required script behavior:

- Builds `yune-rime-api` for `x86_64-pc-windows-msvc` by default.
- Copies the DLL as `dist/lib/rime.dll`.
- Copies the import library as `dist/lib/rime.lib`.
- Copies `rime.pdb` when Cargo emits it.
- Copies TypeDuck fork headers from `target/typeduck-oracle/v1.1.2/extract/dist/include` or a caller-provided `-HeaderSource`.
- Includes any Yune-specific profile accessor header needed by the chosen WI-2 handshake.
- Fails if the TypeDuck header source lacks `config_list_append_string`.
- Fails if the packaged DLL does not export the accessor required by the chosen WI-2 handshake.
- Fails if the smoke cannot prove the profile table data size and append-slot presence.
- Runs or invokes a host-shaped lifecycle smoke against the packaged DLL. Reuse the existing `crates/yune-rime-api/tests/dynamic_loader.rs` / `tests/frontend_hosts/` pattern rather than inventing a slot-only smoke.
- Allows `-SkipSmoke` only for hosts that cannot load Windows DLLs, and the script output must say that `-SkipSmoke` is not a valid M10 completion gate.

Steps:

- [ ] Remove the unconditional parked `throw` only after the script has a profile smoke path.

- [ ] Add a smoke section that resolves `rime_get_typeduck_profile_api` from `dist/lib/rime.dll` unless WI-2 chose a package-only default alias. The smoke must fail on a null pointer, non-positive `data_size`, or missing append slot evidence.

- [ ] Extend the dynamic-loader harness or add a sibling test so the packaged DLL path can be supplied explicitly.

  Required behavior:

  - load `target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\lib\rime.dll`;
  - resolve the chosen TypeDuck profile accessor;
  - run the same native host lifecycle shape as `frontend_hosts::native::run_native_host_lifecycle`;
  - select `jyut6ping3_mobile` if the package includes the TypeDuck assets, or record the exact asset blocker;
  - exercise `config_list_append_string` through the profile table with the deployer-shaped completion/display-language patch.

- [ ] Keep the existing build/copy layout unless WI-1 proves TypeDuck-Windows expects a different path.

- [ ] Run package smoke without bypass switches.

  ```powershell
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-typeduck-windows.ps1
  ```

  Expected: package artifacts are created under `target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\`.

- [ ] Record artifact sizes and timestamps.

  ```powershell
  Get-Item target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\lib\rime.dll,
           target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\lib\rime.lib |
      Select-Object FullName,Length,LastWriteTimeUtc
  ```

Acceptance:

- `scripts/package-typeduck-windows.ps1` succeeds without `-NoBuild` and without `-SkipSmoke` on a Windows host with MSVC.
- The smoke proves the chosen TypeDuck profile accessor/alias.
- The packaged DLL passes a host-shaped lifecycle gate, not only a symbol/slot gate.
- The documented package layout matches the files TypeDuck-Windows consumes.

### WI-4 - Verify TypeDuck Profile Behavior Through Yune Tests

**Purpose:** make sure the engine and ABI behavior needed by TypeDuck-Windows is already green before the frontend is swapped.

**Files:**

- Read/modify only if failing: `crates/yune-core/tests/cantonese_parity.rs`.
- Read/modify only if failing: `crates/yune-rime-api/tests/typeduck_profile_abi_surface.rs`.
- Read/modify only if failing: `crates/yune-rime-api/src/tests/config_api.rs`.
- Modify docs only for status updates if tests already pass.

Steps:

- [ ] Run the profile ABI tests.

  ```powershell
  cargo test -p yune-rime-api --test typeduck_profile_abi_surface
  cargo test -p yune-rime-api config_list_append
  ```

  Expected: profile table and helper behavior pass.

- [ ] Run the TypeDuck Cantonese parity suite.

  ```powershell
  cargo test -p yune-core --test cantonese_parity
  ```

  Expected: all active TypeDuck v1.1.2 parity tests pass.

- [ ] Check for ignored M10-relevant Cantonese tests.

  ```powershell
  rg -n "#\[ignore|ignore =" crates\yune-core\tests\cantonese_parity.rs
  ```

  Expected: no ignored tests. If ignored tests exist, each one must be classified as required for TypeDuck-Windows, not required for TypeDuck-Windows, or blocked by missing oracle evidence.

- [ ] Reconcile the completion option name across the whole TypeDuck-Windows path.

  ```powershell
  rg -n "enable_completion|enable_word_completion|DISABLE_COMPLETION_VALUE|disable_completion" crates docs scripts target\typeduck-windows-e2e\TypeDuck-Windows
  ```

  Expected: the deployer's `DISABLE_COMPLETION_VALUE` patch, TypeDuck schema YAML, Yune schema reader, and Cantonese parity fixture all agree on the option that actually disables completion. If TypeDuck-Windows patches `common:/disable_completion`, prove that the resulting deployed schema changes `translator/enable_completion` in the Yune path; do not rely on a setting that silently no-ops.

- [ ] Use the native Windows surface as an opportunity to inspect behavior the browser could not. If TypeDuck-Windows exposes schema-menu or userdb-pronunciation surfaces that M14/M16 could only cover through fixtures or browser-limited evidence, add focused evidence here. Do not convert that opportunity into a completion blocker unless the TypeDuck-Windows graduation contract requires it.

- [ ] If a required behavior fails or is still ignored, capture or reuse TypeDuck v1.1.2 goldens and fix the owning module. Do not derive expected bytes from Yune.

Acceptance:

- M10-relevant profile ABI, config append, dictionary-comment, settings-option, completion, correction, prediction, schema-menu, and userdb-pronunciation tests pass or have a documented external blocker.
- No code changes are made here unless a focused failing test proves a gap.

### WI-5 - Build TypeDuck-Windows Against The Yune Package

**Purpose:** prove T1: the package can replace the TypeDuck-HK/librime artifact in the real Windows frontend build.

**Files:**

- External checkout: `target/typeduck-windows-e2e/TypeDuck-Windows`.
- Modify: `docs/plans/m10-reference-typeduck-windows-native-build.md`.
- Modify: `docs/typeduck-windows-backend-requirements.md`.

Steps:

- [ ] Copy or point TypeDuck-Windows to the package from WI-3. Use the exact destination paths discovered in WI-1. If the checkout expects `dist/include` and `dist/lib`, preserve that layout.

- [ ] Make the TypeDuck-Windows build use the header set chosen in WI-2. If WI-2 chose an upstream-shaped `RimeCandidate`, verify the frontend is not compiling against a stale fork header with `double quality`.

- [ ] Build TypeDuck-Windows from the Visual Studio developer shell or documented build shell, using the build entry point recorded in WI-1.

  If WI-1 identified an MSBuild solution, use this command shape with the recorded solution path:

  ```powershell
  msbuild path\to\TypeDuck-Windows.sln /p:Configuration=Release /p:Platform=x64
  ```

  Expected: the frontend/deployer builds and links against the Yune package.

- [ ] If the build fails because the frontend still calls `rime_get_api()` but the chosen WI-2 handshake requires `rime_get_typeduck_profile_api()`, stop and fix the handshake rather than adding fork slots to the default Yune table.

- [ ] If the build fails because the frontend expects fork-shaped `RimeCandidate`/`RimeCandidateListIterator`, stop and fix the WI-2 candidate-layout decision. Do not paper over this by changing Yune's default `RimeCandidate`.

- [ ] Capture the exact build command, exit code, and output artifact paths in `docs/plans/m10-reference-typeduck-windows-native-build.md`.

Acceptance:

- TypeDuck-Windows builds with the Yune package, or the plan records the exact toolchain/source error blocking the build.
- A successful TypeDuck-Windows build is T1 only; continue to WI-6 for T2/T3 runtime behavior.

### WI-6 - Run Packaged Host Lifecycle And Real TypeDuck-Windows Frontend Smoke

**Purpose:** prove T2 and T3 runtime behavior, not just package production.

**Files:**

- External checkout: `target/typeduck-windows-e2e/TypeDuck-Windows`.
- Modify: `docs/plans/m10-reference-typeduck-windows-native-build.md`.
- Modify: `docs/typeduck-windows-backend-requirements.md`.
- Modify: `docs/roadmap.md`.
- Modify: `docs/requirements.md`.

Required T2 packaged host-loader coverage:

- Engine lifecycle: setup, deploy, create session, select `jyut6ping3_mobile`, process keys, read context/status, destroy session, finalize.
- ABI layout: candidate list iteration reads text/comment safely under the WI-2 candidate-layout choice; there is no stride mismatch between caller and DLL.
- Dictionary panel data: candidate comments from `RimeCandidate.comment` expose the TypeDuck dictionary-panel payload.
- Settings/deployer path: display-language list and completion/correction/sentence/learning/Cangjie patch toggles exercise `config_list_append_*` through the chosen profile handshake.
- Input flows: representative Jyutping input, completion/prediction input, correction input, sentence input, reverse lookup if the frontend exposes it, and schema menu behavior.
- Regression guard: default upstream Yune behavior remains isolated from TypeDuck profile behavior.

Required T3 frontend smoke coverage:

- TypeDuck-Windows starts with the Yune package.
- The settings/deployer UI or command path persists the same patches covered by T2.
- Candidate text and comments render in the real candidate panel, including at least one dictionary-panel payload.
- Representative Jyutping input, completion, correction, and sentence cases behave as documented.
- The smoke can be manual/interactive if no automated TSF harness exists, but it must record exact user steps, input text, observed output, package path, TypeDuck-Windows commit, and Yune commit.

Steps:

- [ ] Run the packaged host-loader lifecycle harness from WI-3 against the packaged `rime.dll`.

  Expected: T2 passes with the selected TypeDuck profile accessor, candidate-layout decision, config append path, and representative engine lifecycle.

- [ ] Run the current TypeDuck-Windows automated E2E or smoke harness discovered in WI-1, if one exists.

  If no automated harness exists, run the smallest reproducible lifecycle harness available in the checkout and record the command. Do not invent a Yune-only substitute and call it frontend E2E.

- [ ] If no automated frontend harness exists, run a documented manual interactive TypeDuck-Windows smoke on a Windows host. Full TSF automation is environment-deferred, but the manual smoke must still prove the real frontend uses the Yune package.

- [ ] Capture evidence as text logs and, where the frontend is visible, screenshots or recorded output under a path named in the TypeDuck-Windows checkout or in `target/typeduck-windows-e2e/`.

- [ ] Record the exact TypeDuck-Windows commit, Yune commit, package artifact paths, and commands in `docs/plans/m10-reference-typeduck-windows-native-build.md`.

- [ ] If manual frontend steps are required, record each input and observed output in the doc. The evidence must be precise enough that another worker can reproduce it.

Acceptance:

- T2 passes through a packaged DLL, not the Cargo dev artifact.
- T3 has documented real TypeDuck-Windows smoke evidence. Automated TSF coverage is optional if manual T3 is precise and reproducible.
- If T2 or T3 is unavailable, M10 remains blocked with exact missing tool, harness, or runtime failure.

### WI-7 - Documentation Closeout

**Purpose:** make the repo state honest after implementation or after a blocker.

**Files:**

- Modify: `docs/typeduck-windows-backend-requirements.md`.
- Modify: `docs/roadmap.md`.
- Modify: `docs/requirements.md`.
- Modify: `docs/fork-parity-ledger.md` if new TypeDuck fork-only ABI or behavior was added.
- Modify: `docs/plans/m10-reference-typeduck-windows-native-build.md`.
- Modify: `docs/plans/m10-plan-typeduck-windows-resume.md`.

Steps:

- [ ] If all gates pass, mark M10 complete in `docs/roadmap.md` and move the plan under `docs/plans/archive/`.

- [ ] If only T0/T1/T2 pass, keep M10 parked/blocked in `docs/roadmap.md`, name the exact missing tier, and do not archive the plan as complete.

- [ ] If T1 and T2 pass and T3 manual smoke passes, mark M10 complete and explicitly record that full automated TSF E2E remains environment-deferred if no automated harness exists.

- [ ] Update `docs/typeduck-windows-backend-requirements.md` checklist:

  - ABI profile accessor/handshake.
  - `RimeCandidate` / `RimeCandidateListIterator` layout decision.
  - Candidate comment behavior.
  - Cantonese behavior parity.
  - Native package smoke.
  - T1 TypeDuck-Windows build/link.
  - T2 packaged host-loader lifecycle.
  - T3 real TypeDuck-Windows frontend smoke.

- [ ] Update `docs/requirements.md` with complete or blocked status for Windows native package and frontend validation.

- [ ] If a new profile-only slot such as `start_quick` was added, update `docs/fork-parity-ledger.md` with provenance, scope, tests, and default-ABI isolation.

Acceptance:

- Docs agree on whether M10 is complete or blocked, and name the highest tier reached.
- No doc claims T3 real frontend smoke unless WI-6 T3 passed.

---

## Final Verification Gates

Run these before committing a completed or blocked M10 update:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-rime-api rime_frontend_struct_layout_matches_librime_header
cargo test -p yune-rime-api --test typeduck_profile_abi_surface
cargo test -p yune-rime-api --test dynamic_loader
cargo test -p yune-rime-api config_list_append
cargo test -p yune-core --test cantonese_parity
cargo test --workspace
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-typeduck-windows.ps1
git diff --check
```

Run the real TypeDuck-Windows T1 build command from WI-5 and the T2/T3 commands from WI-6 if the toolchain/frontend host is available. If unavailable, record the exact missing command output and keep M10 blocked at the highest verified tier.

## Commit Policy

Use scoped commits:

1. Package/ABI handshake tests and implementation.
2. Package script and native smoke.
3. TypeDuck-Windows E2E docs/evidence.
4. Roadmap/requirements closeout.

Push each completed scoped commit directly to `origin/main`, unless the user requested a branch.

## Progress Notes

- 2026-06-21: Draft plan created for review. No implementation started.
- 2026-06-21: Revised after external review. Added the TypeDuck fork `RimeCandidate`/iterator layout audit, tiered T0/T1/T2/T3 completion model, stronger preference for the named profile accessor over a package-only `rime_get_api()` alias, pinned TypeDuck-Windows checkout handling, completion-option reconciliation, and packaged dynamic-loader lifecycle smoke.
- 2026-06-21: M10 resume implementation reached T2 and remains blocked short of completion. T0 chose the upstream-shaped default ABI plus `rime_typeduck_profile_api.h`: TypeDuck v1.1.2 fork headers widen `RimeCandidate` with `double quality` and add `start_quick` / `config_list_append_*` to the default table, so they are audit evidence, not Yune package default headers. `scripts/package-typeduck-windows.ps1` now packages `target\typeduck-windows-native\x86_64-pc-windows-msvc\dist\lib\rime.dll`, `rime.lib`, upstream-shaped `rime_api.h` / `rime_levers_api.h`, and `rime_typeduck_profile_api.h`; it rejects `-SkipSmoke`, rejects fork-shaped default headers, loads the packaged DLL, resolves `rime_get_typeduck_profile_api()`, verifies `config_list_append_{bool,int,double,string}`, and runs `dynamic_loader_harness_loads_packaged_typeduck_profile_dll`. Yune deploys explicit `common:/disable_completion` to `translator/enable_completion: false` for the TypeDuck-Windows `DISABLE_COMPLETION_VALUE` path while leaving schema-default `common:/disable_completion?` inactive unless selected. The TypeDuck-Windows checkout is pinned at `f3ffcfe3b6a3018b1c3c9d256a6f0d587a2d2e27` and does not directly read `RimeCandidate.quality`, but `WeaselDeployer/TypeDuckSettings.cpp` still calls `rime_get_api()->config_list_append_string(...)`, so a frontend/profile-accessor handshake patch is still required for a Yune package. T1 attempted `msbuild target\typeduck-windows-e2e\TypeDuck-Windows\weasel.sln /p:Configuration=Release /p:Platform=x64` and stopped at `CommandNotFoundException`; `msbuild`, `devenv`, `cmake`, `nuget`, and `nmake` are missing from PATH. T3 did not run because no TypeDuck-Windows binary was built against the Yune package.
