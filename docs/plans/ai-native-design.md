# AI-Native Input Layer — Design

> **Status:** Active · **Milestone:** M11 (AI-native input layer) · **Updated:** 2026-06-18 · **Type:** design

> **Audience.** Yune maintainers + an executing agent. North-star architecture for
> the AI-native layer; the completed first executable slice is in
> [`ai-native-cli-slice-plan.md`](./ai-native-cli-slice-plan.md). **Revised after a
> judge-panel design review** (3 alternative architectures + an adversarial
> invariant critic): the review confirmed the scope but tightened several
> invariants from "by convention" to "structural" and surfaced a real userdb-leak
> hazard — folded in below. Contracts are the durable part; specifics will evolve.
>
> **Why now.** AI-native is a *separate layer above the compatibility foundation*
> (`decisions.md`). The first slice rides the direct `yune-cli run` path + the
> finished engine (M0–M8), not the TypeDuck-Web/WASM seam, the TypeDuck-Windows
> ABI/package surface, or the `yune-rime-api` frontend lifecycle. That boundary is
> what makes it parallel-safe with M9/M10.

## 1. Goal & scope
Make AI/LLM assistance a first-class **source of candidates, ranking, context, and
memory** — *without* making classic RIME input slower, less predictable, or
network-dependent. Covers **AI-01…AI-07** (`requirements.md`). Out of scope for now:
production remote-LLM, a GUI, and exposing AI through native frontends (AI stays off
by default there until proven in the CLI — AI-07).

