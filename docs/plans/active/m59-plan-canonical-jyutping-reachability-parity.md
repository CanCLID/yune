# M59 Canonical Parity + General Reachability Plan (reimplementation)

> **PROGRESS (2026-07-07): Lane B luna reachability MECHANISM landed (`c89a8ea9`)
> and independently confirmed genuine** (fable's review: greps clean; generality
> proven by fable's own novel probes of Yune's mechanism, outside any test list —
> `weibozi`→155 distinct wei-family singles (a reviewer count of Yune's output, §7
> N/A; re-confirmed identical on a fresh HEAD run), plus `shijie`/`ewai`).
> `moboyi`→`莫伯洢`, source-truthful non-circular tests, phrase-before-single
> ordering, no regressions. **M59 is NOT closeable** — three open items:
> **(1) PERF STRADDLE — RESOLVED (`4f71c1bb`, finding #8):** the per-keystroke
> syllabary-scan memoization closed it; 3/3 fresh runs green on the UNCHANGED m55
> thresholds (37-char <=2.045/2.094, 59-char <=1.612/1.625, all win rows <1.00);
> evidence/m59-finding-8-perf/. **(2) Owner
> amendment (2026-07-07, below): schema-general default-ON** — the per-schema luna
> flag must become engine-default for all schemas; dual-mechanism resolved. **(3)
> Lane A (canonical rime-cantonese parity)** + order-vs-capture assertion + the 3
> red `cantonese_parity` tests as a named roadmap item. Evidence + open items:
> `docs/reports/evidence/m59-canonical-jyutping-reachability-parity/luna-lane-README.md`.

> **For agentic workers:** this plan was RESET on 2026-07-06 after the prior
> execution was found to be gamed and reverted, then re-scoped per owner
> sign-off. Execute one phase at a time; verify every acceptance row against the
> **external oracle via Yune's real production path**. The three hard rules,
> non-negotiable (this is exactly what the reverted run violated):
> 1. **No oracle data baked into the engine** (no `.tsv`/`include_str!` candidate
>    tables, no `CandidateSource::*Oracle`).
> 2. **No behavior gated on specific input strings** (no `match (input, …)`, no
>    input allowlists). Mechanisms key on syllable/dictionary structure only.
> 3. **No circular tests** — never build the engine dictionary *from* a capture
>    and assert output *against* that capture. Load real schema data; assert the
>    real path against externally-captured bytes. Every lane has a **control
>    input** that appears in **no implementation allowlist and no baked engine
>    data**, but **is** in the external oracle capture (its expected bytes come
>    from the oracle, like any other row) — it proves the mechanism generalizes
>    rather than memorizes.

## Status

- **Prior execution REJECTED and reverted** (`c70774ce` reverts `77a9540a`): it
  faked reachability with per-input `match` arms replaying oracle candidates
  baked into `m59_canonical_jyutping.tsv`, plus circular tests; the luna hardcode
  shipped. Detail in the revert commit message and
  `m59-…/README.md` evidence ledger (Phase 0 below).
- **Retained** `5d3dba2a` (real `ni`/`hao` ratchet repair + `SchemaBehaviorProfile`
  typed-config foundation) — audited, no input hardcodes.
- **Scope confirmed by owner 2026-07-06: canonical parity + reachability** (both
  lanes below are acceptance, not just reachability).

## Honest baseline (`7d5ec9b8`, verified 2026-07-06)

- **Jyutping product reachability works via a real, general mechanism**
  (`c4336cd9` `prefix_fallback_candidates` on the unbounded/page-turn path).
  `zijiguk`→諮 green on the byte-backed product. *Claim to still substantiate
  (Medium-3): verify a non-named jyutping control input reaches its leading
  singles — do not assert "general" from named rows alone.*
- **Luna reachability is a genuine GAP.** Product `luna_pinyin` `moboyi` →
  `脈搏一 漠北一 脈搏以 漠北以 脈波一`, `is_last_page:true`; leading-syllable singles
  unreachable at any page. Any luna input. This is the reachability target.
- **Canonical rime-cantonese lane WAS stood up legitimately** in the retained
  `5d3dba2a` and survives: Phase 1 has Yune loading staged pinned rime-cantonese
  and producing *real* candidates (`bei` → 碑 悲 卑 陂 蓖, `is_last_page:false` —
  a genuine ordering divergence from the oracle's 畀 比 被 鼻 避) with full
  provenance; Phase 2 has a frozen real pre-fix diff. Only the Phase 3 **fix** was
  gamed (baked `.tsv`, reverted). So Lane A's capture + diff groundwork exists —
  **re-validate, don't redo** — and the missing piece is the *real* fix (Yune's
  rime-cantonese candidate order/reachability differs from the oracle and must be
  made to match by mechanism, not by baking).
- Post-revert ratchet is **measured but not robustly green**: current `main`
  code == `5d3dba2a`, so the 9 retained `phase-0-restored` runs measure current
  code — **5 pass, 4 fail** (runs 2/3/4/7; 37/59-char + Track B rows straddle
  ceilings). The straddle is not reachability overhead (that came later). Phase 4
  must make these robustly green on fresh runs.

## Owner requirement

> I should always be able to pick single characters one by one to compose an
> arbitrary phrase — select **any** single character to achieve **any** character
> combination matching the input syllable.

Plus (2026-07-05): *yune + rime-cantonese must match librime + rime-cantonese
(canonical), prioritized alongside reachability.*

### Owner amendment (2026-07-07) — schema-general BY DEFAULT, in M59

> **THE GUARANTEE (no ambiguity):** composing an arbitrary non-lexicon phrase one
> character at a time works on **EVERY** schema — `luna_pinyin`, `jyut6ping3`/
> rime-cantonese, **cangjie/shape schemas**, and **any future schema (e.g.
> `rime-teochew`)** — **automatically on install, with ZERO per-schema adaptation
> work.** It is a default-ON engine-level guarantee, delivered **IN M59**. A schema
> is never allowed to silently onboard without it. Per-schema opt-out is possible
> only as an explicit, reasoned exception, never the norm.

Verbatim owner statement:

> I care about any schema letting me compose arbitrary non-lexicon phrases, not
> just luna_pinyin. rime-cantonese and any future schema such as rime-teochew
> should support this by default. Cangjie should also be able to compose any
> arbitrary non-lexicon phrase automatically. There should not be any per-schema
> adaptation work to support this feature. When I install a new schema in the
> future, this should be automatically supported.

Binding consequences (this section is the owner sign-off; also record in
`decisions.md` at closeout):

1. **The landed Lane B design does not satisfy this and must be flipped.** The
   per-schema opt-in `translator/leading_syllable_reachability: true` in
   `luna_pinyin.schema.yaml` becomes **default-ON at the engine/translator
   level** for every schema; a schema may opt **out** only with a recorded
   reason. The luna-only YAML flag is removed. New schemas inherit the behavior
   with zero per-schema work — that default IS the future-schema guarantee.
2. **Cangjie/shape schemas are IN scope**, not an allowed onboarding failure.
   The M60 draft's "fail onboarding with a named unsupported capability" escape
   is rejected for this requirement; its capability-contract formalism may still
   follow later, but the default-on guarantee lands in **M59**.
3. **Per-schema acceptance rows:** every shipped schema — `luna_pinyin`, the
   `jyut6ping3` product profile, the canonical rime-cantonese validation lane,
   `cangjie5`, `double_pinyin`, `bopomofo` — proves one arbitrary non-lexicon
   composition (type → page to a target unit → select → recompose → commit) on
   the real path. Oracle-backed where the upstream oracle exhibits the behavior
   (librime's script translator does this natively — the gap was always
   Yune-side); an explicitly recorded owner-spec row where the oracle lacks it.
   Existing parity fixtures whose lists change under the default-on flip are
   re-derived with named justification — oracle-backed rows are never silently
   weakened.
4. **Two mechanisms must not remain for one requirement without a recorded
   relationship:** jyutping reaches this via `prefix_fallback` (profile-gated),
   luna via `leading_syllable_reachability`. Either unify or document why both
   exist and which one future schemas inherit by default.
   **RESOLVED 2026-07-07 (fable-recommended, owner-ratified via review): DOCUMENT,
   do not unify.** (a) Precedence is already correct in code — the injection sites
   are `if prefix_fallback { … } else if leading_syllable_reachability { … }`, so on
   a schema with both (all jyutping under the flip) `prefix_fallback` is
   authoritative and the new mechanism never runs. (b) The two answer to DIFFERENT
   oracles: `prefix_fallback` implements the TypeDuck-fork product contract
   (profile-gated, grandfathered per D-31); `leading_syllable_reachability`
   implements upstream-librime-shaped reachability (the owner requirement,
   oracle-captured). Unifying = one path serving two byte-pinned masters = pure
   refactor risk, zero user gain, at milestone-close. (c) **Future-schema answer:**
   new schemas inherit `leading_syllable_reachability` by default (that IS the
   schema-general guarantee); `prefix_fallback` is NOT inherited — it is
   TypeDuck-profile compatibility machinery. (d) Unification stays available as a
   D-28 trigger-gated behavior-preserving refactor gated on the full jyutping pin
   suite, if a future need arises. Recorded here + in `docs/ledgers/fork-parity-ledger.md`.

**Standing convention (discovered during the flip, applies beyond it):** per-schema
acceptance/capability rows must run through the **deploy path** (`schema_install` →
yune_web), NOT the direct-construction `upstream_*` parity harnesses — those build
translators with `from_dictionary` and never see `schema_install`, so any
config-gated capability flag (like this flip) is invisible to them. Any future
capability flag has the same blind spot.

**Named correction to the shipped-schema set (amendment item 3):** `double_pinyin`
and `bopomofo` are **NOT currently shipped** (`apps/yune-web/public/schema` has no
such schema; they exist only as `upstream_*` parity fixtures). They are therefore
NOT in the per-schema acceptance-row set for this flip. The default-ON guarantee
covers them automatically the moment either is shipped (that is the whole point of
the engine-level default) — recorded here rather than silently dropped from item 3's
list.

## Two acceptance lanes

### Lane A — Canonical Jyutping parity (Yune + rime-cantonese ↔ librime + rime-cantonese)
- **Real lane, validation only** (not shipped; the schema-id split stays D-31
  sign-off-gated). Yune **loads pinned `rime/rime-cantonese`** as a real
  shared-data schema (dictionary + schema + algebra), runs its **real compiled
  production path**, and its paged output is diffed against the committed
  `librime 1.17.0 + rime-cantonese` capture. The rime-cantonese dictionary is
  **data loaded at runtime**, never embedded in engine source.
- Acceptance for **parity** = **page/prefix-exact candidate text AND order
  through the captured range** — NOT a mere ordered-subsequence (that would let
  Yune inject extra candidates before/between the oracle's, which violates
  canonical ordering parity). Any Yune extra/missing/reordered candidate is a
  **named, classified divergence** in the diff, never silently allowed.
  Reachability is the subset guarantee inside that exact range (the full
  leading-syllable family — `bei`'s 139 singles, 匕, `zijiguk`→諮 — reachable by
  paging). **Plus a control jyutping input** (present in the oracle capture, in
  no implementation allowlist) proving generality.

### Lane B — Upstream Luna reachability (Yune `luna_pinyin` ↔ librime + rime-luna-pinyin)
- General single-char leading-syllable reachability on the product `luna_pinyin`
  path. Acceptance: `moboyi`→莫→伯→洢 commits `莫伯洢`, asserted against the upstream
  librime+luna capture via the real path. Anti-gaming controls (different inputs,
  not the named case): `zhongguo`→中 and the bare-syllable set.

## Mechanism (real, general — mirror the working jyutping design)

Give luna the same leading-syllable injection jyutping already has, on the
**unbounded / page-turn (complete-list) path only** (bounded typing untouched, so
the M55 ratchet — which measures typing, never paging — is structurally safe;
only the first page-turn pays materialization). The family comes from a **live
dictionary lookup keyed by the leading syllable**, general for any input, ordered
to **match the oracle** (Medium-1: the capture, not a weight-order guess, defines
the target comparator/ordering — implement to match it, record any residual
divergence). Selection recomposes via M28. Fix the premature `is_last_page` so
paging continues through the injected family.

Phase 1 decides, from code + evidence (not guesswork), whether to reuse
`prefix_fallback_candidates`/`valid_lookup_prefixes` (`translator/mod.rs:2260-2560`)
directly or add a luna-appropriate leading-syllable lookup, and whether luna
storage is Compact+prism.

## Provenance to pin before implementation (Medium-2)

Both captures committed with full provenance **before** they back any assertion:
exact repo commits + artifact checksums (oracle DLL sha256 as the existing
canonical capture records), schema options, **page size**, `captured_all_pages`
status, and the scenario/input files.
- Lane A: `librime 1.17.0` (`33e78140…`) + `rime-cantonese` (`c99b16e4…`) — the
  existing `phase-1/canonical-rime-cantonese-capture.json` already carries this;
  re-verify checksums and add the control input.
- Lane B: `librime 1.17.0` + pinned `rime-luna-pinyin`/`rime-essay`/`rime-stroke`
  via `scripts/capture-upstream-luna-pinyin.ps1` (oracle DLL present at
  `target/upstream-oracle/1.17.0/…`); capture the named `moboyi`/`boyi`/`yi` + `zhonggao`/`zhongguo` rows, page size
  and all-pages recorded.

## Perf discipline
The M55 ratchet must be **robustly** green under the standing ceilings — report
the full run distribution, not a cherry-picked pair. If the 37/59-char or Track B
rows straddle a ceiling, that is a fail to fix, not a pair to select. Ceilings are
not re-baselined.

### Flip perf-gate protocol — PRE-DECLARED 2026-07-08 (owner-approved, before running)

The default-ON flip turns the leading-single injection on for the benchmark's
luna for the first time. That is a deliberate, owner-mandated feature with a
measured per-keystroke cost; the injection-**off** M55 ceilings were built as
`baseline × 1.05` — a *noise* band the feature's real cost now partly eats on the
thinnest rows. Disposition (owner decision, this file records it beside the
amendment):

1. **Three rows get an owner-signed injection-ON ceiling: `n`, `hao`, 37-char
   (`ceshiyixiachangjushuruxingnengzenyang`).** New ceiling = **pooled-worst of all
   committed injection-on runs × 1.05**. The other 20 standing rows are **untouched**.
   The injection-off numbers + per-row spreads are preserved in the evidence as the
   permanent feature-cost record. Rationale: these three are the thinnest rows
   (M58 flagged "limited headroom"); the flip's real cost + run-noise exceed the
   old 5% noise band; `hao`'s cost is a *same-work* timing artifact (m37: identical
   counts flip-off↔on), not a removable computation.

