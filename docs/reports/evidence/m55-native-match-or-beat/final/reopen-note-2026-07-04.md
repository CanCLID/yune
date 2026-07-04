# M55 Reopen Note - 2026-07-04

Superseded status: this is a historical reopen record. The current M55 closeout
is `../phase-5-final/closeout-2026-07-04.md`, which closes M55 green after the
Phase 3R/4/5 follow-up work. Keep this file as evidence for the reopen point;
do not treat it as the current milestone status.

At the Phase 2R reopen point, the earlier Phase 2 no-go call remained valid as
evidence for the `YUNE-POET/1` byte-backed poet access path: it reduced Track A
memory, but could not hold the full M55 latency ratchet.

Step 0 corrects the landing error from that closeout. Runtime consumption of
`<dict>.poet.bin` is now explicitly opt-in with `YUNE_POET_BYTE_BACKED=1`;
default main ignores present poet artifacts at the consumption point. The deploy
path may still emit and validate poet artifacts, but default `luna_pinyin`
does not attach the byte-backed poet source unless the flag is set.

Evidence:

- Reviewer red verification that triggered the reopen:
  `docs/reports/evidence/m55-native-match-or-beat/final/review-m52-gate-verify/`
- Step 0 green M52 re-verification:
  `docs/reports/evidence/m55-native-match-or-beat/reopen/step-0-m52-green/`

Step 0 M52 threshold result:

- 37-character Luna row: `2.940x` against `3.267x` ceiling.
- 59-character Luna row: `2.310x` against `2.447x` ceiling.
- Track A peak working set: `186,200,064 B` against `198,000,000 B` ceiling.

The default-off fix returns the startup/latency gate to green and returns Track A
memory to the M52-era shape (`~186 MB` in this run). The `~110.5 MB` peak from
the Phase 2 closeout is now treated as flag-on research/no-go evidence, not the
default product path.

At that point, Phase 2R had to redesign the poet artifact/access path behind
the flag. The flag could become default-on only in the commit where the full M55
ratchet was green with byte-backed poet enabled in two consecutive same-run
benchmark runs and Track A memory was at or below `125 MB`. Until that happened,
flag-on benchmark results were research evidence and did not gate main.
