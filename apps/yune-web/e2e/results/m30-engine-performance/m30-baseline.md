# M30 Baseline

Date: 2026-06-22

## Native Baseline

Native command:

```powershell
cmd /c "cargo bench -p yune-rime-api --bench frontend_baselines > target\m30-frontend-baselines-before.txt 2>&1"
```

Native evidence: `native-before.md`.

Key before values:

| Metric | Before |
| --- | ---: |
| `startup_real_jyut6ping3_mobile_runtime_ready` median | `6,242,614.900us` |
| `startup_trace_jyut6ping3_mobile_translator_install` median | `5,340,382us` |
| `startup_trace_jyut6ping3_mobile_spelling_algebra_expand` median | `5,015,352us` |
| `startup_trace_jyut6ping3_mobile_translator_index_build` median | `144,074us` |
| Single-startup after-ready bytes | `1,103,331,328` |
| Single-startup peak bytes | `1,123,745,792` |
| `hai` engine-only p95 | `8,628.800us` |
| `jigaajiusihaa` engine-only p95 | `15,794.408us` |
| `jigaajiusihaa` correction engine-only p95 | `125,883.923us` |

## Browser Baseline

Browser command:

```powershell
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; $env:M29_EVIDENCE_LABEL='m30-before'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M29 PERF" --workers=1
```

Copied evidence:

- `browser-startup-before.json`
- `typing-keydown-to-paint-before.json`
- `typing-attribution-before.json`

Startup:

| Path | Total |
| --- | ---: |
| Fresh startup | `5,256ms` |
| Reload startup | `5,096ms` |

Typing attribution:

| Scenario | Total median/p95/max | Owner p95 worker/native/map/react/paint |
| --- | --- | --- |
| `hai` | `19/51/51ms` | `25/25/1/25/6ms` |
| `longPhrase` | `21/71/71ms` | `54/46/0/4/15ms` |
| `longComposition` | `20/50/60ms` | `35/35/0/2/14ms` |
| `paging` | `5/5/5ms` | `3/3/0/1/1ms` |
| `reverseLookup` | `13/14/14ms` | `8/8/0/4/10ms` |

## Reading

The browser rows are user-visible evidence, not native engine proof. The `hai` total p95 is a first-key/browser path and should not be claimed as an engine win unless the native/worker owner moves. Lever A should be evaluated primarily through native startup/memory rows, with browser startup and typing treated as a regression guard.
