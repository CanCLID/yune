# M59 Canonical Parity + General Reachability Plan (reimplementation)

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
      2,3,4,7 failed = run-until-green on code == current `main`).

### Phase 1 — Re-validate the retained lanes + captures (do not redo blindly)
- [ ] **Lane A (largely retained in `5d3dba2a`):** re-validate the staged
      rime-cantonese lane actually loads/deploys and that
      `yune-canonical-rime-cantonese-load-*.json` came from Yune's real path;
      re-verify capture provenance/checksums. Confirm the shipped
      `jyut6ping3.dict.yaml` is NOT the pinned canonical data (README records a
      SHA mismatch — verify). Defuse `is_typeduck_jyut6ping3_profile` so the
      canonical lane does not inherit TypeDuck shims (typed config, M23 pattern).
      **Add the missing control jyutping input** to the capture.
- [ ] **Lane B:** trace the luna complete-list/page-turn injection point; record
      storage/prism facts. Add the `moboli` control to the luna capture.
- [ ] Any newly captured rows committed with full provenance; Yune pre-fix output
      recorded alongside.

### Phase 2 — Re-validate / extend the diff (retained pre-fix diff exists)
- [ ] The frozen pre-fix diff (`phase-2/canonical-pre-fix-diff.json`) is retained
      and real — re-validate it reproduces from the current honest baseline, then
      extend it with the control inputs. Classified: reachability /
      selection-recomposition / admission over-under / order-only. This diff —
      not any model, not a baked table — is the Phase 3 spec.

### Phase 3 — Implement per the diff (general mechanism)
- [ ] Luna leading-syllable injection (unbounded path, general, no allowlist, no
      baked data); fix `is_last_page`; ordering matches the capture.
- [ ] Any Lane-A canonical fixes the diff proves, scoped so Track A/luna typing
      is untouched.
- [ ] Compiled-path, non-circular tests incl. **both control inputs**
      (`moboli`→莫伯李; a non-named jyutping control). Jyutping guards
      (`zijiguk`→諮, `beingo`→畀, add `beingo`→匕) stay green.

### Phase 4 — Perf, gates, docs, close
- [ ] M55 ratchet robustly green (distribution reported, ceilings held) — this
      re-measures post-revert main too; WEB-03 tripwire; `cargo fmt --check`,
      `clippy -D warnings`, `cargo test --workspace`; first-page-turn
      materialization guard.
- [ ] Replace residual `starts_with("jyut6ping3")` gates with typed config.
- [ ] **Docs (Low):** add active-M59 to `roadmap.md`/`requirements.md`; evidence
      README finalized; move plan to `completed/`.

## Non-Goals
- No per-input gating; no baked oracle data; no circular tests (see the three
  rules at top).
- No re-ranking/promotion beyond the oracle's order.
- No re-baselining of M55 ceilings; no run-until-green.
- No shipping the canonical rime-cantonese lane in the product / no schema-id
  rename — validation only, D-31 sign-off-gated.
- M60 (schema-general reachability) is deliberately **after** M59; do not fold it in.