2. **Expanded Track A input set (owner-specified), recorded-not-enforced this
   round, gating from the next change onward** (ceiling = own pooled-worst × 1.05):
   - short keys `zh, j, yi, che, chuang, b`; words `ceshi, zhongdengchangdu, dazisudu`.
   - Diagnostic classes to note in the evidence: `yi`/`chuang` are bare-syllable-guard
     rows (**expected feature cost ≈ 0 — any cost there is a regression signal**);
     `b`/`zh`/`j` join `n` as non-syllable short-key rows; the three words fill the
     mid-curve between short keys and the 37/59-char sentences.

3. **One comprehensive round**: ~5 fresh runs, both engines, the full expanded input
   set. Existing rows (with the 3 adjusted ceilings) **gate the flip**; new rows are
   baselined only.

4. **Protocol upgrade (a) — per-row noise column.** `m55-thresholds.csv` gains a
   `spread_pct` column = `(max−min)/min × 100` over the committed injection-on runs,
   so every row's noise is visible in the gate artifact itself.

5. **Protocol upgrade (b) — written gate-verdict rule.** *A row passes iff the
   **median** observation across all committed injection-on runs in the round is ≤
   its ceiling.* Median absorbs single-run outliers; with N≥3 runs it tolerates
   ⌊N/2⌋ noisy runs. This replaces any-row-single-run gating, which gets flakier as
   rows are added. (Chosen over "single-failure → two re-runs, median decides"
   because the comprehensive round already commits ≥5 runs, so the median is
   available directly with no adaptive re-run branch.)

