# Natural learned-phrase verification — 2026-10-13

This follow-up records the user-confirmed Cantonese phrases for the #8/#9 regression examples at source `97810ddc7e6fea5ab820b4952146bb14c0b048d9`. It supersedes the learned inputs and corresponding measurements in the [earlier local verification](issue8-local-evidence.md), which remains as a historical record. These results do not verify later PR revisions. Issues #8 and #17 remain open for the TypeDuck oracle captures and Windows ratchet; WEB-06 remains deferred.

This is a narrative summary, not an oracle fixture or an input consumed by tests. Its raw packet is retained locally outside Git and is not published with this document. The hashes below identify the retained files; independent inspection of the raw measurements requires access to those files or a new source-bound run.

## Cases and scope

| Case | Input | Text and purpose |
| --- | --- | --- |
| Browser commit/reload/retype and native timing | `cungfukhokzaap` (14 keys) | 重複學習; the real fresh candidate is committed and restored as an additional learned row |
| Native production page comparison | `coengjamzitsyujapcaksi` (22 keys) | Explicitly seeded 長音節輸入測試, with code `coeng4 jam1zit3 syu1jap6 cak1 si3`; covers the long exact multi-segment learned row |
| Retained low-level regression | `fan` | The existing `fa n6` / `FAN-LEARNED` fixture preserves the learned-row plus duplicate-heavy window case |

The long input's fresh first candidate is currently **場音節輸入測時**, not the intended phrase. Its fresh commit also records a different code, `coeng4 jam1zit3 syu1jap6 caak1 cak1 si4`. Therefore the long case is explicitly seeded; it does not claim a natural commit/retype or external-oracle result. The ABI test requires the intended seeded row to be present, then compares the complete text/comment page projection with the forced-complete first page. The short phrase retains the sentence-plus-learned duplicate-row assertion.

No engine, schema, ABI/export, or oracle fixture changed. The public registry, eight stress scenarios / 186 keys, 47-syllable sweep, cadence and latency ceilings are unchanged; the learned normal-lane input changed from nine to 14 keys. Validator diagnostics now derive counts and key names from the input, and the repeated-character mutation corrupts its second `a`.

## Sources and retained evidence

- Positive source and test source: `97810ddc7e6fea5ab820b4952146bb14c0b048d9`, clean for build and measurements.
- Negative-control artifact: `2653ecc36dbff381e8f8701a7fb099976eb99e67`, the retained #9 artifact containing the earlier faulty engine. The current test drives that artifact; its source is recorded separately from the artifact source.
- Native CLI production source: `09195a983363153a03aa3c7f97f26a8b4f49b776`. Core, API source, CLI, Cargo configuration, scripts, runtime, app and schema objects were independently confirmed identical to the positive source. Only tests and documentation changed.
- Raw packet outside Git: `issue8-natural-phrases/20261013T132900Z`. It contains native and CLI probes, exact commands, negative/positive raw diagnostics, source bindings, logs, toolchain identities and the archived public artifact. Earlier packets remain intact.
- Toolchain: Emscripten 4.0.23, Rust 1.96.1, Node 22.16.0, npm 11.16.0; Playwright 1.61.0 / Chromium 149.0.7827.55 on macOS 26.6.2 arm64. The sweep JSON records the build toolchain; the separate, locally retained `browser-tooling.json` records the browser version and executable SHA-256 (`11e393326c7d20a7c56641a7c65def33ea9c280da3b0b74cf8563b07989a0ee3`).

## Verification

The original seven-command Rust sequence remains recorded at `89d44076` in the earlier receipt. For this test-data change, the changed owning test and compilation/lint checks ran on the new source; the unchanged full workspace suite was not repeated.

