# Lane A classified diff (FROZEN) — 2026-07-08

Yune's real production canonical path (pinned rime-cantonese, via the lane-a-runner)
vs the committed oracle capture (`librime 1.17.0` + rime-cantonese `c99b16e4…`).
Per-direction classification per D-48 (order-exact through the captured range).
**This is a disposition-table candidate for owner sign-off — NOT self-disposed.**

## Methodology (fable-required)
- Paged runner: `--sequence "<input>{Page_Down}…"` to depth, page size 5, mirroring
  how the oracle capture was collected (NOT the complete-path list — different path
  class). Warm user dir. Runner: `../lane-a-runner/`.
- Per-input page budget `min(ceil(oracle_count/5)+12, 90)`. **The 3 huge inputs
  (`ngohaig` 2050, `n`/`nri` 1309) are capped at 90 pages (450 candidates)** — their
  under-admission counts are lower bounds through the paged depth.
- Guard: Yune total count + `is_last_page` recorded vs the oracle count, so admission
  classifies separately from order. `classified.json` has the raw rows.

## The diff (every input diverges, all four directions)

| input | Yune# | last? | oracle# | 1st order Δ | under | over | classes |
|---|---|---|---|---|---|---|---|
| be | 70 | no | 4 | idx1 | 0 | 66 | over-admission / order |
| bei | 205+ | no | 139 | idx0 | 4 | 70 | under / over / order |
| bein | 70 | no | 5 | idx0 | 5 (all) | 70 | under / over / order |
| being | 151 | yes | 140 | idx0 | 5 | 16 | under / over / order |
| beingo | 148 | yes | 142 | idx0 | 6 | 12 | under / over / order |
| beix | 65 | no | 36 | idx0 | 36 (all) | 65 | under / over / order |
| beixngoxx | 100 | no | 38 | idx0 | 2 | 64 | under / over / order |
| ngohaig | 46 | **yes** | 2050 | idx0 | 2005 | 1 | under / over / order |
| ngohaigo | 46 | yes | 113 | idx0 | 68 | 1 | under / over / order |
| n | 455+ | no | 1309 | idx0 | 1212 | 358 | under / over / order |
| nri | **0** | — | 1309 | — | 1309 (all) | 0 | under-admission (total) |
| mgoi | 11 | yes | 4 | idx1 | 0 | 7 | over-admission / order |
| zijiguk | 387 | yes | 416 | idx0 | 30 | 1 | under / over / order |

Representative tops (Yune | oracle): `bei` 碑悲卑陂蓖 | 畀比被鼻避 · `n` 那挐南哪 | 我你呢諗女 ·
`zijiguk` 知而焗咁… | 諮議局子怡… · `mgoi` 唔該… (matches idx0) then 唔該你喇 | 唔該唔呣嘸.

## Root cause — VERIFIED, and my first hypothesis was WRONG

**Reachability-OFF canonical `bei` run (2026-07-08): IDENTICAL to reachability-ON —
碑 悲 卑 陂 蓖 羆 萆 鵯 犤 庳 椑 詖.** Toggling the M59 flip's leading-single injection
OFF did **not** change the order. **The injection is NOT the primary owner of the
Lane A divergence** (it was my surfaced hypothesis; the verification refutes it).

The real owner is **Yune's BASE canonical candidate ranking for rime-cantonese**:
Yune ranks 碑 (bei1) and obscure chars (羆萆鵯犤) high where librime ranks the common
畀 (bei2 "give") / 比 / 被 by dictionary weight. So Yune's weight/frequency/sort
handling for the rime-cantonese dictionary diverges from librime's candidate
generation — this is a **candidate-generation defect, not a feature tension.**
Root-cause candidates to diagnose: dict weight loading (the multi-file
`import_tables` merge + `essay-cantonese` vocabulary weighting), the by_weight sort,
and tone-vs-frequency ordering.

Secondary, still real: `nri`→**0** (oracle 1309) and `ngohaig` 46 vs 2050 — a
segmentation/fuzzy divergence, its own diagnosis.

## Disposition — this is a DIAGNOSIS increment, not (yet) an owner exception

Because the divergence is a base candidate-generation defect (not the reachability
feature), the D-47-vs-D-48 tension I first framed **does not apply as the primary
lever**. The next step is **root-cause diagnosis** of Yune's rime-cantonese ranking
vs librime (weights → sort → tone), then a fix by mechanism against the capture, per
the three hard rules. Only genuinely-beyond-oracle rows (if any survive the ranking
fix) become owner-signed-exception candidates. `nri`/`ngohaig` fuzzy gap is a
separate named work item.

**Lesson recorded:** I over-attributed to the injection and surfaced it as dominant;
the reachability-OFF control run corrected it. Verification before disposition is
exactly why D-48 forbids self-disposition — the control run changed the diagnosis.

Raw per-input rows: `classified.json`.
