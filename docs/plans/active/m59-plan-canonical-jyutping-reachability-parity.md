# M59 Canonical Parity + General Reachability Plan (reimplementation)

> **PROGRESS (2026-07-07): Lane B luna reachability MECHANISM landed (`c89a8ea9`)
> and independently confirmed genuine** (fable's review: greps clean, novel input
> `weibozi`→155 wei singles proves generality). `moboyi`→`莫伯洢`, control
> `moboli`→`莫伯李`, source-truthful non-circular tests, phrase-before-single
> ordering, no regressions. **M59 is NOT closeable** — three open items:
> **(1) PERF STRADDLE (corrected):** the "ratchet green twice" claim was favorable
> sampling; an independent run failed `n`/37-char/59-char (pre-existing standing
> straddle, Lane B perf-neutral) — needs a real fix, no run-until-green. **(2) Owner
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
  path. Acceptance: `moboyi`→莫→伯→洢 commits `莫伯洢`, **and the control
  `moboli`→莫伯李** (an input in no test list), asserted against the upstream
  librime+luna capture via the real path.

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
  `target/upstream-oracle/1.17.0/…`); capture named rows **and the `moboli`
  control**, page size and all-pages recorded.

## Perf discipline
The M55 ratchet must be **robustly** green under the standing ceilings — report
the full run distribution, not a cherry-picked pair. If the 37/59-char or Track B
rows straddle a ceiling, that is a fail to fix, not a pair to select. Ceilings are
not re-baselined.

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
      Compact+prism → bounded syllabary); `moboli` control added as a real-path
      test (not a capture).
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
      `m59_luna_*`): `moboyi`→莫伯洢 + control `moboli`→莫伯李 + phrase-before-single
      ordering; jyutping `zijiguk`→諮 / `beingo`→畀 stay green.
- [ ] **(Lane A)** non-named jyutping canonical control; `beingo`→匕 named guard.

### Phase 4 — Perf, gates, docs, close
- [x] Lint/format + focused suites green: `cargo fmt --check`, `clippy -D
      warnings`; luna `m59_luna` 3/3, `upstream_luna_pinyin_parity` 14/14,
      jyutping `m58` reachability 3/3.
- [ ] **PERF STRADDLE — OPEN (not green).** The earlier "ratchet green twice,
      healthy margins, not straddling" claim was **favorable sampling and is
      retracted**: an independent run (`luna-lane-ratchet-claude-verify/`) failed
      `n` 2.947 (>2.890), 37-char 2.165 (>2.094), 59-char 1.653 (>1.625). These
      rows STRADDLE their ceilings (pre-existing standing debt; Lane B is
      perf-neutral by design). M59 cannot close on perf until this is genuinely
      fixed — distribution reported, no re-baseline, no run-until-green.
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
7. **`moboli` control has no oracle provenance** — capture it + `zhonggao`/`zhongguo`
   -class rows (the acceptance inputs finding 1 needs) in the same run.
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
