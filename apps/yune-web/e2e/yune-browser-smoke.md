# yune-web Manual Browser Smoke Procedure

Fallback procedure for real browser validation when automated browser runner unavailable (per D-08, D-09).

## Purpose

Manual real-browser smoke test for the tracked `yune-web` app and Yune runtime seam. This procedure is ONLY for browser/tooling blockers — it MUST still use a real browser, NOT package-local fake tests.

## Prerequisites

1. Tracked `apps/yune-web/` Vite app
2. Yune WASM artifact with `yune_web_*` exports
3. Built `@yune-ime/yune-web-runtime` package
4. Explicit tracked schema assets under `apps/yune-web/public/schema/`
5. Node.js and npm
6. Modern browser (Chrome/Firefox/Safari)

## Procedure

### Automated Playwright Entry Points

The automated suite is the primary real-browser gate when Playwright is available:

```bash
npm --prefix apps/yune-web/e2e run test:e2e
```

The suite currently contains 28 tests. The full `test:e2e` run is the merge/honesty
gate and must remain green with the same assertions.

For inner-loop work only, a representative smoke subset is tagged with `@smoke`:

```bash
npm --prefix apps/yune-web/e2e run test:e2e:smoke
```

The smoke subset covers composition, candidate-list rendering, the M20
prediction-never-first control, M16 sentence composition parity, and M13 AI-off
identity/source-label safety. Passing smoke is useful during development, but it
does not replace the full 28-test gate.

Worker, engine, schema, and schema-delivery changes also own a focused browser
latency hard stop. This command runs only the latency matrix, never the broad
suite:

```bash
npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public
```

The command starts an exact preview of the already-built
`public-demo/dist`, requires its source/hash-bearing `build-info.json`, fully
reconciles the local package to `public-artifact-manifest.json`, verifies the
served worker/app/WASM/schema-manifest bytes, and proves the split Jyutping prism
startup path. Release packaging pins Rust `1.96.1`, Emscripten `4.0.23`, and
the SDK-provided Node `22.16.0`; the receipt rejects an ambient toolchain or a
binary built by different versions. It uses
keydown-to-double-`requestAnimationFrame` paint-proxy
diagnostics (the established `totalKeydownToPaintMs` metric, not compositor
presentation timing) under 4x main-thread Chromium CPU throttling plus loopback-only,
synthetic 4x proportional ASCII-letter `processKey` service-time amplification
and a sustained 250 ms key interval. Every timed key self-verifies the worker
multiplier, lower and upper delay bounds, and effective service-time ratio. This
is a queue-stress profile, not empirical 4x-device proof. Every timed prefix
must expose the configured six-row visible page, a total candidate count of at
least six, and a nonempty first candidate. The receipt also records the final
six-row text order and nullable production source fields for all eight fixed
scenarios and verifies the strongest existing pinned or accepted
first-candidate guard. The timed production path deliberately leaves
`yune_inspector` disabled, so ordinary source labels remain null; the lack of a
complete page-size-6 source-bearing oracle is carried as an explicit
per-scenario residual. Consecutive diagnostic `keydownAt` timestamps must prove
that the actual 250 ms cadence had no catch-up gap below `200 ms` and that at
least 90% of each scenario's gaps stayed within the predeclared
`200..312.5 ms` range (the one-key Cangjie row correctly has zero gaps). Longer
shared-host scheduler delays remain explicit receipt rows and contribute to the
sustained-load ratio; they are not mislabeled as Yune processing latency. The
gate fails at p95 above 750 ms, any key above 1000 ms, or any
schema/split-part/manifest request during a timed window after the selected
schema reaches ready. Its learned TypeDuck row must survive a page reload before
timing. `YUNE_WEB_LATENCY_P95_MS`,
`YUNE_WEB_LATENCY_MAX_MS`, `YUNE_WEB_LATENCY_CPU_THROTTLE`, and
`YUNE_WEB_LATENCY_KEY_INTERVAL_MS` are explicit diagnostic overrides; release
evidence uses the binding defaults. The public runner rejects any receipt that
does not contain the exact eight-scenario order, 186 verified key diagnostics,
4x/4x loopback profile, 250 ms cadence, and 750/1000 ms ceilings; a diagnostic
override may report a non-release result but cannot produce a release-grade
pass. A non-250 ms interval diagnostic validates cadence against its own
predeclared `0.8x..1.25x` bounds; only the exact 250 ms / `200..312.5 ms`
profile is release-grade.

