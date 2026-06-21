# M17 — Upstream Language Model (Poet) Implementation Plan

> **Status:** Finished · **Milestone:** M17 (upstream sentence/LM poet) · **Closed:** 2026-06-21 · **Type:** execution plan
> **For agentic workers:** implement task-by-task; steps use checkbox (`- [ ]`) syntax.

**Goal:** Implement the upstream `rime/librime 1.17.0` statistical sentence path (`poet` + `grammar`) so that `luna_pinyin` single-best SENTENCE composition and full-page multi-sentence LATTICE output match the captured upstream oracle, and un-ignore the two blocked parity stubs in `crates/yune-core/tests/upstream_luna_pinyin_parity.rs` (`zhongguo_phrase_mechanics_parity_is_blocked` at line 107; `full_sentence_lattice_parity_for_zhongguo_is_blocked` at line 376).

**Completion note (2026-06-21):** M17 is complete. `luna-pinyin-sentence.json`
and `luna-pinyin-lattice.json` were captured from the pinned upstream 1.17.0
release binary before implementation. Yune now owns a `poet` module with the
upstream null-grammar penalty (`-13.815510557964274`) and installs the upstream
sentence model only for the `luna_pinyin` script-translator profile. The two
M17-owned `upstream_luna_pinyin_parity` blockers are active tests, TypeDuck
`jyut6ping3` sentence tuning remains profile-gated, and ABI structs/tables were
unchanged. Learned `.gram`/octagram and broader contextual translation remain
future named-target work.

**Architecture:** A new owned `poet` module in `crates/yune-core/src/` reproduces librime's `Poet::MakeSentence`/`MakeSentences` (`src/rime/gear/poet.{h,cc}`) as a log-space Viterbi (single-best, the `DynamicProgramming` strategy) + beam (multi-sentence, the `BeamSearch` strategy) DP over a dictionary `WordGraph`. The crucial grounded fact: `luna_pinyin` ships an `essay.txt` preset vocabulary but **no `.gram` grammar model**, so the oracle's `Poet` runs the `grammar_ == nullptr` branch — `MakeSentenceWithStrategy<DynamicProgramming>` — and `Grammar::Evaluate` returns `entry_weight + kPenalty` where the verbatim upstream constant is `kPenalty = -13.815510557964274` (≈ `ln(1e-6)`, often shorthand `-13.82`) — a fixed per-word log-probability penalty, *not* a learned bigram model (confirmed below). M17 owns this constant and the upstream scoring/comparator (`CompareWeight`, `LeftAssociateCompare`) behind a **named upstream profile flag** so it does **not** disturb the existing TypeDuck `sentence_candidate` heuristic (`translator/mod.rs:577-702`, `sentence_piece_quality = ln(weight) - 21.0`) calibrated to the M21 jyut6ping3 fixture.

**Tech Stack:** Rust (`yune-core`), the existing M12 oracle harness (`scripts/oracle-rime-probe.cs` driving the pinned `1.17.0` `rime.dll`), provenance-guarded JSON fixtures under `crates/yune-core/tests/fixtures/upstream-1.17.0/`, the `upstream_luna_pinyin_parity.rs` test, and `oracle_fixture_provenance.rs`.

---

## Grounded current state (cite-backed)

