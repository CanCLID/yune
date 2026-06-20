# M21 — TypeDuck-Web Product Behavioral Comparison Protocol

> **Status:** Active · **Type:** comparison protocol (validation activity) · **Updated:** 2026-06-20 · **Depends on:** M20 (merged) · **Critical path:** no (qualitative real-world sanity check that feeds the backlog)

> **For agentic workers:** This is **not** an engine milestone and produces **no fixes by itself** — it produces a *divergence gap ledger*. It compares Yune's internal harness against the real deployed product as a *behavior/feel* target, **not a hard oracle**. The hard oracle remains the captured TypeDuck `v1.1.2` fixtures. Run this **after M20 merges** (M20 gives the harness the toggles needed to match the product's settings).

**Goal:** Systematically compare the **core IME behavior** (candidate set, ranking, auto-composition, fuzzy/容錯, simplification, reverse-lookup, paging) of:

- **Yune harness** — `third_party/typeduck-web/` driven by the Yune engine (the internal playground M20 builds), and
- **the real product** — `https://github.com/TypeDuck-HK/TypeDuck-Web`, deployed at `https://www.typeduck.hk/web/`,

so we know **which differences are real engine gaps, which are expected-by-design, and which are pending M17–M19** — without chasing noise.

> **Surface reminder** (see `CONVENTIONS.md` → "Web surface terminology"): `packages/yune-typeduck-runtime/` is the runtime bridge; `third_party/typeduck-web/` is the internal harness compared *here*; the deployed `typeduck.hk/web` is the real product. These are three different things.

## Why the deployed product is a target, not an oracle

The deployed product runs the **actual TypeDuck fork engine** in a browser, so it is a genuine real-world reference. But it is a **moving, less-controlled target**, not a reproducible oracle:

- The hard, reproducible, non-circular oracle stays the captured **`v1.1.2` fixtures** under `crates/yune-core/tests/fixtures/typeduck-v1.1.2/`.
- **If this comparison finds a real divergence in a should-match behavior, the fix path is: capture it as a `v1.1.2` oracle golden and fix against that** — never chase the live site directly.

---

## Section 0 — Confounder controls (do this FIRST; it is most of the value)

A naive "diff the two apps" drowns in apples-to-oranges noise. Pin these before recording anything; if a confounder cannot be pinned, **downgrade to qualitative spot-check and do not report ranking diffs as bugs.**

1. **Engine + dictionary version skew.** The deployed product may run a **newer** engine/dict than our pinned `v1.1.2`. Establish and stamp the deployed version (build info / about page / asset hashes if observable). Treat version-skew differences as *expected*, not Yune bugs.
2. **Fresh userdb / learning state.** Both engines learn. Compare with cleared learning on both: a private/incognito window + cleared IndexedDB/site data on the product; a fresh userdb in the harness. Otherwise ranking diverges from learning history, not engine logic.
3. **Same schema + dict.** Confirm both use `jyut6ping3` (mobile) with a comparable dictionary; note any entry/weight delta. Different dicts ⇒ different candidate sets/ranking for non-engine reasons.
4. **Matched settings.** Align toggles using the M20 controls: completion, correction, `combine_candidates`, simplification, `prediction_never_first`, prediction threshold, page size. Record the exact settings on both sides.

---

## Section 1 — Input corpus

Run the **same typed inputs** in both apps (reuse the M20 guided scenarios + ledger cases):

| Input | Targets |
|---|---|
| `nei` | baseline candidate list |
| `ngo` | classic top candidate / prediction-never-first |
| `santai` | long-entry prediction (`身體` + `身體健康`) |
| `sigin` | code-path prediction (`市建局` without a `市建` word) |
| `m`, `mgoi` | standalone-`m` + fuzzy/容錯 (`ng→m`) |
| `ngohaigo` | auto-composition / sentence (`我係個`) |
| `loengnincin`, `leoicijyu` | cross-boundary dictionary-phrase composition (`兩年前`; live-site observation expected `類似於`, but the hard `v1.1.2` oracle composes `類似如`) — the M21-GAP-01 investigation; verify each input's jyutping against the dict first |
| `hou` | homograph grouping vs separate (`combine_candidates`) |
| tone-letters (`seov`…) | `letter_to_tone` preedit |
| a 1-edit typo | correction (on/off) |
| an `hk2s` case | simplification toggle |
| a reverse-lookup case | dictionary-panel pronunciations |
| a multi-page input | paging behavior |

## Section 2 — Comparable outputs (record per input)

Top-N candidate text · candidate order / highlighted index · candidate comments / Jyutping · long-entry predictions · auto-composed sentence rows · paging behavior · commit result · visible state labels (e.g. `全形`/`半形`).

## Section 3 — Divergence classification key (the heart of the protocol)

Label every difference. This is the fork-parity ledger applied to a live diff:

