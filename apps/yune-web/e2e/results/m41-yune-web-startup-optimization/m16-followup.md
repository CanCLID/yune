# M16 Follow-Up After M41 Defaults Fix

**Date:** 2026-06-26

**Purpose:** Verify whether the M41 shipped-defaults fix also restores the old
M16 browser parity rows for combine candidates and sentence composition.

## Commands

```powershell
npm.cmd --prefix apps/yune-web run build
```

Result: PASS.

```powershell
$env:YUNE_WEB_APP_URL = "http://127.0.0.1:5173"
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M16 combine candidates browser default matches M14|M16 sentence composition browser path matches M14" --workers=1
```

Result: FAIL.

The production build was served with `npm.cmd --prefix apps/yune-web run
preview -- --port 5173 --host 127.0.0.1` for the Playwright run.

## Result

The M41 defaults fix remains valid: the UI shows the deploy-default toggles are
on for completion, sentence, user dictionary, combined candidates, and
prediction-never-first without a startup deploy marker.

The two M16 rows still fail on the post-M41 production build:

- `M16 combine candidates browser default matches M14`: expected the first
  five M14 `hou` candidates (`好`, `號`, `豪`, `毫`, `浩`), but the browser
  returned only `好`.
- `M16 sentence composition browser path matches M14 @smoke`: expected a
  visible candidate panel for `ngohaigo`, but the browser showed `0/0`
  candidates and no `.candidate-panel .candidates tbody`.

## Verdict

The legacy M16 combine/sentence rows are now evidenced as current post-M41
browser-parity failures. They are not part of the M41 startup closeout claim and
need a separate browser-parity cleanup plan if promoted back to active gates.
