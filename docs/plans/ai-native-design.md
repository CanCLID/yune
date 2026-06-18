# AI-Native Input Layer — Design

> **Status:** Active · **Milestone:** M11 (AI-native input layer) · **Updated:** 2026-06-18 · **Type:** design

> **Audience.** Yune maintainers + an executing agent. North-star architecture for
> the AI-native layer; the first executable slice is in
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
1. **Never block or slow classic input.** The synchronous per-key path (`Engine::refresh_candidates`, engine.rs) must **never invoke provider/model code**. It only reads an already-staged, **input-keyed** result from engine state via a non-blocking check; if no result for the *current* input is staged, it uses classic order. S1's mock provider is called by the direct CLI harness outside this hot path, then the ready result is staged into the engine; S2 replaces the CLI call with a background worker/cache.
2. **Classic candidates always available** when AI is disabled, pending, or failed.
3. **AI candidates are source-labeled and never auto-commit by default — enforced at the *commit boundary*.** `commit_candidate`/`commit_highlighted` must receive a commit intent such as `CommitIntent::{DefaultConfirm, ExplicitSelection}`. An `Ai` candidate is rejected for `DefaultConfirm` (Space/Return/default highlighted commit) and is only commit-capable through an explicit user selection/navigation path (and even then, see #7).
4. **Local-first.** Baseline works with mock/local providers; remote LLM is opt-in, never required.
5. **Deterministic fallback & observability.** The fallback decision recorded per key event is a **discrete, input-derived enum** (`ready | pending | off`) — computed from whether a result for the current input was staged *before* this event — never from elapsed wall-clock time. Tests use pure mock providers.
6. **Privacy is opt-in, inspectable, clearable.** Sensitive contexts disable learning *and* remote calls; default to **sensitive** when unknown.
7. **AI memory is separate from the librime userdb.** A committed `Ai` candidate must **not** stage librime userdb learning; classic dictionary/userdb compatibility (measured vs librime) must be untouched by AI.

## 3. What already exists (build on, don't replace)
Pipeline (`crates/yune-core/src/engine.rs`): translate → userdb extend → sort by `quality` → `CandidateFilter`s → `CandidateRanker`s → set `context.candidates` (engine.rs:805–826). Seams: `trait Translator`; `trait CandidateRanker { try_rerank(&Context,&[Candidate]) -> RerankResult }` with `RerankResult::{Pending,Ready}` (lib.rs:75/105, `Pending` = classic order preserved); `Engine::add_ranker` (engine.rs:95); `Candidate { text, comment, source: CandidateSource, quality }`, `enum CandidateSource` (state.rs — `Ai` already exists as a **unit** variant). Commit path: `commit_candidate` (engine.rs:740–762) stages `pending_userdb_learning`; the host drains it to a `*.userdb` file (`session.rs`). `UserDbCommitMetadata` already carries `candidate_source` (userdb.rs:11); `BackdatedScanPolicy { scans_ai_ranker_memory: false }` (userdb.rs) already anticipates AI memory being excluded from userdb scans.

## 4. Architecture

### 4.1 Candidate provision — `AiCandidateProvider` (AI-01, AI-03)
New owned module `crates/yune-core/src/ai/`. `trait AiCandidateProvider { fn name(&self) -> &'static str; fn provide(&self, ctx: &Context, budget: Duration) -> AiResult }` with `enum AiResult { Pending, Ready { for_input: String, candidates: Vec<Candidate> } }`.

Provider execution is **host/orchestrator-owned**, not engine-refresh-owned:
- **S1:** the direct CLI `run` path calls the pure `MockAiProvider` outside the engine hot path and stages the returned `Ready { for_input, candidates }` through an engine method such as `stage_ai_result`.
- **S2+:** a background worker/cache replaces the CLI call, but the engine still only consumes staged input-keyed results.

The engine stores staged results, not provider trait objects. **Off unless a result is staged** → AI-off path byte-identical to today.
- **Source label stays a UNIT variant.** Keep `CandidateSource::Ai` as the existing unit variant — a `{ provider, confidence: f32 }` struct **breaks `#[derive(Eq)]`** on `CandidateSource` (f32 isn't `Eq`) and ripples into `UserDbCommitMetadata`/`assert_eq!` sites. Carry `provider`/`confidence` in `Candidate.comment` for now (already surfaced by CLI render/transcript). Promote to a struct variant only in S2, when a merge policy actually consumes confidence.
- AI candidates are appended **after** classic candidates and are never auto-committed.

### 4.2 Non-blocking ranking, merge, and the async model (AI-02)
- **Staged-result model:** AI candidate results are always keyed by a **snapshot of the full current input** (`composition.input`, plus ideally caret/segment). A result must **carry the input it was computed for** (`Ready { for_input, candidates }`) and the engine applies it **only if it matches the current input** — so a stale result can never apply mid-render.
- **Async model (S2):** a single background worker owns all model/I/O and writes to the staged-result cache; the key thread only performs the same non-blocking staged-result read introduced in S1.
- **One deterministic merge function is the SOLE writer of `context.candidates` ordering.** It takes classic + AI inputs and **pins the top classic candidate at index 0** unless an explicit config opts in to AI preemption. This replaces letting a `CandidateRanker` return a full-replacement vector with total control of ordering (today's `Ready(..)` does exactly that — a hazard for "don't starve the top classic candidate").
- Budget/timeout is deterministic: the worker is the only slow thing and the engine never awaits it → slow/failed ⇒ classic order, zero added key-event latency.

### 4.3 Context & privacy (AI-04, AI-06)
Today `Context` (state.rs) has **no** field/app/sensitivity data and no verdict type, so AI-04/06 **cannot be enforced until S3 adds a `privacy_class` to `Context`** (or the `ContextProvider` output). Both the memory-write path and any remote call must read it; default to **sensitive** on absence. S3 sequences privacy *before* any remote backend.

### 4.4 Memory & personalization (AI-05)
`MemoryStore` in `crates/yune-core/src/ai/` — vocab/phrase/style, inspectable/clearable/disable-able, **separate type** from `UserDb`. **Enforcement point (required in S1):** gate `pending_userdb_learning` on source in `commit_candidate` — when `candidate.source == CandidateSource::Ai`, do **not** stage librime userdb learning (route it to `MemoryStore` instead). `MemoryStore` persistence (when added) uses its **own file namespace**, never the `*.userdb`/`*.userdb.txt` paths compatibility tests measure.

### 4.5 Provider backends
`mock` (deterministic; tests + CLI demos) → `local-model` (on-device) → optional `remote`. Backend-agnostic trait; baseline ships mock+local.

## 5. Observability & CLI playground (AI-07)
CLI flags on the direct core runner enable a mock/local provider per run (`yune-cli run --ai-provider mock|local|none`, default `none`). The ABI-backed `frontend` command remains AI-free in S1 so M9/M10 validation surfaces do not change. The transcript records, per key event: AI source labels, the **discrete `ai_decision` (`ready|pending|off`)** (§2.5), and the merge result — observable and diffable in the CLI before any native frontend depends on it.

## 6. Phasing (slices → requirements)
- **S1 — provider interface + mock in direct CLI `run`** (AI-01/03/07) **implemented 2026-06-18**, including the three cheap, safety-critical enforcement fixes promoted from later slices: (a) source-gate userdb learning on `Ai` at the commit boundary; (b) commit intent that blocks `Ai` default auto-commit while allowing explicit selection; (c) the single deterministic merge function pinning the top classic candidate. Mock is pure/synchronous (no thread, no clock) and is called outside `Engine::refresh_candidates`. *(See the slice plan.)*
- **S2 — async budget worker + input-keyed results + merge policy that consumes confidence** (AI-02); promote `CandidateSource::Ai` to a struct variant here.
- **S3 — `ContextProvider` + privacy classifier** (AI-04/06), before any remote backend.
- **S4 — `MemoryStore`** (AI-05), provably outside the userdb namespace.
- **S5 — local-model backend**; remote stays optional/later.

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
