# M59 Lane A ranking-fix — blast-radius scoping (owner/fable checkpoint)

Brief: `../lane-a-ranking-diagnosis-2026-07-09/README.md` (root cause + fix spec +
blast-radius section). **This increment pins every named surface's PRE-FIX state and
produces a fix design + per-surface disposition PREDICTION for sign-off. NO engine
code.** The fix does not land until the dispositions below are signed off.

## 1. Pre-fix state — pinned (current `main` @ `12ad47d8`)

Measured 2026-07-09; no engine changes since. **Two surprises the "pin first" step
surfaced:** three cantonese_parity rows are *already RED* on `main` (the documented
"3 pre-existing fails"), and one of them is a TypeDuck pin the blast-radius note assumed
green.

| Surface | Command | Pre-fix state |
|---|---|---|
| Canonical Standard bare syllable (the bug) | `yune-cli … --schema jyut6ping3 --sequence "bei"` (staged rime-cantonese) | `bei` page 0 = `碑 悲 卑 陂 蓖` (code-grouped), `quality:0`; 畀 @~28. **This is what the fix changes.** |
| TypeDuck product `beingo`→畀@6 | `cantonese_parity::m58_current_yune_web_profile_reaches_beingo_bei_at_typeduck_rank` | **GREEN** — 畀 reachable @6, leading `俾我`. Green pin to PRESERVE. |
| TypeDuck product fixture positions (畀@6, 畀@3, 諮@27) | `cantonese_parity::m58_profile_reachability_fixture_records_typeduck_v112_positions` | **GREEN** (asserts fixture values only, not live Yune). |
| TypeDuck product `zi`→諮@27 | `cantonese_parity::m58_current_yune_web_profile_reaches_zi_advice_at_typeduck_rank` | **RED (pre-existing)** — fails at the leading-row assert: `zi` leads with `就`, expected `自`. Never reaches the 諮@27 check. |
| TypeDuck product `nei` golden | `cantonese_parity::m21_closeout_rows_match_typeduck_v112_real_dictionary_goldens` | **RED (pre-existing)** — Yune `[呢 我 你 外 能]` (qualities 5198,5197,5196,5195,5194) vs oracle `[你 呢 尼 妮 彌]`. |
| TypeDuck product prediction boundary | `cantonese_parity::m21_prediction_limit_preserves_m14_short_completion_boundary` | **RED (pre-existing)** — `我` vs `呢`. |
| Whole cantonese_parity suite | `cargo test -p yune-core --test cantonese_parity` | **38 passed, 3 failed** (the 3 above; matches the plan's "exactly the 3 pre-existing fails"). |
| luna parity | `cargo test -p yune-core --test upstream_luna_pinyin_parity` | **GREEN — 14 passed, 13 ignored.** |
| M59 injection-ordering | `cargo test -p yune-rime-api --test yune_web -- m59 reach` | **GREEN — 7 passed.** |
| M55 ratchet baseline | `docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv` | Standing ceilings recorded (23 rows). Not re-run this increment; the fix must re-run with `-DeployProductBeforeBenchmark`. |

**Consequence for the blast-radius note:** the "TypeDuck frozen pins 畀@6/諮@27, D-48-preserved"
line is only half-true on `main` today — 畀@6 (beingo) is green, but **諮@27 (zi) is already
red** (regressed to a leading `就`, tolerated as a pre-existing fail, likely since the M59
flip). The 3 red rows are TypeDuck **product-path** ranking issues (materialized non-zero
quality), a *different* mechanism than the canonical full-list `quality:0` bug.

## 2. Fix design

**Root cause (confirmed):** the full-list path (`translator/mod.rs:2345`
`candidates_for_lookup_codes`) fetches each tone-variant spec (`bei1`,`bei2`,`bei3`,`bei6`)
in storage-iteration order (per-code weight-desc, from `sort_rime_table_entries`
`source.rs:1029`, code-primary) and concatenates them in spec/code order. The global
`engine.rs:1491` sort is by `quality`, but on this path `quality` does not distinguish
tone groups, so the stable sort preserves the code-grouped concatenation.

**Pin FIRST (a real discrepancy the fix increment must resolve):**
`to_candidate()` sets `quality = raw_quality` (`query_table.rs:71`) and
`format_candidate_for_lookup` recomputes `quality = raw_quality.exp() + initial_quality`
(`mod.rs:1601-1605`) — yet the CLI shows `quality:0` and the order is code-grouped. So
"just populate quality" is too glib: the increment must first trace **why the computed
quality does not rank tone groups** on the canonical source-fallback path (is
`raw_quality` uniform/zero at query time here? does the per-code storage order dominate a
degenerate quality?). The design below is correct at either resolution.

**Option A — pool + rank inside `candidates_for_lookup_codes` (recommended).** After the
per-spec loop, sort the concatenated `candidates` by **per-reading weight descending**
before returning (respecting the category/completion tiers already in
`lookup_candidate_order`). Self-contained; does not depend on `engine.rs:1491`; and this
function is **full-list-only** — the TypeDuck product path uses the separate
`bounded_candidates_for_lookup_codes`, so Option A cannot touch it.

**Option B — materialize per-reading weight into `quality`** so the existing
`engine.rs:1491` global sort ranks it. Smaller diff, but relies on the global sort and
interacts with any other candidates in the same engine pass; only viable once the
discrepancy above is pinned.

**Per-reading weight (load-bearing, fable):** rank by the **entry weight** =
`essay_weight(text) × dict-percentage` for that specific `(text,code)` reading (already
computed by `apply_rime_preset_vocabulary_weights`), NOT a char-global/essay-max weight.
The entry weight is per-reading, so polyphones whose corpus weight belongs to another
reading stay low (畀 bei2 100% vs 畀 bei3 3%; 費 bei3 3% not `fai3`; 脾 bei2 0%; 輩 bei3 0%).
This is the same weight that already produces the correct within-`bei1` order — the fix
only extends it across specs.

**Must preserve (M59 injection):** the leading-syllable injection
(`leading_syllable_reachability`) splices singles with phrase-before-single ordering on
the page-turn/complete path. A cross-spec weight sort must NOT re-sort injected singles
up into the phrase block — scope the re-rank to the exact-code tone-variant merge, leaving
the injection splice ordering intact.

## 3. Per-surface disposition PREDICTION

| Surface | Path used | Prediction | Why | Validation when fix lands |
|---|---|---|---|---|
| **Canonical Standard bare syllable** | full-list `candidates_for_lookup_codes`, `quality:0` | **WILL MOVE (intended)** | The fix re-ranks the pooled tone-variants by per-reading weight → `bei`→`畀 比 被 鼻 避` matching oracle. | Re-run the lane-a-runner diff over the 13-input capture; `bei` head = 畀比被鼻避; residual order-only rows → D-48 table. |
| **TypeDuck `beingo`→畀@6 (green)** | bounded-ordered `bounded_candidates_for_lookup_codes` + prefix_fallback | **UNAFFECTED (must stay green)** | Different function; product path materializes non-zero quality and injects 畀 via prefix_fallback; `beingo` base is phrase completions, not tone-variants. | `m58_current_yune_web_profile_reaches_beingo_bei_at_typeduck_rank` stays green. |
| **TypeDuck `zi`→諮@27 (RED pre-existing)** | product/ordered path | **UNAFFECTED — stays red** (neither fixed nor worsened) | `zi` fails at the leading row (`就` vs `自`) on the product path, orthogonal to the canonical full-list fix. Highest-uncertainty pin: `zi` DOES tone-expand, so IF the product's tail materialization to rank 27 ever routes through `candidates_for_lookup_codes`, watch it. | Confirm the 3 pre-existing fails are unchanged (`就`/`我`/`我` still); `zi` still red for the same reason, not a new one. |
| **TypeDuck `nei` / prediction-limit (RED pre-existing)** | product/ordered path | **UNAFFECTED — stays red** | Product-path ranking (materialized quality 5198…), different mechanism. | Same failing values pre/post. |
| **luna parity (green)** | sentence-model / bounded; luna dict is UNTONED | **UNAFFECTED (predicted)** | luna single syllables do not tone-expand (`untoned_dictionary()` true) → no multi-tone-spec pooling; the fix's cross-spec merge does not fire. LOW-MED risk if a luna parity fixture exercises the full-list path with fuzzy/correction multi-specs. | `upstream_luna_pinyin_parity` stays 14/14. |
| **M59 injection (green)** | page-turn/complete + injection splice | **UNAFFECTED IF injection ordering preserved** | The re-rank must not disturb phrase-before-single. MED risk — this is the interaction to guard. | `yune_web m59 reach` stays 7/7; `m59_luna_*` phrase-before-single intact. |
| **M55 ratchet** | typing = bounded path (not full-list) | **GREEN (predicted)** | The fix is on the full-list/page-turn path; the ratchet measures typing (bounded), structurally untouched. | Re-run with `-DeployProductBeforeBenchmark` (M38); confirm standing ceilings, no re-baseline. |

**Why the 3 pre-existing fails stay unchanged (robust to the tail-routing uncertainty):**
all three fail on their **first-page / leading rows** (`zi` leads `就`≠`自`; `nei` first
five; the prediction-limit boundary), which are served by the **bounded** path
(`bounded_candidates_for_lookup_codes`, materialized quality) — never by the full-list
`candidates_for_lookup_codes` the fix touches. So even if the product's deeper tail
materialization were to route through the full-list path, it could not change a first-page
assertion. The "unaffected" prediction for the red rows therefore does not depend on
resolving whether the product tail uses the full-list path.

## 4. Open questions for sign-off

1. **Approve pinning the quality discrepancy first** (§2 "Pin FIRST") before writing the
   re-rank — the one-line "populate quality" is not yet proven to be the fix.
2. **Option A vs B** — recommend A (self-contained, full-list-only, cannot touch the
   product path). Confirm.
3. **The 3 pre-existing TypeDuck fails** are OUT of scope for this canonical fix but share
   a "cross-spec/product ranking" smell. File as a separate product-lane ranking work item,
   or fold into a later increment? (Recommend: separate item; the canonical fix must be
   verified to leave them exactly as-is.)
4. **Residual order-only rows** after the canonical fix go to the owner as the D-48
   disposition table (per-input, order-exact through the captured range).

No engine code written. Awaiting sign-off on §3 predictions + §4 questions.
