# Native In-Process Benchmark

This run uses the Rust native_inprocess_benchmark bench and loads each engine DLL directly in the measured process. It does not use the historical managed .NET/PInvoke benchmark host.

- Track A: luna_pinyin, Yune versus librime 1.17.0.
- Track B: jyut6ping3_mobile, Yune Cantonese profile/product path.
- Track A inputs: n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru.
- Track B inputs: neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung.
- Summary comparison: summary-comparison.csv.
- Threshold gate: threshold-check.csv against docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv.

## CORRECTION (2026-07-07) — this independent run also measured injection-OFF luna

This run deployed the **source** product schema
(`apps/yune-web/source/public/schema/luna_pinyin.schema.yaml`), which carries no
`leading_syllable_reachability` flag. Under the pre-flip `schema_install`
(`…unwrap_or(false)`) the leading-single injection was therefore **OFF**, so the
23/23 result here — like the sibling `run-1..3` — certified the *no-injection*
luna path, not the memoized injection path it was intended to validate. The M59
default-ON flip is the first ratchet to run with the injection actually on
(`m59-flip-ratchet` / `m59-flip-skip-ratchet`); treat those as the first honest
measurement of the mechanism's cost, not a regression from these green runs. See
`../README.md` "CORRECTION (2026-07-07)" for the full disposition. Standing lesson:
**benchmark schema provenance must match the shipped product config under test.**
