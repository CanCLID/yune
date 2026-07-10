# M59 Increment 0 signed closeout baseline

This curated evidence slice records the fresh pre-implementation performance
baseline at source commit `457751824b8944676dc44912b9ce31ff29d78403`.
The owner signed all nine newly derived latency-ratio ceilings on 2026-07-09.
They are installed in
`../m55-native-match-or-beat/thresholds/m55-thresholds.csv`.

The checked-in executable aggregate was regenerated from clean source commit
`fbf02362b14873e93dc744852015e84909b6ca57`. `gate-verdict.csv` passes all
32 aggregate-median rows against the signed, unloosened ceilings (17 Track A
latency rows, the standing startup/session/memory guards, and the existing
Track B product rows). Its SHA-256 is
`55ca3e1781fea2bc90ba1802d151e2843d6aea0b5bf5b2396670fa1c58ba6959`.
The version-6 atomic-publication sidecar is
`gate-verdict.provenance.json`, SHA-256
`ff2c03ac8c09e979fb88a2e04ef60d1b368f2e8cc57428f0f5e7332e647d26cb`;
it binds the five run inputs, thresholds, effective invocation, tool bytes,
and gate bytes without recording checkout- or user-specific host paths.

This is the executable replay of the pre-implementation Increment 0 baseline,
not the final post-behavior-change M59 performance acceptance. M59-REACH-04
therefore remains open until five fresh expanded Track A and Track B rounds are
captured and aggregated from the final implementation commit.

Five completed logical runs (`r1`, `r2`, `r3`, `r4`, and `r5-retry1`)
measured the same immutable Yune DLL. The 23 previously standing rows passed
their unchanged aggregate median gates. Those rows include both the
37-character
`ceshiyixiachangjushuruxingnengzenyang` case and the 59-character
`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` case;
neither long-row ceiling was re-baselined here.

## Signed rows

`expanded-ceiling-derivation.csv` contains all five observations, the pooled
worst value, spread, exact `worst x 1.05` derivation, and F3 ceiling. The signed
set is:

| Input | Ceiling |
| --- | ---: |
| `zh` | 1.047 |
| `j` | 4.372 |
| `yi` | 6.098 |
| `che` | 1.160 |
| `chuang` | 1.357 |
| `b` | 3.775 |
| `ceshi` | 0.966 |
| `zhongdengchangdu` | 0.342 |
| `dazisudu` | 1.098 |

The historical `yi` derivation in
`../m59-flip-final/expanded-row-baselines.csv` is corrected from `6.163` to
`6.164` (`5.870 x 1.05 = 6.1635`, formatted F3). That historical row is
superseded by the fresh signed `6.098` ceiling.

## Evidence map and boundaries

- `SIGNATURE-SUMMARY.txt`, `expanded-ceiling-derivation.csv`,
  `standing-gate-verdict.csv`, and `historical-vs-fresh-ceilings.csv` record the
  signature basis and verdicts.
- `gate-verdict.csv` and `gate-verdict.provenance.json` are the fail-closed,
  executable 32-row aggregate and its atomic-publication provenance. They cover
  the complete 17-input Track A set (including the 37- and 59-character rows),
  standing startup/session/memory guards, and Track B product rows, including
  the existing 61-character product input.
- `fixed-round-provenance.txt`, `run-provenance.csv`, `round-state.txt`,
  `BUILD-ISOLATION-DISCLOSURE.txt`, and `COMMANDS-ERRATUM.txt` record source,
  binary, workspace, and replay boundaries.
- Each completed run directory preserves only the top-level comparison,
  threshold, M37, memory-owner, command, environment, external-provenance, and
  authoritative invocation files needed to audit the derivation.
- `failed-premeasurement-r5/` records the excluded exit-101 setup failure and
  its 182-file transient manifest. It contains no measurement rows; the
  unchanged retry is logical run 5.
- `rejected-variable-dll-round/` records why the earlier five-run round was
  rejected: its benchmark invocation rebuilt and changed the measured DLL
  between runs.
- `full-local-evidence-file-manifest.csv` documents the complete fixed-DLL
  local bundle. `committed-slice-manifest.csv` inventories this curated slice.

The complete raw fixed-DLL and rejected-variable-DLL bundles remain at
`C:\y59b\docs\reports\evidence\m59-closeout-baseline-fixed` and
`C:\y59b\docs\reports\evidence\m59-closeout-baseline`, respectively, until
M59 closeout. They are local preservation paths and are not claimed committed.
The curated tracked slice intentionally contains no fixed DLL, per-engine raw
subdirectories, console logs, samples, or deploy workspace payloads.

In every completed run, `actual-invocation.txt` is the authoritative replay
record. The generated `commands.txt` omitted the explicit fixed `YuneDll` and
external `UpstreamOracleRoot` arguments; `COMMANDS-ERRATUM.txt` documents that
limitation.