| Label | Meaning | Action |
|---|---|---|
| `matches` | Same observable behavior | none |
| `expected-by-design` | F08 prediction **ranking order** (we track upstream-1.17.0 ranking + knobs, **not** fork byte-parity); composition word-penalty / `matching_code_size` preedit (`do-not-preserve`); F09 display-language columns (UI-side) | none — **do not log as a bug** |
| `pending-M17-M19` | Differences that depend on the **statistical LM lattice** (poet/octagram not implemented). **Not** dictionary-phrase composition (e.g. `我係個`, `兩年前`), which is M15 should-match — route that to `unexpected-composition-gap`. | defer; note only |
| `product-only-UI` | Layout/UX of the shipping product, not engine behavior | out of scope here |
| `unexpected-candidate-gap` | Missing/extra candidates in the **core set** | **investigate** |
| `unexpected-composition-gap` | Auto-composition (incl. **dictionary-phrase sentence composition** like `兩年前`/`類似於`) / fuzzy / simplification / reverse-lookup pronunciation differs | **investigate** |
| `unexpected-ranking-gap` | Ranking differs in a **non-prediction** path that should match | **investigate** |
| `needs-engine-investigation` | Unclear; capture and triage | triage |

**Should-match (ledger `preserve✓`) behaviors** — a divergence here is real signal: core candidate set, fuzzy/容錯, auto-composition fallback, `combine_candidates`/separate, `hk2s` simplification, reverse-lookup pronunciations, `letter_to_tone`, `show_full_code`.
**Expected-to-diverge** — prediction ranking order (F08), composition penalty / `matching_code_size` (do-not-preserve), LM sentence lattice (M17), display columns (F09), all UI.

> **Dictionary-phrase composition is should-match; only its *ranking* may diverge.**
> Producing the composed sentence *at all* (e.g. `兩年前` from `兩年`+`前`) is M15
> dictionary-driven behavior and a `preserve✓` should-match target — it is **not** the
> deferred M17 poet/octagram LM. The `do-not-preserve` part (ledger note 5) is only the
> fork's *word-penalty / fewer-syllables preference among valid composed sentences*.
> So the decision rule is: **no composed sentence where the oracle has one = correctness
> gap (`unexpected-composition-gap`, investigate); composed sentence present but ranked
> differently = expected-by-design.** This is exactly what M21-GAP-01 below must resolve.

## Section 4 — Evidence capture

- **Deployed product:** **manual / one-time capture only.** Do **not** build an automated Playwright scraper against `typeduck.hk` (third-party site: fragile DOM, ToS). Capture screenshots + transcribed candidate lists + JSON notes; stamp: browser, date, deployed URL, observed engine/dict version, settings, fresh-userdb confirmation.
- **Yune harness:** the M20 playground via Playwright or manual; stamp: Yune commit, M20 branch/commit, schema/config state, settings.
- Store under `third_party/typeduck-web/e2e/results/m21-product-comparison/` with a dated reference snapshot.

**Settings profile snapshot (record per run, both sides).** Capture the exact control
states up front so any divergence is attributable to a setting, not an unrecorded
difference (this operationalizes the Section 0 "matched settings" confounder):

| Setting | Yune harness (M20 control) | Deployed product |
|---|---|---|
| completion (`enable_completion`) | | |
| correction (`enable_correction`) | | |
| auto-composition (`enable_sentence`) | | |
| input memory (`enable_user_dict`) | | |
| combine vs separate (`combine_candidates`) | | |
| prediction never-first | | |
| prediction threshold | | |
| simplification (`hk2s`) | | |
| full-shape / ASCII mode | | |
| userdb state (fresh / accumulated) | | |
| engine + dict version | Yune commit | observed product version |

## Section 5 — Output: the gap ledger

Produce a table, **not immediate fixes**:

| Input | Product output | Yune output | Label | Disposition |
|---|---|---|---|---|

Disposition ∈ { real bug → capture a `v1.1.2` golden + fix against it · `pending-M17-M19` · `expected-by-design` · `product-UX` · `out-of-scope` }. Real "should-match" divergences feed the improvement backlog as oracle-golden work; everything else is recorded so it is not re-investigated.

## Section 5b — M21-GAP-01: multi-syllable dictionary-composition divergence (fixed)

> **Status:** classified and fixed on 2026-06-20 · `unexpected-composition-gap`
> confirmed by `v1.1.2` oracle capture. Evidence:
> `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m21-sentence-composition.json`
> and
> `third_party/typeduck-web/e2e/results/m21-product-comparison/2026-06-20T0748Z-m21-gap-01-sentence-composition/`.

**Observation (manual, harness).** Toneless multi-syllable inputs whose target is a
*dictionary-phrase* sentence return a chaotic candidate list instead of the composed
sentence, while `typeduck.hk/web` composes them:

- `loengnincin` → product expectation and `v1.1.2` oracle: `兩年前`
- `leoicijyu` → live-site/product expectation was `類似於`, but the hard
  `v1.1.2` oracle composes `類似如`; Yune follows the oracle fixture.

