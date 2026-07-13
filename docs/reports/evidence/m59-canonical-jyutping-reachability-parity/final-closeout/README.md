# M59 final closeout evidence

Status: **VALID — M59 closes under the full D-47/D-48 contract.**

The production source under test is `5fa986d80a6f1481f7c04c64af41467d3767bf61`.
The final E2E synchronization fixes, policy update, documentation, and this
packet land in the commit containing this file; they do not modify production
Rust or browser-runtime code.

## Final verdicts

- D-47 performance: the final behavior source `5879405c` remains green at
  `32/32` aggregate rows and `160/160` individual observations under the signed
  ceilings. No production code changed after that source; REACH-03 at
  `5fa986d8` changed manifests, coverage metadata, tooling, and tests only.
- D-48 Lane A: strict `13/13` and all `5,705` captured positions exact.
- D-48 Lane B: all seven captured inputs exact by candidate text and position;
  the deployed 37-character and 59-character page shapes are exact.
- D-48 Cangjie: marked fixture `12/12`, owning suite `3 passed / 0 ignored`,
  unmarked lane `12/12` unchanged.
- REACH-03: 60 manifest assets, 10 schema-asset dispositions, three
  configuration carriers, and nine executable validation rows.
- WEB-04: the source-current browser gate proves all four plain/Octagram rows,
  the plain-Luna default-off control, and missing-model fail-closed fallback.

## Native closeout

The first exact `cargo test --workspace` run stopped during
`cantonese_parity` after a 301,989,888-byte allocation failed. It reported no
assertion failure, and its already-completed format, full-workspace Clippy,
CLI, core-library, and provenance targets were retained. To avoid an expensive
duplicate run, only the failed and never-reached targets were resumed:

- `cantonese_parity`: `41 passed / 0 failed` with one test thread;
- remaining Yune-core integration targets: all executed tests passed, with
  eight declared ignores preserved;
- `cargo test -p yune-rime-api`: 476 executed tests passed, four declared
  ignores preserved;
- Yune-core and API doc-tests: green;
- `cargo build --release -p yune-rime-api`: green.

See [native verdict](native-verdict.md) and `logs/native/`.

## Web and package closeout

- app typecheck: green;
- fail-closed Emscripten build with
  `YUNE_WEB_WASM_REQUIRE_EMSCRIPTEN=1`: green;
- runtime preparation and public package build: green; all manifest assets were
  hash-verified while copying the 132,572,773-byte schema payload;
- pinned Octagram model: 10,513,408 bytes and SHA-256
  `574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3`;
- tracked app build: green;
- unique current functional Playwright coverage: 79 passed and five declared
  public-mode skips across `yune-web.spec.ts` and the current WEB-05 controls.

The functional run was split into disjoint subsets so already-passed tests were
not repeated after serial failures. Every failure and targeted recovery is
preserved in `logs/web/`. After independent review, only the two affected
synchronization rows were re-run; both passed, and their shared receipt is also
preserved.

See [web verdict](web-verdict.md), [artifact hashes](artifact-hashes.csv), and
`browser/`.

## Explicit non-gates

- The 150-sample startup benchmark was not run. It is a historical measurement
  suite with no M59 correctness threshold and would add substantial duplicate
  runtime.
- WEB-05's same-WASM baseline was not rebaselined. It intentionally pins an
  older WASM hash and is not a valid cross-binary M59 correctness comparison.
- The WEB-05 public-demo-only control and four M31 public-mode cases retain
  their declared environment-gated skips; the public package itself built and
  verified successfully.

No binary payloads are stored here. The WASM, JS glue, model, screenshots, and
compiled assets are represented only by text receipts and hashes.

## Packet map

- `provenance.txt` — source, pins, toolchain, and gate dispositions.
- `artifact-hashes.csv` — generated WASM/JS/model sizes and SHA-256 values.
- `commands.md` — exact command sequence and recovery boundaries.
- `native-verdict.md` — retained and resumed native gate accounting.
- `web-verdict.md` — build/browser accounting and stale-test fixes.
- `d47-d48-reconciliation.md` — final decision/requirement reconciliation.
- `logs/` — normalized text receipts, including all failures.
- `browser/` — M58/WEB-04 JSON and text receipts only.
- `packet-manifest.csv` — path, byte size, and SHA-256 for every packet file
  except the manifest itself.
