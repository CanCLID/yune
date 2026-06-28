# WEB-03 Latency Regression Fix - Local Browser Evidence

Date: 2026-06-28 local

This focused Playwright probe uses the rebuilt local public-demo bundle after
the bounded sentence/prefix fallback fix. It covers the Track A Luna inputs,
the Track B Jyutping inputs, and the `ngogokdak` phrase regression guard.

Files:

- `samples.json` - full per-schema browser samples, startup memory, and console
  error arrays.
- `summary.csv` - flat latency and WASM heap rows for report tables.
- `final-quick-jyutping.json` - final sanity check after the last WASM rebuild,
  covering the affected Jyutping rows only.

Important read:

- The deployed public page before this fix was measured separately on
  2026-06-28 and reproduced the Jyutping long-input regression:
  `sihaacoenggeoisyujapgecukdou` exact keydown-to-paint `3764 ms`,
  `taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng` `1518 ms`, with
  WASM memory still `160.0 MiB`.
- The rebuilt local public-demo after this fix records the same two inputs at
  `130 ms` and `74 ms`, with ready/peak WASM memory still `160.0 MiB`.
- The final quick Jyutping-only sanity check after the last WASM rebuild records
  the same two inputs at `142 ms` and `78 ms`, also with ready/peak WASM memory
  at `160.0 MiB`.
- Luna remains at `64.0 MiB`; the 37- and 59-character Luna rows record
  `43 ms` and `75 ms`.

This evidence is a browser-harness latency follow-up. It does not change the
native Track A verdict and does not claim a new memory reduction beyond the
already measured WEB-03 `160.0 MiB` byte-backed launch path.
