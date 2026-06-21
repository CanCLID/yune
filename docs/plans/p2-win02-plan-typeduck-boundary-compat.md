# P2-WIN-02 TypeDuck Boundary Compatibility Implementation Plan

> **Status:** Active - **Milestone:** P2-WIN-02 (TypeDuck Windows boundary compatibility) - **Updated:** 2026-06-21 - **Type:** execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the Yune TypeDuck-profile boundary incompatibilities found by TypeDuck-Windows Phase 0C, then rerun the Windows Notepad TSF smoke with a rebuilt Yune package.

**Architecture:** Keep the default upstream `RimeApi` and upstream-shaped 3-pointer `RimeCandidate` ABI unchanged. Fix TypeDuck-specific behavior behind the existing TypeDuck profile/schema path: rich dictionary-panel comment bytes, session/schema lifecycle behavior exposed by the Windows package, and any packaged-DLL boundary bug proven by tests. Treat TypeDuck-HK/librime `v1.1.2` as the oracle for the `jyut6ping3` profile only.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), TypeDuck `v1.1.2` JSON oracle fixtures, `scripts/package-typeduck-windows.ps1`, the local `TypeDuck-Windows` `dev` smoke harness, Visual Studio/MSBuild for final Windows validation.

---

## Why This Milestone Exists

TypeDuck-Windows Phase 0C completed in the separate `TypeDuck-Windows` repo on `dev`:

- `4ac2510` - `Document Phase 0C candidate boundary evidence`
- `c7ddedb` - `Document Phase 0C verifier evidence`

Phase 0C classified the blocker as **1: Yune TypeDuck-profile compatibility / boundary bug**.

Evidence lives in:

- `C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows\docs\evidence\p2-win01-phase0c-2026-06-21\README.md`
- `C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows\docs\evidence\p2-win01-phase0c-2026-06-21\ngohaig-candidate-boundary-diff-summary.json`
- `C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows\docs\evidence\p2-win01-phase0c-2026-06-21\rime-candidate-layout-probe.txt`
- `C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows\docs\evidence\p2-win01-phase0c-2026-06-21\appverifier-server-rime-create-session-stack-excerpt.txt`

Important Phase 0C findings:

- Active TypeDuck-Windows and packaged Yune headers agree on x64 `RimeCandidate` layout: size 24, `text=0`, `comment=8`, `reserved=16`.
- The embedded TypeDuck fork header is wider (`quality` at offset 16), but that wider layout is not used at the active Yune-backed adapter boundary.
- Yune and TypeDuck `v1.1.2` agree on the first four `jyut6ping3` `ngohaig` candidate texts.
- Yune does **not** emit TypeDuck `v1.1.2` rich dictionary-panel comments for those rows:
  - TypeDuck oracle comments begin with actual bytes `0c 0d 31 2c` (`\f\r1,`).
  - Yune currently emits a display marker for the sentence candidate and literal text like `\fngo5hai6` for later rows.
- AppVerifier changes the original Phase 0A crash into a pre-candidate hang in Yune-backed `RimeCreateSession` / `RimeSelectSchema`. That blocker reproduces through console IPC, so it is not first explained by Notepad, DirectWrite, or WeaselUI rendering.

## Scope

In scope:

- Promote the Phase 0C `ngohaig` boundary evidence into Yune-owned fixtures/tests.
- Make the TypeDuck `jyut6ping3` profile emit TypeDuck `v1.1.2` rich comment bytes for the Windows-facing `ngohaig` path.
- Investigate and fix or explicitly separate the AppVerifier `RimeCreateSession` / `RimeSelectSchema` hang.
- Rebuild the TypeDuck Windows package and verify the local TypeDuck-Windows IPC and Notepad smoke after the Yune fix.
- Update roadmap/plans/evidence with the final classification.

Out of scope:

- Widening `RimeCandidate` or default `RimeApi`.
- Adding the TypeDuck fork `quality` field to the default ABI.
- Starting YuneHost, WebView2, candidate-window rewrite, repo extraction, or other Windows product work.
- Changing the separate M25 TypeDuck-Web dogfooding implementation unless a shared regression is proven.
- Treating `typeduck.hk/web` as the hard oracle; the oracle for this milestone is TypeDuck-HK/librime `v1.1.2`.

## Execution Notes