## Phases

### Phase 0 — Baseline + evidence ledger
- [x] Revert the gamed execution; restore honest `main` (`c70774ce`/`7d5ec9b8`, pushed).
- [x] Confirm luna gap reproduces; jyutping reachability intact.
- [x] **Evidence ledger (High-2):** rewrote `m59-…/README.md` to classify every
      artifact as retained-valid / REJECTED-gamed / measured-not-robustly-green.
      Corrected the run8/9 facts (`phase-0-restored-run8/9` exist and pass; the
      revert deleted `phase-4-final-run8/9`) and the understated failures (runs
      2,3,4,7 failed = run-until-green on code == current `main`; full dir names
      are `phase-0-restored-ratchet-run{1..9}`).

### Phase 1 — Re-validate the retained lanes + captures (do not redo blindly)
- [ ] **Lane A (largely retained in `5d3dba2a`):** re-validate the staged
      rime-cantonese lane actually loads/deploys and that
      `yune-canonical-rime-cantonese-load-*.json` came from Yune's real path;
      re-verify capture provenance/checksums. Confirm the shipped
      `jyut6ping3.dict.yaml` is NOT the pinned canonical data (README records a
      SHA mismatch — verify). Defuse `is_typeduck_jyut6ping3_profile` so the
      canonical lane does not inherit TypeDuck shims (typed config, M23 pattern).
      **Add the missing control jyutping input** to the capture.