| Exact command | Result |
| --- | --- |
| `cargo fmt --check` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo test -p yune-rime-api --test yune_web yune_web_adapter_real_assets_learned_multisegment_page_matches_complete_refresh -- --exact` | 1 passed, 0 failed; both new phrases and retained learned `fan` pages match complete refresh |
| `bash apps/yune-web/public-demo/build-public-release.sh` | Pass with the pinned toolchain and retained npm 11 wrapper; lockfiles unchanged |
| `node node_modules/@playwright/test/cli.js test --config playwright.latency.config.ts --workers=1 --retries=0 --grep 'normal Jyutping typing stays responsive'` | Expected failure against the old artifact: learned final worker 408 ms, above the unchanged 50 ms limit |
| `node node_modules/@playwright/test/cli.js test --config playwright.latency.config.ts --workers=1 --retries=0 --grep @issue17-sweep` | Pass: 47/155, aggregate p95 68 ms / max 197 ms; worst syllable p95 197 ms; zero timed page requests |
| `npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public` | Pass: stress 8/186, p95 122 ms / max 157 ms; normal p95 50 ms / max 55 ms; learned retype p95/max 50/50 ms and final worker 1 ms |

Focused Playwright commands ran in `apps/yune-web/e2e` with the pinned Node binary. The negative preview used port 4197 and `YUNE_WEB_EXPECTED_SOURCE_COMMIT=2653ecc36dbff381e8f8701a7fb099976eb99e67`; the sweep used port 4198 and expected source `97810ddc7e6fea5ab820b4952146bb14c0b048d9`, with `YUNE_WEB_ISSUE17_SWEEP=1`, required toolchain receipt, and the binding 4×/4×, 250 ms, 750/1000 ms profile. Evidence/output directories were fresh and external. The public runner used port 4199 with the opt-in flag unset. No measured result was retried.

Both old and current artifacts committed 重複學習, restored it after reload, grew its candidate count from one to two, and retained 14 exact prefixes and direct actions. The old final worker time was 408 ms; the current time was 1 ms. Thus the readable replacement still detects the original learned-retype performance regression.

For native timing, each 60-repeat sequence ran twice in the same seeded user directory. Second-run times were **3.197867458 s** for `cungfukhokzaa{Escape}` × 60 and **3.084229459 s** for `cungfukhokzaap{Escape}` × 60: full-minus-prefix **−0.113637999 s**, below the 0.6 s ceiling. All exits were 0; the userdb stayed byte-identical (`0b4028ddf207839061f5febed93f8e6f5d042a0b462a378752d5aa5ffb6cc35f`). This paired check is not a general speedup claim.

Raw timings, prefixes, page shapes, amplification, cadence, learned-state proof and asset hashes were independently checked. The successful build was reused after correcting an external orchestration script's nonexistent source-binding path; that error occurred before any timing started.

## Hashes

| Artifact or receipt | SHA-256 |
| --- | --- |
| Public artifact manifest | `6ff024dd920c18889c107f66105656b3567ea045ab558b614f3e498b702b6606` |
| Positive WASM | `c8013a123e235bebad9f5d6dddb043f9ea66e49d497b31d524b8c36c0bfc7711` |
| Negative WASM | `f02336e074d36c29c8aa6919395919edcc8d217ea689ce674865a92bd777d55d` |
| Public artifact archive | `e9d69519acb233614ff3f9a2d2378c417ad07eb50150fd2ecaf0205c614e83f4` |
| Negative normal/learned receipt | `c4328196011b856c189e410c81f53895b80b8dcb5836c4cfead8fb349602fed2` |
| Positive normal/learned receipt | `4dfef7d6447de53c391b1bc5a1075e9a5e16ec5911d54584d0722531742fab83` |
| Positive stress receipt | `22685b4986d24cd96eef08727b71f0b2b51f54113825576349313d240f7b08ba` |
| Syllable sweep receipt | `6656767187bf83f974598957fcf1cbf55261d972f8fcb75e042c22bf2ddacc09` |
| Separate browser/toolchain metadata | `6a5d92a4bc22ca2a042ab4a5d20a58f19e812873dbdc43c3ddd62aa068f2f4ca` |

Still pending: TypeDuck v1.1.2 page-size-6 oracle captures and production-path comparisons for the nine affected syllables; the source-bound Windows ratchet with five complete runs and all 32 aggregate checks passing. The ignored oracle test remains blocked. No production deployment was performed.
