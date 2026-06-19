# M13 AI-Native Frontend Exposure Implementation Plan

> **Status:** Finished · **Milestone:** M13 (AI-native frontend exposure) · **Closed:** 2026-06-19 · **Type:** execution plan

> **For agentic workers:** implement this plan **task-by-task**; steps use checkbox (`- [x]`) syntax for tracking. Do not start a later task before the earlier task's acceptance passes. The AI-off byte-identity gate (Task 1) must hold at every step.

**Goal:** Expose M11's completed CLI/core AI layer through the **TypeDuck-Web** frontend — **default-off, local-first**, and gated by the same safety invariants already proven in the CLI — without changing any core or TypeDuck compatibility behavior. This is the first real-frontend test of the product thesis.

## Architecture (the decisions that shape everything)

**1. The provider runs in Rust, never in JS.** The `LocalModelProvider` is reused verbatim from M11; JS carries **no** provider logic. yune-rime-api already depends on yune-core, so the provider is importable into the WASM module.

**2. Two-pass, so the per-key path never invokes provider code (preserves M11 invariant #1 literally).** M11 requires the synchronous per-key path to *never* run provider/model code — it only reads an already-staged result ([ai-native-design.md §2.1](../ai-native-design.md)). M13 honors this with two calls per keystroke:

- `yune_typeduck_process_key` is **unchanged** — it returns the classic response (plus any AI already staged for the *current* input from a prior pass) and **never runs the provider**. AI-off is therefore byte-identical by construction, and even AI-on's *first* response for a new input is classic-only.
- A **new** `yune_typeduck_stage_ai(state)` export runs `LocalModelProvider::provide(engine.context(), budget)` **synchronously**, calls `engine.stage_ai_result(result)`, and returns the AI-augmented response. The TypeDuck-Web Web Worker calls it **after** posting the classic result to the UI (e.g. next microtask), so classic candidates are never delayed; AI rows arrive as a bounded **second-pass update**.

**Honest latency claim:** classic input is never blocked or slowed (returned first, and the worker is off the main UI thread); AI adds bounded second-pass latency on the worker only. The roadmap states it this way — M13 does **not** claim zero-added-latency for the AI path itself.

**3. All M11 safety gates are inherited, not re-implemented.** Classic-first merge ([engine.rs:940](../../../crates/yune-core/src/engine.rs)), no-auto-commit ([engine.rs:845](../../../crates/yune-core/src/engine.rs)), no-userdb-leak, and privacy ([ai/mod.rs:77](../../../crates/yune-core/src/ai/mod.rs)) live in `yune_core::Engine`; the WASM host reaches them by calling standard APIs.

**4. Local-first and sensitive-by-default.** The browser supplies no app/field, so `Context.ai_context.privacy_class` defaults to `Sensitive`. `AiPrivacyPolicy::allows_provider` permits `Local`/`Mock` and **blocks `Remote`** under sensitive — so remote is impossible by policy, and M13 ships `Local` only. **Consequence:** `allows_learning` ([ai/mod.rs:85](../../../crates/yune-core/src/ai/mod.rs)) is **false** under sensitive, so AI-memory learning is **suppressed** in the default browser context (see Task 4) — that is correct and intended for M13.

**Tech stack:** Rust (`yune-core`, `yune-rime-api` WASM/Emscripten module), the `@yune-ime/typeduck-runtime` TS package, the patched TypeDuck-Web app, Playwright browser E2E.

---

## Current state (build on, do not fork)

- **M11 AI core is complete and reachable from WASM.** `YuneTypeDuckState` ([typeduck_web.rs:18](../../../crates/yune-rime-api/src/typeduck_web.rs)) holds a `session_id`; `with_session()` ([session.rs:241](../../../crates/yune-rime-api/src/session.rs)) yields the `SessionState.engine` with `stage_ai_result()` ([engine.rs:117](../../../crates/yune-core/src/engine.rs)) and `snapshot()` ([engine.rs:750](../../../crates/yune-core/src/engine.rs)). **No new core wiring is required.**
- **The provider is synchronous.** `LocalModelProvider::provide` ([local_model.rs:98](../../../crates/yune-core/src/ai/local_model.rs)) runs without threads; **do not** use `AiWorker::spawn` (threads don't port to Emscripten).
- **Source is dropped at the C ABI.** `RimeCandidate` ([abi.rs:52](../../../crates/yune-rime-api/src/abi.rs)) is only `{text, comment, reserved}`, and `RimeGetContext` ([context_api.rs:193](../../../crates/yune-rime-api/src/context_api.rs)) discards `CandidateSource`. So source labels **cannot** be read in `copy_candidate()` — they must come from `engine.snapshot()` (Task 2).
- **The provider's default rules are Mandarin pinyin.** `LocalModelProvider::sample()` ([local_model.rs:106](../../../crates/yune-core/src/ai/local_model.rs)) keys on `nihao`/`hao`; the proven TypeDuck-Web real-assets path is `jyut6ping3_mobile`, so M13 must supply a deterministic rule that fires on a chosen jyutping input (Task 1/5).
- **The frontend is GO-WITH-CONDITIONS** (M9 / D-P10-13). Per-key seam: worker → `adapter.ts processKey` → `translateResponse` → `CandidatePanel`. Browser E2E harness: `third_party/typeduck-web/e2e/` (`playwright.config.ts`, `yune-typeduck.spec.ts`, run steps in `yune-browser-smoke.md`).

## Non-goals (explicitly deferred)

- Remote LLM providers (blocked by sensitive-default policy; M13 ships `Local` only).
- The async / second-Web-Worker provider port (`AiWorker`).
- AI exposure through Windows or any non-TypeDuck-Web frontend.
- Any change to classic-input defaults, the upstream `RimeApi` table/`RimeCandidate` ABI, or TypeDuck compatibility behavior.
- **Persisted / reload-surviving browser AI memory.** Under the sensitive default, learning is suppressed anyway; a future host-context opt-in (non-sensitive classification) is required before `MemoryStore` persistence is meaningful, and it must use the `.ai-memory` namespace, never `*.userdb`.

## File map

- Modify [`crates/yune-rime-api/src/typeduck_web.rs`](../../../crates/yune-rime-api/src/typeduck_web.rs) — `YuneTypeDuckState` gains `ai_enabled` + a lazily-built provider; new `yune_typeduck_set_ai_enabled` and `yune_typeduck_stage_ai`; source-label merge from `engine.snapshot()` in the Yune-owned JSON path. **`process_key` and `RimeCandidate` are untouched.**
- Modify [`crates/yune-rime-api/src/bin/typeduck_web_module.rs`](../../../crates/yune-rime-api/src/bin/typeduck_web_module.rs) and [`scripts/typeduck-exports.txt`](../../../scripts/typeduck-exports.txt) — register the two new exports.
- Modify [`crates/yune-rime-api/tests/typeduck_web.rs`](../../../crates/yune-rime-api/tests/typeduck_web.rs) — native contract tests (the deterministic fallback gate).
- Modify `packages/yune-typeduck-runtime/src/response.ts` (+ wrapper) — optional `source` on the candidate; `setAiEnabled()` + `stageAi()` bindings.
- Modify `third_party/typeduck-web/yune-integration/{adapter.ts,response.ts}` — `RimePreferences.enableAI`, `customize()` → `setAiEnabled`, propagate `source`, and the worker's classic-then-AI pipelining.
- Modify `third_party/typeduck-web/source/src/{types.ts,hooks.ts,App.tsx,Preferences.tsx,Candidate.tsx,worker.ts,rime.ts}` — default-off toggle, the new `stageAi` serialized action, classic-first then second-pass AI render, distinct AI-row rendering.
- Modify `third_party/typeduck-web/e2e/yune-typeduck.spec.ts` — AI safety scenarios.
- Modify `docs/{roadmap.md,requirements.md,decisions.md,CONVENTIONS.md}` — close M13 once evidence lands.

---

## Task 1 — Rust: two-pass in-WASM AI orchestration (roadmap item 0)

**Purpose:** Add an opt-in flag and a separate `stage_ai` pass that runs the M11 provider in Rust **after** the classic response, so the key path stays provider-free and AI-off stays byte-identical.

**Files:** `typeduck_web.rs`, `typeduck_web_module.rs`, `typeduck-exports.txt`, `tests/typeduck_web.rs`, and a jyutping rule for the provider.

**Steps:**

- [x] Add `ai_enabled: bool` (default `false`) and a lazily-built `LocalModelProvider` to `YuneTypeDuckState`; reset both in `yune_typeduck_cleanup`.
- [x] Construct the provider with a **deterministic rule that fires on a chosen `jyut6ping3` input** (e.g. a jyutping syllable string → a candidate), so the browser actually shows an AI row. Keep it local/rule-backed (no network).
- [x] Add `yune_typeduck_set_ai_enabled(state, enabled: i32)` (model on `yune_typeduck_set_option`, [typeduck_web.rs:278](../../../crates/yune-rime-api/src/typeduck_web.rs)). **On disable, clear any staged AI for the current input** by staging `AiResult::Off` through `engine.stage_ai_result()` ([engine.rs:117](../../../crates/yune-core/src/engine.rs)) — otherwise a previously-staged result for the unchanged input would still merge and break AI-off byte-identity.
- [x] Add `yune_typeduck_stage_ai(state)`: inside `with_session()`, run `provider.provide(engine.context(), AI_BUDGET)` synchronously, call `engine.stage_ai_result(result)`, then build and return the response (with source labels, Task 2). Define `const AI_BUDGET: Duration` (small, e.g. 25ms; provider is deterministic so this only bounds it). If `ai_enabled` is false, return the classic response unchanged.
- [x] **Leave `yune_typeduck_process_key` unchanged.** Register both exports; append symbols to `typeduck-exports.txt`.

**Acceptance:**
- `process_key` output is **byte-identical** to before this task for all inputs (it is the unchanged shipping code).
- With `ai_enabled = true`: calling `process_key` then `stage_ai` yields a visible AI candidate **after the classic top candidate**, with **index 0 still the classic top**; a `Pending`/empty result changes nothing.
- `stage_ai` with `ai_enabled = false` returns the classic response (no provider run).

---

## Task 2 — Source-labeled candidates via `engine.snapshot()` (roadmap item 2)

**Purpose:** Surface the AI source label without touching the `RimeCandidate` ABI — by reading the engine's own candidate list.

**Files:** `typeduck_web.rs` (the Yune-owned JSON path), `response.ts`, `adapter.ts`/`response.ts`, `Candidate.tsx`.

**Steps:**

- [x] In `yune_typeduck_stage_ai`'s response builder, after staging, read `engine.snapshot()` ([engine.rs:750](../../../crates/yune-core/src/engine.rs)) to get the engine `Candidate`s (which carry `CandidateSource`). **Align them to the current page** (`page_no * page_size + index` against the `RimeContext` candidates already in the JSON) and attach an explicit `source` field per row (`"ai:local"` for `Ai`, omitted/`null` for classic). **Do not** modify `RimeCandidate` or `copy_candidate`, and do **not** encode the label in `comment`.
- [x] Extend `TypeDuckCandidate` (`packages/yune-typeduck-runtime/src/response.ts:3`) with optional `source?: string`; propagate through `translateResponse` (`third_party/typeduck-web/yune-integration/response.ts:39`) into `RimeResult.candidates`.
- [x] Render AI rows distinctly in `Candidate.tsx`; handle missing `source` (classic rows) gracefully.

**Acceptance:** AI candidates carry `source: "ai:local"` end-to-end aligned to the right page row and render with a visible marker; classic rows are unchanged; the `RimeCandidate` ABI and `process_key` are untouched.

---

## Task 3 — Default-off opt-in toggle (roadmap item 1)

**Files:** `types.ts` (`RimePreferences`), `adapter.ts` (`customize`), runtime wrapper (`setAiEnabled`), `hooks.ts`, `App.tsx`, `Preferences.tsx`, `worker.ts`.

**Steps:**

- [x] Add `enableAI: boolean` to `RimePreferences`; **default `false`** in `usePreferences()` (`hooks.ts:69`).
- [x] Add `setAiEnabled(enabled)` + `stageAi()` to the runtime wrapper, calling the two new exports.
- [x] In `customize()` (`adapter.ts:405`), map `enableAI` → `runtime.setAiEnabled(...)` (a runtime flag, **not** a deployed `customize` config key).
- [x] **Deliver the second pass as a separate serialized worker action** (the worker protocol resolves one result per action). Register a new `stageAi` action in `rime.ts` `allActions` and the `worker.ts` `actions` map ([rime.ts:72](../../../third_party/typeduck-web/source/src/rime.ts), [worker.ts:256](../../../third_party/typeduck-web/source/src/worker.ts)); it returns a `RimeResult` rendered through the **existing `handleRimeResult` path**. Per keystroke the worker calls `processKey` (render classic), then **only if `enableAI`** calls `stageAi` (render the AI-augmented update). The existing request/response **serialization** guarantees ordering — a `processKey` for the next key cannot interleave — and the engine's `matches_input` check makes a late pass safe (silently dropped if input moved on). *(A push `aiResultUpdated` listener — the worker already supports `type: "listener"` messages — is an acceptable alternative; the serialized second action is simpler.)*
- [x] Add the toggle to `Preferences.tsx` and the `App.tsx` customize effect.

**Acceptance:** fresh load shows AI off and output identical to today; toggling on makes AI rows appear as a second-pass update without a redeploy; toggling off calls `set_ai_enabled(false)`, which **clears any staged AI for the current input**, so the current composition *and* the next `process_key` are byte-identical classic and the second pass stops.

---

## Task 4 — Commit-boundary, privacy & memory (roadmap items 3-4)

**Purpose:** Prove the inherited M11 gates hold through the browser path, with the **correct** memory claim.

**Files:** `tests/typeduck_web.rs` (native), reused by Task 5 (browser).

**Steps:**

- [x] **No auto-commit:** native test — with AI on, Space/Return/default-confirm commits the **classic** top candidate, never an AI row (inherited from [engine.rs:845](../../../crates/yune-core/src/engine.rs)); committing an AI row requires explicit numeric/click selection.
- [x] **No userdb leak:** native test — committing an AI candidate leaves the librime userdb unchanged.
- [x] **Privacy + suppressed learning (the corrected claim):** native test — the WASM path's `privacy_class` is `Sensitive`; a `Remote` provider would be blocked before `provide()`, and **AI-memory learning is suppressed** (`allows_learning == false`). So in the default browser context an AI commit **neither writes the userdb nor records to `MemoryStore`** — it is purely a UI selection. (Reload-surviving AI memory is a deferred non-goal pending a non-sensitive host-context opt-in.)

**Acceptance:** all three native assertions pass; classic learning on the shared path is unchanged.

---

## Task 5 — Browser-E2E safety evidence (roadmap item 5)

**Purpose:** Prove the gates in a **real browser**, reusing the HR-5 Playwright harness, with an input that actually yields AI rows on `jyut6ping3_mobile`.

**Files:** `third_party/typeduck-web/e2e/yune-typeduck.spec.ts`.

**Steps (add scenarios to the existing `test.describe` suite, [spec:95](../../../third_party/typeduck-web/e2e/yune-typeduck.spec.ts)):**

- [x] Use the Task 1 jyutping input that triggers the deterministic AI rule, and confirm the AI row lands on a **visible, selectable page**.
- [x] **AI-off byte-identity:** with AI off, drive the input; assert the candidate panel state equals the HR-5 baseline for that input.
- [x] **AI-on labeled second pass:** toggle AI on (Task 1 runtime export, no reload), type the input; assert AI rows appear **after the classic top candidate** as the second-pass update, carry the `source` marker, and **`candidates[0]` equals the AI-off `candidates[0]`** (classic index-0 preserved).
- [x] **No auto-commit:** with AI on, Space-commit → committed text is the classic top, not an AI row.
- [x] **Explicit AI selection:** click/select the AI row → it commits and is `source`-labeled.
- [x] Capture screenshots + state JSON (record the AI-mode flag in each snapshot) and PASS/FAIL to `browser-run.log`, per the existing harness helpers.

**Acceptance:** all scenarios pass with zero console warning/error entries; the runtime toggle lets AI-off/AI-on be compared without a page reload.

---

## Task 6 — Docs, verification, close-out

**Files:** `docs/{roadmap.md,requirements.md,decisions.md,CONVENTIONS.md}`, this plan.

**Steps:**

- [x] Flip roadmap M13 items 0-5 to `Done` **only** as each acceptance lands; add `M13-*` requirement IDs mirroring the gates; add `decisions.md` `D-26` recording the **two-pass in-Rust** exposure model, the sensitive-default/suppressed-learning stance, and default-off/local-first; update CONVENTIONS if the fixture/test workflow changes.
- [x] Run the full gate (each command separately):

```powershell
cargo fmt
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix packages/yune-typeduck-runtime test
npm --prefix packages/yune-typeduck-runtime run build
git diff --check
```

- [x] **Run the browser E2E (the core M13 proof — not optional):** follow `third_party/typeduck-web/e2e/yune-browser-smoke.md` to install deps, build/serve the patched app and worker, then run Playwright against the AI scenarios, e.g.:

```powershell
npm --prefix third_party/typeduck-web/e2e install
# build + serve the patched TypeDuck-Web app + worker per yune-browser-smoke.md, then:
npx --prefix third_party/typeduck-web/e2e playwright test yune-typeduck.spec.ts
```

  Completion **requires** committed real-browser evidence (screenshots + state JSON + `browser-run.log`) for the AI scenarios; a green written gate without this does not close M13.
- [x] Stage explicit paths only; commit M13 as its **own scoped commit**, then archive this plan in a separate docs commit.

---

## Completion criteria

- **AI-off output is byte-identical to today** in the browser (hard gate), proven by an E2E scenario.
- AI candidates render, are `source`-labeled, appear **after** classic as a **second-pass update**, and **never occupy index 0**.
- Classic input is **never delayed** by AI (returned first); AI's bounded second-pass runs on the off-main-thread worker. The "non-blocking" claim is stated at this granularity, not as zero-added-latency for the AI path.
- AI candidates **never auto-commit**; default/Space/Return always commits classic; explicit selection commits an AI row.
- An AI commit **never touches the librime userdb**; under the sensitive browser default, **AI-memory learning is suppressed** (no `MemoryStore` write); only `Local`/`Mock` run (no remote).
- The provider runs **in Rust** (two-pass `stage_ai`); the JS side carries no provider logic; `process_key` and `RimeCandidate` are unchanged.
- Native `typeduck_web` tests, `cargo test --workspace`, clippy `-D warnings`, the TS runtime test/build, **and a real-browser Playwright E2E** with zero warning/error entries all pass.

## Review checklist

- [x] Provider runs in Rust via a **second-pass `stage_ai`**, never inside `process_key` and never in JS.
- [x] AI-off path is the unchanged shipping `process_key`; byte-identity proven.
- [x] Source labels come from `engine.snapshot()` aligned to the page; `RimeCandidate` ABI untouched.
- [x] Memory claim is correct: no userdb leak **and** suppressed learning under sensitive default (no MemoryStore write).
- [x] Classic index-0, no-auto-commit, privacy gates asserted in **both** native and browser tests.
- [x] E2E uses a jyutping input that actually yields a selectable AI row; the verification gate includes the Playwright run.
- [x] Default-off shipped; remote impossible by policy; M13 lands as a scoped commit and the plan is archived after the gate passes.
