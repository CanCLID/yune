# M32 AI-Native Public Demo Product Layer Implementation Plan

> **Status:** Planned - **Milestone:** M32 (AI-native public demo/product expansion) - **Created:** 2026-06-22 - **Type:** execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand Yune's AI-native layer into a richer public-demo/product surface after an explicit product-priority decision, without changing deterministic classic input behavior or delaying the Windows frontend by default.

**Architecture:** M32 builds on the completed M11 core/CLI AI layer and the completed M13 default-off TypeDuck-Web exposure. Classic input remains provider-free, synchronous, and byte-identical when AI is off. AI is a second-pass, explicitly labeled, local-first layer with privacy and memory controls; any remote-provider path requires an explicit decision and remains off by default. The first public Yune web demo should keep AI hidden or developer-only unless M32 deliberately changes that with reviewed evidence.

**Tech Stack:** Rust (`yune-core` AI modules, `yune-rime-api` TypeDuck-Web exports), `packages/yune-web-runtime`, TypeDuck-Web Vite + React + Tailwind, Playwright, existing M11/M13 AI tests and evidence, and the M31 public-demo deployment harness.

---

## Status

Planned. Do not fold M32 into M31. M31 may only add AI posture gates; M32 owns richer AI UX, public-demo AI controls, and any provider-policy expansion. M32 should not run ahead of P2-WIN-01 unless the user explicitly chooses an AI/web-product detour over the Windows product track.

## Scope

In scope:

- A public-demo AI UX that makes AI candidates understandable without preempting classic candidates.
- A product-priority decision that says whether AI should be exposed publicly now, remain hidden/developer-only, or wait for native frontend context.
- Local-first AI controls, including clear default-off state and explicit enable/disable behavior.
- Privacy/sensitive-context controls and evidence that unknown contexts default to sensitive.
- AI memory controls that remain separate from librime userdb and can be inspected, cleared, or disabled.
- Browser and native tests proving the M11/M13 invariants still hold through the product surface.
- A public-demo evidence report if AI is exposed on the M31 deployment.

Out of scope:

- Changing classic candidate ranking, comments, commit semantics, or upstream/TypeDuck oracle behavior.
- Making remote AI calls by default.
- Storing provider secrets in the repo or browser bundle.
- Exposing AI through Windows/native frontends without a separate host-context and privacy plan.
- Replacing the deterministic TypeDuck-Web harness with a new product repo.
- Turning on AI in the first public web demo by accident or as a side effect of M31.
- Running before P2-WIN-01 by default.

## Preconditions

- M31 is complete or the public-demo harness is stable enough to run browser evidence locally.
- P2-WIN-01 has either resumed as the primary product track, or the user has explicitly decided to prioritize AI/web-product work first.
- `docs/plans/reference/m11-design-ai-native.md` and `docs/plans/completed/m13-plan-ai-native-frontend-exposure.md` are read before implementation.
- AI-off output identity, classic index 0, no default AI auto-commit, no userdb leak, privacy gating, and local-first behavior are treated as hard invariants.

## Acceptance Gates

- `M32-AI-00`: M32 starts only after an explicit product-priority decision records whether AI stays hidden/developer-only, becomes public opt-in, or waits for native frontend context.
- `M32-AI-01`: AI-off output is byte-identical to the classic path through native runtime and browser evidence.
- `M32-AI-02`: AI never runs in the synchronous classic per-key path; classic candidates render first and remain available.
- `M32-AI-03`: AI candidates are labeled, appear after the classic top candidate, and never become default-commit candidates unless a future explicit config changes that with tests.
- `M32-AI-04`: Sensitive or unknown contexts suppress remote calls and AI memory learning.
- `M32-AI-05`: AI memory is inspectable, clearable, disable-able, and never written into librime `*.userdb`.
- `M32-AI-06`: Any remote-provider experiment is decision-logged, default-off, secret-free in git, and absent from public build unless explicitly enabled by deployment config.
- `M32-AI-07`: Public-demo browser evidence proves the UX and safety gates; hidden or disabled AI is documented instead of implied.

## File Responsibilities