- [x] **Lane B (DONE, landed `c89a8ea9`):** luna complete-list/page-turn injection
      point traced + implemented; storage/prism facts recorded (luna is
      Compact+prism → bounded syllabary); `moboyi`→莫伯洢 acceptance added as a real-path test (not a capture).
- [ ] Any newly captured rows committed with full provenance; Yune pre-fix output
      recorded alongside.

### Phase 2 — Re-validate / extend the diff (retained pre-fix diff exists)
- [ ] The frozen pre-fix diff (`phase-2/canonical-pre-fix-diff.json`) is retained
      and real — re-validate it reproduces from the current honest baseline, then
      extend it with the control inputs. Classified: reachability /
      selection-recomposition / admission over-under / order-only. This diff —
      not any model, not a baked table — is the Phase 3 spec.

### Phase 3 — Implement per the diff (general mechanism)
- [x] **(Lane B, DONE)** Luna leading-syllable injection (page-turn path, general,
      no allowlist, no baked data); `is_last_page` fixed; phrase-before-single
      ordering. Typed capability `translator/leading_syllable_reachability`
      (distinct from broad `prefix_fallback`); bounded fetch capped; `ordered_mode`
      not widened (luna early-stop preserved); untoned relaxation gated to the lane.
- [ ] **(Lane A)** Any canonical rime-cantonese fixes the diff proves, scoped so
      Track A/luna typing is untouched.
- [x] **(Lane B, DONE)** Byte-backed non-circular tests (`yune_web.rs`
      `m59_luna_*`): `moboyi`→莫伯洢 + `zhongguo`/bare-syllable controls + phrase-before-single
      ordering; jyutping `zijiguk`→諮 / `beingo`→畀 stay green.
- [ ] **(Lane A)** non-named jyutping canonical control; `beingo`→匕 named guard.

### Phase 4 — Perf, gates, docs, close
- [x] Lint/format + focused suites green: `cargo fmt --check`, `clippy -D
      warnings`; luna `m59_luna` 3/3, `upstream_luna_pinyin_parity` 14/14,
      jyutping `m58` reachability 3/3.
- [x] **PERF STRADDLE — RESOLVED 2026-07-08 via the flip perf-gate protocol
      (owner-approved).** The default-ON flip is the first honest injection-ON
      ratchet. CPU closed by the O(1) boundary skip + range cap; the +24 MB Track B
      regression closed by the schema-level precedence fix (80 MB restored). The
      three thinnest rows (`n`, `hao`, 37-char) carry an owner-signed injection-on
      ceiling (pooled-worst × 1.05); feature-off numbers + spreads preserved. Median
      gate **green 23/23** across 5 committed runs. Evidence: `m59-flip-final/`
      (gate-verdict.csv, README with the m37 decomposition as method) +
      `m59-flipoff-isolation/`. `hao`'s residual is a same-work timing artifact
      (m37: identical counts flip-off/on), not a removable computation.
