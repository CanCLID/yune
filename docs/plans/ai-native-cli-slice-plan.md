# AI-Native Slice S1 — Provider Interface + Mock in the CLI

> **Status:** Active · **Milestone:** M11 (AI-native input layer) · **Updated:** 2026-06-18 · **Type:** execution plan

> **Audience.** An executing agent. Slice **S1** of the AI-native layer
> ([design](./ai-native-design.md)): the minimal, source-labeled, non-blocking,
> off-by-default AI candidate path in the CLI. Delivers **AI-01, AI-03, AI-07** —
> **plus three cheap, safety-critical enforcement fixes the design review promoted
> into S1** (userdb-leak gate, commit-boundary no-auto-commit, single merge
> function). Revised after the judge-panel review.
>
> **Parallel-safe.** S1 touches **only `crates/yune-core` and the direct
> `yune-cli run` path** — not `yune-rime-api`, not the ABI-backed
> `yune-cli frontend` command, not `yune-rime-api/src/typeduck_web.rs`, and not
> any browser/Windows path. That is why it runs alongside M9/M10 without file
> conflicts. Stage by path on the shared checkout.
>
> **Post-S2 note.** This plan records the completed S1 slice. S1 intentionally
> kept `CandidateSource::Ai` as a unit variant; S2 later promoted it to
> `{ provider, confidence }` using fixed-point `AiConfidence` once the merge
> policy consumed confidence.

## Goal
A direct core CLI run can enable a **mock AI provider** contributing a **source-labeled**,
**non-blocking** AI candidate that **never auto-commits** and — critically —
**never writes the librime userdb**, while a default run stays **byte-identical**
to today. Proves the layer's safety contracts before any ABI, browser, Windows,
or native frontend path depends on it.

## Build on what exists (do NOT fork)
Pipeline `crates/yune-core/src/engine.rs`: translate → userdb extend → sort by `quality` → filters → rankers → set candidates (engine.rs:805–826). `CandidateRanker`/`RerankResult::{Pending,Ready}` (lib.rs:75/105) is the existing non-blocking primitive, but S1 must not route provider execution through rankers or `refresh_candidates`. `Candidate { text, comment, source: CandidateSource, quality }`; `CandidateSource::Ai` already exists as a **unit** variant (state.rs). **Commit/userdb path:** `commit_candidate` (engine.rs:740–762) stages `pending_userdb_learning` for *every* committed candidate; the host drains it to a `*.userdb` file (`session.rs`). `UserDbCommitMetadata.candidate_source` (userdb.rs:11) already exists; `BackdatedScanPolicy { scans_ai_ranker_memory: false }` already anticipates this gate. The direct CLI path is `crates/yune-cli/src/sample_core.rs`; the ABI-backed `crates/yune-cli/src/rime_frontend.rs` stays untouched in S1.

## Work items (one commit each)

### WI-1 — Source labeling (keep `Ai` a UNIT variant)
- Keep `CandidateSource::Ai` as the existing **unit** variant — do **not** add `{ provider, confidence }`: an `f32` field breaks `#[derive(Eq)]` on `CandidateSource` and ripples into `UserDbCommitMetadata`/`assert_eq!` sites. Carry `provider`/`confidence` in `Candidate.comment` (e.g. `"ai:mock 0.62"`).
- Surface the label in the direct core JSON transcript (`crates/yune-cli/src/transcript.rs`, via `FixtureOutput`). The ABI frontend human renderer can stay unchanged because S1 does not expose AI through `frontend`.
- **Acceptance:** build stays warning-clean; an AI candidate serializes with `source: "ai"` and a clear comment label; existing tests pass.

### WI-2 — `AiCandidateProvider` + mock + staged-result merge
- Owned module `crates/yune-core/src/ai/mod.rs`: `trait AiCandidateProvider { fn name(&self)->&'static str; fn provide(&self, ctx:&Context, budget:Duration)->AiResult }`, `enum AiResult { Pending, Ready { for_input: String, candidates: Vec<Candidate> } }`. `MockAiProvider` returns one deterministic source-labeled suggestion or `Pending` from a fixed input→suggestion map — **pure/synchronous** (no thread, no clock).
- **Do not install providers into `Engine`.** Provider execution is orchestrated by `crates/yune-cli/src/sample_core.rs` outside the engine hot path. Add engine-side staged-result storage/API, for example `Engine::stage_ai_result(AiResult)`, that only records a ready input-keyed result and refreshes candidates from that staged data.
- Add **one deterministic merge function** that is the **sole writer** of final `context.candidates` ordering: classic candidates first (preserving their order/indices), matching-input AI candidates appended after, and the **top classic candidate pinned at index 0** (no AI preemption in S1). Do *not* let a ranker full-replace ordering.
- **Acceptance:** no staged AI result ⇒ candidate output **byte-identical** to baseline; a stale `Ready { for_input, ... }` for a different input changes nothing; with a matching mock result, a labeled AI row appears **after** classic candidates and index 0 stays classic; `Pending` changes nothing.

