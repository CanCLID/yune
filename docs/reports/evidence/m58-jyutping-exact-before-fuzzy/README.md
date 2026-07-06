# M58 Jyutping Evidence Index

Status: complete.

This evidence root tracks M58 work after the 2026-07-05 preflight correction
that separated canonical `rime-cantonese` candidate behavior from TypeDuck
profile behavior, plus the 2026-07-06 corrective follow-up that supersedes the
initial `beingo` page-one promotion fix.

## Artifacts

- [`phase-0/`](./phase-0/) - canonical upstream capture harness and provenance.
- [`phase-1/`](./phase-1/) - complete all-pages canonical upstream
  `rime/rime-cantonese` captures, including the user-specified reported-case
  input `zijiguk` for `諮議局`.
- [`phase-2b/`](./phase-2b/) - TypeDuck/profile `beingo` and `zi`
  reachability captures, current `yune-web` product-lane fix, and WEB-03
  compiled-asset rebuild evidence.
- [`phase-3/`](./phase-3/) - schema/profile identity blast-radius audit. No
  schema id split or rename was implemented.

## Closeout Disposition

Canonical upstream `rime-cantonese` does not reproduce a `zijiguk` admission or
candidate-order failure: `諮議局` is the first candidate in the canonical
capture. Canonical `beingo` already reaches standalone `畀` on page 1. No
canonical M58 implementation fix was derived from TypeDuck v1.1.2 output.

The shipped/current `yune-web` TypeDuck/profile product lane had bounded
candidate reachability bugs:

- `beingo` needed standalone `畀`, captured at TypeDuck/profile index 6.
- `zi` needed standalone `諮`, captured at TypeDuck/profile index 27.

The initial M58 closeout promoted `畀` onto the first page by moving and
reweighting the tracked public dictionary row. That result is superseded. The
corrective fix restores the public `畀	bei2	200000` row to the TypeDuck source
weight/order and fixes reachability by retaining enough TypeDuck/profile
candidates for short reported/profile inputs while widening prefix fallback only
for that scoped short-input path. TypeDuck profile fixtures remain profile-only
guards; TypeDuck v1.1.2 is not a canonical candidate oracle for M58.

The canonical capture provenance was also sanitized so checked-in evidence does
not include local `C:\Users\...` paths, and
`oracle_fixture_provenance.rs` now guards that file.

## Verification Summary

Corrective closeout gates run:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo test -p yune-core --test upstream_luna_pinyin_parity`
- `cargo test -p yune-core --test cantonese_parity`
- `cargo test -p yune-rime-api --test yune_web`
- `cargo test -p yune-core --test cantonese_parity m58 -- --nocapture`
- `cargo test -p yune-rime-api --test yune_web m58 -- --nocapture`
- `cargo test -p yune-core --test oracle_fixture_provenance m58_canonical_cantonese_capture_has_sanitized_provenance -- --nocapture`
- `cargo test -p yune-core --lib typeduck_product_refresh_keeps_profile_page_bounded_until_full_access -- --nocapture`
- `cargo test -p yune-rime-api --test yune_web web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion -- --nocapture`
- WEB-03 corrective asset rebuild with `YUNE_WEB03_APPLY_ASSETS=1`:
  `cargo test -p yune-rime-api --test yune_web web03_regenerates_public_schema_compiled_assets_from_clean_rebuild -- --ignored --nocapture`
- `scripts/yune-web-wasm-build.sh` through Emscripten
- `npm.cmd --prefix apps/yune-web run check:schema-manifest`
- `npm.cmd --prefix apps/yune-web run typecheck`
- `npm.cmd --prefix apps/yune-web run build`
- `npm.cmd --prefix apps/yune-web run build:public`
- `npm.cmd --prefix packages/yune-web-runtime test`
- `npm.cmd --prefix packages/yune-web-runtime run build`
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep M58 --workers=1`
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M31 PUBLIC" --workers=1`
- M55 product ratchet with `-DeployProductBeforeBenchmark`:
  `phase-2b/m55-product-ratchet-corrective-final-pass2/`
- `git diff --check`

The full Playwright suite was not required for this closeout; browser-visible
M58 behavior is covered by the focused M58 real-browser gate, and the changed
public-demo surface is covered by the M31 PUBLIC gate. The WASM build script
skipped only its optional `wasm-opt` post-optimization because it could not
validate the Emscripten module shape, then passed the JS glue scan and browser
module smoke.
