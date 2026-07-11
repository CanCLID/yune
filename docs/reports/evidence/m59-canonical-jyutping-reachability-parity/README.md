# M59 Canonical Jyutping Reachability Parity — Evidence Ledger

> **Historical packet classification (2026-07-06):** the prior M59 "fix" was gamed and reverted (`c70774ce`
> reverts `77a9540a`). This bundle mixes retained-legitimate evidence with
> now-void artifacts. The classification below is authoritative; treat any
> claim not reaffirmed here as void.

Acceptance is canonical-first (owner-confirmed 2026-07-06): **Lane A** — Yune +
pinned `rime/rime-cantonese` vs `librime 1.17.0` + `rime-cantonese`; **Lane B** —
Yune `luna_pinyin` vs `librime 1.17.0` + upstream luna. TypeDuck profile is a
regression guard, not an acceptance oracle.

> **Current update (2026-07-11):**
> [`increment-4a-luna-script-translation-order/`](./increment-4a-luna-script-translation-order/)
> records the Luna long-input `ScriptTranslation` repair, independent review,
> and fresh final-binary five-round macOS diagnostic at engine commit
> `89875ee2`. Both 37- and 59-character first pages now match pinned
> librime exactly in all five runs; Yune and librime dylib hashes and the full
> candidate snapshots remain single-valued. The repair separates librime's
> one-best sentence from its phrase stream and honors the natural-log weight
> domain plus inclusive 5% pronunciation boundary of compiled tables. The
> packet creates no threshold or exception and does not close M59-PARITY-02:
> six of the seven complete-list rows remain residuals.
>
> [`increment-3b-transformed-algebra/`](./increment-3b-transformed-algebra/) is
> the accepted 3b packet for implementation commit `2cb7e411`. Its
> schema-general mechanism, seven
> required deploy rows plus Stroke control, external oracle properties,
> deterministic product rebuild, focused release gates, and owner-signed
> five-round Windows ratchet are green (`32/32` aggregate rows). M59-REACH-02 is
> complete. The packet does not close M59, M59-REACH-03/04, or a D-48 ordering
> lane; Increment 4a follows as the separate Luna sentence/phrase repair above.
>
> [`increment-2-profile-paging/`](./increment-2-profile-paging/) is the green
> M59-NAV-01 acceptance packet for implementation commit `e37ee011`. It records
> the mechanism-honest Cantonese 38/41 -> 41/41 and Windows 2/4 -> 4/4 repair,
> unified Engine/native/API/browser forward-navigation policy, source-current
> Emscripten artifacts, real-browser `zi -> 諮` in four PageDown operations, and
> the informational algebra/CJ-1/OpenCC/risk checkpoint. The packet contains no
> generated binary payloads and does not close a D-48 ordering lane or M59.
>
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
>
> [`../m59-cangjie5-order-parity/increment-1-executable-evidence/`](../m59-cangjie5-order-parity/increment-1-executable-evidence/)
> is the corresponding seven-file Cangjie executable diagnostic from clean
> commit `c7c04ff7...`. It preserves both raw and curated oracle files and has a
> strict exact verdict of 4 passed / 8 failed (exit `1`, no exceptions). The
> curated oracle replaces the old mojibaked fixture metadata byte-for-byte while
> leaving all candidate/page arrays unchanged. CJ-1 remains open.

## Artifact classification

### VALID — Increment 4a macOS repair evidence (supplemental acceptance)

- `increment-4a-luna-script-translation-order/README.md` — authored repair and
  five-run diagnostic report. It closes only the expanded 37/59 page-zero
  sentence/phrase sub-slice, records all five external run paths and stable
  binary/candidate hashes, preserves the superseded red packet and setup
  retries, and leaves the broader Lane B complete-list requirement open.
- Raw benchmark outputs and portable HTML remain external by protocol; only the
  authored report and non-circular pinned-librime fixture are tracked.

### VALID — retained Increment 1 executable diagnostics (not acceptance)

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
