# M31 Public Readiness Gate

Date: 2026-06-24

Status: complete with one measured delivery caveat.

- P2-WIN-02 status: complete in current roadmap/plans; public demo does not
  widen the Yune/TypeDuck-Windows ABI boundary.
- P2-WIN-01 priority: unaffected. M31 changes are browser delivery/UI/devops
  scoped and do not edit Windows TSF/frontend shell work.
- Engine performance scope: M36 remains closed historical evidence. M31 does
  not reopen M36 and does not claim browser startup/typing wins.
- Public identity: public UI, package, Wrangler config, build package, harness
  path, and evidence use `yune-web`.
- Physical path migration: complete. The repo-owned harness moved to
  `apps/yune-web/`; `source/` remains the ignored upstream-derived checkout
  under that canonical path.

Deployment gate:

- Local Cloudflare Pages preview passed at `http://127.0.0.1:8788/`.
- Wrangler Pages command surface was checked, and the deploy used the Pages
  Direct Upload manifest path.
- Production deploy passed at
  `https://yune-web.pages.dev`.
- Deployed Playwright smoke passed 3/3.

Measured caveat:

- The public boot path is pruned to the single `jyut6ping3_mobile` surface and
  excludes non-public schema families such as Cangjie, Loengfan, longpress, and
  10-key assets.
- The current TypeDuck product schema still requires Luna and scholar lookup
  assets during boot for the public behavior it exposes. M31 records this as a
  delivery caveat instead of claiming every reverse-lookup dependency is fully
  deferred.