- `docs/plans/reference/m11-design-ai-native.md`: source of truth for AI architecture and invariants.
- `docs/plans/completed/m13-plan-ai-native-frontend-exposure.md`: source of truth for existing TypeDuck-Web AI exposure and safety evidence.
- `crates/yune-core/src/ai/`: owns provider, privacy, memory, and local-model behavior.
- `crates/yune-core/src/engine.rs`: owns staged AI merge and commit-boundary safety.
- `crates/yune-rime-api/src/typeduck_web.rs`: owns browser/WASM AI exports and response source labels.
- `crates/yune-rime-api/tests/typeduck_web.rs`: owns native browser-runtime AI contract tests.
- `packages/yune-web-runtime/`: owns TypeScript AI control bindings.
- `apps/yune-web/yune-integration/`: owns bridge-level AI wiring.
- `apps/yune-web/source/`: owns public-demo AI UI.
- `apps/yune-web/e2e/`: owns browser evidence.
- `docs/roadmap.md`, `docs/requirements.md`, `docs/decisions.md`: own milestone status, requirements, and any remote-provider decision.

---

## Task 0 - Product Priority And AI Exposure Decision

**Files:**

- Read: `docs/roadmap.md`
- Read: `docs/plans/active/p2-win01-plan-typeduck-windows-next.md`
- Read: `docs/plans/reference/m11-design-ai-native.md`
- Read: `docs/plans/completed/m13-plan-ai-native-frontend-exposure.md`
- Create: `apps/yune-web/e2e/results/m32-ai-product/product-priority-decision.md`
- Create: `apps/yune-web/e2e/results/m32-ai-product/m11-m13-invariant-audit.md`

- [ ] **Step 0.1: Record the product-priority decision**

Write `product-priority-decision.md` with one of these decisions:

```markdown
# M32 Product Priority Decision

Decision: hidden_developer_only | public_opt_in | defer_until_native_frontend

Windows priority: P2-WIN-01 remains primary | AI/web product intentionally prioritized first

Reason:

Public exposure:

Remote provider policy:
```

Expected default:

- Use `hidden_developer_only` or `defer_until_native_frontend` unless the user explicitly asks to expose AI publicly.
- If `P2-WIN-01` is ready to proceed and not blocked, do not start M32 unless the decision says AI/web work is intentionally prioritized first.

- [ ] **Step 0.2: Extract the invariant checklist**

Create `m11-m13-invariant-audit.md` with this checklist:

```markdown
| Invariant | Existing owner | M32 proof |
| --- | --- | --- |
| AI-off byte identity | M13 native/browser tests | new native + browser smoke |
| provider-free classic key path | M11/M13 architecture | source audit + native test |
| classic index 0 | engine merge | native + browser candidate order |
| no default AI auto-commit | engine commit intent | native + browser Space/Return |
| no userdb leak | engine commit boundary | native memory/userdb assertion |
| sensitive default | AiPrivacyPolicy | native privacy assertion |
| local-first | provider policy | browser network posture |
```

- [ ] **Step 0.3: Run current AI gates before changing behavior**

Run:

```powershell
cargo test -p yune-core ai -- --nocapture
cargo test -p yune-rime-api --test typeduck_web ai -- --nocapture
npm.cmd --prefix packages\yune-typeduck-runtime test
```

Expected:

- Existing AI gates pass before M32 changes start.

## Task 1 - Product Policy And Provider Decision

**Files:**

- Create: `docs/plans/active/m32-ai-product-policy.md`
- Modify only if a new durable decision is made: `docs/decisions.md`

- [ ] **Step 1.1: Write the AI public-demo product policy**

Create `docs/plans/active/m32-ai-product-policy.md` with these sections:

```markdown
# M32 AI Product Policy

## Default State

AI is off by default. For the first public Yune web demo, AI is hidden or developer-only unless the M32 product-priority decision explicitly chooses public opt-in. Classic input is fully functional without AI.

## Provider Policy

The public demo uses local/rule-backed providers by default. Remote providers are absent unless a later explicit deployment decision enables them.

## Commit Policy

AI candidates are never committed by Space/Return/default confirm. Explicit numeric/click selection is required.

## Privacy Policy

Unknown browser context is sensitive. Sensitive context suppresses remote calls and AI memory learning.

## Memory Policy

AI memory is separate from librime userdb, inspectable, clearable, and disable-able.
```

