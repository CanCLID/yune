# P2-WIN-01 TypeDuck-Windows Next Product Plan

> **Status:** Draft / external review incorporated - **Track:** Phase 2 Windows frontend product - **Created:** 2026-06-21 - **Updated:** 2026-06-21 - **Type:** strategy and execution-gate plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a modern Windows IME product for TypeDuck that uses Yune as the only runtime engine, while making an explicit repo/architecture decision before large implementation work starts.

**Architecture:** Treat Yune Phase 1 as the completed engine base. The Windows project is a Phase 2 product/frontend track: TSF integration, process/lifecycle, candidate UI, settings, installer, diagnostics, and product polish live in the Windows frontend repo, while Yune exposes the engine through the upstream-shaped default ABI plus the named TypeDuck profile ABI. The first architectural decision is the process model: a shared server/IPC spine is the default because it matches Yune's process-global runtime/session/userdb model and is the path M10 already proved; in-process TSF-per-app loading must win an explicit spike before it displaces that default. The existing `TypeDuck-HK/TypeDuck-Windows` checkout is reference material, build evidence, and a likely source for TSF/IPC/server/caret-positioning shell extraction, not a constraint on the new product design.

**Tech Stack:** Yune Windows package (`rime.dll` plus `rime_typeduck_profile_api.h`), Windows Text Services Framework (TSF), C++20 / C++/WinRT for low-level IME shell work unless a spike proves another language safer, the proven Weasel/TypeDuck server+IPC model unless Phase 0 rejects it, native HWND/DirectComposition-style rendering for the first low-latency candidate window, WebView2 hosting the M24 React/Tailwind settings and dictionary-panel UI unless a spike rejects it, MSBuild or a consciously chosen replacement build system, installer/update tooling selected after audit.

---

## Boundary

This plan is deliberately **not** a Yune core-engine milestone.

- **In scope:** Windows IME frontend, TSF registration, session lifecycle, Yune package loading, candidate window, dictionary-panel display, settings UI, schema/profile controls, installer/update path, diagnostics, crash reporting, accessibility, high-DPI behavior, and release packaging.
- **In scope with Yune coordination:** any new frontend requirement that cannot be expressed through current `rime_get_api()` or `rime_get_typeduck_profile_api()` must become a named Yune API/profile proposal with tests before implementation.
- **Out of scope:** keeping librime as a runtime fallback, widening the default Yune `RimeApi`, preserving old Weasel UI architecture for its own sake, blindly porting historical commits, or treating upstream Weasel implementation details as product requirements.
- **Reference only:** old TypeDuck/librime and old TypeDuck-Windows behavior remain useful for product comparison, compatibility probes, and regression fixtures.

## Current Inputs

- Yune M10 proved that stock TypeDuck-Windows can load packaged Yune, create a session, process `ngohaig`, and return `status.schema_id=jyut6ping3` with candidate/context data through IPC.
- M10 did **not** prove interactive TSF typing into a real application or visible candidate-window rendering. The first Phase 2 smoke should close that gap before any large rewrite.
- TypeDuck-Windows Phase 0A was attempted on `dev` in commit `03d3608` (`Document Yune Windows Phase 0 audit`): the Yune-backed IPC console smoke passed, setup/deployer registration passed, but the real Notepad TSF smoke hung/crashed in `TypeDuckServer.exe` while consuming/rendering candidates.
- Treat the Phase 0A crash as a **Yune/frontend boundary** issue until PageHeap/Application Verifier, `RimeCandidate` layout checks, and candidate/comment byte parity against the TypeDuck `librime` fork oracle rule that out.
- M24 produced a Cantonese-first TypeDuck-Web dogfood UI on Vite + React + Tailwind + local components, including settings, status, typeface, candidate-layout, and dictionary/detail surfaces. Treat it as a reuse candidate for Windows settings/dictionary UI through WebView2, not as a candidate-window rendering engine.
- The local TypeDuck-Windows checkout is at `C:\Users\laubonghaudoi\Documents\GitHub\TypeDuck-Windows` on `dev`, with untracked planning/workflow artifacts already present. Do not sweep those into unrelated commits.
- Existing TypeDuck-Windows modules worth auditing:
  - `WeaselTSF/` - TSF text service boundary.
  - `WeaselServer/` and `WeaselIPCServer/` - server process and IPC lifecycle.
  - `WeaselIPC/` - client/server wire format helpers.
  - `RimeWithWeasel/` - current RIME engine adapter.
  - `WeaselUI/` - candidate window and multi-hint panel behavior.
  - `WeaselDeployer/` and `WeaselSetup/` - settings, deployer, installer.
  - `INTEGRATION_PLAN.md`, `LIBRIME_INTEGRATION_PLAN.md`, and `docs/commit-inventory.md` - historical modernization analysis.