- This milestone should run in a clean Yune worktree after the active M25 edits are committed, or in a separate clean worktree created from current `origin/main`.
- If the current Yune worktree contains M25 changes, stage only P2-WIN-02 files and do not sweep web dogfooding artifacts into this milestone.
- Before any elevated Windows command, IME install/register step, AppVerifier enablement, or machine registry cleanup, pause for explicit user approval.
- Disable AppVerifier and clean test IME registration before finishing any Windows smoke attempt.

## Task 0 - Prepare A Clean Yune Slice

**Files:**
- Read: `docs/roadmap.md`
- Read: `docs/plans/p2-win01-plan-typeduck-windows-next.md`
- Read: `C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows\docs\evidence\p2-win01-phase0c-2026-06-21\README.md`

- [ ] **Step 0.1: Confirm repository state**

Run:

```powershell
git fetch origin --prune
git status --short --branch
git log --oneline -5 --decorate
```

Expected:

- If the worktree is clean, continue on `main`.
- If M25 files are dirty, either wait for the M25 session to finish or create a separate worktree for P2-WIN-02. Do not edit M25 files.

- [ ] **Step 0.2: Confirm TypeDuck-Windows evidence state**

Run:

```powershell
git -C C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows fetch origin --prune
git -C C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows log --oneline -5 --decorate
Get-Content C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows\docs\evidence\p2-win01-phase0c-2026-06-21\README.md
```

Expected:

- `TypeDuck-Windows` `dev` includes `4ac2510` and `c7ddedb`.
- Phase 0C classification is boundary bug.

## Task 1 - Promote Phase 0C Evidence Into Yune Fixtures

**Files:**
- Create: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-windows-boundary-ngohaig.json`
- Modify: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/oracle-manifest.json`
- Modify: `crates/yune-core/tests/fixtures/typeduck-v1.1.2/README.md`
- Modify: `crates/yune-core/tests/cantonese_parity.rs`

- [ ] **Step 1.1: Add a focused oracle fixture**

Create `jyut6ping3-windows-boundary-ngohaig.json` from the TypeDuck-Windows Phase 0C evidence. Keep the oracle rows, the Yune-current rows, and the diff summary so future readers can see both the desired bytes and the failing baseline.

Minimum fixture shape:

```json
{
  "oracle": {
    "engine": "TypeDuck-HK/librime",
    "engine_tag": "v1.1.2",
    "engine_commit": "74cb52b78fb2411137a7643f6c8bc6517acfde69",
    "schema": "jyut6ping3",
    "captured_from": "TypeDuck-Windows Phase 0C",
    "evidence_commit": "4ac2510"
  },
  "schema": "jyut6ping3",
  "input": "ngohaig",
  "expected_first_rows": [
    {
      "index": 0,
      "text": "\u6211\u4fc2\u500b",
      "comment_prefix_hex": "0c0d312c",
      "comment_starts_with": "\f\r1,"
    },
    {
      "index": 1,
      "text": "\u6211\u4fc2",
      "comment_prefix_hex": "0c0d312c",
      "comment_starts_with": "\f\r1,"
    },
    {
      "index": 2,
      "text": "\u6211\u55ba",
      "comment_prefix_hex": "0c0d312c",
      "comment_starts_with": "\f\r1,"
    },
    {
      "index": 3,
      "text": "\u6211",
      "comment_prefix_hex": "0c0d312c",
      "comment_starts_with": "\f\r1,"
    }
  ]
}
```

The actual fixture may store the complete TypeDuck comments copied from `typeduck-v112-direct-jyut6ping3-ngohaig.json` instead of only prefixes. Prefer complete comments when the byte capture is available.

- [ ] **Step 1.2: Register fixture provenance**

Update `oracle-manifest.json` and `README.md` to name:

- TypeDuck-HK/librime `v1.1.2`
- TypeDuck schema commit `1bed1ae6a0ab48055f073774d7dfd152a171c548`
- TypeDuck-Windows evidence commits `4ac2510` and `c7ddedb`
- Evidence path `docs/evidence/p2-win01-phase0c-2026-06-21/`

- [ ] **Step 1.3: Add a locked-fixture test**

In `cantonese_parity.rs`, add a locked fixture test that asserts:

- input is `ngohaig`
- first four expected texts are stable
- every expected comment starts with actual `\u{000c}\r1,`
- the first comment includes `composition`