**Why this is not obviously expected.** It is *dictionary-phrase composition (M15
scope), not the M17 poet/octagram LM*, and it is **asymmetric**: the same-shaped
`ngohaigo` → `我係個` (a 2-syllable phrase + 1 syllable) composes **top-1 on a fresh
userdb** in the M21 snapshot (`run-manifest.json` fresh context). So a flat/chaotic
result for `loengnincin`/`leoicijyu` is a real anomaly, not merely the
`do-not-preserve` word-penalty ranking. Mechanically, Yune has no librime
syllable-graph speller; `sentence_candidate()` ([`translator/mod.rs`](../../crates/yune-core/src/translator/mod.rs))
is a fallback-only Viterbi gated on `candidates.is_empty()` over toneless-aliased
substring probes — but note the obvious "zero matches / no tone-stripping" theory is
**false** (the schema's `derive/\d//` adds toneless aliases and `ngohaigo` composes),
so the cause must be found empirically, not assumed.

**Oracle-first investigation result:**

- Corpus verified against the production `jyut6ping3.dict.yaml`: `loengnincin`,
  `leoicijyu`, `ngohaigo`, `loengjathau`, `geijatcin`, and `gamjatheoi`.
- Pre-fix Yune source-aware evidence showed the fallback gate **did fire** and
  produced a `sentence` row at index 0, but the sentence text was wrong for five
  of six cases:
  `loengnincin` → `𦧲五官前次我`, `leoicijyu` → `呢在次如中`,
  `loengjathau` → `兩一後`, `geijatcin` → `機一次我`, and
  `gamjatheoi` → `今一靴時`. `ngohaigo` already matched.
- The `v1.1.2` oracle composed all six cases at top-1:
  `兩年前`, `類似如`, `我係個`, `兩日後`, `機日前`, `今日去`.
- Classification: `unexpected-composition-gap`. The problem was sentence path
  scoring: raw frequency exponentials let high-frequency short pieces dominate
  dictionary phrase compositions. The fix changes sentence path scoring to
  log-space with an oracle-backed word penalty, preserving fallback-only gating
  and adding the TypeDuck-style synthetic `composition` lookup row for composed
  sentence comments.
- The word penalty is a Yune heuristic, effectively "prefer fewer pieces, then
  use frequency as a tiebreak", validated on these six fixture cases rather than
  an oracle-exported quantity. Harder composition tradeoffs that need true
  phrase probabilities remain M17 poet/LM territory, and this composition corpus
  should be expanded opportunistically as new real product cases appear.
- Post-fix source-aware evidence shows all six Yune top sentence rows match the
  oracle top text, with `fallback_gate=fired_returned_sentence`.

**Protocol retained for future M21 gaps:**

1. **Reproduce at a pinned Yune commit.** Run `loengnincin`, `leoicijyu`, `ngohaigo`
   (working control), and 2–3 more analogous two-word cross-boundary inputs through the
   real harness on the production `jyut6ping3.dict.yaml`; capture the **actual** full
   ranked candidate JSON (top-N text, codes, preedit, and whether/where the `☯` sentence
   row appears) — not a screenshot. Investigate *why* a flat multi-candidate list appears
   when `sentence_candidate()` returns a single path (check whether the visible rows come
   from completion/correction/userdb rather than the sentence path, and whether the
   `translator/mod.rs` `candidates.is_empty()` fallback gate is even firing).
2. **Capture `v1.1.2` oracle goldens for the same inputs** under `jyut6ping3_mobile`
   against the production dict via `scripts/capture-typeduck-jyutping.ps1` +
   `oracle-rime-probe.cs`. Store a provenance-stamped locked fixture (e.g.
   `jyut6ping3-m21-sentence-composition.json`) beside `jyut6ping3-m14-options.json`.
   Non-circular: expected bytes come from the oracle binary, never from Yune or the live
   site.
3. **Classify per Section 3.** Oracle composes the sentence **and** Yune produces none /
   only chaos → `unexpected-composition-gap` (correctness regression that reopens
   fork-parity-ledger note 5). Oracle composes it and Yune produces it but ranks lower →
   already-decided `do-not-preserve`; record as expected, do **not** fix.
4. **Conditional fix — only if step 3 proves a correctness regression.** Minimal change
   in `sentence_candidate()` / the fallback gate, plus a regression test asserting against
   the captured `v1.1.2` golden. Then update the ledger note-5 row and add a tracked
   engine work item. No ABI / `RimeCandidate` change; AI stays default-off.

## Guardrails

- **Complements, does not replace,** the `v1.1.2` fixture parity (which stays the reproducible gold standard).
- The deployed site is a **moving target** — re-stamp its version every run; a diff may be version skew, not a Yune bug.
- **Do not chase** F08 prediction-ranking or M17–M19 LM gaps as bugs — they are expected.
- **No automated scraping** of the deployed product.
- **Off the parity critical path** — timebox it; it is a sanity check + backlog feeder, not a gate.

## When to run

After **M20 merges** (the harness needs the controls to match the product's settings). The output gap ledger informs the M17–M19 priority discussion and any new `v1.1.2` golden captures.