## Strategic Recommendation

The product direction should be **Yune-only**, not dual-engine. A librime fallback would preserve technical debt and make product bugs harder to classify.

The process direction should be **shared server/IPC by default**. Yune's current runtime paths, sessions, user data, and API tables are process-global; a single server process gives cross-application learning, one deploy/runtime owner, crash isolation, and direct reuse of the M10-proven TypeDuck-Windows path. Loading Yune in-process inside every TSF host application is allowed only if Phase 0 proves safe userdb locking, per-app lifecycle, security, and crash behavior.

The repo direction should be **decision-gated**, with this default bias:

1. **Prefer a fresh Windows product repo if the audit can isolate a small platform shell.** This gives us a clean architecture, modern UX, and a product identity not constrained by old Weasel design.
2. **Do not start from a blank TSF implementation unless a spike proves it is cheap.** TSF registration, edit sessions, key sinks, composition, language bar behavior, IPC, installer registration, candidate-window positioning, and cleanup are high-risk platform glue. The existing TypeDuck-Windows repo has working examples and should be mined aggressively.
3. **Use a hybrid extraction path if needed.** Start a fresh repo, but transplant or adapt the smallest proven slices of `WeaselTSF`, `WeaselServer`, `WeaselIPC`, installer registration, smoke harnesses, and caret/candidate-window positioning until a native Yune-first shell is stable. Keep old UI/settings design out unless it earns its place.
4. **Continue in the existing repo only if audit proves extraction is riskier than in-place modernization.** In-place work is acceptable if it is the fastest route to a shippable Yune-only Windows IME, but it should still delete librime fallback assumptions and old architecture debt as explicit milestones.

## Platform Stack Notes

- Microsoft TSF is the official Windows text-service framework for IMEs and language input services; the actual IME boundary should treat TSF as the platform contract.
- WebView2 is the preferred first spike for settings, diagnostics, and dictionary-panel reuse because M24 already produced the Cantonese-first React/Tailwind UI. The spike must prove runtime availability, packaging, theme/font integration, host-command safety, and log/export plumbing before this becomes a product dependency.
- Windows App SDK / WinUI 3 remains the native fallback for settings, diagnostics, setup helpers, and product windows if WebView2 reuse proves too heavy or awkward.
- Do not use WebView2 for the first inline candidate window unless latency, focus, IME positioning, and accessibility are proven. Candidate rendering starts native.
- Candidate UI should start as a small native renderer with deterministic layout, high-DPI support, and clear test hooks. It can become prettier after it is stable.

## Phase Plan

### Phase 0A - Existing Build Interactive TSF Smoke

**Goal:** Close the real post-M10 frontend gap before choosing a repo strategy:
prove the already-buildable TypeDuck-Windows shell can type through Yune in a
real Windows text field.

**Why first:** M10 proved stock server/client IPC, not interactive TSF typing.
This smoke gives an immediate dogfood target and prevents a repo rewrite from
masking a platform-shell issue.