Run:

```powershell
cargo test -p yune-core --test cantonese_parity typeduck_v112_windows_boundary_ngohaig_fixture_is_locked
```

Expected:

- Pass. This step only locks the oracle fixture and does not test Yune behavior yet.

## Task 2 - Add Failing Yune Boundary Tests

**Files:**
- Modify: `crates/yune-core/tests/cantonese_parity.rs`
- Modify or create: `crates/yune-rime-api/tests/typeduck_windows_boundary.rs`
- Inspect: `crates/yune-core/src/filter/mod.rs`
- Inspect: `crates/yune-core/src/translator/mod.rs`
- Inspect: `crates/yune-rime-api/src/schema_install.rs`
- Inspect: `crates/yune-rime-api/src/session.rs`

- [ ] **Step 2.1: Add a core rich-comment regression**

Add a `cantonese_parity` test that drives the same core path currently used for TypeDuck `jyut6ping3` sentence/composition candidates and checks `ngohaig` comments against the new fixture.

The assertion must fail on current Yune by showing one of these bad shapes:

- `" \u{262f} "` for the sentence candidate
- literal `"\\fngo5hai6"` instead of actual `"\u{000c}\r1,..."`

Run:

```powershell
cargo test -p yune-core --test cantonese_parity yune_jyut6ping3_ngohaig_comments_match_windows_boundary_oracle
```

Expected before implementation:

- Fail with comment mismatch.

- [ ] **Step 2.2: Add an ABI/runtime boundary regression**

Add an integration test that uses the Rime ABI path, not only pure core objects:

- initialize with a temp shared/user data layout containing `jyut6ping3` schema and dictionary assets
- create a session
- select `jyut6ping3`
- process `ngohaig`
- call `RimeGetContext`
- read candidate text/comment through `RimeCandidate`
- assert the first four comments start with actual `\u{000c}\r1,`

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_windows_boundary yune_abi_jyut6ping3_ngohaig_comments_match_v112
```

Expected before implementation:

- Fail with the same comment mismatch.

- [ ] **Step 2.3: Add a session lifecycle regression for the verifier blocker**

Add a focused test that repeats the packaged Windows lifecycle without AppVerifier:

- `RimeInitialize`
- `RimeStartMaintenance(FALSE)`
- `RimeCreateSession`
- `RimeGetStatus`
- `RimeSelectSchema("jyut6ping3")`
- `RimeProcessKey` for `ngohaig`
- `RimeGetContext`
- `RimeDestroySession`
- repeat the session once

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_windows_boundary yune_abi_repeated_jyut6ping3_session_lifecycle_stays_responsive
```

Expected:

- Pass without AppVerifier on current Yune, or fail with a deterministic lifecycle error that must be fixed in Task 4.

## Task 3 - Fix TypeDuck Rich Comment Byte Compatibility

**Files:**
- Primary modify: `crates/yune-core/src/filter/mod.rs`
- Inspect and modify only if the failing test shows the escape is introduced before dictionary lookup filtering: `crates/yune-core/src/translator/mod.rs`
- Inspect and modify only if the failing test shows the TypeDuck profile is not selecting the right lookup/comment formatter: `crates/yune-rime-api/src/schema_install.rs`
- Test: `crates/yune-core/tests/cantonese_parity.rs`
- Test: `crates/yune-rime-api/tests/typeduck_windows_boundary.rs`

- [ ] **Step 3.1: Trace where literal `\\f` enters comments**

Inspect:

- `CommentFormat::parse` and `CommentFormatFormula::parse_xform`
- `StaticTableTranslator::with_comment_format`
- `combine_lookup_comments`
- `DictionaryLookupFilter::comment_for_candidate`
- `install_schema_dictionary_lookup_filter_from_config`

The bug to locate is the conversion from schema/comment-format data to candidate comments. TypeDuck rich comments need an actual form-feed byte `\u{000c}` followed by carriage-return rows, not the two-character text sequence backslash + `f`.

- [ ] **Step 3.2: Preserve profile isolation**

Before changing code, decide whether the fix is:

- a general RIME comment-format escape handling fix, with upstream/schema tests proving it is not TypeDuck-only, or
- a TypeDuck `jyut6ping3` profile fix behind `is_typeduck_jyut6ping3_profile()`.

