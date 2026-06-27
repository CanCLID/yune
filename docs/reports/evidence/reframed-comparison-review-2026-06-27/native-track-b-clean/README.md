# Native In-Process Benchmark

This run uses the Rust `native_inprocess_benchmark` bench and loads each engine DLL directly in the measured process. It does not use the historical managed .NET/PInvoke benchmark host.

- Track A: luna_pinyin, Yune versus librime 1.17.0.
- Track B: jyut6ping3_mobile, Yune Cantonese profile/product path.
- Track A inputs: $TrackAInputs.
- Track B inputs: $TrackBInputs.

Track B validity: this run used `-DeployProductBeforeBenchmark`.
`product_path_status.csv` records `compiled_ready=true`,
`selected_storage=byte_backed`, table/prism `mmap`, and `source_fallback=false`
for both `jyut6ping3` dictionaries. Use this directory as the clean
2026-06-27 Track B rerun.