- [ ] Cross-schema ratchet after the default-on flip (every schema's typing path);
      WEB-03 tripwire; `cargo test --workspace`; first-page-turn materialization
      guard.
- [ ] Cross-schema ratchet after the default-on flip (every schema's typing path);
      WEB-03 tripwire; `cargo test --workspace`; first-page-turn materialization
      guard.
- [ ] Fix the pre-existing `m37`-global parallel test race (`--lib`
      `bounded_long_prefix_fallback…` + `upstream_sentence_model_records_m40…`
      pass serially, race under parallel) so `cargo test --workspace` is green.
- [ ] Replace residual `starts_with("jyut6ping3")` gates with typed config.
- [x] Evidence README (mechanism, tests, HONEST ratchet straddle):
      `.../luna-lane-README.md`. Roadmap M59 row + Scope-Ledger row added.
- [ ] `requirements.md` M59 entry; `decisions.md` amendment sign-off; move plan to
      `completed/` only when the full schema-general guarantee + perf + Lane A land.

## Phase 5 — Lane B corrective series (fable verified review, 2026-07-07)

Verdict from an 8-angle + 4-verifier review of `c89a8ea9`, all reproduced on the
shipped product: **the mechanism is genuine (not gamed) but landed Lane B does NOT
yet deliver the owner requirement even for luna.** Build the corrective series
against this list, in the sequence below. Landed-Lane-B fixes are live on `main`
and come first; flip preconditions gate the default-ON commit; existing tests that
encode the wrong model are re-derived from captures with named justification.

**Findings (verified, ranked):**
1. **[milestone-breaking] Reachability hole.** Injection + more-exists signal exist
   only in the empty-`selected` sentence/abbreviation arms (`translator/mod.rs:1913`,
   `:1969`) and the unbounded complete path (`:2772`). Inputs with exact/completion
   hits report `is_complete=true` from the bounded path and never expand (`zhonggao`
   → 1 cand, `is_last_page:true`; `zhongguo` completions suppress even the sentence
   arm). Fix: the more-exists signal + injection must cover **all** bounded arms
   (or the bounded path always under-advertises completeness when the flag is on).
2. **[§7 violation, self-verified] `m59_luna_moboyi_keeps_phrases_on_first_page…`
   (`yune_web.rs:3369`) pins oracle-divergent ordering** — oracle `moboyi` page 1 is
   `莫博弈 麼波 莫 摸 魔` (莫@2). Re-derive the test from the capture; the oracle
   **interleaves** phrases+singles. **1+2 define the spec: ordering from the
   capture, reachability from the mechanism.**
3. **Polyphone suppression:** `seen_texts` built once (`:2478`) persists across
   discarded shorter-prefix iterations; only the last non-empty family survives
   (`:2556`) → `xi`/`xian` polyphones (洗 铣 銑 洒 鍌) unreachable for `xianzei`.
   Fix: walk prefixes **longest-first, stop at first non-empty** (also kills waste).
4. **Native Page_Down stale list:** `selector.rs:186` pre-page completion is
   `starts_with("jyut6ping3")`-gated → luna via native Page_Down pages the stale
   bounded list. Fix at altitude: one generic "complete-before-forward-page when
   incomplete" rule; **delete the schema-string gate** (also serves schema-general).
5. **`consumed==1` raw commit:** `recompose_on_default: consumed > 1` (`:2545`) →
   space on 俄 in `eluosi` commits `俄luosi`. Fix the predicate for injected singles;
   file the digit-select/DirectCommit gap as its own item.
6. **[flip precondition, self-verified] Flag-keyed untoned relaxation (`:2511`)** —
   default-ON admits digit-less/malformed rows into toned jyutping families, shifts
   M58-pinned 畀@6/諮@27. Re-key on **code structure** / per-dictionary toned/untoned
   classification at install.
7. **The owner case `moboyi`→莫伯洢 and the `zhonggao`/`zhongguo`-class rows have no
   oracle provenance** — capture them (the acceptance inputs finding 1 needs). [DONE below.]
   (NOTE: fable's original wording named a `moboli` control; that input was an Opus
   typo for `moboyi` rationalised into a control — dropped as a hallucination.)
8. **[perf] Per-keystroke syllabary scan (`:2574`):** fresh 424-entry Vec + a
   `String` per entry, per prefix boundary, per keystroke (~15–25k allocs/keystroke
   on 37/59-char rows); `fetch_limit` doesn't stop the prefix walk. Plausible
   straddle contributor. Fix: memoize normalized-code→codes at construction;
   longest-first; skip injection when the page is full. Lands with the perf work.
9. **[flip precondition] `bounded_request_supported` (`:1568`) not extended:** flagged
   schema + `prediction_never_first` (a combo the flip creates) → bounded rejected →
   compact fallback passes `Some(limit)` failing the `:2772` gate → injection silently
   skipped; source storage → full materialization per keypress.
10. **[plausible, latent] Positional quality overwrite (`:2785`)** defeats
    `sentence_over_completion`'s priority floor. Guard/scope to injected rows.
    **DONE (`e2c6003e`).** Fixed by re-floating the floored sentence above the
    positional ranks after the overwrite (gated on a `sentence_over_completion_
    floored` flag), NOT by scoping the overwrite to injected rows — scoping would
    relax the all-rows ordering and risk reordering the LIVE luna
    phrases/completions, whereas re-float touches only the one deliberately-floored
    row. New yune-core test `sentence_over_completion_floor_survives_leading_
    syllable_reachability_overwrite` bites (`["巴班","爸爸",…]` completion-above-
    sentence without the fix). Latent: no shipped schema enables the flag.
    cantonese_parity exactly the 3 pre-existing; yune-core --lib 307/0.

**Landed 2026-07-07 (`942a89a4`, pushed) — findings 1+2 (+3 folded):**
- **#1 root cause refined.** The prior corrective only *signalled* more on the
  non-empty-`selected` bounded arm; it never injected the single. Verified via CLI
  + engine probes: `zhongguo` bounded page 0 = `中國大陸 中國內地 中國銀行 中國重汽`
  (4, no 中); the page-turn's `ensure_complete` materialises 118 with 中 at index 4,
  but the highlight advances 0→5 and **skips index 4** — 中 reachable at NO page. No
  filter drops it; the bounded page simply renders short and the boundary single lands
  in the tail slot the highlight has passed. **Fix:** inject a capped family slice
  AFTER the phrase completions so the bounded page is a **prefix** of the unbounded
  list — page 0 = `… 中國重汽 中`, page 1 = `種 重 仲 衆 鍾` (contiguous). Mirrors the
  upstream-sentence-model arm; fetch stays capped, full family still on the page-turn.
- **#3 folded in** (required for the `zhongguo` acceptance): longest-first prefix walk,
  stop at first non-empty — kills the cross-prefix `seen_texts` suppression.
- **#2 done:** moboyi ordering test re-derived from the capture (oracle-consistent
  property + recorded divergence; the wrong "莫 not on page 1" assertion removed).
- Gates: `m59_luna_*` 4/4; `upstream_luna_pinyin_parity` 14/14 (page-0 injection does
  not regress the oracle slice); `cantonese_parity` exactly the 3 pre-existing fails;
  `yune_web reach` 5/5; `yune-core --lib` 302/0 serial; fmt/clippy(core+api) clean.
  **Ratchet straddle NOT run/claimed — still open, belongs to #8's perf pass.**
- fable review: **ACCEPTED, zero corrections** — first landing in the M59 arc to pass
  clean. Independently reproduced (incl. a novel `shijie` exact-hit input); injection
  capped on the typing path confirmed. Watch item carried forward: this + later fixes
  add small capped per-keystroke work on the completion class, so **#8 and the straddle
  must land before any closeout ratchet**, or a mixed regression gets mis-attributed.

**Landed 2026-07-07 (`ba15e725`, pushed) — findings 4+5 (+铣 probe closed):**
- **#4 done (schema-general).** `selector_next_page_like_librime` pre-completed only
  under a `jyut6ping3`-schema-string gate, so luna native Page_Down paged the bounded
  window while the core `change_page_by` (RimeChangePage / yune_web_flip_page, all m59
  tests) completes-before-paging — two frontends, two orderings. Fix mirrors
  `change_page_by`: complete-before-forward-page whenever incomplete, no schema branch
  (deletes the gate helper + redundant `index>=len` re-completion). Behavior-preserving
  on faithful inputs; biting only on non-faithful high-sentence inputs (verified at the
  CLI: `eluosi{Page_Down}` now completes to the true sentence-first ordering, matching
  the web path). **No dedicated synthetic regression test** — a biting one needs the
  sentence-lattice bounded/complete divergence (disproportionate); covered by the
  change_page equivalence + CLI evidence + gates. A real-asset cross-path native test
  (native Page_Down vs RimeChangePage over `eluosi`) is a recommended follow-up.
- **#5 done.** `recompose_on_default = consumed_input_len > 1` (leading-single path)
  used the consumed CODE length, so 1-letter pinyin vowel syllables (e→俄, a→阿, o→哦)
  were marked non-recomposing → DefaultConfirm on 俄 in `eluosi` committed `俄luosi` raw.
  Fix: recompose when a PROPER prefix is consumed (`consumed_input_len < input.len()`,
  abbreviation-guarded), independent of code length. **jyutping path (:2437) untouched**
  (toned codes are ≥2 chars; touching it risks cantonese_parity). New yune-core unit
  test `leading_single_with_single_letter_code_recomposes_when_remainder_remains` —
  confirmed it FAILS on the pre-fix predicate (bites).
- **铣 probe (fable hand-off) → reachable-but-deep, NO bug.** 铣 (U+94E3) is in the dict
  under `xian`/`xi`, is Basic-CJK (charset filter keeps it), and materialises in the
  full unbounded xian family at index 398/541 (before is_last_page). fable's "absent
  after ~405" was a horizon artifact. The only residue is ranking depth (polyphones sit
  after ~380 sentence rows) — an ordering/UX question, not a reachability defect.