The same pre-publish run also binds a separate unamplified normal-typing
canary for `jyut6ping3` using the reported 47-key input
`ngodeigungsijigaahaidoumaaigangeihaaijansougeoi`. It types at a 100 ms
interval, requires all 47 exact prefixes, no catch-up gap below `80 ms`, and at
least 90% of its 46 measured cadence gaps within `80..125 ms`. It fails above
p95 `150 ms`, max `250 ms`, or max worker queue
wait `100 ms`. Every prefix must expose a six-row candidate page with a
nonempty first candidate, and the final page must expose six nonempty candidate
texts. Because this exact input has no pinned external
candidate-order fixture, the canary deliberately binds responsiveness and page
shape only; it does not promote Yune's own output into an oracle.

The Playwright file remains declaration-ordered with one worker and zero
retries, but its two measurements have independent failure semantics. A red
4x release row still blocks publication and is preserved, while the exact 1x
normal-typing canary runs first so its own receipt cannot be replaced by a
skip. On failure the runner emits the complete JSON receipts and their SHA-256
hashes into the retained Cloudflare build log in addition to the compact
summaries. The complete receipts are gzip/base64 encoded into bounded log chunks,
and the recorded SHA-256 covers their exact JSON file bytes. The runner seeds
explicit incomplete receipts before preview/browser setup so a setup failure is
also distinguishable from a measurement that never started.

The cadence driver preserves its absolute phase during normal operation. If a
host timer arrives late, it rebases the next deadline instead of generating a
short catch-up gap. The original long gap remains recorded as a delayed-host
row and lowers the sustained-load ratio; any catch-up gap is red, and a profile
with less than 90% of gaps inside the active unchanged range is also red. The
public runner does not retry a measured red.

For a deployed canary, set an explicit URL and use the direct command:

```bash
YUNE_WEB_EXPECTED_SOURCE_COMMIT="$(git rev-parse HEAD)" \
YUNE_WEB_APP_URL=https://yune-web.pages.dev/ \
npm --prefix apps/yune-web/e2e run test:e2e:input-latency
```

The deployed-origin canary intentionally cannot activate the synthetic worker
hook; the source/hash-identical loopback preview is the binding pre-publish
worker gate. Preserve the 80 ms calibration as a nonbinding burst/queue-stress
diagnostic and rerun it only when the worker queue architecture changes.

### Step 1: Install Dependencies

```bash
npm --prefix packages/yune-web-runtime run build
npm --prefix apps/yune-web install
```

Record in `blocker.md` if install fails.

### Step 2: Build Worker

```bash
npm --prefix apps/yune-web run worker
```

Record in `blocker.md`:

- Command: `npm --prefix apps/yune-web run worker`
- Missing: generated WASM pair or dependency install

### Step 3: Start Dev Server

```bash
npm --prefix apps/yune-web run start
```

Open browser to dev server URL (e.g., `http://localhost:5173`).

Record in `blocker.md` if server fails.

### Step 4: Load Explicit Assets

In browser app:

1. Locate asset configuration UI or dev console
2. Load explicit TypeDuck-Web provenance YAML assets:
   - `default.yaml`
   - Schema YAML (e.g., `luna_pinyin.schema.yaml`)
   - Dictionary YAML (e.g., `luna_pinyin.dict.yaml`)
3. Verify asset validation output in console

Record in `e2e/results/asset-validation.log`.

### Step 5: Composition Flow (D-08/D-10)

1. Click input field to focus
2. Type schema-valid keys (e.g., `a`, `b`, `c`)
3. Verify composition appears (preedit visible in UI)
4. Verify candidate list visible
5. Take screenshot: `screenshot-composition.png`
6. Take screenshot: `screenshot-candidates.png`

Record in `manual-smoke-checklist.md`:

- Composition: PASS | FAIL | BLOCKED
- Candidate list visible: PASS | FAIL | BLOCKED

### Step 6: Candidate Paging (D-08/D-10)