- [ ] **Step 1.2: Decide whether remote providers are in this slice**

Choose one of these decisions and record it in the policy:

- `remote_provider = absent`: no remote provider code or config in M32.
- `remote_provider = hidden_dev_only`: code exists behind build/runtime config, no public build exposure, no secrets in git.
- `remote_provider = public_opt_in`: requires a new `docs/decisions.md` entry before implementation and a secret-management plan.

Expected default:

- Use `remote_provider = absent` unless the user explicitly approves a different policy.
- Keep public exposure as `hidden_developer_only` unless `product-priority-decision.md` explicitly chooses `public_opt_in`.

## Task 2 - Native AI Safety Regressions

**Files:**

- Modify: `crates/yune-rime-api/tests/typeduck_web.rs`
- Modify only if needed: `crates/yune-core/src/ai/`
- Create evidence: `apps/yune-web/e2e/results/m32-ai-product/native-ai-gates.md`

- [ ] **Step 2.1: Add native tests for the M32 product surface**

Add or extend focused tests with names like:

```rust
#[test]
fn m32_ai_off_matches_classic_response_bytes() {
    // Drive TypeDuck-Web runtime with AI disabled.
    // Drive the same input after AI enable/disable round trip.
    // Assert the classic response bytes are unchanged.
}

#[test]
fn m32_ai_candidates_are_explicit_selection_only() {
    // Enable local AI and stage a result.
    // Assert classic row 0 remains classic.
    // Assert default confirm commits classic.
    // Assert explicit AI row selection is required for AI commit.
}

#[test]
fn m32_ai_memory_never_writes_librime_userdb() {
    // Explicitly select an AI candidate.
    // Assert take_pending_userdb_learning() returns None.
    // Assert AI memory behavior follows privacy policy.
}
```

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_web m32_ai -- --nocapture
```

Expected before UI/product changes:

- Tests pass if existing M13 behavior already covers the invariant, or fail only where M32 needs additional product controls.

## Task 3 - AI UX In TypeDuck-Web

**Files:**

- Modify: `packages/yune-web-runtime/src/`
- Modify: `apps/yune-web/yune-integration/`
- Modify: `apps/yune-web/source/src/`
- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Regenerate: `apps/yune-web/patches/yune-web-runtime.patch`
- Create evidence: `apps/yune-web/e2e/results/m32-ai-product/browser-ai-ux.md`

If Task 0 records `defer_until_native_frontend`, stop before this task and close M32 as deferred-with-policy rather than adding web AI UX. If Task 0 records `hidden_developer_only`, keep controls behind a developer flag and do not expose them in the public default build.

- [ ] **Step 3.1: Make AI mode legible**

Add UI state that clearly distinguishes:

- AI off
- local AI enabled
- AI result pending
- AI result ready
- AI unavailable because sensitive/default policy blocks the selected provider

The UI must not use modal explanations for normal typing. It should use compact labels, disabled states, and candidate-row markers.

- [ ] **Step 3.2: Keep candidate interaction explicit**

Add or verify browser behavior:

- Space/Return commits classic candidate.
- Number/click can explicitly select an AI row.
- AI rows are labeled with source.
- AI rows never replace the classic top row.

- [ ] **Step 3.3: Run browser AI UX evidence**

Run:

```powershell
npm.cmd --prefix packages\yune-typeduck-runtime test
npm.cmd --prefix packages\yune-typeduck-runtime run build
npm.cmd --prefix apps\yune-web\source run build
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M32 AI" --workers=1
```

Expected:

- Browser evidence under `apps/yune-web/e2e/results/m32-ai-product/` shows AI-off, AI-on, explicit AI selection, and no default AI auto-commit.

## Task 4 - Privacy And Memory Controls

**Files:**

- Modify: `crates/yune-core/src/ai/`
- Modify: `crates/yune-rime-api/src/typeduck_web.rs`
- Modify: `apps/yune-web/source/src/`
- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Create evidence: `apps/yune-web/e2e/results/m32-ai-product/privacy-memory.md`

- [ ] **Step 4.1: Expose memory controls only if native support is present**

If existing M11 memory APIs are sufficient, expose:

- inspect memory entries
- clear AI memory
- disable AI memory

If the browser host cannot safely persist AI memory yet, expose only disabled/clear status and record why in `privacy-memory.md`.

- [ ] **Step 4.2: Assert sensitive default behavior**

Add native and browser evidence proving:

- unknown browser context is sensitive
- sensitive context suppresses remote calls
- sensitive context suppresses AI memory learning
- explicit standard context is required before AI memory can learn

- [ ] **Step 4.3: Assert no userdb leakage**

Run:

```powershell
cargo test -p yune-core ai_memory -- --nocapture
cargo test -p yune-rime-api --test typeduck_web m32_ai_memory -- --nocapture
```

Expected:

- AI commits do not create librime userdb learning events.
- AI memory files use the `.ai-memory` namespace if persistence is enabled.

## Task 5 - Public Demo Evidence

**Files:**

- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Create evidence: `apps/yune-web/e2e/results/m32-ai-product/public-demo-ai.md`

- [ ] **Step 5.1: Run local public-demo smoke**

Run against the M31 public-demo build:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File apps\yune-web\public-demo\build.ps1
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M32 AI" --workers=1
```