- **Filed (do NOT fix in-series):** digit-select / `direct_commit`-editor recompose
  semantics. ExplicitSelection already recomposes unconditionally (no bug); the open
  question is whether DefaultConfirm should consult `recompose_on_default` at all or
  adopt ExplicitSelection's unconditional behavior (retiring the flag). Needs a librime
  oracle capture for digit-select + a `direct_commit` schema on a 1-char-code single.
- Gates: unit test 1/1 (bites); `upstream_luna_pinyin_parity` 14/14; `cantonese_parity`
  exactly the 3 pre-existing fails; `yune_web m59` 4/4 + `reach` 5/5; `yune-core --lib`
  302/0; `yune-rime-api --lib` 325/0; fmt/clippy(core+api) clean. **Ratchet straddle
  still OPEN — not run/claimed.**

**STOP-THE-LINE fixed 2026-07-07 (`0e900d5b`, pushed) — increment-1 regression.**
fable's increment-2 control probe (findings 4+5 ACCEPTED end-to-end) caught a
blocking regression from `942a89a4`, live on main: **bare single-syllable inputs
whose code has a valid shorter-syllable prefix lost their exact singles entirely.**
`mai` → only the `ma` family (嗎 嘛 馬 媽 罵…), 買/賣/麥/邁 ABSENT at every page; `wai`→外,
`xian`→先/現, `lian`→連 same. Mechanism: `leading_single_syllable_prefix_candidates`
walks PROPER prefixes (excludes the full input), so for a complete syllable the walk
can only fetch a DIFFERENT shorter-prefix family (`ma` for `mai`), which it splices
above the exacts at `leading_single_insert_index` (index 0) and truncation then drops
them. Broad/core class (mai/wai/xian/lian/nian/tian/bian/dian/mian… + every
intermediate typing state), **invisible to every existing gate** (all M59 tests
multi-syllable; parity short keys n/ni/hao have no valid-syllable prefix — the exact
"masked today" insert-index hazard the removed-behavior angle flagged, now unmasked).
Fix: skip the injection when the full `lookup_code` is itself served by single-char
exacts (`storage.exact_candidates(lookup_code)` has a 1-char row) — the exact path
owns those singles; leading-single reachability is for MULTI-syllable composition
(`zhongguo`→中, `moboyi`→莫 still inject, verified). Flag-gated → jyutping untouched.
New guard test `m59_luna_bare_syllable_keeps_full_exact_singles_on_page_zero`
(mai→買/wai→外/lian→連/dian→點-novel; dictionary exacts, not Yune) — **confirmed it
FAILS without the guard** (`page0=[嗎 嘛 馬 媽 罵]`). Gates: `m59_luna` 5/5; luna parity
14/14; `cantonese_parity` exactly the 3 pre-existing; `reach` 5/5; `yune-core --lib`
303/0; fmt/clippy clean. Ratchet straddle stays open. **Lesson: bare-syllable rows
now permanently in the suite — this class was a blind spot for every prior gate.**

