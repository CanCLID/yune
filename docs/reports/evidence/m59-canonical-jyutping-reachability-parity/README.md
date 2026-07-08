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

### MEASURED BUT NOT ROBUSTLY GREEN — Phase 0 ratchet runs
- `phase-0-baseline-ratchet-run1/` — real: reproduced the red ratchet after
  `c4336cd9` (`ni`/`hao` etc.). Keep as baseline.
- `phase-0-restored-ratchet-run1..9/` — **all nine exist and are tracked.**
  Current `main` code is **identical to `5d3dba2a`** (the revert removed only
  `77a9540a`'s code), so these runs **do** measure current code. Result:
  **5 pass (runs 1, 5, 6, 8, 9), 4 fail (runs 2, 3, 4, 7)** — the 37/59-char and
  Track B rows straddle their ceilings. So current `main` is *measured but not
  robustly green* (run-until-green), which is not acceptable acceptance
  evidence; and the straddle is **not** reachability overhead (reachability was
  added later, in the reverted `77a9540a`). Phase 4 must make these rows robustly
  green on fresh runs under the standing ceilings.
- What the revert **deleted** was `phase-4-final-ratchet-run8/9` (the gamed
  Phase 4 closeout) — **not** the `phase-0-restored` runs. (This corrects this
  README's own earlier draft, which conflated the two.)

## Capture commands (for reproduction; provenance to be re-pinned in Phase 1)
Lane A: `scripts/capture-upstream-rime-cantonese.ps1 … -Inputs bei,beingo,zijiguk,<control> …`
Lane B: `scripts/capture-upstream-luna-pinyin.ps1 … -ScenarioInput …luna-pinyin-m59-scenarios.json …`.
Ratchet: the standing M55 command with `-DeployProductBeforeBenchmark -FailOnRegression` and the M55 thresholds csv.
