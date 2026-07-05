# macOS Native In-Process Benchmark

This run uses the Rust `native_inprocess_benchmark` bench and loads Yune plus
a locally built upstream librime 1.17.0 dylib in process. It mirrors the
corrective Windows benchmark shape: every keypress is followed by
`get_context/free_context`.

- Track A: `luna_pinyin`, Yune versus upstream librime 1.17.0.
- Track B: Yune TypeDuck jyut6ping3_mobile product guard.
- Claim verdict: `macos-verdict.md`.
- Summary comparison: `summary-comparison.csv`.
- Claim-shape check: `claim-shape-check.csv`.
