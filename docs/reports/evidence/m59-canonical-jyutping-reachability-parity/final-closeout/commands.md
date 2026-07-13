# Commands

All commands ran from the repository root unless a working directory is shown.
Cargo artifacts used `C:\m59-final-native-target`; browser evidence used
`C:\m59-final-closeout-5fa986d8`.

## Native

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The workspace test stopped from host memory pressure during
`cantonese_parity`. Completed targets were retained, then only the missing
surface resumed:

```powershell
$env:CARGO_TARGET_DIR='C:\m59-final-native-target'
cargo test -p yune-core --test cantonese_parity -- --test-threads=1
cargo test -p yune-core --test oracle_fixture_provenance --test 'upstream_*'
cargo test -p yune-rime-api
cargo test -p yune-core --doc
cargo build --release -p yune-rime-api
```

## Web build and package

```powershell
npm.cmd --prefix apps/yune-web run typecheck
npm.cmd --prefix apps/yune-web/e2e ci
```

```bash
set -euo pipefail
cd /c/Users/laubonghaudoi/Documents/GitHub/yune
export EMSDK_QUIET=1
source /c/Users/laubonghaudoi/tools/emsdk/emsdk_env.sh >/dev/null
unset CARGO_TARGET_DIR CARGO_ENCODED_RUSTFLAGS RUSTFLAGS \
  CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER
export YUNE_WEB_WASM_REQUIRE_EMSCRIPTEN=1
./scripts/yune-web-wasm-build.sh
```

```powershell
npm.cmd --prefix apps/yune-web run build:public
npm.cmd --prefix apps/yune-web run fetch:octagram-dev-model
npm.cmd --prefix apps/yune-web run build
npm.cmd --prefix apps/yune-web run start -- --host 127.0.0.1 --port 5173 --strictPort
```

## Functional browser acceptance

The Playwright Node entrypoint was used so Windows did not parse the regex
alternation as a command pipe. No `--max-failures` override was used. All Node
commands below ran from `apps/yune-web/e2e` with one worker, the same fresh
WASM/server, and these environment variables:

```powershell
$env:YUNE_WEB_APP_URL='http://127.0.0.1:5173'
$env:YUNE_WEB_EVIDENCE_DIR='C:\m59-final-closeout-5fa986d8\web\playwright-evidence'
```

The first npm-script attempt ran from the repository root. PowerShell parsed
the regex alternation as a pipeline, so this was a launcher setup failure, not
a behavior result; its short output was overwritten before preservation:

```powershell
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- yune-web.spec.ts --grep 'WEB-04|M58' --workers=1
```

The accepted focused run used the Node entrypoint:

```powershell
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep 'WEB-04|M58' --workers=1
```

The initial remainder run exposed the asynchronous test assumptions while
preserving serial-suite behavior:

```powershell
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts web05-control-surface.spec.ts --grep-invert 'WEB-04|M58' --workers=1
```

The M28 recovery and current WEB-05 controls were then run independently:

```powershell
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep 'M28 PARTIAL selecting a prefix candidate keeps the tail composing' --workers=1
Remove-Item Env:WEB05_PUBLIC_DEMO_E2E -ErrorAction SilentlyContinue
node node_modules/@playwright/test/cli.js test --config playwright.config.ts web05-control-surface.spec.ts --grep-invert 'same-WASM' --workers=1
```

The first disjoint remainder was listed and executed with the same pattern:

```powershell
$pattern='WEB-04|M58|WASM heap metrics|Default Jyutping composes|UI language switcher|M31 PUBLIC|M31 UX|M24 startup timing|M25 DOGFOOD-01|M25 DOGFOOD-03|M26 PERF startup attribution|M27 PERF startup marker|M29 PERF startup|M27 PERF controls|M28 PARTIAL selecting'
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep-invert $pattern --list
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep-invert $pattern --workers=1
```

The M20 test was repeated once to confirm the failure and once after its
test-only synchronization fix:

```powershell
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep 'M20 Prediction never first persists schema customization' --workers=1
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep 'M20 Prediction never first persists schema customization' --workers=1
```

The final 22-test tail deliberately excluded the historical DOGFOOD-02 row.
Its first listing included that row (23 listed); the second listing verified
the exclusion count before the 22-test execution:

```powershell
$pattern='M20 guided scenarios|M20 combine_candidates|M20 prediction threshold|Shift toggles ASCII mode|M20 live session controls|M20 display controls|M16 |Candidate paging$|Keyboard paging shortcuts|Candidate selection|Number keys commit|Deletion removes|Backspace mutates|Deploy returns|Customize returns|Persistence sync|Reload/reinitialize'
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep $pattern --list
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep $pattern --grep-invert 'M25 DOGFOOD-02' --list | Select-Object -Last 3
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep $pattern --grep-invert 'M25 DOGFOOD-02' --workers=1
```

The number-key recovery, final six tests, and M58 shared-helper recovery were:

```powershell
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep 'Number keys commit visible candidates' --workers=1
$pattern='Deletion removes|Backspace mutates|Deploy returns|Customize returns|Persistence sync|Reload/reinitialize'
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep $pattern --workers=1
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep 'M58 yune-web TypeDuck profile reaches oracle-ranked reported candidates' --workers=1
```

After independent review identified stale-diagnostic risk, the final focused
recovery reset the diagnostic stream before each affected action and re-ran
only those two tests. The server used the already-built assets and was launched
directly to avoid another prepare/build cycle:

```powershell
# from apps/yune-web
node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 5173 --strictPort

# from apps/yune-web/e2e
node node_modules/@playwright/test/cli.js test --config playwright.config.ts yune-web.spec.ts --grep 'M20 Prediction never first persists schema customization|Number keys commit visible candidates' --workers=1
```

Every preserved test receipt is under `logs/web/`; the review recovery is
`22-playwright-sync-review-recovery.txt`.

The only reruns were the specific failed tests after their test-only fixes:

```text
M28 PARTIAL selecting a prefix candidate keeps the tail composing
M20 Prediction never first persists schema customization
Number keys commit visible candidates
M58 yune-web TypeDuck profile reaches oracle-ranked reported candidates
```

The final M58 rerun was required because its shared selection helper gained a
state-change wait. WEB-04 did not use either modified helper and retained its
source-current 3/3 result.
