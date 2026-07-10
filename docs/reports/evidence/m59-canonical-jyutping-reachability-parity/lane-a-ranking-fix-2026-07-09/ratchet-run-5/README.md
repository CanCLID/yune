# Native In-Process Benchmark

This run uses the Rust native_inprocess_benchmark bench and loads each engine DLL directly in the measured process. It does not use the historical managed .NET/PInvoke benchmark host.

- Track A: luna_pinyin, Yune versus librime 1.17.0.
- Track B: jyut6ping3_mobile, Yune Cantonese profile/product path.
- Track A inputs: n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru,zh,j,yi,che,chuang,b,ceshi,zhongdengchangdu,dazisudu.
- Track B inputs: neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung.
- Summary comparison: summary-comparison.csv.
- Threshold gate: threshold-check.csv against docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv.