Expected:

- AI is off on first load.
- If Task 0 chose `hidden_developer_only`, the public-default UI does not expose AI controls and a developer-flag route/build is used for AI UX evidence.
- If Task 0 chose `public_opt_in`, enabling local AI does not reload the engine.
- Classic candidate row 0 remains classic.
- Explicit AI row selection works.

- [ ] **Step 5.2: Run deployed public-demo smoke when M31 has a URL**

Read the deployed URL from the M31 evidence file, then run:

```powershell
$DeployedUrl = Get-Content apps\yune-web\e2e\results\m31-yune-web-public-demo\deployed-url.txt -Raw
$env:YUNE_WEB_APP_URL = ($DeployedUrl.TrimEnd("/") + "/web/")
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M32 AI" --workers=1
```

Expected:

- Same safety gates pass on the deployed public demo.
- If AI is hidden/developer-only, the deployed public-default smoke proves AI is absent/off and the developer-only evidence remains local unless a separate deployment config exposes it.
- If M31 does not have a deployed URL, record `public deployed AI smoke blocked: no M31 URL` in `public-demo-ai.md` and do not claim deployed AI readiness.

## Task 6 - Closeout

**Files:**

- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Modify only if provider policy changes: `docs/decisions.md`
- Move when complete: `docs/plans/active/m32-plan-ai-native-public-demo-product-layer.md` to `docs/plans/completed/`

- [ ] **Step 6.1: Run full gates**

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core ai -- --nocapture
cargo test -p yune-rime-api --test typeduck_web m32_ai -- --nocapture
cargo test --workspace
npm.cmd --prefix packages\yune-typeduck-runtime test
npm.cmd --prefix packages\yune-typeduck-runtime run build
npm.cmd --prefix apps\yune-web\source run build
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M32 AI" --workers=1
git diff --check
```

Expected:

- Full gates pass.
- If TypeDuck-Web source changed, patch reverse/forward checks pass.

- [ ] **Step 6.2: Update requirements and roadmap**

Add `M32-AI-REQ-*` rows matching the acceptance gates. Update roadmap with:

- the product-priority decision
- whether AI is public-visible or still hidden
- provider policy
- evidence path
- deployed URL status
- any blocked native/frontend exposure

- [ ] **Step 6.3: Commit scoped changes**

Stage only M32 files:

```powershell
git status --short
git add -- docs\roadmap.md docs\requirements.md docs\decisions.md docs\plans\m32-plan-ai-native-public-demo-product-layer.md docs\plans\m32-ai-product-policy.md crates packages apps\yune-web
git commit -m "Expand AI-native public demo layer"
git push origin main
```

Only include `docs\decisions.md`, `crates`, `packages`, or `apps\yune-web` paths if they actually changed.

## Review Questions

- Should M32 start before P2-WIN-01, or should it stay deferred while Windows product work resumes?
- Should M32 expose AI publicly at all, or keep it hidden behind a local-only developer control after the safety evidence lands?
- Should remote providers remain absent, or should a separate provider decision be written before any remote experiment?
- Which context fields can the browser public demo honestly provide for privacy classification?
- Is browser AI memory persistence desirable now, or should memory remain native/CLI-only until a clearer product need exists?