1. Continue typing to generate multiple candidates
2. Press PageDown key
3. Verify candidate page changes
4. Verify page indicator updates
5. Take screenshot: `screenshot-candidate-paging.png`

Record in `manual-smoke-checklist.md`:

- Candidate paging: PASS | FAIL | BLOCKED

### Step 7: Candidate Selection → Commit (D-08/D-10)

1. Press selection key (e.g., `1`, `2`, `3` or Space/Enter)
2. Verify candidate selected
3. Verify committed text appears in output field
4. Take screenshot: `screenshot-candidate-selection.png`

Record in `manual-smoke-checklist.md`:

- Candidate selection: PASS | FAIL | BLOCKED
- Commit output: PASS | FAIL | BLOCKED

### Step 8: Deletion Flow (D-08/D-10)

1. Type new composition
2. Press Delete key to remove candidate
3. Verify candidate removed OR delete path triggered
4. Press Backspace to mutate composition
5. Verify composition updated

Record in `manual-smoke-checklist.md`:

- Delete candidate: PASS | FAIL | BLOCKED
- Backspace mutation: PASS | FAIL | BLOCKED

### Step 9: Deploy Flow (D-08/D-10)

1. Locate deploy action (button/shortcut)
2. Trigger deploy
3. Verify visible success/error evidence
4. Check browser console for deploy result

Record in `manual-smoke-checklist.md`:

- Deploy: PASS | FAIL | BLOCKED
- Deploy evidence visible: PASS | FAIL | BLOCKED

### Step 10: Customize Flow (D-08/D-10)

1. Locate customize action (settings panel/shortcut)
2. Trigger customize with config ID, key, value
3. Verify visible success/error evidence
4. Check browser console for customize result

Record in `manual-smoke-checklist.md`:

- Customize: PASS | FAIL | BLOCKED
- Customize evidence visible: PASS | FAIL | BLOCKED

### Step 11: Persistence Sync (D-11)

Critical persistence timing MUST be verified:

#### Before Init

1. Open browser dev console
2. Reload app page
3. Check console for `syncFromPersistenceBeforeInit` marker
4. Verify IDBFS/persistence loaded before runtime init

Record in `persistence-sync.log`:

```text
syncFromPersistenceBeforeInit: <timestamp> PASS|FAIL
```

#### After Mutation

1. Perform deploy or customize action
2. Check console for `syncToPersistenceAfterMutation` marker
3. Verify IDBFS/persistence flushed after mutation

Record in `persistence-sync.log`:

```text
syncToPersistenceAfterMutation: <timestamp> PASS|FAIL
```

#### Reload/Reinitialize

1. Reload browser page (full reload)
2. Re-initialize app if needed
3. Verify persisted customization/user state restored
4. Check that previous deploy/customize settings survive
5. Take screenshot: `screenshot-persistence-after-reload.png`

Record in `persistence-sync.log`:

```text
Reload/reinitialize: <timestamp> PASS|FAIL
Persisted state verified: <timestamp> PASS|FAIL
```

### Step 12: Record Console Errors

Copy all browser console errors to `e2e/results/browser-console.log`.

### Step 13: Capture Blockers

For ANY blocked flow, record in `e2e/results/blocker.md`:

````markdown
# Browser E2E Blocker

**Category**: yune-web app | Yune adapter/runtime | environment/tooling

**Command Attempted**:
```bash
npm --prefix apps/yune-web install
npm --prefix apps/yune-web run start
```

**Missing Dependency**:
node/npm, generated WASM pair, or browser automation dependency

**Install Hint**:
Record exact tool/version gap from the command output.

**Fallback Evidence**:
npm install succeeded, npm run start succeeded, manual browser smoke executed

**Blocker Impact**:
Composition flows tested manually, persistence timing verified via console logs

**Flow Results**:
- Composition: PASS
- Candidate paging: PASS
- Candidate selection: PASS
- Deletion: PASS
- Deploy: PASS
- Customize: PASS
- Persistence: PASS
````

## Evidence Requirements

After manual smoke, `e2e/results/` MUST contain:

