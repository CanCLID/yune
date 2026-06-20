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
| `pending-M17-M19` | Sentence/LM-lattice-dependent differences (poet/octagram not implemented) | defer; note only |
| `product-only-UI` | Layout/UX of the shipping product, not engine behavior | out of scope here |
| `unexpected-candidate-gap` | Missing/extra candidates in the **core set** | **investigate** |
| `unexpected-composition-gap` | Auto-composition / fuzzy / simplification / reverse-lookup pronunciation differs | **investigate** |
| `unexpected-ranking-gap` | Ranking differs in a **non-prediction** path that should match | **investigate** |
| `needs-engine-investigation` | Unclear; capture and triage | triage |

**Should-match (ledger `preserve✓`) behaviors** — a divergence here is real signal: core candidate set, fuzzy/容錯, auto-composition fallback, `combine_candidates`/separate, `hk2s` simplification, reverse-lookup pronunciations, `letter_to_tone`, `show_full_code`.
**Expected-to-diverge** — prediction ranking order (F08), composition penalty / `matching_code_size` (do-not-preserve), LM sentence lattice (M17), display columns (F09), all UI.

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

## Guardrails

- **Complements, does not replace,** the `v1.1.2` fixture parity (which stays the reproducible gold standard).
- The deployed site is a **moving target** — re-stamp its version every run; a diff may be version skew, not a Yune bug.
- **Do not chase** F08 prediction-ranking or M17–M19 LM gaps as bugs — they are expected.
- **No automated scraping** of the deployed product.
- **Off the parity critical path** — timebox it; it is a sanity check + backlog feeder, not a gate.

## When to run

After **M20 merges** (the harness needs the controls to match the product's settings). The output gap ledger informs the M17–M19 priority discussion and any new `v1.1.2` golden captures.
