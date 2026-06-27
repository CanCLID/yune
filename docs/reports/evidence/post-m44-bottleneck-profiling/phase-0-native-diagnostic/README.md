# Native In-Process Benchmark

This run uses the Rust native in-process benchmark and loads each engine DLL
directly in the measured process. It does not use the historical managed
.NET/PInvoke benchmark host.

- Track A: luna_pinyin, Yune versus librime 1.17.0.
- Track B: jyut6ping3_mobile, Yune Cantonese profile/product path.
- Track A inputs: `n`, `ni`, `hao`, `zhongguo`,
  `ceshiyixiachangjushuruxingnengzenyang`,
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`,
  `cszysmsrsd`, `zybfshmsru`.
- Track B inputs: `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`.
