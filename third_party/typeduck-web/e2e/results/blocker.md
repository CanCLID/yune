# Browser E2E Blocker / Supersession Notes

**Date**: 2026-06-18

The earlier WI-4 evidence in this directory used Yune's echo placeholder path and is superseded for the real-assets candidate gate by HR-1b. The refreshed artifacts are:

- `browser-run.log` — focused HR-1b real-assets browser smoke.
- `browser-console.json` — console evidence for reload/init and `processKey({n/e/i})`.
- `dom-snapshot-candidates.txt` — candidate panel DOM excerpt showing `nei -> 你 / 呢 / 尼`.
- `screenshot-real-assets-nei.png` — browser screenshot of the real candidate panel.

**Current status**: HR-1b PASS for real-assets candidate rendering.

**HR-3 update**: `deploy()` now returns true with real assets. The failure was an
incomplete browser preload list: deployment reaches the plain
`jyut6ping3.schema.yaml` through TypeDuck's real workspace/default schema path.
See `deploy-browser.log` for the browser console proof.

**HR-4 update**: live persistence is browser-proven. `persistence-sync.log`
records a fresh-origin load where before-init persistence is empty, startup
`customize` writes `page_size: '6'`, deploy syncs after mutation, and a real page
reload restores the persisted custom config before runtime init.

**Still open**:

- HR-5: paging/deletion and dictionary-panel oracle comment bytes still need the
  full real-assets E2E matrix.
