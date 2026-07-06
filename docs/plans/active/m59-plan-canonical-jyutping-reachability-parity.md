# M59 General Single-Character Reachability Plan (reimplementation)

> **For agentic workers:** this plan was RESET on 2026-07-06 after the prior
> execution was found to be gamed and reverted. Execute one phase at a time,
> verify each against the **external oracle via the real production path**, and
> never bake oracle candidates into the engine or gate behavior on specific
> input strings.

## Status

- **Prior execution REJECTED and reverted** (`c70774ce` reverts `77a9540a`).
  The "Complete M59" commit faked reachability: per-input `match` arms on
  `bei/beingo/zijiguk` (jyutping) and `ziyiju/moboyi/yiju/boyi/yi` (luna)
  replaying oracle candidates baked into engine source
  (`m59_canonical_jyutping.tsv`), plus circular tests (engine dictionary built
  *from* the capture, rows injected *from* the capture, output asserted *against*
  the capture). The luna hardcode shipped: `moboyi` returned the oracle answer
  while `moboli` (any other input) still dead-ended. Full detail in the revert
  commit message.
- **Retained** `5d3dba2a` (the real `ni`/`hao` ratchet repair + the
  `SchemaBehaviorProfile` typed-config foundation) — audited, no input hardcodes.

## Honest baseline (`c70774ce`, verified 2026-07-06)

- **Jyutping product reachability already works, generally** — delivered by
  `c4336cd9` via `prefix_fallback_candidates` on the unbounded/page-turn path.
  `zijiguk`→諮 passes on the byte-backed product (`yune_web.rs` test green);
  works for arbitrary jyutping inputs, not a fixed list.
- **Luna reachability is a genuine GAP (not faked).** Product `luna_pinyin`
  over `moboyi` → `脈搏一 漠北一 脈搏以 漠北以 脈波一`, `is_last_page:true`;
  the leading-syllable single-char family (莫 摸 魔 …) is unreachable at any page.
  Same for any luna input. This is the real target of M59.
- Workspace compiles; jyutping guards green.

## The one requirement (owner, restated)

> I should always be able to pick the single characters one by one to compose an
> arbitrary phrase — select **any** single character to achieve **any**
> character combination matching the input syllable.

Concretely: typing `moboyi` (or `moboli`, or anything), I can page to 莫, select
it, the remainder recomposes to `boyi`/`boli`, I page to 伯, select, then 洢/李,
committing `莫伯洢`/`莫伯李`. **General**, for any input — proven on a control
input that is *not* in any test list. Jyutping already satisfies this; luna must.

## Mechanism (real, general — mirror the jyutping design)

1. **Give luna the same leading-syllable injection jyutping already has**, on the
   **unbounded / page-turn (complete-list) path only**. When the complete
   candidate list is materialized (first forward page-turn), emit the full
   leading-syllable single-character family from the luna dictionary (all `mo`
   chars for a leading `mo` syllable), ordered after the sentence/phrase
   candidates, in the dictionary's own weight order.
2. **Typing path untouched.** The per-keystroke bounded request keeps today's
   behavior and cost — so the M55 ratchet (which measures typing, never paging)
   is structurally unaffected. Only the first page-turn pays for materialization.
3. **General, no input allowlist, no baked data.** The family comes from a live
   dictionary lookup keyed by the leading syllable, for *any* input. No
   `match (input, …)`, no `.tsv`, no `CandidateSource::M59CanonicalOracle`.
4. **Selection recomposes** via existing M28 partial-selection machinery: select
   a leading single, its span commits, the remainder re-translates.
5. **Fix the premature `is_last_page`.** Luna currently marks the short sentence
   list complete; completeness must reflect the injected family so paging
   continues.

Open design question to resolve in Phase 1 by reading the code, not guessing:
whether luna storage is Compact+prism (so `bounds_compact_fallback_expansion` /
`valid_lookup_prefixes` can be reused directly) or whether a luna-appropriate
leading-syllable lookup is needed. Decide from `dictionary/source.rs` +
`translator/mod.rs:2260-2560`, recorded with evidence.

## Non-circular test discipline (the trap that sank the last attempt)

- Expected candidates come from **upstream `rime/librime 1.17.0` + upstream
  `rime-luna-pinyin/essay/stroke`**, captured with
  `scripts/capture-upstream-luna-pinyin.ps1` (oracle DLL present at
  `target/upstream-oracle/1.17.0/…`), committed with provenance. Jyutping
  cross-checks use the committed canonical rime-cantonese capture.
- Tests drive **Yune's real production path** (compiled/byte-backed product via
  `yune-cli frontend` or the `yune_web` ABI, as the jyutping `zijiguk` test does)
  and assert against the **externally-captured bytes**. Never build the engine
  dictionary from the capture; never assert a fixture against itself.
- **Mandatory control input:** at least one reachability test uses an input that
  appears in **no** allowlist and **no** capture-as-injection — e.g. `moboli` →
  compose `莫伯李` — proving generality, not memorization.

## Perf discipline

- The M55 ratchet must be **robustly** green under the standing ceilings — not
  run-until-green. Report the distribution, not a cherry-picked pair; if the
  37/59-char or Track B rows straddle a ceiling, that is a fail to fix, not a
  pair to select. Ceilings are not re-baselined.

## Phases

### Phase 0 — Baseline (DONE)
- [x] Revert the gamed execution; restore honest `main` (`c70774ce`, pushed).
- [x] Confirm luna gap reproduces and jyutping reachability intact.

### Phase 1 — Locate the luna injection point
- [ ] Trace the luna complete-list/page-turn path; decide whether to reuse
      `prefix_fallback_candidates`/`valid_lookup_prefixes` or add a luna
      leading-syllable lookup. Record the storage/prism facts with evidence.
- [ ] Capture the upstream luna oracle for the named rows **and >=1 control**
      (`moboli` or similar) with committed provenance; record Yune's pre-fix
      output alongside.

### Phase 2 — Implement the general mechanism
- [ ] Leading-syllable single-char injection on the unbounded path only,
      general, no input allowlist, no baked data; fix `is_last_page`.
- [ ] Selection/recomposition works (M28) for the injected singles.

### Phase 3 — Verify (non-circular, incl. control)
- [ ] Compiled-path tests: `moboyi`->莫->伯->洢 commit `莫伯洢`, AND the control
      (`moboli`->`莫伯李`) — asserted from captured upstream bytes via the real
      path. Jyutping guards (`zijiguk`->諮, `beingo`->畀, add `beingo`->匕) stay
      green.

### Phase 4 — Perf + gates + close
- [ ] M55 ratchet robustly green (distribution reported, ceilings held);
      WEB-03 tripwire; `cargo fmt --check`, `clippy -D warnings`,
      `cargo test --workspace`; first-page-turn materialization guard.
- [ ] Replace the residual `starts_with("jyut6ping3")` gates with typed config
      (Phase 4 cleanup carried over). Evidence README; roadmap/requirements;
      move plan to `completed/`.

## Non-Goals
- No per-input gating; no baked oracle data; no circular tests.
- No re-ranking/promotion beyond the oracle's order.
- No re-baselining of M55 ceilings.
- No elaborate "canonical validation lane" — the deliverable is general
  product reachability verified against the upstream oracle, not a separate
  staged schema. (Canonical rime-cantonese cross-check stays a verification
  input, not a shipped lane.)
