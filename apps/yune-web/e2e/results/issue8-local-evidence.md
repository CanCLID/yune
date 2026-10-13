# Issue #8 / #17 local verification — 2026-10-13

The local verification passed. Issues #8 and #17 remain open for the TypeDuck reference captures and the source-bound Windows ratchet. This receipt records a local macOS browser build; no production deployment or WEB-06 work was performed.

## Source and evidence identity

- Base/native executable source: `09195a983363153a03aa3c7f97f26a8b4f49b776` (merged #21 / #9).
- Rust gate source: `89d440765de1efaf7b6e6eae098f0e4736b4032e`, tree `390ff30929f85f22eca5606661d30f6515419516`, clean throughout all seven commands.
- Final browser source: `3a71792900b7e4c6d798286101a8b98843aabd06`, tree `224004ce6c6c026864e911c50fecabe59089e2ec`, clean at build and measurement.
- The `crates`, `scripts`, `Cargo.toml`, `Cargo.lock`, runtime package, app source, schema assets, and public latency runner Git objects are identical across these three sources. Only the browser spec and its instructions changed after the Rust run; this receipt is a subsequent documentation-only addition.
- Raw evidence is retained outside Git: packet `issue8-local-evidence/20261013T080035Z`, final browser attempt `attempt-2`; native packet `issue8-native-retype/20261013T080104Z`. The external packets include commands, logs, raw per-key diagnostics, failed attempts, toolchain metadata, source binding, and the archived public artifact.
- Input identity: the 47 bare entries of Rust `BOUNDED_PAGE_SYLLABLES`, in order, 155 keystrokes. JSON UTF-8 SHA-256: `c928cace8b2103fe8c2f18bc2ca2dc1ae4190703e74585c09605bf45da0b0233`.

## Exact Rust commands

| Command | Result | Wall time |
| --- | --- | ---: |
| `cargo fmt --check` | Pass | 1.311 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass | 10.463 s |
| `cargo test --workspace` | 28 targets; 1,188 passed, 0 failed, 13 ignored | 4,498.879 s |
| `cargo test -p yune-core --test cantonese_parity` | 42 passed, 0 failed, 1 ignored | 306.350 s |
| `cargo test -p yune-core --test upstream_luna_pinyin_parity` | 26 passed, 0 failed, 3 ignored | 1.800 s |
| `cargo test -p yune-core --test upstream_cangjie5_composition_parity` | 3 passed, 0 failed, 0 ignored | 0.792 s |
| `cargo test -p yune-rime-api --test yune_web` | 56 passed, 0 failed, 2 ignored | 2,610.383 s |

These commands ran once as the requested sequence. The workspace and explicit ABI test both executed `yune_web_adapter_real_assets_bounded_syllable_page_matches_complete_refresh` and `yune_web_adapter_real_assets_learned_multisegment_page_matches_complete_refresh`: the production bounded/complete pages matched, including `sikgaanei`, `ngohaihoenggongjan`, and `fan`. The bounded-refresh counters, learned-row placement test, out-of-window complete-refresh guard, first-page comparison, and unrelated-userdb guard all passed. Existing parity fixtures and the ignored TypeDuck oracle test were unchanged.

## Native learned retype

The release CLI used real `jyut6ping3_mobile` assets and one learned `sik6gaa3 nei5` / 食價你 row. Each sequence ran twice in the same user directory; the second run was measured, with 60 repeats per sequence. The independent parent-agent measurement was:

| Sequence | Time |
| --- | ---: |
| `sikgaane{Escape}` × 60 | 6.322223792 s |
| `sikgaanei{Escape}` × 60 | 6.353971458 s |
| Full minus prefix | **0.031747666 s**, pass against ≤0.6 s |

Both commands exited 0. The userdb stayed byte-identical: SHA-256 `a7f3dc03aacef466c684dcf13bb42c87999c0539a8ca40b5aba37636abf12016`. The external receipt retains the exact CLI argument arrays, sequence hashes, binary hash, warm runs, measured runs, and source binding.

## Browser build and commands

The pinned `bash apps/yune-web/public-demo/build-public-release.sh` succeeded with Emscripten 4.0.23, Rust 1.96.1, Node 22.16.0, and npm 11.16.0. The first npm 10.9.2 attempt failed frozen-lock validation on an existing optional Vite/esbuild peer. The npm 11 recovery kept all lockfiles unchanged; its exact package inventory and wrapper are retained externally. No dependency or application source was changed.

Chromium headless shell 149.0.7827.55 / Playwright 1.61.0 ran on macOS 26.6.2 arm64. The receipt pins the SDK repository/release, compiler, Node, browser executable hash, and full served inventory. All 74 public artifact files were byte-verified before the sweep and independently rehashed afterward.

With the clean source commit above and a preview at `http://127.0.0.1:4198/`, the focused command was:

```sh
YUNE_WEB_APP_URL=http://127.0.0.1:4198/ \
YUNE_WEB_EXPECTED_SOURCE_COMMIT=3a71792900b7e4c6d798286101a8b98843aabd06 \
YUNE_WEB_REQUIRE_TOOLCHAIN_RECEIPT=1 YUNE_WEB_ISSUE17_SWEEP=1 \
YUNE_WEB_LATENCY_CPU_THROTTLE=4 YUNE_WEB_LATENCY_KEY_INTERVAL_MS=250 \
YUNE_WEB_LATENCY_P95_MS=750 YUNE_WEB_LATENCY_MAX_MS=1000 \
YUNE_WEB_LATENCY_EVIDENCE_DIR="$EXTERNAL_SWEEP_DIR" \
YUNE_WEB_LATENCY_OUTPUT_DIR="$EXTERNAL_PLAYWRIGHT_DIR" \
node node_modules/@playwright/test/cli.js test --config playwright.latency.config.ts --grep @issue17-sweep --workers=1 --retries=0
```

The working directory was `apps/yune-web/e2e`; `node` was the pinned Node binary. Output paths were fresh directories outside every Git worktree. The unchanged public command then ran from the repository root, with the opt-in flag unset and an external output directory:

```sh
npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public
```

| Lane | Result |
| --- | --- |
| New opt-in sweep | 47/47 syllables, 155/155 prefix diagnostics; aggregate p95 65 ms, max 185 ms; worst per-syllable p95 185 ms |
| Sweep profile | Page size 6; 250 ms cadence; 4× main thread and worker amplification; six final candidates for every input; zero timed page requests |
| Font/stress evidence | All 1,491 declared font faces loaded before typing; 155 positive measured worker ratios; zero timer-resolution-zero samples in the accepted run |
| Unchanged public stress gate | 8 scenarios / 186 keys; aggregate p95 123 ms, max 159 ms; release/profile/candidate guards passed |
| Exact normal input | p95 50 ms, max 50 ms |
| Learned `sikgaanei` retype | p95 49 ms, max 49 ms; final action worker **1 ms <50 ms** |

The parent independently recomputed percentile/max values, prefixes, page sizes, cadence, amplification, request absence and artifact hashes from raw receipts. The existing scenario registry and runner remain unchanged. This sweep measures page shape and timing; the Rust comparisons prove bounded/complete self-consistency. Neither substitutes for external TypeDuck candidate-order captures.

## Failure retention and review

The first sweep at `232c11a6` completed 47/155 but was rejected: lazy font requests occurred during typing, cross-context timestamp subtraction exceeded 2 ms by one floating-point unit, and one sub-clock-resolution worker sample lacked a finite amplification ratio. That raw red remains retained. Test-only corrections preload all declared font faces without typing or warming the engine, use a sweep-only 2.001 ms clock tolerance (default public gate stays 2 ms), and explicitly classify exact zero samples while requiring a positive calibration for each syllable. The accepted run used the corrected source; no measured run was retried without a diagnosed correction.

Two focused boundary tests passed independently in the parent and both review agents. Separate requirement-compliance and code-quality/evidence reviews found no actionable issues. Earlier failure probes demonstrated that a failed atomic receipt replacement preserves the incomplete seed, a changed Cangjie prism is rejected before timing, existing receipt paths cannot be overwritten, worktree evidence paths are rejected, and relaxed ceilings fail preflight. Those probes are source-bound in their retained logs; they were not represented as full browser acceptance runs.

## Artifact and receipt hashes

| Evidence | SHA-256 |
| --- | --- |
| Public artifact manifest | `f0af03f7457c0972da898b080bfc46885b99596408ce1964b34662996a833716` |
| Schema manifest | `5565dfbaab79ce2d5309418338c2ed446138f3958a0023bceebcfd4c6311be86` |
| WASM | `c8013a123e235bebad9f5d6dddb043f9ea66e49d497b31d524b8c36c0bfc7711` |
| Archived public artifact | `86eaebb1594b1d58d074de19a46a3f5743993dff7a41b4460e60f366e56f6008` |
| Issue #17 raw sweep | `076dd16804791879c40359eb6241bea18c1164ed6a4cc5e52cfd2db0672fb47b` |
| Public stress receipt | `e64261b4831b2df1bf1b41d89ef2839e0bad7b874f8562bd38da2b3c13301e0c` |
| Normal/learned-retype receipt | `64dead785ef5fd0baad6c2de170a81d3d395e5dea8d8cfdaa9847a97b57b756b` |

## Still pending

- TypeDuck-HK/librime v1.1.2 page-size-6 external oracle rows for `fan`, `hou`, `man`, `lei`, `zung`, `dou`, `siu`, `wui`, and `wan`, followed by production-path comparisons. `typeduck_bounded_first_page_oracle_for_duplicate_text_syllables_is_blocked` remains ignored with its explicit blocker.
- The source-bound Windows performance ratchet: five complete runs, aggregated against `scripts/native-ratchet/m55-thresholds.csv`, with all 32 aggregate checks passing.
- Issues #8 and #17 remain open. WEB-06 remains deferred.