### WI-3 — Commit-boundary safety (no auto-commit + userdb-leak gate)  ← highest priority
- **No auto-commit:** add an internal commit intent such as `CommitIntent::{DefaultConfirm, ExplicitSelection}`. `commit_highlighted` and Space/Return/default confirm paths use `DefaultConfirm`; direct candidate-selection paths use `ExplicitSelection`. In `commit_candidate` (engine.rs:740–762), reject `CandidateSource::Ai` when the intent is `DefaultConfirm`, so an AI candidate is never the default/space-committed selection. Explicit selection may commit AI, subject to the userdb gate below.
- **userdb-leak gate (the §2.7 invariant):** when the committed `candidate.source == CandidateSource::Ai`, **do not** stage `pending_userdb_learning` (route to the future `MemoryStore` instead). Guard it so **classic** learning on the shared hot path is unaffected.
- **Acceptance:** committing an `Ai` candidate leaves `userdb().entries()` unchanged **and** `take_pending_userdb_learning()` returns `None`; committing a **classic** candidate still stages learning exactly as before.

### WI-4 — CLI flag + observability (AI-07)
- Add `--ai-provider mock|none` (default `none`) to the direct `run` command only. Leave `frontend` / `frontend-check` flags unchanged in S1.
- `mock` causes `sample_core.rs` to call `MockAiProvider` outside `Engine::refresh_candidates`, then stage the returned input-keyed result into the engine.
- Transcript records, per key event or final direct-run snapshot, the AI source label(s) and a **discrete, input-derived** `ai_decision` enum (`ready | pending | off`) — computed from whether a result for the *current* input was staged, **never** from wall-clock time (keeps the transcript deterministic, consistent with the no-timestamp contract).
- **Acceptance:** `cargo run -p yune-cli -- run --ai-provider mock …` shows the labeled AI row + the `ai_decision`; default `cargo run -p yune-cli -- run …` output is unchanged.

### WI-5 — Safety tests (enforce the invariants)
Owned tests in `yune-core` + `yune-cli`:
- AI-off runs keep fixture JSON stable; `Pending`-provider runs keep classic
  candidates/context stable while still recording the deterministic
  `ai_decision: "pending"` diagnostic.
- A stale staged result keyed to a different input is ignored.
- AI candidate present + source-labeled when mock active; **index 0 stays classic**.
- **No `Ai` auto-commit**; `CommitIntent::DefaultConfirm` rejects AI and default/space commit is always classic.
- **userdb isolation:** after an `Ai` commit, `userdb().entries()` unchanged + `take_pending_userdb_learning()` is `None`; after a classic commit, learning is staged as before.
- Deterministic (mock is pure; assertions are value-based, not timing-based).

## Out of scope for S1 (later slices)
Async background worker + input-keyed results + time budget and the `CandidateSource::Ai` struct variant (**S2**); `ContextProvider` + privacy classifier (**S3**); persisted `MemoryStore` (**S4**); local-model/remote backends (**S5**).

## Quality gate
`cargo fmt` · `cargo clippy --workspace --all-targets -- -D warnings` · focused `yune-core` + `yune-cli` tests · `cargo test --workspace`. Per CONVENTIONS: own each slice (new behavior in `crates/yune-core/src/ai/` + matching tests; keep `lib.rs`/`main.rs` facades). Do not touch `yune-rime-api`, M9 TypeDuck-Web files, or Windows packaging files in S1; stage by path.

## Implementation evidence

S1 is implemented in `crates/yune-core` and the direct `yune-cli run` path only.
The focused evidence is:

- `cargo test -p yune-core` — green, including staged-result merge, stale/pending
  result ignore, no-default-AI-commit, and userdb-isolation tests.
- `cargo test -p yune-cli` — green, including `run --ai-provider mock`, default
  fixture stability, pending-provider candidate stability, and transcript
  `ai_decision` coverage.
- `cargo run -q -p yune-cli -- run --ai-provider mock nihao` — emits classic
  `你好` first, keeps echo second, appends `你好呀` with `source: "ai"` and
  `comment: "ai:mock 0.62"`, and records `"ai_decision": "ready"`.
- `cargo run -q -p yune-cli -- run nihao` — stays classic-only and omits
  `ai_decision`.

## Checklist
- [x] WI-1 — `CandidateSource::Ai` kept unit; provider/confidence in `comment`; CLI labeling
- [x] WI-2 — `AiCandidateProvider` + `MockAiProvider` (pure, CLI-orchestrated) + staged-result merge pinning index 0
- [x] WI-3 — commit-intent no-auto-commit **and** userdb-leak gate (no `pending_userdb_learning` for `Ai`)
- [x] WI-4 — direct `run --ai-provider` flag + discrete `ai_decision` transcript field
- [x] WI-5 — safety tests (AI-off fixture stability, pending-provider candidate stability, stale-result ignored, labeled, index-0-classic, no-auto-commit, **userdb isolation**, deterministic)

---

*Draft 2026-06-18, revised after the judge-panel design review. First slice of the AI-native layer ([design](./ai-native-design.md)); delivers AI-01/03/07 + the promoted safety gates; parallel-safe with M9/M10.*
