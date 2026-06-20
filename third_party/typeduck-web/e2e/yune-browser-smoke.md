# TypeDuck-Web Manual Browser Smoke Procedure

Fallback procedure for real browser validation when automated browser runner unavailable (per D-08, D-09).

## Purpose

Manual real-browser smoke test for patched TypeDuck-Web + Yune runtime seam. This procedure is ONLY for browser/tooling blockers — it MUST still use a real browser, NOT package-local fake tests.

## Prerequisites

1. Patched TypeDuck-Web source (apply `patches/yune-typeduck-runtime.patch`)
2. Yune WASM artifact with `yune_typeduck_*` exports
3. Built `@yune-ime/typeduck-runtime` package
4. Explicit TypeDuck-Web-owned YAML assets (per `e2e/assets/README.md`)
5. Bun installed (or npm fallback)
6. Modern browser (Chrome/Firefox/Safari)

## Procedure

### Step 1: Apply Patch

```bash
cd third_party/typeduck-web/source
git apply ../patches/yune-typeduck-runtime.patch
git status  # Verify patch applied
```

Record in `blocker.md` if patch fails.

### Step 2: Install/Build Upstream

```bash
cd third_party/typeduck-web/source

# Prefer Bun (upstream uses Bun)
bun install

# Fallback if Bun unavailable
npm install

# Build worker
bun run worker  # or npm run worker
```

Record in `blocker.md`:

- Command: `bun install` or `npm install`
- Missing: `bun` executable (if npm fallback used)
- Install hint: https://bun.sh

### Step 3: Start Dev Server

```bash
bun run start  # or npm run start
```

Open browser to dev server URL (e.g., `http://localhost:5173`).

Record in `blocker.md` if server fails.

### Step 4: Load Explicit Assets

In browser app:

1. Locate asset configuration UI or dev console
2. Load explicit TypeDuck-Web-owned YAML assets:
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

```markdown
# Browser E2E Blocker

**Category**: TypeDuck-Web app/source | Yune adapter/runtime | environment/tooling

**Command Attempted**:
```bash
bun install
bun run start
```

**Missing Dependency**:
bun executable (npm fallback used)

**Install Hint**:
https://bun.sh

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
```

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

---

**Phase**: 10-typeduck-web-app-integration-and-e2e
**Plan**: 10-03 (Real browser E2E/smoke validation)
**Requirement**: TYPEDUCK-E2E-03, D-08, D-09, D-10, D-11
**Status**: Manual browser smoke fallback procedure for tooling blockers
