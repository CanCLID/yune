# M59 Canonical Jyutping Reachability Parity — Evidence Ledger

> **Status (2026-07-06):** the M59 "fix" was gamed and reverted (`c70774ce`
> reverts `77a9540a`). This bundle mixes retained-legitimate evidence with
> now-void artifacts. The classification below is authoritative; treat any
> claim not reaffirmed here as void.

Acceptance is canonical-first (owner-confirmed 2026-07-06): **Lane A** — Yune +
pinned `rime/rime-cantonese` vs `librime 1.17.0` + `rime-cantonese`; **Lane B** —
Yune `luna_pinyin` vs `librime 1.17.0` + upstream luna. TypeDuck profile is a
regression guard, not an acceptance oracle.

## Artifact classification

### VALID — retained from `5d3dba2a` (the real perf fix commit)
- `phase-1/canonical-rime-cantonese-capture.json` — Lane A oracle capture with
  full provenance (librime `33e78140…`, rime-cantonese `c99b16e4…`, `rime.dll`
  SHA-256 `86b4c735…`). Real. Needs a **control input** added.
- `phase-1/yune-canonical-rime-cantonese-load-bei.json` — Yune's **real**
  production path over staged pinned rime-cantonese (`bei` → 碑 悲 卑 陂 蓖,
  `is_last_page:false`): a genuine ordering divergence from the oracle
  (畀 比 被 鼻 避). Legitimate; the canonical lane really was stood up.
- `phase-1/upstream-luna-pinyin-m59-*.json` — Lane B upstream luna captures +
  scenario snapshots.
- `phase-2/canonical-pre-fix-diff.json` and the `yune-canonical-*` /
  `yune-upstream-luna-*` page captures — the **real** frozen pre-fix diff
  (Yune's real output vs oracle). This is the Phase 3 spec. Named pre-fix gaps:
  canonical `beingo` 畀@page0/idx3 & 匕@page6/idx4 unreached; `zijiguk` 諮議局-first
  & 諮@page45/idx2 unreached; luna `ziyiju` 諮@page5/idx3 unreached; luna `moboyi`
  committed 脈搏一 (oracle: 莫伯洢).

### REJECTED — gamed, reverted with `77a9540a`
- All Phase 3 fix artifacts and `phase-4-final-ratchet-run8/9` (deleted by the
  revert). Any prior text asserting a working M59 fix is void — the "fix"
  replayed oracle candidates baked into `m59_canonical_jyutping.tsv` behind
  per-input `match` arms with circular tests.

### NOISY / MISLEADING — Phase 0 ratchet runs (correcting the prior README)
- `phase-0-baseline-ratchet-run1/` — real: reproduced the red ratchet after
  `c4336cd9` (`ni`/`hao` etc.). Keep as baseline.
- `phase-0-restored-ratchet-run1..9/` — the perf fix's ratchet attempts. The
  prior README claimed "run8/run9 passed" and called only run7 a failed noisy
  rerun. **Both corrections:** run8/9 were **deleted by the revert** (do not
  cite), and **runs 2, 3, 4, and 7 all failed** (37/59-char and Track B rows
  straddling ceilings), not just run7 — i.e. run-until-green. This is not
  acceptable acceptance evidence.
- **Post-revert ratchet is UNMEASURED.** The surviving numbers included the
  now-reverted reachability overhead; current `main` must be re-benchmarked in
  Phase 4 and must be *robustly* green under the standing ceilings.

## Capture commands (for reproduction; provenance to be re-pinned in Phase 1)
Lane A: `scripts/capture-upstream-rime-cantonese.ps1 … -Inputs bei,beingo,zijiguk,<control> …`
Lane B: `scripts/capture-upstream-luna-pinyin.ps1 … -ScenarioInput …luna-pinyin-m59-scenarios.json …` (add `moboli` control).
Ratchet: the standing M55 command with `-DeployProductBeforeBenchmark -FailOnRegression` and the M55 thresholds csv.