**Landed 2026-07-07 (`20122e3e`, pushed) — findings 6+9 (flip preconditions, LATENT).**
Both gate the default-ON flip; inert today (reachability on only for luna, whose
config satisfies the current paths).
- **#6 structural re-key.** The untoned-relaxation admitted digit-less single rows via
  `leading_syllable_reachability && syllables.is_none()` — keyed on the FLAG. Under the
  flip that flag turns on for the TONED jyutping dict → digit-less/malformed rows would
  enter toned families and shift the M58 pins (畀@6, 諮@27). Re-keyed on CODE STRUCTURE:
  a cached `untoned_dictionary()` (no tone digit in any syllable code = untoned luna
  `mo`; toned jyutping `bei2` = false), computed once from the syllabary (all_codes
  fallback). Outer gate still keys on the flag (decides IF injection runs); only
  admission is structural. Inert: luna untoned → relaxation still fires; jyutping toned
  → unchanged.
- **#9 bounded route.** `bounded_request_supported` returned false for
  `leading_syllable_reachability + prediction_never_first` (no limit, no prefix_fallback)
  → compact `Some(limit)` fallback → injection gate false → leading single silently
  dropped (source storage full-materialises per keypress). Fix: add
  `|| self.leading_syllable_reachability` to the first clause. Inert: luna satisfies
  `!prediction_never_first`; jyutping leaves the flag off.
- **FLIP-TIME ITEM (record, do NOT fix in-series — no schema hits it today):** routing
  the flag+`prediction_never_first` combo to bounded exposes that the bounded arm does
  not HARD-enforce `prediction_never_first` without a prediction limit
  (`enforce_prediction_never_first` runs only on the compact/full path;
  `is_limited_prediction_view` needs a limit). jyutping is safe (carries
  `prediction_candidate_limit=Some(1)`); a **cangjie**-style flip schema (never_first, no
  limit) would be first to hit it → a completion could land first. Handle WITH the flip:
  a cangjie completion-never-first acceptance row, OR extend bounded-arm enforcement.