**Checklist:**
- [ ] Start from the pinned TypeDuck-Windows checkout and the M10-proven Yune package path.
- [ ] Rebuild the existing TypeDuck-Windows solution/server/test artifacts from a clean command sequence.
- [ ] Install/register the existing TSF IME on a Windows host.
- [ ] Activate the IME in Notepad.
- [ ] Type `ngohaig`, observe live composition, visible candidate data, commit a candidate, and record the resulting text.
- [ ] Repeat in one Chromium-based text field if Notepad passes.
- [ ] Save logs/screenshots under a deterministic evidence directory in the Windows working repo.
- [ ] Record exact remaining blockers if interactive TSF typing cannot be completed.

**Acceptance gate:** A reviewer can see whether the existing TypeDuck-Windows
platform shell can already dogfood Yune interactively, before any fresh-repo or
extraction decision is made.

### Phase 0B - Audit, Process Model, And Repo Decision

**Goal:** Decide fresh repo, existing repo, or hybrid extraction with evidence.

**Files to read in TypeDuck-Windows:**
- `INTEGRATION_PLAN.md`
- `LIBRIME_INTEGRATION_PLAN.md`
- `docs/commit-inventory.md`
- `README.md`
- `INSTALL.md`
- `WeaselTSF/**`
- `WeaselServer/**`
- `WeaselIPC*/**`
- `RimeWithWeasel/**`
- `WeaselUI/**`
- `WeaselDeployer/**`
- `WeaselSetup/**`

**Required outputs:**
- `docs/windows-next-audit.md` in the Windows working repo.
- `docs/windows-next-repo-decision.md` in the Windows working repo.
- `docs/windows-next-process-model.md` in the Windows working repo.

**Checklist:**
- [ ] Record the current branch, remote, dirty files, and untracked planning artifacts.
- [ ] Build or reproduce the current TypeDuck-Windows baseline from a clean command sequence.
- [ ] Re-run the Yune-backed M10 stock IPC smoke and save the evidence path.
- [ ] Include the Phase 0A interactive TSF result and decide whether it is good enough for daily dogfood.
- [ ] Decide process model explicitly:
  - default `shared-server-ipc` if the existing Weasel/TypeDuck server spine remains the safest fit for Yune's process-global runtime and userdb model,
  - `in-process-tsf` only if a spike proves safe per-app lifecycle, userdb locking, crash behavior, and privacy behavior,
  - `hybrid-server` if TSF is extracted into a fresh repo while the Yune host remains a shared process.
- [ ] Classify each existing module as `reuse`, `rewrite`, `reference-only`, or `delete`.
- [ ] Treat `WeaselTSF`, `WeaselServer`, `WeaselIPC`, installer/registration code, smoke harnesses, and `WeaselUI` caret/candidate positioning as first-class audit targets, even if visible UI is rewritten.
- [ ] Decide repo path:
  - `fresh-repo` if TSF/IPC/installer slices can be extracted cleanly.
  - `existing-repo` if the platform shell is too intertwined to extract safely.
  - `hybrid-extraction` if a fresh repo is still desired but the first milestone imports minimal shell code.
- [ ] Name the first executable smoke target before product UI work starts.

**Acceptance gate:** A reviewer can read the audit and understand exactly why
the process model and repo strategy were chosen, which old modules are allowed
to survive, how Yune is loaded, and how the first dogfoodable Windows IME smoke
will be reached.

### Phase 0C - Boundary Crash Diagnosis

**Goal:** Localize the interactive Notepad crash before any rewrite or product
spike hides the real compatibility problem.

**Why now:** The IPC console path proves Yune can generate candidates through
the server path. It does not prove the TypeDuck-Windows UI consumes Yune's
candidate structs, ownership model, or rich comment bytes correctly.