- **Yune's current sentence path is dictionary-driven with NO statistical LM.** `StaticTableTranslator::sentence_candidate` (`crates/yune-core/src/translator/mod.rs:577-702`) is a Viterbi DP that scores each piece with `sentence_piece_quality(q) = q.max(1.0).ln() - TYPEDUCK_SENTENCE_WORD_PENALTY` where `TYPEDUCK_SENTENCE_WORD_PENALTY = 21.0` (`translator/mod.rs:16,40-42`). The penalty is explicitly *"a Yune-internal heuristic calibrated to the M21 TypeDuck v1.1.2 sentence-composition fixture"* (`translator/mod.rs:15`) — i.e. the recent `fix(m21): close typeduck sentence composition gap` (commit `2ddac82a`). It is **fallback-only**, gated on `candidates.is_empty()` (or `sentence_over_completion`) at `translator/mod.rs:554-572`, returns a **single** `CandidateSource::Sentence` row with comment `" ☯ "` (`translator/mod.rs:695-701`), and emits no full multi-sentence page.
- **The two blocked tests are panic-stubs, not real assertions.** `zhongguo_phrase_mechanics_parity_is_blocked` (`tests/upstream_luna_pinyin_parity.rs:107-110`) panics `"add a non-circular phrase/language-model source fixture before enabling"`; `full_sentence_lattice_parity_for_zhongguo_is_blocked` (`tests/upstream_luna_pinyin_parity.rs:376-380`) panics `"capture a complete upstream language model fixture before enabling this parity test"`. The active `upstream_luna_pinyin_fixture_is_locked` test already asserts the *committed/highlighted* `zhongguo → 中國` winner (`tests/upstream_luna_pinyin_parity.rs:69-78`), and `yune_table_translator_matches_upstream_luna_pinyin_single_code_first_page` deliberately **filters out** `zhongguo` (`tests/upstream_luna_pinyin_parity.rs:88-91`) — so the multi-syllable sentence/lattice surface is exactly the M17 gap. (The other two ignored stubs — `ascii_punct...` line 382 and `punctuation_immediate_commit...` line 388 — are processor-level, **M18 scope, not M17**.)
- **The LM data is obtainable and the model is `grammar==nullptr`.** The fixtures already pin `rime/rime-essay` at `48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed` with `vocabulary: "essay.txt"` (`tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json:27,31`; `luna-pinyin-selection.json:27-35`). `luna_pinyin.schema.yaml` declares no `grammar:` and no `translator/contextual_suggestions`, so upstream `Poet` is constructed `new Poet(language(), config)` with `grammar_ == nullptr`; `Grammar::Evaluate(context, text, weight, is_rear, /*grammar=*/nullptr)` returns `entry_weight + kPenalty` (upstream `src/rime/gear/grammar.h @33e78140`). The `kPenalty = -13.815510557964274` (= `ln(1e-6)`) per-word penalty is the *entire* "language model" for luna_pinyin — there is no octagram bigram file to obtain. **`is_rear` is forwarded only to `grammar->Query(...)`, so the null-grammar value is independent of `is_rear`.**
- **Upstream scoring/structures to mirror** (`src/rime/gear/poet.{h,cc}`, `translator_commons.cc`, `vocabulary.h`):
  - `WordGraph = map<int, map<int, DictEntryList>>` (start_pos → end_pos → entries).
  - `Sentence::Extend(entry, end_pos, new_weight)` accumulates `text`, `code`, `components_`, `word_lengths_`; `weight` is set to the externally-computed `new_weight` (sum of per-word `Grammar::Evaluate`).
  - Two comparators: `CompareWeight` (weight only) and `LeftAssociateCompare` (weight, then **fewer words**, then word-length ordering).
  - `MakeSentence` = single-best DP via `MakeSentenceWithStrategy<DynamicProgramming>` when `grammar_ == nullptr` (keeps one best `Line` per end position; updates only when `compare_(best, new_line)`); `MakeSentences(graph, total_length, preceding_text, count, cutoff_threshold)` = `MakeSentenceWithStrategy<BeamSearch>`, `beam_width = max_sentences * 3`, per-position lists, `text_hash` dedup (`hash*31 + c`, keep higher weight), relative-deviation `cutoff_threshold` pruning that accelerates downward, and `kMaxLineCandidates = 7` retained per ending position.
- **The TypeDuck heuristic must not be the upstream path.** Yune's `21.0` penalty and `ln`-of-raw-weight are calibrated to jyut6ping3 (`translator/mod.rs:15`, fork-parity-ledger note 5 at line 127). Upstream essay weights are already **log-domain** values summed directly with `kPenalty` per word — a different scoring space. Sharing one code path would regress one target to fix the other; M17 keeps them separate (see Non-goals).

---

## Scope / Non-goals

**In scope**
- A new owned `poet` module reproducing upstream `MakeSentence` (single-best `DynamicProgramming`) and `MakeSentences` (multi-sentence `BeamSearch`) with the `grammar==nullptr` scoring: `sum(entry.weight) + kPenalty * word_count`, plus `CompareWeight` and `LeftAssociateCompare` comparators.
- A `Grammar` trait + the null-grammar `Evaluate` constant, structured so a future `.gram`/octagram model can plug in without rework (interface only; no model implementation).
- Non-circular capture of two new goldens from the pinned `1.17.0` binary: the `zhongguo` multi-syllable phrase/sentence surface, and a full-page sentence **lattice** (multiple sentence candidates per page).
- Integration into the sentence/lattice path **behind a named upstream-profile flag** (e.g. `with_upstream_poet(true)` / a schema-driven `grammar`/`poet` selector), leaving TypeDuck behavior byte-identical.
- Turning the two `#[ignore]` panic-stubs into real oracle assertions.

