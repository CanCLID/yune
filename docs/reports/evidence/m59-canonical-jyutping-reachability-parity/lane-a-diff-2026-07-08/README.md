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

## Root-cause hypotheses (to verify before disposition)

1. **The M59 flip's leading-single injection is over-riding librime's natural
   candidate generation on the canonical lane (DOMINANT).** Reachability is ON for
   this lane (Standard profile, flip default — landmine disposition). The
   over-admission is injected single chars + phrases (`畀我啊`, `唔該你喇`, `比亞迪汽車`);
   the order divergence is the injection ranking the leading-single family first
   (碑悲卑…, 那挐南…) where librime ranks by frequency (畀比被…, 我你呢…). This is the
   `injected single-char family` D-48 flagged — but it **reorders the top**, not just
   extends the tail. **Verification pending: a reachability-OFF canonical run of `bei`**
   — if it collapses toward 畀比被鼻避, the injection is confirmed as the primary owner.
2. **Segmentation / fuzzy-correction gaps (secondary, real).** `nri`→**0** (Yune emits
   nothing; oracle fuzzes nri→nei→你, 1309) and `ngohaig` 46 vs 2050 point to Yune's
   canonical fuzzy/segmentation diverging from librime independent of the injection.
   Needs its own diagnosis.

## Disposition question — OWNER SIGN-OFF (per D-48, no self-disposition)

The core tension is **D-47 (reachability ON, schema-general) vs D-48 (order-exact
parity for Lane A)**: on the canonical lane the injection that delivers reachability
is what breaks order parity. Options to weigh (not chosen here):
- (a) **Suppress the injection on prefix-having canonical inputs** — reachability
  comes from librime's own natural candidate set (which already contains the single
  chars); the injection only fires where librime dead-ends.
- (b) **Inject AFTER the oracle range** — preserve librime's order/set through the
  captured range, append the family beyond it (order-exact holds; over-admission
  becomes a named beyond-oracle-depth exception).
- (c) **Owner-signed exception** citing the reachability feature, scoping Lane A
  order parity to a librime-comparable sub-range.

Plus: the `nri→0` / `ngohaig` segmentation-fuzzy gap is a **separate named work item**
needing its own oracle-backed diagnosis (not the injection).

Raw per-input rows: `classified.json`.