## 2. Non-negotiable invariants (enforced structurally, not by convention)
1. **Never block or slow classic input.** The synchronous per-key path (`Engine::refresh_candidates`, engine.rs) must **never invoke provider/model code**. It only reads an already-staged, **input-keyed** result from engine state via a non-blocking check; if no result for the *current* input is staged, it uses classic order. S1's direct CLI provider call is replaced in S2 by `AiWorker`: the direct CLI submits context snapshots and polls/receives worker results outside the engine hot path.
2. **Classic candidates always available** when AI is disabled, pending, or failed.
3. **AI candidates are source-labeled and never auto-commit by default — enforced at the *commit boundary*.** `commit_candidate`/`commit_highlighted` must receive a commit intent such as `CommitIntent::{DefaultConfirm, ExplicitSelection}`. An `Ai` candidate is rejected for `DefaultConfirm` (Space/Return/default highlighted commit) and is only commit-capable through an explicit user selection/navigation path (and even then, see #7).
4. **Local-first.** Baseline works with mock/local providers; remote LLM is opt-in, never required.
5. **Deterministic fallback & observability.** The fallback decision recorded per key event is a **discrete, input-derived enum** (`ready | pending | off`) — computed from whether a result for the current input was staged *before* this event — never from elapsed wall-clock time. Tests use pure mock providers.
6. **Privacy is opt-in, inspectable, clearable.** Sensitive contexts disable learning *and* remote calls; default to **sensitive** when unknown.
7. **AI memory is separate from the librime userdb.** A committed `Ai` candidate must **not** stage librime userdb learning; classic dictionary/userdb compatibility (measured vs librime) must be untouched by AI.

## 3. What already exists (build on, don't replace)
Pipeline (`crates/yune-core/src/engine.rs`): translate → userdb extend → sort by `quality` → `CandidateFilter`s → `CandidateRanker`s → set `context.candidates` (engine.rs). Seams: `trait Translator`; `trait CandidateRanker { try_rerank(&Context,&[Candidate]) -> RerankResult }` with `RerankResult::{Pending,Ready}` (`Pending` = classic order preserved); `Engine::add_ranker`; `Candidate { text, comment, source: CandidateSource, quality }`, `CandidateSource::Ai { provider, confidence }` with fixed-point `AiConfidence` so `Eq` remains valid. Commit path: `commit_candidate` stages `pending_userdb_learning`; the host drains it to a `*.userdb` file (`session.rs`). `UserDbCommitMetadata` already carries `candidate_source` (userdb.rs); `BackdatedScanPolicy { scans_ai_ranker_memory: false }` (userdb.rs) already anticipates AI memory being excluded from userdb scans.

## 4. Architecture

### 4.1 Candidate provision — `AiCandidateProvider` (AI-01, AI-03)
Owned module `crates/yune-core/src/ai/`. `trait AiCandidateProvider { fn name(&self) -> &'static str; fn provide(&self, ctx: &Context, budget: Duration) -> AiResult }` with input-keyed `enum AiResult { Pending { for_input: String }, Ready { for_input: String, candidates: Vec<Candidate> } }`.

Provider execution is **host/orchestrator-owned**, not engine-refresh-owned:
- **S1:** the direct CLI `run` path called the pure `MockAiProvider` outside the engine hot path and staged the returned `Ready { for_input, candidates }` through `Engine::stage_ai_result`.
- **S2:** the direct CLI now uses `AiWorker` to run the provider on a background thread; the key path only polls ready results and submits snapshots.

The engine stores staged results, not provider trait objects. **Off unless a result is staged** → AI-off path byte-identical to today.
- **Source label is structured after S2.** `CandidateSource::Ai { provider, confidence }` carries a provider string plus fixed-point `AiConfidence`, avoiding the `f32`/`Eq` trap while giving the merge policy metadata to consume. The CLI still serializes `source: "ai"` and keeps the human-readable label in `Candidate.comment`.
- AI candidates are appended **after** classic candidates and are never auto-committed.

### 4.2 Non-blocking ranking, merge, and the async model (AI-02)
- **Staged-result model:** AI candidate results are always keyed by a **snapshot of the full current input** (`composition.input`, plus ideally caret/segment). A result must **carry the input it was computed for** (`Ready { for_input, candidates }`) and the engine applies it **only if it matches the current input** — so a stale result can never apply mid-render.
- **Async model (S2 implemented):** a single `AiWorker` owns provider/model work and returns input-keyed results; the CLI key path only polls completed results and stages them.
- **One deterministic merge function is the SOLE writer of `context.candidates` ordering.** It takes classic + AI inputs, **pins the top classic candidate at index 0**, and orders AI candidates by confidence after classic candidates unless a future explicit config opts in to AI preemption. This replaces letting a `CandidateRanker` return a full-replacement vector with total control of ordering.
- Budget/timeout is deterministic: the worker is the only slow thing and the engine never awaits it → slow/failed ⇒ classic order, zero added key-event latency.

### 4.3 Context & privacy (AI-04, AI-06)
Today `Context` (state.rs) has **no** field/app/sensitivity data and no verdict type, so AI-04/06 **cannot be enforced until S3 adds a `privacy_class` to `Context`** (or the `ContextProvider` output). Both the memory-write path and any remote call must read it; default to **sensitive** on absence. S3 sequences privacy *before* any remote backend.

### 4.4 Memory & personalization (AI-05)
`MemoryStore` in `crates/yune-core/src/ai/` — vocab/phrase/style, inspectable/clearable/disable-able, **separate type** from `UserDb`. **Enforcement point (implemented in S1):** gate `pending_userdb_learning` on source in `commit_candidate` — when `candidate.source.is_ai()`, do **not** stage librime userdb learning (route to `MemoryStore` in S4). `MemoryStore` persistence (when added) uses its **own file namespace**, never the `*.userdb`/`*.userdb.txt` paths compatibility tests measure.

### 4.5 Provider backends
`mock` (deterministic; tests + CLI demos) → `local-model` (on-device) → optional `remote`. Backend-agnostic trait; baseline ships mock+local.

## 5. Observability & CLI playground (AI-07)
CLI flags on the direct core runner enable a mock provider per run (`yune-cli run --ai-provider mock|none`, default `none`; local provider remains S5). The ABI-backed `frontend` command remains AI-free so M9/M10 validation surfaces do not change. The transcript records AI source labels and the **discrete `ai_decision` (`ready|pending|off`)** (§2.5) — observable and diffable in the CLI before any native frontend depends on it.

## 6. Phasing (slices → requirements)
- **S1 — provider interface + mock in direct CLI `run`** (AI-01/03/07) **implemented 2026-06-18**, including the three cheap, safety-critical enforcement fixes promoted from later slices: (a) source-gate userdb learning on `Ai` at the commit boundary; (b) commit intent that blocks `Ai` default auto-commit while allowing explicit selection; (c) the single deterministic merge function pinning the top classic candidate. Mock is pure/synchronous (no thread, no clock) and is called outside `Engine::refresh_candidates`. *(See the slice plan.)*
- **S2 — async budget worker + input-keyed results + merge policy that consumes confidence** (AI-02) **implemented 2026-06-18**: `AiWorker`, keyed pending/ready results, fixed-point `AiConfidence`, and confidence-ordered AI rows after classic candidates.
- **S3 — `ContextProvider` + privacy classifier** (AI-04/06), before any remote backend.
- **S4 — `MemoryStore`** (AI-05), provably outside the userdb namespace.
- **S5 — local-model backend**; remote stays optional/later.

## 6.1 Implementation Evidence
- **S1 evidence:** `cargo test -p yune-core`, `cargo test -p yune-cli`,
  `cargo run -q -p yune-cli -- run --ai-provider mock nihao`, and
  `cargo run -q -p yune-cli -- run nihao` prove the direct CLI mock path,
  source labeling, no-default-AI-commit, and userdb isolation.
- **S2 evidence:** `cargo test -p yune-core` covers `AiWorker`, keyed pending
  results, confidence-bearing `CandidateSource::Ai`, stale-result rejection, and
  confidence-ordered AI rows after classic rows. `cargo test -p yune-cli` and
  the same CLI proof commands show the direct runner now uses the worker while
  preserving default classic output and mock `ai_decision` output.

## 7. Risks / open questions
- **A non-conforming provider can still block its host/orchestrator** — S1 keeps provider calls out of `Engine::refresh_candidates`; later hosts must preserve that boundary. The engine-side contract is only "consume staged input-keyed results".
- **Worker lifecycle on a single-shot CLI run** can make transcripts vary; S1's pure-synchronous mock avoids this (no worker yet).
- **Stale-result correctness depends on keying by the FULL input** (+ ideally caret/segment); a coarse session key would let a stale result apply.
- **Tail-appended AI rows interact with filters that run after the append and with paging** (`DEFAULT_PAGE_SIZE`); a filter could drop/reorder them — validate.
- **Privacy false-negatives are the highest-severity S3+ risk** — default sensitive; fail closed.
- **The commit-boundary source gate touches the hot path shared with classic input** — guard it so classic learning is unaffected (test both paths).
- **`MemoryStore` persistence format/location is unspecified** — must provably never use the `*.userdb` namespace.

## 8. Safety invariants the test suite must enforce
- AI-off (or `pending`/failed) yields **byte-identical** classic output to today.
- Committing an `Ai` candidate leaves `userdb().entries()` unchanged **and** `take_pending_userdb_learning()` returns `None` (the §2.7 / §4.4 gate).
- No `Ai` candidate auto-commits; `CommitIntent::DefaultConfirm` rejects AI candidates and the default/space-committed candidate is always classic.
- The merge function pins the top classic candidate at index 0 (no AI preemption unless explicitly configured).
- The recorded `ai_decision` is derived from current input, not wall-clock.
- A provider exceeding its budget never delays a key event; sensitive context (S3+) ⇒ no remote call, no memory write (assert with a recording mock).
- `MemoryStore` writes never touch `*.userdb` files.

---

*Draft created 2026-06-18; revised the same day after a judge-panel design review (verdict: sound, needs targeted edits — folded in). Builds on the M4 `CandidateRanker` hook; gated by the AI-native standing principles in [`../decisions.md`](../decisions.md) and AI-01…AI-07 in [`../requirements.md`](../requirements.md).*
