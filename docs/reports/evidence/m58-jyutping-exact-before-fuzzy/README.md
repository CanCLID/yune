# M58 Jyutping Evidence Index

Status: complete for M58 closeout.

This evidence root tracks M58 work after the 2026-07-05 preflight correction
that separated canonical `rime-cantonese` candidate behavior from TypeDuck
profile behavior.

## Artifacts

- [`phase-0/`](./phase-0/) - canonical upstream capture harness and provenance.
- [`phase-1/`](./phase-1/) - complete all-pages canonical upstream
  `rime/rime-cantonese` captures, including the user-specified reported-case
  input `zijiguk` for `諮議局`.
- [`phase-2b/`](./phase-2b/) - TypeDuck/profile `beingo` capture, current
  `yune-web` product-lane fix, and WEB-03 compiled-asset rebuild evidence.
- [`phase-3/`](./phase-3/) - schema/profile identity blast-radius audit. No
  schema id split or rename was implemented.

## Closeout Disposition

Canonical upstream `rime-cantonese` does not reproduce a `zijiguk` admission
failure: `諮議局` is the first candidate in the canonical capture. Canonical
`beingo` already reaches standalone `畀` on page 1.

The shipped/current `yune-web` TypeDuck/profile product lane still had a
page-size-6 reachability bug: standalone `畀` fell just past the first page for
`beingo`. M58 fixes that lane by reordering and reweighting the tracked
`apps/yune-web/public/schema/jyut6ping3.dict.yaml` row for `畀	bei2`, then
regenerating the compiled public schema assets and schema manifests. Historical
TypeDuck profile fixtures remain profile-only guards; TypeDuck v1.1.2 is not a
canonical candidate oracle for M58.

## Verification Summary

Final closeout verification passed with:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo test -p yune-core --test upstream_luna_pinyin_parity`
- `cargo test -p yune-core --test cantonese_parity`
- `cargo test -p yune-rime-api --test yune_web`
- `cargo test -p yune-core --test cantonese_parity m58 -- --nocapture`
- `cargo test -p yune-rime-api --test yune_web m58_yune_web_browser_app_assets_surface_beingo_standalone_bei_first_page -- --nocapture`
- `cargo test -p yune-core --lib m37_metrics_test_enable_is_thread_local -- --nocapture`
- `cargo test -p yune-rime-api --test yune_web web03_regenerates_public_schema_compiled_assets_from_clean_rebuild -- --ignored --nocapture`
- `npm.cmd --prefix apps/yune-web run check:schema-manifest`
- `npm.cmd --prefix apps/yune-web run typecheck`
- `npm.cmd --prefix apps/yune-web run build`
- `npm.cmd --prefix apps/yune-web run build:public`
- `npm.cmd --prefix packages/yune-web-runtime test`
- `npm.cmd --prefix packages/yune-web-runtime run build`
- focused M58 Playwright candidate-output gate:
  `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep M58 --workers=1`
- `git diff --check`

The M55 Track B ratchet was not run because M58 did not change translator caps,
product-path performance thresholds, or benchmark thresholds. The full
Playwright suite was not run; M58's browser-visible claim is covered by the
focused real-browser candidate-output gate under `phase-2b/browser/`.

During verification, the ignored generated `target/typeduck-oracle/v1.1.2`
capture tree was moved aside to
`target/typeduck-oracle/v1.1.2.m58-capture-20260705` before running the final
Rust gates. That local tree is not committed evidence; the checked-in
`typeduck-profile-beingo-capture.json` is the TypeDuck/profile evidence.
