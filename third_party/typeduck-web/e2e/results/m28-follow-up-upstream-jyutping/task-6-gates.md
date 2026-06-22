# M28 Follow-up Task 6 Gates

Date: 2026-06-22

## Commands

| Gate | Result | Notes |
| --- | --- | --- |
| `cargo fmt --check` | Pass | Re-run after formatter and Clippy fixes. |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass | Initial Clippy wildcard warning fixed in `partial_consumed_len_for_commit`. |
| `cargo test -p yune-core --test upstream_luna_pinyin_parity` | Pass | 12 passed. Caught and verified the upstream sentence-model gating regression. |
| `cargo test -p yune-core --test cantonese_parity` | Pass | 35 passed. Caught and verified the `nri` abbreviation fallback default-confirm regression. |
| `cargo test -p yune-rime-api --test typeduck_web -- --test-threads=1` | Pass | 28 passed, finished in 918.50s. Serial run used to make the long real-asset cases observable. |
| `cargo test --workspace` | Pass | Includes `typeduck_web`; 28 `typeduck_web` tests passed in 931.01s inside the workspace run. |
| `npm.cmd --prefix packages/yune-typeduck-runtime test` | Pass | 5 files, 65 tests. |
| `npm.cmd --prefix packages/yune-typeduck-runtime run build` | Pass | TypeScript compile passed. |
| `scripts/typeduck-wasm-build.sh` with Emscripten env | Pass | Fresh `yune-typeduck.js` and `yune-typeduck.wasm` copied to `third_party/typeduck-web/source/public/`. |
| `npm.cmd --prefix third_party\typeduck-web\source run build` | Pass | Vite build and worker bundle passed. |
| `npm.cmd run test:e2e -- --grep "M28 FOLLOW-UP" --workers=1` from `third_party\typeduck-web\e2e` | Pass | 2 browser tests passed in 28.3s. |
| `git apply --reverse --check ..\patches\yune-typeduck-runtime.patch` from TypeDuck-Web source | Pass | Checked against the edited nested source checkout. |
| `git apply --check ...\third_party\typeduck-web\patches\yune-typeduck-runtime.patch` from a clean temporary source worktree | Pass | Worktree used locked TypeDuck-Web `03f9afd2cf6ca75653197f2193f24d1cd0adbd83` with schema submodule initialized. |
| `git diff --check` | Pass | Exit 0; PowerShell reported line-ending normalization warnings for edited docs only. |

## Evidence

- Hybrid oracle capture: `third_party/typeduck-web/e2e/results/m28-follow-up-upstream-jyutping/oracle-capture.md`
- Target decision: `third_party/typeduck-web/e2e/results/m28-follow-up-upstream-jyutping/target-decision.md`
- Ranking diagnosis: `third_party/typeduck-web/e2e/results/m28-follow-up-upstream-jyutping/phrase-prefix-diagnosis.md`
- Browser Space/default-confirm evidence: `third_party/typeduck-web/e2e/results/m28-follow-up-upstream-jyutping/browser-space-default-confirm.json`
- Browser ranking evidence: `third_party/typeduck-web/e2e/results/m28-follow-up-upstream-jyutping/browser-upstream-ranking.json`

## Notes

- The upstream Jyutping fixture captured sentence-first `caksijathaacoenggeoizi` ranking and did not capture the originally hypothesized `測試` phrase-prefix row. The accepted implementation therefore targets the fixture-backed sentence segmentation/ranking delta, not an invented phrase-prefix expectation.
- Stock upstream logs the missing TypeDuck `dictionary_lookup_filter`; this remains out of scope for the hybrid composition/ranking oracle and under the TypeDuck v1.1.2/profile-comment boundary track.
- Space/default-confirm now recomposes only partial rows marked safe for default recomposition. One-letter abbreviation fallbacks keep legacy raw-tail default-confirm behavior, preserving the `nri -> 我ri` TypeDuck v1.1.2 fixture.
