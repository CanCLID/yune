# M30 M29 Evidence Check

Date: 2026-06-22

Purpose: verify that the M29 markdown baseline used by M30 matches the committed browser JSON before making M30 performance claims.

## JSON Shape

Files:

- `third_party/typeduck-web/e2e/results/m29-performance/browser-startup-before.json`
- `third_party/typeduck-web/e2e/results/m29-performance/browser-startup-after.json`
- `third_party/typeduck-web/e2e/results/m29-performance/typing-attribution-before.json`
- `third_party/typeduck-web/e2e/results/m29-performance/typing-attribution-after.json`

Confirmed startup keys:

- `freshStartup.marker.totalMs`
- `reloadStartup.marker.totalMs`
- `startupTotalsMs.fresh`
- `startupTotalsMs.reload`

Confirmed typing keys:

- `scenarioSummaries.<scenario>.totalKeydownToPaintMs.p95`
- `scenarioSummaries.<scenario>.ownerP95Ms.nativeOrWorkerProcess`

## Values

| Item | Before | After | Native/worker before | Native/worker after |
| --- | ---: | ---: | ---: | ---: |
| Fresh startup | `5299ms` | `5378ms` | - | - |
| Reload startup | `5211ms` | `5245ms` | - | - |
| `hai` p95 keydown-to-paint | `61ms` | `62ms` | `25ms` | `26ms` |
| Long phrase p95 keydown-to-paint | `50ms` | `59ms` | `46ms` | `46ms` |
| Long composition p95 keydown-to-paint | `39ms` | `44ms` | `33ms` | `35ms` |
| Paging p95 keydown-to-paint | `13ms` | `16ms` | `2ms` | `2ms` |
| Reverse lookup p95 keydown-to-paint | `16ms` | `29ms` | `7ms` | `7ms` |

Result: `third_party/typeduck-web/e2e/results/m29-performance/native-startup-after.md` matches the committed JSON shape and values. Browser startup and typing remain flat/mixed attribution evidence, not M29 engine-win claims.