**Checklist:**
- [ ] Run `TypeDuckServer.exe` under PageHeap/Application Verifier, for example `gflags /p /enable TypeDuckServer.exe /full`, and capture the first corrupting allocation/free stack.
- [ ] Confirm the frontend strides and owns the candidate array according to the exact `RimeCandidate` layout exported by the packaged Yune headers.
- [ ] Compare Yune's `ngohaig` candidate text/comment bytes against stock TypeDuck-Windows backed by the TypeDuck `librime` fork oracle for the same input.
- [ ] Pay special attention to rich dictionary-panel comments using control-byte payloads, because they are rendered only in the interactive candidate path.
- [ ] If Yune output differs in a renderer-incompatible way, file the issue as a Yune TypeDuck-profile compatibility or host-contract bug.
- [ ] If candidate layout and comment bytes are proven compatible, classify the remaining crash as WeaselUI/DirectWrite debt and decide whether to patch or bypass the old renderer.
- [ ] Save curated crash and diff evidence under `docs/evidence/` in the Windows repo.
- [ ] Re-run the Notepad TSF smoke after the fix or isolation and commit the result.

**Acceptance gate:** A reviewer can tell whether the crash was caused by Yune
candidate/comment compatibility, the frontend renderer, or an explicitly
isolated platform-shell issue.

### Phase 1 - Yune Host Contract Spike

**Do not start this phase until Phase 0C is reviewed.**

**Goal:** Build the smallest Windows executable that loads packaged Yune and drives a real session without old frontend UI.

**Expected components:**
- `YuneHost` wrapper around `rime.dll`, `rime_get_api()`, and `rime_get_typeduck_profile_api()`.
- deterministic lifecycle: setup, initialize, deploy, create session, select `jyut6ping3`, process keys, read status/context/candidates, destroy session, cleanup.
- default-off AI staging seam modeled on TypeDuck-Web's `stage_ai` flow. It may be a no-op in this milestone, but the host/IPC contract must reserve a way to request labeled second-pass AI candidates later without changing the main key-processing path.
- sensitive-context input flag that can suppress learning, AI, and private logs when TSF reports password/secure contexts.
- log file with package path, schema id, session id, key events, candidate count, and errors.

