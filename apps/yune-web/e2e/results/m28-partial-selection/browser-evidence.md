# M28 Browser Partial Selection Evidence

> **Status:** Captured - **Milestone:** M28 (TypeDuck partial candidate selection) - **Updated:** 2026-06-22 - **Type:** evidence

## Command

```powershell
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M28 PARTIAL" --workers=1
```

Result: PASS, `1 passed`.

## Evidence

- JSON: `apps/yune-web/e2e/results/m28-partial-selection/browser-partial-selection.json`
- Input: `caksijathaacoenggeoizi`
- First explicit selection: `測`
- Raw-tail guard: `測sijathaacoenggeoizi` was not inserted.
- Remaining input stayed composing and completed through `是日`, `下場`, and `句子`.
- Final browser value: `測是日下場句子`
- User feel target `測試一下長句子` was recorded as not reached by the captured TypeDuck v1.1.2 oracle flow.
