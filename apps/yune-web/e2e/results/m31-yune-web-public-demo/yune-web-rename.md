# yune-web Rename Evidence

Date: 2026-06-24

M31 physically moved the repo-owned browser harness from
`third_party/typeduck-web/` to `apps/yune-web/`.
The `source/` child remains an ignored upstream-derived checkout, but its active
working path is now `apps/yune-web/source/`; the committed Yune-owned state is
the patch, integration bridge, public-demo package, E2E evidence, and docs.

Canonical yune-web identity completed for the deployable surface:

- Document title: `yune-web`.
- Header brand: `yune-web`.
- Package name: `yune-web`.
- Public deployment config name: `yune-web`.
- Public build package path: `apps/yune-web/public-demo/`.
- Repo-owned harness path: `apps/yune-web/`.
- Path migration: `path_migration = complete`.
- Provenance link: `/PROVENANCE.md`.
- E2E evidence scope: `m31-yune-web-public-demo`.

The old TypeDuck-Web name remains only in provenance/history for the
upstream-derived shell and archived historical evidence.