**Non-goals (explicit)**
- **octagram C++ plugin ABI** and any `.gram` bigram model loading — luna_pinyin needs none; deferred until a named target requires a learned grammar.
- **`contextual_translation` / `contextual_suggestions`** beyond satisfying the two named tests (luna_pinyin does not enable it).
- **TypeDuck `jyut6ping3` sentence composition** — dictionary-driven, NO LM; it keeps the existing `sentence_candidate` heuristic (`translator/mod.rs`) and the M21-GAP-01 fix. M17 must not touch its scoring.
- Any `RimeApi`-table / `RimeCandidate` change; the upstream-first default ABI is preserved (poet output is internal `Candidate` state only).
- Bit-for-bit librime poet internals (beam memory layout, `find_top_candidates` micro-structure) — reproduce observable ranking/text, not C++ data structures.
- The M18 processor-level stubs (`ascii_punct`, punctuation immediate-commit) — separate milestone.

> **This is the HEAVY item and is NOT required for TypeDuck-Web parity.** It advances upstream `luna_pinyin` depth only. Keep it off the TypeDuck/web critical path; do not let it block or alter M9/M13/M16/M20 web gates.

---

## Tasks

### Task 1 — Capture the two blocked goldens first (oracle-measured, non-circular)
- [ ] Confirm the schema/essay provenance the new fixtures must carry: `schema_data: rime/rime-luna-pinyin` @ `18a80335…`, `rime/rime-essay` @ `48c7538f…`, `vocabulary: essay.txt` (match the existing headers in `luna-pinyin-selection.json:23-35`).
- [ ] Extend `scripts/oracle-rime-probe.cs` (or add a luna-pinyin sentence composer) to capture, for `zhongguo` and at least 2–3 more multi-syllable inputs (e.g. `zhongguoren`, `beijing`, a 3+ word phrase), the **full first page** of candidates with `text`, `comment`, `quality` (use `RimeCandidateWithQuality`, already defined at `oracle-rime-probe.cs:40-46`), `highlighted_candidate_index`, `page_size`, `page_no`, and `is_last_page`.
- [ ] **Lock the exact upstream null-grammar constant** by reading `src/rime/gear/grammar.h @33e78140` verbatim and recording `kPenalty = -13.815510557964274` (with `is_rear` confirmed irrelevant on the null branch) into the fixture/source-row notes, so Task 2 uses the full-precision value rather than the `-13.82` shorthand.
- [ ] Write `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence.json` (single-best `zhongguo` phrase surface) and `luna-pinyin-lattice.json` (full multi-sentence page) with the standard `oracle`/`schema`/`module_list`/`capture` header so `oracle_fixture_provenance` accepts them. Carry the **complete** essay rows needed to reconstruct each candidate's weight (extend the existing essay-row policy from `luna-pinyin-selection.json`), and set an explicit `source_row_policy` documenting that this is the LM/lattice surface.
- [ ] Update `oracle-manifest.json` / the fixtures README to register the two new fixtures.
- **Acceptance:** Both fixtures exist, pass `oracle_fixture_provenance`, and reproduce the upstream `1.17.0` engine identity; bytes come from the captured oracle binary, never from Yune. A new `*_fixture_is_locked`-style test asserts the headers and that `zhongguo`'s captured highlighted winner is `中國` (consistent with `upstream_luna_pinyin_parity.rs:74-77`).