Do not change default upstream `luna_pinyin` comment behavior without an upstream `1.17.0` fixture.

- [ ] **Step 3.3: Implement the smallest byte-compatible fix**

The fixed Yune output for Windows-facing `jyut6ping3` `ngohaig` must satisfy:

- candidate 0 comment starts with actual `\u{000c}\r1,`
- candidate 0 comment includes a composition row and component rows
- candidates 1-3 comments start with actual `\u{000c}\r1,`
- comments contain carriage-return row separators where the TypeDuck oracle does
- `RimeCandidate.comment` remains a NUL-terminated UTF-8 C string with no interior NUL

- [ ] **Step 3.4: Run the focused tests**

Run:

```powershell
cargo test -p yune-core --test cantonese_parity yune_jyut6ping3_ngohaig_comments_match_windows_boundary_oracle
cargo test -p yune-rime-api --test typeduck_windows_boundary yune_abi_jyut6ping3_ngohaig_comments_match_v112
```

Expected:

- Both pass.

## Task 4 - Investigate And Fix The Session-Creation Boundary Blocker

**Files:**
- Modify only if the failing stack identifies a Yune lifecycle issue: `crates/yune-rime-api/src/session.rs`
- Modify only if schema selection is implicated: `crates/yune-rime-api/src/schema_selection.rs`
- Modify only if schema install is implicated: `crates/yune-rime-api/src/schema_install.rs`
- Test: `crates/yune-rime-api/tests/typeduck_windows_boundary.rs`
- Test: `scripts/package-typeduck-windows.ps1`

- [ ] **Step 4.1: Reproduce without AppVerifier first**

Run the lifecycle regression from Task 2:

```powershell
cargo test -p yune-rime-api --test typeduck_windows_boundary yune_abi_repeated_jyut6ping3_session_lifecycle_stays_responsive -- --nocapture
```

Expected:

- Pass. If it fails, fix the deterministic failure before any Windows smoke.

- [ ] **Step 4.2: Build the packaged DLL**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-typeduck-windows.ps1
```

Expected:

- Package succeeds.
- Dynamic loader smoke passes.
- Packaged headers still reject fork-shaped default `RimeCandidate`.

- [ ] **Step 4.3: Re-run a non-invasive TypeDuck-Windows IPC smoke**

Use the local `TypeDuck-Windows` repo and the rebuilt Yune package. Do not register the IME for this step.

Expected:

- `TestTypeDuckIPC.exe /console` returns `status.schema_id=jyut6ping3`.
- `ngohaig` candidates return.
- Candidate comments now carry rich `\f\r1,` payloads.

- [ ] **Step 4.4: If AppVerifier is needed again, pause for approval**

If the package and IPC smoke pass but the earlier verifier hang still needs confirmation, pause and ask the user before enabling Application Verifier or running any elevated command.

When approved, enable verifier only for `TypeDuckServer.exe`, reproduce, collect evidence, disable verifier, and verify cleanup in the same turn.

## Task 5 - Rerun The Interactive Windows Smoke

**Files:**
- Update in TypeDuck-Windows if smoke evidence is captured there:
  - `docs/evidence/p2-win02-boundary-fix-YYYY-MM-DD/README.md`
  - `docs/windows-next-audit.md`
  - `docs/windows-next-process-model.md`
  - `docs/windows-next-repo-decision.md`
- Update in Yune:
  - `docs/plans/p2-win01-plan-typeduck-windows-next.md`
  - this plan, if acceptance changes

- [ ] **Step 5.1: Pause for approval before IME registration**

The Notepad smoke mutates Windows input-method state. Ask for explicit approval before setup/deployer registration.

- [ ] **Step 5.2: Register the Yune-backed TypeDuck-Windows build**

Use the normal TypeDuck-Windows setup/deployer path from the current local `dev` checkout and the rebuilt Yune package.

Expected:

- Setup/deployer exit successfully.
- TypeDuck TIP/layout keys are present only during the smoke.

- [ ] **Step 5.3: Run Notepad smoke**

Type `ngohaig`, confirm:

- Notepad does not hang.
- `TypeDuckServer.exe` does not crash.
- live composition reaches Yune.
- visible candidates are shown.
- committing the top candidate produces the expected text.

- [ ] **Step 5.4: Run one Chromium/WebView-like field smoke if Notepad passes**

Expected:

- A Chromium/WebView-like text field accepts the same composition and commit path.

- [ ] **Step 5.5: Clean up Windows state**

Before finishing:

- unregister/uninstall the test IME registration
- disable AppVerifier if it was enabled
- verify no active TypeDuck layout remains
- verify no `TypeDuckServer.exe`, `TestTypeDuckIPC.exe`, or Notepad smoke process remains
- document any reboot-pending leftover file

## Task 6 - Final Gates And Docs

**Files:**
- Modify: `docs/roadmap.md`
- Modify: `docs/plans/p2-win01-plan-typeduck-windows-next.md`
- Modify: `docs/plans/p2-win02-plan-typeduck-boundary-compat.md`
- Modify only if durable requirement wording changes: `docs/requirements.md`
- Modify only if a decision changes: `docs/decisions.md`

- [ ] **Step 6.1: Run Rust gates**

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_windows_boundary
cargo test -p yune-rime-api --test typeduck_profile_abi_surface
cargo test -p yune-rime-api --test dynamic_loader
```

