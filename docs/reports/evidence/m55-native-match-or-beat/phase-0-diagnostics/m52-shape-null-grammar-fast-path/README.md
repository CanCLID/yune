# Native In-Process Benchmark

This run uses the Rust native_inprocess_benchmark bench and loads each engine DLL directly in the measured process. It does not use the historical managed .NET/PInvoke benchmark host.

- Track A: luna_pinyin, Yune versus librime 1.17.0.
- Track B: skipped for this run.
- Track A inputs: n,ni,hao,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong.
- Track B inputs: neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung.
- Summary comparison: summary-comparison.csv.
- Threshold gate: threshold-check.csv against docs\reports\evidence\m52-track-a-guardrails-and-disposition\track-a-thresholds.csv.