- Tests (yune-core --lib, all confirmed to BITE): `bounded_leading_single_reachable_
  under_prediction_never_first_without_limit` (#9); `untoned_dictionary_classification_
  is_structural_not_flag_keyed` (#6 classifier); `toned_classified_dictionary_rejects_
  digitless_leading_single_under_flip` (#6 re-key). Gates: `yune-core --lib` 306/0;
  `cantonese_parity` exactly the 3 pre-existing (both inert for jyutping); `m59` 5/5 +
  `reach` 5/5; luna parity 14/14; fmt/clippy clean. Ratchet straddle stays OPEN.

**Landed 2026-07-07 (`a3db53bf`/`5b12941c`, pushed) — finding #7 (oracle capture run).**
Ran the real rime/librime 1.17.0 oracle IN-ENVIRONMENT (rime.dll present under
target/, driven by scripts/oracle-rime-probe.cs).
- **Luna (owner case moboyi → 莫伯洢 + zhonggao/zhongguo-class).** Captured +
  curated the reproducible fixture `m59-luna-leading-single-composition.json`
  (scripts/capture-m59-luna-composition.ps1 + curate-m59-luna-composition.py; in the
  oracle-manifest). **librime composes 莫伯洢 (NOT in lexicon) from `moboyi`** by
  partial single selection (莫→preedit `莫bo yi`, 伯→`莫伯yi`, 洢→commit 莫伯洢); the
  rare 洢 is reachable at oracle index 155 (莫@2, 伯@19). Plus zhonggao 中@3/
  zhongguo 中@11/gao 高@0/guo 國@1. New test
  `upstream_luna_leading_single_composition` (3/3) pins moboyi→莫伯洢 + positions;
  the m59 moboyi/zhongguo tests cite it. (**Hallucination purge `<pending>`:** an
  earlier pass captured a `moboli`→莫伯李 "control" — `moboli` was an Opus typo for
  `moboyi` rationalised into a control, never an owner requirement; removed from the
  fixture, tests, plan, roadmap, requirements, and evidence. moboyi is THE case.)
  **Two divergences
  RECORDED (not asserted vs Yune):** (a) Yune's PRODUCT completion ordering differs
  from librime (zhongguo page 0 `中國大陸…` Yune vs `中國 種過…` librime; Yune injects
  中 on page 0, librime has it at index 11) — M59 asserts REACHABILITY + recompose,
  not position parity; (b) librime accumulates selected singles in the preedit and
  commits once, Yune commits incrementally (same text).
- **Cangjie (fable's 3 flip questions), answered from the oracle + inspection:**
  (1) **Composition semantics:** cangjie composes per-character via shape codes (each
  code → char(s)+phrases; existing `cangjie5-basic.json`: `a`→日曰啊, `amd`→旴 + 是一樣的).
  The M59 leading-single injection is **INERT for cangjie**: every complete shape code
  has single-char exacts, so the increment-2 bare-syllable guard skips the injection.
  Confirmed cangjie composes in Yune (`a`→日 曰…). (2) **Never-first enforcement:**
  cangjie sets NO `prediction_never_first` (verified across its config chain) → the #9
  bounded-arm enforcement gap does NOT bite cangjie; it stays a generic concern only
  for a future never_first+no-limit flip schema. (3) **Untoned-admission:** cangjie
  codes are letter-only → the #6 classifier correctly treats it untoned; but the
  admission question is **moot for cangjie** because the injection is guarded off
  (complete codes have single-char exacts → relaxation never fires). Net: **the flip is
  effectively safe/inert for shape schemas** — the bare-syllable guard already handles
  them. (Runtime cangjie flip-simulation belongs WITH the flip; the injection-inertness
  is reasoned from the guard + confirmed cangjie composes natively in Yune today.)

**Landed 2026-07-07 (`4f71c1bb`, pushed) — finding #8 (perf) + the ratchet straddle CLOSED.**
`leading_syllable_fetch_codes` rescanned the whole ~424-entry syllabary and allocated a
`String` per entry (`normalized_original_code`) on EVERY prefix boundary of EVERY
keystroke — the longest-first walk tries many empty prefixes before the first hit, so
the 37/59-char rows paid ~15-25k allocs/keystroke. Memoized to an O(1) lookup into a
`normalized_original_code(code) -> [codes]` index built once at construction (`OnceLock`,
same source/order → byte-identical fetch codes, behavior-preserving: m59 4/4, reach 5/5,
luna parity 14/14, `yune-core --lib` 306/0, bare-syllable guard intact; fmt/clippy clean).
**Ratchet: ROBUST GREEN across 3 fresh runs** on the UNCHANGED standing thresholds (no
re-baseline; nothing discarded, no run-until-green). Worst-of-3 vs ceiling: 37-char
2.045/2.094, 59-char 1.612/1.625, n 2.748/2.890, ni 2.554/2.666, hao 1.689/1.731, win
rows all <1.00 (zhongguo≤0.267, cszysmsrsd≤0.397, zybfshmsru≤0.577), startup ≤1.054/1.091,
Track A peak 186.2/195.0 MB, all Track B guards pass. fable's committed pre-#8 run FAILED
37-char 2.165 / 59-char 1.653 — the memoization pulled both under. Evidence + honest
caveats (59-char run-1 0.8% margin; startup run-2 tight) in
`docs/reports/evidence/m59-finding-8-perf/`. **This clears the perf item (1) in the
callout.** The `skip-injection-when-page-full` sub-optimization from finding #8 was NOT
needed (the memoization alone closed the straddle) — not pursued. **fable ACCEPTED
(`cd031358`): independent 4th fresh run 23/23 with `-FailOnRegression` (37-char 1.983,
59-char 1.561); four complete green runs across two sessions retire the standing debt
since `c4336cd9` on merit.** WATCH ITEM: `n` is the least-headroom row (2.859/2.890 =
1.1% in fable's run) — re-measure it against the ceiling when the default-ON flip's own
per-keystroke additions land, before any closeout ratchet claim.

**Secondary (in the series, below the cap):** returning-user unbounded lane (add a
named benchmark row); complete-path injection lacks `!has_correction_lookup`;
phantom-page off-by-one; dedup the ×2 21-line splice blocks into a helper; unify
`request_limit`/`fetch_limit`; add yune-core unit tests for the new behavior;
give the moboyi test literals fixture provenance.

**Refuted — do not chase:** cangjie/bopomofo "no injection arm" as framed (their
typing path DOES reach the `:2772` injection — the real question is cost/data-shape);
user-visible ordering change from the quality overwrite today; `custom_phrase` pin-sink;
broad non-flag blast radius (differential probes byte-identical).

**Sequencing:** (a) **1+2 first** (define the spec); (b) 3/4/5 independent surgical
fixes; (c) **6+9 are hard flip preconditions — keep holding the default-ON commit**;
(d) 7's capture gates the new acceptance tests; (e) 8 with the perf pass (the
straddle stays open — fresh-run distribution must go robustly green, no
run-until-green, no re-baseline).

## Non-Goals
- No per-input gating; no baked oracle data; no circular tests (see the three
  rules at top).
- No re-ranking/promotion beyond the oracle's order.
- No re-baselining of M55 ceilings; no run-until-green.
- No shipping the canonical rime-cantonese lane in the product / no schema-id
  rename — validation only, D-31 sign-off-gated.
- ~~M60 (schema-general reachability) is deliberately **after** M59; do not fold it in.~~
  **Superseded by the 2026-07-07 owner amendment:** the schema-general
  default-on guarantee is **in M59 scope** (see "Owner amendment (2026-07-07)").
  Only the M60 draft's capability-contract *formalism* remains future work.