### Task 2 — Owned `poet` module: structures + null-grammar scoring
- [ ] Add `crates/yune-core/src/poet/mod.rs` with: a `WordGraph` (start→end→`Vec<WordEntry{text, code, weight}>`), a `SentencePath`/`Line` carrying `weight`, `word_count`, `text`, `text_hash`, `word_lengths`, and a `predecessor`-style backtrace; reuse `Candidate`/`CandidateSource::Sentence` for output.
- [ ] Implement the upstream null-grammar score: `evaluate(entry_weight) = entry_weight + UPSTREAM_NO_GRAMMAR_PENALTY` with `const UPSTREAM_NO_GRAMMAR_PENALTY: f64 = -13.815510557964274; // upstream grammar.h kPenalty (== ln(1e-6)), grammar-absent branch; shorthand -13.82`. Match the verbatim upstream literal — do **not** hardcode the rounded `-13.82`. Define a `Grammar` trait with `query(context, word, is_rear) -> f64` so a model can later replace the constant; provide a `NullGrammar` returning the constant (independent of `is_rear`).
- [ ] Implement `CompareWeight` (weight desc) and `LeftAssociateCompare` (weight desc, then fewer words, then word-length order) as the path-replacement comparator, matching `poet.cc`.
- **Acceptance:** Unit tests in `crates/yune-core/tests/poet_scoring.rs` lock: (a) the `-13.815510557964274` constant (assert against `ln(1e-6)` to full f64 precision) and per-word accumulation on a hand-built `WordGraph`; (b) both comparators' tie-break order. No dependency on Yune-produced expected bytes (synthetic inputs + hand-computed sums only).

### Task 3 — `MakeSentence` (single-best) + `MakeSentences` (beam lattice)
- [ ] Implement `make_sentence(word_graph, total_len, preceding_text) -> Option<SentencePath>` as the single-best DP (`DynamicProgramming` strategy: one best `Line` per end position, updated only when `compare(best, new_line)`), mirroring `poet.cc` traversal start→end over graph edges.
- [ ] Implement `make_sentences(word_graph, total_len, preceding_text, max_sentences, cutoff_threshold) -> Vec<SentencePath>`: `BeamSearch` strategy, beam width `max_sentences * 3`, per-position candidate lists capped at `kMaxLineCandidates = 7`, `text_hash` dedup (`hash*31 + c`, keep higher weight), relative-deviation `cutoff_threshold` pruning with the upstream downward-acceleration behavior, output sorted by the active comparator.
- [ ] Convert paths to `Candidate`s preserving word boundaries (so downstream preedit/segmentation stays correct), `source = CandidateSource::Sentence`.
- **Acceptance:** `poet_scoring.rs` covers a small fixed `WordGraph` where the hand-computed best path and a deterministic top-K ordering are asserted (e.g. a two-segmentation graph where the `kPenalty` per-word term flips the winner from "many short words" to "fewer longer words"). Pure-unit, non-circular.

### Task 4 — Build the `WordGraph` from the dictionary lookup over an input
- [ ] Add a builder that, given the translator's input and `entries_by_code` (`translator/mod.rs:68`), enumerates substring spans (respecting delimiters, mirroring the existing `sentence_candidate` span walk at `translator/mod.rs:600-653`) and fills `WordGraph[start][end] = entries`, using **upstream essay weights as-is** (log-domain), not the TypeDuck `ln(...)` transform.
- [ ] Honor `enable_completion` final-segment expansion only where the captured oracle does (compare against the Task 1 fixture; do not invent completion behavior).
- **Acceptance:** A unit test builds a `WordGraph` for `zhongguo` from the Task 1 fixture's source rows and asserts the expected spans/entries (e.g. `zhong|guo` and `zhongguo` edges present with their essay weights).

### Task 5 — Integrate behind a named upstream profile (do not disturb TypeDuck)
- [ ] Add a translator flag (e.g. `StaticTableTranslator::with_upstream_poet(bool)`, default `false`) or a `grammar`/`poet` schema selector that routes the sentence path through the new `poet` module instead of `sentence_candidate`. When off, behavior is byte-identical to today (TypeDuck/jyut6ping3 untouched).
- [ ] In the poet path, replace the single-row `☯` fallback with: single-best sentence (Task 3 `make_sentence`) for the SENTENCE surface, and a full sentence page (`make_sentences`) for the LATTICE surface, matching the Task 1 goldens' page shape/order.
- [ ] Keep the fallback **gating** semantics the oracle shows (when an exact full-input phrase exists vs. composed) — verify against the fixture, do not assume.
- **Acceptance:** With the flag **off**, the full existing `upstream_luna_pinyin_parity.rs` + `cantonese_parity.rs` suites stay green (TypeDuck `sentence_candidate`/M21-GAP-01 unchanged, confirmed by re-running). With the flag **on** for luna_pinyin, the Task 1 goldens reproduce. No `RimeApi`/`RimeCandidate` change (verify `oracle_fixture_provenance` ABI guards untouched).

