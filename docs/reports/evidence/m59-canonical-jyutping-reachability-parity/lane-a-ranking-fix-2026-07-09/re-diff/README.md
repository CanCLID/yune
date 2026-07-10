# Lane A re-frozen 13-input classified diff (post-fix, 2026-07-09)

Method: Yune paged CLI runs (`{Page_Down}` to `is_last_page` or the page cap) over the
staged rime-cantonese + `lane-a-runner/default.yaml`, mirroring the oracle capture's own
paging methodology; compared against `canonical-13-input-oracle-capture.json` (librime
1.17.0 `33e78140`, dll sha256 recorded in the capture header; rime-cantonese `c99b16e4`;
counts cross-match the pre-fix `../../lane-a-diff-2026-07-08/classified.json` o_counts
row-for-row). Page caps: 90 page-downs (≈450 rows) for `ngohaig`/`n`/`nri`, oracle-range
+2 pages for the rest — rows marked `y_capped` compare through the captured range only.
Classifier: `re-classified.json` (first-seen candidate order, typing states excluded).

## Fixed by the tone-merge re-rank (order-exact through the captured range)

| input | result |
|---|---|
| `be` | exact through the oracle's **complete** 4-row list |
| `beix` | exact through the oracle's **complete** 36-row list |
| `bei` | exact through 138 of 139 after removing ONE dropped char (祕, class 2) — remaining deltas are two adjacent equal-weight tie swaps (class 3) and cap-artifact tail unders (啤/唄/𠹇 sit beyond the 31-page Yune capture) |

Over-admission on these inputs starts only at the tail (e.g. `bei` first extra at index
135, after the singles family) — the owner-required reachability extension beyond oracle
depth, per D-47/M59 injection rules.

## Residual divergence classes (disposition table below)

1. **Phrase/sentence-path ordering** — `being`, `beingo`, `beixngoxx`, `mgoi` (head@1),
   `zijiguk` (Yune sentence `知而焗咁` first vs oracle `諮議局`). Multi-syllable ranking,
   a different mechanism than the fixed bare-syllable tone-merge. Largest remaining class;
   `zijiguk`'s first row is the original M58 motivating input.
2. **Variant-sibling admission drop** — oracle carries both 秘@8 and 祕@9 for `bei`; Yune
   emits 秘 once and never 祕 (essay weights: 祕=2579, 秘 absent — consistent with a
   variants-conversion + duplicate-text dedup on Yune's side where librime keeps both
   rows). One char observed; class may recur on other variant pairs.
3. **Equal-weight tie order** — 詖↔疕 (339=339), 粃↔柲 (327=327): stable storage order vs
   librime's tie order. Predicted by the fix review (tie regime); user-invisible ranking
   quality (identical weights).
4. **Abbreviation/segmentation expansion** — `n` (455+ vs 1309, head 那 vs 我), `nri`
   (0 vs 1309), `ngohaig` (46 vs 2050), `ngohaigo` (46 vs 113), `bein` (head class).
   The pre-declared step-4 class, independent of ranking.
5. **Beyond-oracle-depth tail** — over-admission after the oracle range on every input
   (completions/lettered/injected family), ordering governed by the M59 injection rules.

## Disposition table (owner sign-off, D-48)

| class | recommendation | rationale |
|---|---|---|
| 1 phrase/sentence ordering | **named work item** (next Lane A increment) | real user-visible parity gap (`zijiguk`→諮議局 first is the original bug's surface); mechanism = sentence/phrase scoring, not tone-merge |
| 2 variant-sibling drop | **named work item** (small) | admission loss; suspect simplifier-conversion dedup; verify mechanism then fix |
| 3 equal-weight ties | **owner-signed exception** (revisit only if the diff grows) | weights identical — no ranking-quality difference; matching librime's tie rule would require pinning its merge order |
| 4 segmentation/abbreviation | **named work item** (already filed as the step-4 gap) | independent mechanism; `nri`→0 is a hard functional gap |
| 5 beyond-oracle tail | **owner-signed exception per D-47** | the owner's own reachability requirement; ordering follows M59 injection rules |

Classes 1/2/4 do not block the tone-merge fix landing (different mechanisms, all
pre-declared in the fix scope); classes 3/5 are exception rows for the owner to sign.

**OWNER SIGN-OFF (2026-07-09):** classes 3 and 5 SIGNED as exceptions, with two owner
amendments: (a) class 3 carries an optional fidelity follow-up — read librime's actual
tie semantics from source (it is open source; the tie order is an incidental
implementation detail, not an opaque one) and match it if it reduces to a simple rule
(e.g. dict source row order); (b) class 2's work item is CLASS-WIDE, not bei-specific —
enumerate every variant-sibling pair sharing a code across the whole canonical dict, fix
at the mechanism, and pin oracle-backed rows drawn from the enumeration.

**Validity note:** the post-review refinements (tolerance-domain exclusion, `total_cmp`)
are provably inert on this lane — the staged canonical dict declares no tolerance rules
(grep: 0) and all weights are finite — and the `bei`/`be`/`beix` heads were re-verified
byte-identical on the final rebuilt binary after the refinements landed.

**ERRATA (2026-07-09, GPT review P2):** the first committed `re-classified.json` labeled
`nri` as `match` — a classifier edge-case bug (a zero-length Yune list produced an empty
comparison window, so 0-vs-1309 scored as trivially exact; the prose class-4 row above was
always correct). The classifier now treats an empty Yune list as full-oracle
under-admission and compares uncapped-short lists against the complete oracle list;
`re-classified.json` is regenerated (`nri`: under=1309; `ngohaig`: under=2005).