- `manual-smoke-checklist.md` — All flow PASS/FAIL/BLOCKED status
- `browser-console.log` — Console errors
- `screenshot-*.png` — Screenshots for each flow
- `persistence-sync.log` — Persistence timing evidence
- `blocker.md` — Tooling blocker with command/dependency/fallback (if applicable)
- `asset-validation.log` — Asset loading evidence

## Real Browser Requirement

This procedure MUST use a real browser. Package-local fake module tests do NOT satisfy D-08/TYPEDUCK-E2E-03.

If both automated runner AND manual browser are impossible:

1. Record blocker in `blocker.md` with missing browser environment
2. Run package-local tests as fallback evidence ONLY
3. Clearly label fallback as "NOT satisfying real browser E2E per D-08"
4. Document missing browser/tooling for Plan 10-04 recommendation

### Step 14: M20 Showcase Controls

1. Verify the settings panel has exactly these M20 groups:
   - Active engine controls
   - Live session controls
   - Display controls
2. Verify active controls include Auto-completion, Auto-correction, Auto-composition, Input Memory, AI Candidates, Combine same-text candidates, Prediction never first, and Prediction threshold.
3. Verify live controls include ASCII mode, Full shape, and Simplification.
4. Verify display controls include Display languages, Candidate Jyutping, Reverse code display, and Cangjie version.
5. Confirm `ascii_punct` is not exposed as a working control.
6. Record before/after evidence:
   - `hou` with Combine same-text candidates on and off.
   - Record that the UI's grouped candidate default is an M20 demo default; the raw mobile assets still enable `common:/separate_candidates`.
   - `santai` with Prediction threshold `0` and `50000`.
   - Record the Prediction threshold selector range and step alongside the `50000` real-assets cutoff.
   - Prediction never first with a learned `ngohaigo` -> `ngo` ranking before/after: classic `我` remains first while enabled, and learned `我係個` can move first when disabled.
   - Input Memory with a learned-prediction on-state plus explicit browser-surface N/A for the memory-off candidate-output delta if the current no-crates browser surface still renders an already learned row.
   - Auto-correction as visible correction-row before/after only if the current `jyut6ping3_mobile` browser surface renders one; otherwise record explicit browser-surface N/A, not empty candidates as proof, and cite `cantonese_parity`.
   - Auto-composition with persisted `translator/enable_sentence` snapshots and any current browser-renderable before/after state.
   - `abc` with ASCII mode on.
   - `/` with Full shape off and on.
   - `ngohaigo` with Simplification on.
   - `nei` with Candidate Jyutping shown and hidden.
   - `nei` with English-only display and with Hindi enabled.
7. Run guided scenario buttons for `ngo`, `santai`, `mgoi`, `m`, tone letters, and AI trigger.
8. For show-full-code, Reverse code display, and Cangjie version, use a browser-reachable Cangjie side lookup only if the active browser schema declares a `cangjie` namespace. If the active schema remains `jyut6ping3_mobile`, record them as N/A for this mobile-only browser surface and cite the schema file.

### Step 15: M22 Playground Controls And Multi-Schema

The automated M22 browser slice is the canonical evidence path:

```bash
YUNE_WEB_APP_URL=http://127.0.0.1:5174/web/ \
YUNE_WEB_EVIDENCE_DIR=../e2e/results/m22-remaining-buckets \
npm --prefix apps/yune-web/e2e run test:e2e -- --grep "M22 Bucket" --workers=1
```

It must prove:

- Bucket 1 active controls: `dictionary_exclude`, `traditionalization`, `disabled`, and `extended_charset`.
- `ascii_punct` is not exposed as a working browser toggle.
- Bucket 2 inspector identity still preserves classic candidate output.
- Bucket 3 schema switcher loads `jyut6ping3_mobile`, `cangjie5`, and `luna_pinyin`.
- Reverse lookup works for both `cangjie5` and `luna_pinyin`.
- Evidence files are written under `e2e/results/m22-remaining-buckets/`, including the measured asset manifest.

---

**Phase**: yune-web app integration and E2E
**Plan**: 10-03 (Real browser E2E/smoke validation)
**Requirement**: TYPEDUCK-E2E-03, D-08, D-09, D-10, D-11
**Status**: Manual browser smoke fallback procedure for tooling blockers