### Task 6 — Un-ignore the two blocked tests with real assertions
- [ ] Replace `zhongguo_phrase_mechanics_parity_is_blocked` (`tests/upstream_luna_pinyin_parity.rs:107-110`) with a real test: build the luna engine with the poet flag on from the new `luna-pinyin-sentence.json`, run `zhongguo` (+ the extra inputs), assert the single-best composed sentence text/order matches the oracle.
- [ ] Replace `full_sentence_lattice_parity_for_zhongguo_is_blocked` (`tests/upstream_luna_pinyin_parity.rs:376-380`) with a real test asserting the **full first page** of sentence candidates (text + order + highlighted index + page shape) matches `luna-pinyin-lattice.json`.
- [ ] Remove the two `#[ignore]` attributes; keep the M18 processor stubs (lines 382, 388) ignored and untouched.
- **Acceptance:** `cargo test -p yune-core --test upstream_luna_pinyin_parity` runs the two formerly-ignored tests and they pass against the oracle goldens; `grep '#\[ignore' tests/upstream_luna_pinyin_parity.rs` shows only the two M18 processor stubs remaining.

### Task 7 — Docs, ledger, and roadmap reconciliation
- [ ] Update `docs/roadmap.md` M17 row from "Planned" to completed/active with the grounded `grammar==nullptr`/`kPenalty = -13.815510557964274` finding and the explicit "not required for TypeDuck-Web parity" note.
- [ ] Add/adjust a `fork-parity-ledger.md` cross-reference distinguishing the **upstream poet path** (M17, log-domain + `kPenalty`) from the **TypeDuck heuristic** (`21.0`, note 5 at line 127) so the two penalties are never conflated.
- [ ] Record in `docs/requirements.md` / decisions that octagram and `.gram` grammar remain deferred (no luna_pinyin need).
- **Acceptance:** Docs name both scoring paths, cite the two new fixtures, and state the octagram/`contextual_translation` deferral.

---

## Completion criteria
- Two new provenance-guarded fixtures (`luna-pinyin-sentence.json`, `luna-pinyin-lattice.json`) captured from the pinned `1.17.0` oracle binary, non-circular, passing `oracle_fixture_provenance`.
- An owned `crates/yune-core/src/poet/` module implementing `make_sentence`/`make_sentences` with the upstream null-grammar `kPenalty = -13.815510557964274` per-word penalty (full precision, not `-13.82`), `CompareWeight`/`LeftAssociateCompare`, and a `Grammar` trait seam (no model).
- An owning test (`crates/yune-core/tests/poet_scoring.rs`) for the scoring/comparators/DP on synthetic graphs (non-circular).
- The luna_pinyin sentence/lattice path produces oracle-matching output behind a named upstream profile flag; TypeDuck `jyut6ping3` (`sentence_candidate`, M21-GAP-01) is byte-unchanged.
- `zhongguo_phrase_mechanics_parity_is_blocked` and `full_sentence_lattice_parity_for_zhongguo_is_blocked` are real, passing tests; their `#[ignore]` is removed.
- No `RimeApi`/`RimeCandidate` change; upstream-first ABI preserved.

## Review checklist
- [ ] Goldens' expected bytes come from the captured upstream `1.17.0` binary, not Yune (non-circular). Provenance headers present and enforced.
- [ ] The penalty constant is the verbatim upstream `kPenalty = -13.815510557964274` (`ln(1e-6)`), named, documented as the grammar-absent `Grammar::Evaluate` value, NOT the rounded `-13.82`, and not silently merged with the TypeDuck `21.0` penalty.
- [ ] TypeDuck/jyut6ping3 sentence behavior is provably unchanged (M21-GAP-01 fixture + `cantonese_parity.rs` re-run green with the poet flag off).
- [ ] Each behavior has an owning module + owning test + an explicit oracle comparison target (own-each-slice).
- [ ] No octagram/`.gram`/`contextual_translation` model was added; the `Grammar` trait is interface-only.
- [ ] No `RimeApi`-table / `RimeCandidate` change; default ABI still tracks upstream `1.17.0`.
- [ ] Only the two LM stubs were un-ignored; the M18 processor stubs remain ignored.
- [ ] Web gates (M9/M13/M16/M20) untouched; M17 stayed off the TypeDuck-Web critical path.
