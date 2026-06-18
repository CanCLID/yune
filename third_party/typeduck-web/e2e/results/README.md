# TypeDuck-Web Browser E2E Results

This directory contains evidence from real browser E2E/smoke validation runs.

## Required Result Artifacts

After browser E2E execution, this directory MUST contain:

### Pass Evidence

If browser E2E passes:

- `browser-run.log` — Browser test runner output with composition, candidate, deploy, customize, persistence flows
- `screenshot-composition.png` — Screenshot showing composition after schema-valid key input
- `screenshot-candidates.png` — Screenshot showing visible candidate list
- `screenshot-candidate-paging.png` — Screenshot showing candidate page change
- `screenshot-candidate-selection.png` — Screenshot showing candidate selection → commit output
- `screenshot-persistence-after-reload.png` — Optional screenshot showing persisted state after reload/reinitialize when browser capture succeeds
- `persistence-sync.log` — HR-4 DOM/console diagnostic evidence of sync-before-init, sync-after-mutation, and reload/reinitialize
- `asset-sources.log` — Documented asset sources (explicit TypeDuck-Web-owned YAML)
- `asset-validation.log` — Asset validation output

### Blocker Evidence

If browser tooling/runner is missing (per D-09):

- `blocker.md` — Reproducible blocker with:
  - Exact command attempted
  - Missing dependency/executable
  - Install hint from upstream docs
  - Fallback evidence (manual browser smoke or package-local tests)
  - Category: TypeDuck-Web app/source, Yune adapter/runtime, environment/tooling

If browser runner exists but flows fail:

- `browser-run.log` — Runner output with failure stack
- `blocker.md` — Specific flow blocker (composition, candidate, deploy, customize, persistence)
- `screenshot-*.png` — Failure screenshots where applicable

### Manual Browser Smoke Evidence

If automated browser runner unavailable, manual real-browser smoke produces:

- `manual-smoke-checklist.md` — Completed checklist from `yune-browser-smoke.md`
- `browser-console.log` — Browser console errors captured during manual run
- `screenshot-*.png` — Manual screenshots for each flow
- `persistence-manual-test.log` — Persistence before/after mutation/reload evidence
- `blocker.md` — Tooling blocker with command, dependency, fallback
- `set-option-browser.log` — Focused HR-2 browser smoke proving startup
  `setOption` calls no longer throw adapter/runtime errors

## Result Format Requirements

### browser-run.log

MUST contain:

- Composition flow evidence (key input → preedit visible)
- Candidate paging evidence (PageDown → page change)
- Candidate selection evidence (selection key → commit text)
- Deletion evidence (delete candidate or delete path)
- Deploy evidence (deploy → success/error visible)
- Customize evidence (customize → success/error visible)
- Persistence evidence (sync before init, after mutation, reload/reinitialize)

### blocker.md

MUST contain:

```markdown
# Browser E2E Blocker

**Category**: TypeDuck-Web app/source | Yune adapter/runtime | environment/tooling

**Command Attempted**:
```bash
<exact command>
```

**Missing Dependency**:
<missing executable/library/tool>

**Install Hint**:
<upstream documentation hint or repository README>

**Fallback Evidence**:
<fallback command run, manual smoke steps, or package-local test>

**Blocker Impact**:
<which D-08/D-10/D-11 flows blocked>
```

### persistence-sync.log

MUST contain D-11 evidence:

```text
Persistence timing:
1. syncFromPersistenceBeforeInit: <timestamp> <PASS|FAIL>
2. Runtime initialization: <timestamp> <PASS|FAIL>
3. Composition/deploy/customize mutation: <timestamp> <PASS|FAIL>
4. syncToPersistenceAfterMutation: <timestamp> <PASS|FAIL>
5. Reload/reinitialize: <timestamp> <PASS|FAIL>
6. Persisted state verified: <timestamp> <PASS|FAIL>
```

## Artifact Naming

- Use exact artifact names: `browser-run.log`, `blocker.md`, `persistence-sync.log`
- Screenshots: `screenshot-<flow>.png` (e.g., `screenshot-composition.png`)
- Logs: `<flow>-log` or `<flow>.log`
- No timestamp prefixes/suffixes — artifacts are latest run evidence

## Verification

Result directory MUST be populated after browser E2E:

```bash
ls third_party/typeduck-web/e2e/results/
```

Expected:

- At least one artifact: `browser-run.log` OR `blocker.md`
- If pass: screenshots, logs for each D-08/D-10/D-11 flow
- If blocker: `blocker.md` with command/dependency/fallback

Empty results directory indicates E2E not run.

---

**Phase**: 10-typeduck-web-app-integration-and-e2e
**Plan**: 10-03 (Real browser E2E/smoke validation)
**Requirement**: TYPEDUCK-E2E-03, D-08, D-09, D-10, D-11
**Status**: Result artifact scaffolding with blocker evidence format