Run `cargo test --workspace` if implementation touched shared core/runtime behavior beyond TypeDuck-profile comment shaping or session lifecycle.

- [ ] **Step 6.2: Run package and frontend gates**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-typeduck-windows.ps1
cargo test -p yune-rime-api --test typeduck_web
```

If M25 touched web code in the same final merge window, coordinate with the M25 owner before running or interpreting browser gates.

- [ ] **Step 6.3: Update docs with final classification**

If Notepad passes:

- mark P2-WIN-02 complete in `docs/roadmap.md`
- update `p2-win01-plan-typeduck-windows-next.md` so Windows product work can resume at the repo-decision checkpoint
- record the TypeDuck-Windows smoke evidence path

If Notepad still fails:

- keep P2-WIN-02 active
- classify the remaining failure as Yune compatibility, adapter contract, or WeaselUI debt using evidence
- do not start YuneHost/WebView2 product work

- [ ] **Step 6.4: Commit scoped changes**

Stage only the files for this milestone. Do not stage active M25 web dogfooding files unless this milestone directly changed them.

Expected commit shape:

```powershell
git add -- crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-windows-boundary-ngohaig.json `
  crates/yune-core/tests/fixtures/typeduck-v1.1.2/oracle-manifest.json `
  crates/yune-core/tests/fixtures/typeduck-v1.1.2/README.md `
  crates/yune-core/tests/cantonese_parity.rs `
  crates/yune-rime-api/tests/typeduck_windows_boundary.rs `
  crates/yune-core/src/filter/mod.rs `
  crates/yune-core/src/translator/mod.rs `
  crates/yune-rime-api/src/schema_install.rs `
  crates/yune-rime-api/src/session.rs `
  docs/roadmap.md `
  docs/plans/p2-win01-plan-typeduck-windows-next.md `
  docs/plans/p2-win02-plan-typeduck-boundary-compat.md
git commit -m "Fix TypeDuck Windows boundary comments"
git push origin main
```

Only include source paths that were actually modified.

## Acceptance

P2-WIN-02 is complete only when:

- Yune has a locked TypeDuck `v1.1.2` `ngohaig` Windows-boundary fixture.
- Yune tests prove `jyut6ping3` `ngohaig` comments are byte-compatible with the TypeDuck oracle for the relevant top rows.
- The default `RimeApi` and 3-pointer `RimeCandidate` layout remain unchanged.
- `scripts/package-typeduck-windows.ps1` passes.
- TypeDuck-Windows non-invasive IPC smoke passes with rich comments.
- Interactive Notepad TSF smoke is rerun with the fixed package and either passes, or produces a newly classified non-Yune blocker with committed evidence.
- Windows test registration/AppVerifier state is cleaned up after smoke attempts.

## Review Questions

- Is `ngohaig` sufficient as the first Windows-boundary fixture, or should this milestone also capture `nei`/`hou` through the exact Windows `jyut6ping3` path?
- Should the comment escape fix be general RIME behavior or gated to the TypeDuck `jyut6ping3` profile?
- Is the AppVerifier `RimeCreateSession` hang actionable inside Yune, or should it be recorded as verifier-specific until a non-verifier lifecycle test fails?
- After Notepad passes, should Phase 2 Windows resume in the existing `TypeDuck-Windows` repo or pause for the planned repo-decision checkpoint first?
