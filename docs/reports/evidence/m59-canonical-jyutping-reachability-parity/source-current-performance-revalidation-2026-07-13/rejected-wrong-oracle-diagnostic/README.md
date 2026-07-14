# Rejected wrong-oracle diagnostic

These text receipts preserve every failed attempt made before the upstream Luna
oracle was reprovisioned. They are **not acceptance runs** and are not inputs to
the parent packet's `gate-verdict.csv`.

All five `443cc636` attempts used upstream shared/build trees
`ba465db53332222393aeb8703f94e20a3fb0e33e2e4973484eb4d2b98b74a281` /
`17a5ed0cafeb085cf546cff4669f5662ff973f6cd3e6ab321fa9e0c11ba41855`.
Every typed-input latency row passed, but the compiled upstream tree installed
an extra component per session and made the same three environment-sensitive
rows red:

| Run | Peak working set (ceiling `195,028,378`) | Startup ratio (ceiling `1.091`) | Session µs (ceiling `25,470.280`) |
| --- | ---: | ---: | ---: |
| `443cc636` run 1 | `208,318,464` | `4.606` | `289,125.100` |
| `443cc636` run 2 | `208,449,536` | `5.234` | `287,660.900` |
| `443cc636` run 3 | `208,437,248` | `4.844` | `289,326.300` |
| `443cc636` run 4 | `208,531,456` | `5.441` | `286,169.900` |
| `443cc636` run 5 | `208,543,744` | `5.102` | `283,991.000` |
| `5879405c` control | `208,662,528` | `5.037` | `286,409.600` |

Run 1 recorded `status=failed` because it used the fail-on-regression wrapper;
runs 2-5 and the control completed measurement while retaining their red rows.
The clean pre-fix `5879405c` control reproduces the same three failures with the
same tree hashes, demonstrating that the reds do not originate in the
`443cc636` collector repair. The accepted parent runs instead use reprovisioned
trees `3801c4c83ba919e531b80ac27e2c06d116d08b19af2034fcb86e6e17ae1eecf6` /
`7f8ce0b50e8acb3d5e66db55fb17879073e5be05a3a7cdc582745fe1e73bf39c`.

Each subdirectory retains the exact invocation, environment, source/DLL/tree
hashes, run status, summary comparison, and threshold check. Bulky samples,
compiled schemas, DLLs, and executables are intentionally excluded.