**Checklist:**
- [ ] Load the Yune package produced by `scripts/package-typeduck-windows.ps1`.
- [ ] Reject startup if the TypeDuck profile accessor is missing.
- [ ] Create a session and confirm `status.schema_id=jyut6ping3`.
- [ ] Process `ngohaig` and record candidate/context data.
- [ ] Process a reverse-lookup probe such as `` `zhe `` if the packaged schema assets support it.
- [ ] Expose a no-op or disabled `stage_ai`-style call in the host contract, returning no candidates unless explicitly enabled later.
- [ ] Expose a sensitive-context mode and prove it suppresses AI staging, learning, and typed-content logging.
- [ ] Save stdout/log evidence under a deterministic evidence directory.

**Acceptance gate:** A console smoke can prove Yune-only engine lifecycle without TSF, tray UI, settings UI, or installer.

### Phase 2 - Minimal TSF Shell

**Goal:** Register a Windows IME text service that forwards key events to the Yune host and commits text into real applications.

**Working assumption:** C++20 / C++/WinRT is the default because TSF is COM-heavy and existing Weasel/TypeDuck examples are C++.

**Checklist:**
- [ ] Implement or extract TSF registration and unregistration.
- [ ] Implement key event sink, composition lifecycle, edit session commit, and cleanup.
- [ ] Route key events to the Phase 1 Yune host through the Phase 0-selected process model.
- [ ] If using the shared-server model, prove server startup, reconnect, crash recovery, and clean shutdown.
- [ ] Map TSF password/secure contexts to the Yune host sensitive-context flag: no learning, no AI staging, and no typed-content logs.
- [ ] Keep all UI optional; first smoke can use logs and simple candidate data.
- [ ] Test in Notepad and one Chromium-based app.
- [ ] Verify install, activate IME, type `ngohaig`, commit candidate, deactivate, uninstall, and reboot-safe cleanup.

**Acceptance gate:** A minimal installed IME can type through Yune in real Windows text fields and can be removed cleanly.

### Phase 3 - Candidate Window V1

**Goal:** Replace log-only candidate display with a fast, stable, readable candidate window.

**Checklist:**
- [ ] Render horizontal and vertical candidate list modes.
- [ ] Support page size, page navigation, highlighted candidate, numbering, and selection.
- [ ] Render candidate comments and dictionary-panel details without inline control markers.
- [ ] Support high-DPI scaling, multi-monitor positioning, dark/light themes, and IME focus transitions.
- [ ] Keep the first candidate window native and small; do not introduce a broad UI framework here before latency/focus are proven.

**Acceptance gate:** Candidate UI remains correctly positioned and responsive while typing in Notepad, Chromium, Office-like rich text, and at least one high-DPI display setting.

### Phase 4 - Settings And Product UI

**Goal:** Build a modern TypeDuck settings experience without carrying old Weasel settings debt.

**Working assumption:** WebView2 hosting the M24 React/Tailwind settings and
dictionary-panel UI is the first settings/product-UI spike. WinUI 3 / Windows
App SDK is the native fallback if WebView2 reuse fails packaging, accessibility,
host-command, or lifecycle gates.

**Checklist:**
- [ ] Cantonese-first labels with clear English support text.
- [ ] Decide whether the M24 React settings/dictionary UI can be reused directly through WebView2.
- [ ] Schema selection and current engine status.
- [ ] TypeDuck profile controls mapped to real Yune config/profile behavior.
- [ ] Candidate UI controls: horizontal/vertical, page size, fonts, comments, Jyutping display, dictionary panel.
- [ ] If using WebView2, define a narrow host bridge for settings read/write, deploy, diagnostics export, and dictionary detail data; do not expose arbitrary local files or shell commands.
- [ ] Diagnostics page: engine version, schema versions, package path, user data dir, last deploy result, log open/export.
- [ ] Safe reset/redeploy controls with confirmation.

**Acceptance gate:** A non-developer can configure the IME without editing YAML, and every visible control either changes a real setting or is absent.

### Phase 5 - Packaging, Installer, Updates, And Diagnostics

**Goal:** Make the IME installable, diagnosable, and maintainable outside a developer shell.

**Checklist:**
- [ ] Package Yune runtime, schema assets, TSF shell, server/host process, settings app, and candidate UI.
- [ ] Install and register TSF service with clear rollback on failure.
- [ ] Uninstall cleanly, including service shutdown and registry cleanup.
- [ ] Add versioned logs and a support bundle export.
- [ ] Decide update mechanism after security review; do not add auto-update silently.
- [ ] Add crash/error handling that produces actionable diagnostics without leaking private typed content by default.

**Acceptance gate:** Fresh install, upgrade, uninstall, and support-bundle export work on a non-development Windows user account.

### Phase 6 - Product Hardening

**Goal:** Turn the prototype into a daily-usable Windows IME.

**Checklist:**
- [ ] App compatibility matrix: Notepad, Chromium, Edge, Office, VS Code, terminal, Electron apps, UWP/WinUI apps.
- [ ] Accessibility matrix: screen-reader behavior, keyboard-only settings navigation, contrast, high-DPI.
- [ ] Performance matrix: first activation, first key latency, candidate render latency, deploy time, memory use.
- [ ] Failure-mode matrix: missing schema, corrupt user data, Yune DLL missing, profile accessor missing, IPC server crash, settings write failure.
- [ ] Release checklist: signing decision, installer artifact, changelog, known issues, rollback path.

**Acceptance gate:** The product can be dogfooded as a daily IME with known issues documented and no critical install/uninstall or text-entry failures.

## Repo Decision Matrix

| Choice | Choose when | Avoid when |
|---|---|---|
| Fresh repo | We can isolate TSF/IPC/server/installer shell contracts; old UI/settings are mostly debt; product UX needs a clean architecture | TSF/server/installer extraction becomes the main project and blocks basic typing |
| Existing repo | The old platform shell is too intertwined to extract safely; build/release path already works; fastest daily-dogfood path matters most | In-place work keeps old librime assumptions, stale UI architecture, or branch drift as permanent constraints |
| Hybrid extraction | We want fresh architecture but need proven TSF/IPC/server/installer/caret-positioning pieces to reduce risk | The extracted code becomes an unreviewed copy of the old repo with a new name |

**Default recommendation after review:** pursue **hybrid extraction toward a fresh Yune-first repo while preserving the proven shared server/IPC spine**, unless Phase 0 proves the old shell cannot be extracted safely. This honors the product goal of doing better than Rime/Weasel while avoiding a blind TSF rewrite and avoiding an in-process Yune instance in every host app.

## External Review Follow-Up

The first external strategic review agreed with the Yune-only direction and the
decision-gated repo strategy, but required five plan changes before handoff:

- Make the process-model decision explicit. The default is the shared
  server/IPC model because it fits Yune's process-global singleton and userdb
  model; in-process TSF-per-app loading must prove itself.
- Reuse the M24 React/Tailwind settings and dictionary UI through WebView2 if
  packaging/accessibility/security gates pass; keep the latency-critical inline
  candidate window native.
- Reserve a default-off `stage_ai`-style second-pass candidate seam in the host
  and IPC contract from the start.
- Map TSF password/secure contexts into Yune's sensitive-context privacy gate:
  no learning, no AI staging, and no typed-content logs.
- Add Phase 0A interactive Notepad typing with the existing build before any
  repo rewrite or extraction commitment.

The second external review of the Phase 0A/0B deliverables endorsed the process
model, repo decision, evidence discipline, and privacy/AI/WebView2 boundaries,
but corrected one important attribution risk:

- Do not call the Notepad crash "frontend-only" yet. IPC success proves Yune
  candidate generation, but not native consumption/rendering of Yune candidate
  structs and rich comment bytes.
- Add Phase 0C before YuneHost/WebView2 implementation: PageHeap/Application
  Verifier, `RimeCandidate` layout validation, and Yune-vs-TypeDuck-fork
  candidate/comment byte parity for `ngohaig`.
- Re-evaluate the repo decision immediately after the first passing Notepad
  interactive smoke, before temporary work in the old repo becomes permanent.

## Open Questions For Review

- Exactly how much of the existing server/IPC wire protocol should survive unchanged versus be narrowed for a Yune-only host?
- Can WebView2 host the M24 settings/dictionary UI without unacceptable installer, runtime, accessibility, or security cost?
- Is C++/WinRT the best long-term TSF shell language, or should a Rust `windows-rs` spike be attempted after the C++ baseline is understood?
- Should the settings app be a separate WebView2/WinUI executable, part of the server process, or launched through a tray/deployer process?
- Should the first repo be private/new while the old TypeDuck-Windows repo remains the reference, or should the existing repo receive the new architecture directly?
- What signing, installer, and update policy is acceptable for early dogfooding?

## Non-Negotiables

- Yune is the only runtime engine.
- The default Yune `rime_get_api()` ABI remains upstream-shaped.
- TypeDuck fork-only behavior stays behind named profile/API surfaces.
- The Windows plan must explicitly choose the process model before large implementation work; shared server/IPC is the default unless rejected by evidence.
- Host/IPC contracts reserve a default-off AI staging seam, even if Phase 2 ships with AI disabled.
- Password or secure TSF contexts suppress learning, AI staging, and typed-content logging.
- The inline candidate window starts native; WebView2 is for settings/dictionary UI unless a separate candidate-window spike proves it safe.
- Candidate/output behavior changes require oracle or dogfood evidence, not visual guesswork.
- Interactive TSF crashes are boundary problems until `RimeCandidate` layout and candidate/comment byte compatibility are ruled out.
- The old TypeDuck-Windows repo is useful evidence, but the product goal is a better Windows IME, not a Weasel clone.
