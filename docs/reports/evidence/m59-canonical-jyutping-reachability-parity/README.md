# M59 Canonical Jyutping Reachability Parity — Evidence Ledger

> **Status (2026-07-06):** the M59 "fix" was gamed and reverted (`c70774ce`
> reverts `77a9540a`). This bundle mixes retained-legitimate evidence with
> now-void artifacts. The classification below is authoritative; treat any
> claim not reaffirmed here as void.

Acceptance is canonical-first (owner-confirmed 2026-07-06): **Lane A** — Yune +
pinned `rime/rime-cantonese` vs `librime 1.17.0` + `rime-cantonese`; **Lane B** —
Yune `luna_pinyin` vs `librime 1.17.0` + upstream luna. TypeDuck profile is a
regression guard, not an acceptance oracle.

> **Current update (2026-07-10):**
> [`increment-1-executable-evidence/`](./increment-1-executable-evidence/) is
> the current six-file, source-clean, canonical-LF, hash-bound Lane A executable
> diagnostic. Its strict exact verdict is deliberately red (`0/13`, comparator
> exit `1`, no exceptions); it is pre-fix evidence only and does not close D-48
> or M59.
>
> [`increment-1-lane-b-executable-evidence/`](./increment-1-lane-b-executable-evidence/)
> is the corresponding six-file Lane B executable diagnostic from clean commit
> `94c1c61d...`. Its strict exact verdict is also deliberately red (`0/7`,
> comparator exit `1`, no exceptions). The fresh oracle is byte-stable under a
> same-path replay and has been imported into the owning upstream fixture with
> unchanged behavior arrays plus hardened curator-v6 provenance. This packet is
> likewise pre-fix evidence only and does not close D-48 or M59.

## Artifact classification

### VALID — current Increment 1 executable diagnostics (not acceptance)

- `increment-1-executable-evidence/lane-a-oracle.json` — fresh pinned upstream
  oracle raw capture with source/schema cleanliness, binary/tool, option,
  command, output, canonical-text, and paging provenance.
- `increment-1-executable-evidence/lane-a-yune.json` — raw Yune capture with
  clean source, DLL/tool, tree, schema-narrowing, option, command,
  canonical-text, and complete paging provenance.
- `increment-1-executable-evidence/lane-a-exact-diff.json` and
  `lane-a-exact-diff.csv` — strict exact comparator outputs: 0 passed, 13
  failed, expected exit `1`, no exception file.
- `increment-1-executable-evidence/lane-a-manifest.json` — packet-local closure
  for both raw captures, both diff views, packet README, generation/import and
  canonical-text provenance, commands, effective parameters, and all 13
  verdicts. It omits only its own recursive hash.
- `increment-1-lane-b-executable-evidence/lane-b-oracle.json` — fresh pinned
  upstream Luna oracle capture over the ordered seven-row Lane B set, with
  clean schema/tool source, binary/tool, option, command, output,
  canonical-text, same-path replay, and complete paging provenance.
- `increment-1-lane-b-executable-evidence/lane-b-yune.json` — raw Yune capture
  derived from that same fresh oracle, with explicit schema narrowing, clean
  source, DLL/tool/tree/option provenance, and complete paging state.
- `increment-1-lane-b-executable-evidence/lane-b-exact-diff.json` and
  `lane-b-exact-diff.csv` — strict exact comparator outputs: 0 passed, 7 failed,
  expected exit `1`, no exception file.
- `increment-1-lane-b-executable-evidence/lane-b-manifest.json` — packet-local
  six-file closure, fixture-import equivalence, replay hashes, commands,
  effective identity, and all seven red verdicts; it omits only its own
  recursive hash.

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
  At the 2026-07-06 revert baseline, `main` was **identical to `5d3dba2a`**
  (the revert removed only `77a9540a`'s code), so these runs measured that
  historical baseline. Result:
  **5 pass (runs 1, 5, 6, 8, 9), 4 fail (runs 2, 3, 4, 7)** — the 37/59-char and
  Track B rows straddled their ceilings. So that 2026-07-06 baseline was
  *measured but not robustly green* (run-until-green), which is not valid
  acceptance evidence; and the straddle is **not** reachability overhead
  (reachability was added later, in the reverted `77a9540a`). The then-planned
  Phase 4 required these rows to become robustly green on fresh runs under the
  then-standing ceilings.
- What the revert **deleted** was `phase-4-final-ratchet-run8/9` (the gamed
  Phase 4 closeout) — **not** the `phase-0-restored` runs. (This corrects this
  README's own earlier draft, which conflated the two.)

## Capture commands (for reproduction; provenance to be re-pinned in Phase 1)
Lane A: `scripts/capture-upstream-rime-cantonese.ps1 … -Inputs bei,beingo,zijiguk,<control> …`
Lane B: `scripts/capture-upstream-luna-pinyin.ps1 … -ScenarioInput …luna-pinyin-m59-scenarios.json …`.
Ratchet: the standing M55 command with `-DeployProductBeforeBenchmark -FailOnRegression` and the M55 thresholds csv.
