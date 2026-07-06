# M58 Phase 2b TypeDuck/Profile Reachability Disposition

Status: complete.

This phase records shipped/current `yune-web` TypeDuck/profile behavior
separately from canonical `rime-cantonese` behavior. TypeDuck-HK/librime v1.1.2
is used here only as a profile-lane source, not as the canonical `jyut6ping3`
candidate oracle.

## Profile Oracle Capture

The original `beingo` capture remains raw historical evidence:
`typeduck-profile-beingo-capture.json`.

The corrective follow-up adds `zi` / 諮 reachability and records both shipped
reports in:

- `typeduck-profile-reachability-capture.json`

Command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\capture-typeduck-jyutping.ps1 -Fixture Smoke -OracleRoot target\typeduck-oracle\v1.1.2.m58-capture-20260705 -Inputs be,bei,being,beingo,beix,beixngoxx,zi -Output docs\reports\evidence\m58-jyutping-exact-before-fuzzy\phase-2b\typeduck-profile-reachability-capture.json
```

The captured TypeDuck/profile fixture reports `captured_all_pages: true` for all
inputs. Relevant profile facts:

| Input | Page size | Candidate count | First candidates | Relevant reachability |
| --- | ---: | ---: | --- | --- |
| `beingo` | 50 | 47 | `俾我`, `比`, `被`, `備`, `俾`, `避`, `畀`, `鼻` | `俾我` at index 0; standalone `畀` at index 6. |
| `beixngoxx` | 50 | 12 | `俾我`, `比`, `俾`, `畀`, `彼`, `髀`, `吡`, `媲` | `俾我` at index 0; standalone `畀` at index 3. |
| `zi` | 50 | 117 | `自`, `之`, `只`, `至`, `指`, `資`, `支`, `子`, `知`, `字`, `治`, `止` | `自` at index 0; standalone `諮` at index 27. |

## Corrected Product Fix

The first M58 closeout promoted `畀` into the page-size-6 first page for
`beingo` by moving and reweighting the tracked public dictionary row to
`畀	bei2	600000`. That fix is superseded. The raw browser evidence from that
run is retained under `browser/`, and `yune-web-beingo-disposition.json` now
points at the corrective disposition.

The accepted corrective mechanism restores the public dictionary row to the
TypeDuck source value and order:

```text
畀	bei2	200000
```

It then fixes reachability by product/profile-lane candidate retention:

- `jyut6ping3_mobile` bounded candidate refresh retains one TypeDuck/profile
  page of candidates for short reported/profile inputs, so page-size-6 UI
  paging can reach TypeDuck-profile ranks without changing the long WEB-03 row.
- Prefix fallback keeps the first three candidates per fetch code only for the
  same short-input bounded-expansion path, preserving the source-order `比`,
  `俾`, `畀` path for `bei2` fallback without promoting `畀` above its
  TypeDuck/profile rank.
- The browser first page remains ordered for the first page; reported targets
  are reached by PageDown at the captured profile positions.

Current disposition:

- `yune-web-reachability-disposition.json`

Expected product reachability:

| Input | First candidate | Target | TypeDuck/profile index | Browser page size | Expected PageDowns |
| --- | --- | --- | ---: | ---: | ---: |
| `beingo` | `俾我` | `畀` | 6 | 6 | 1 |
| `zi` | `自` | `諮` | 27 | 6 | 4 |

The `zi` / 諮 report is a shipped/product-lane reachability issue in the same
bounded-retention class. It is not reclassified as harmless merely because
canonical `zijiguk` / `諮議局` is canonical-first.

## Asset Rebuild

The compiled public assets were regenerated with the existing WEB-03 harness:

```powershell
$env:YUNE_WEB03_EVIDENCE_DIR = "docs\reports\evidence\m58-jyutping-exact-before-fuzzy\phase-2b\web03-corrective-rebuild"
$env:YUNE_WEB03_APPLY_ASSETS = "1"
cargo test -p yune-rime-api --test yune_web web03_regenerates_public_schema_compiled_assets_from_clean_rebuild -- --ignored --nocapture
```

Evidence files:

- `web03-corrective-rebuild/task2-native-regeneration/workspace-rebuild-reports.json`
- `web03-corrective-rebuild/task2-native-regeneration/workspace-rebuild-reports.csv`
- `web03-corrective-rebuild/task2-native-regeneration/compiled-asset-inventory.csv`

## Focused Guards

Focused guards added or updated for the corrective follow-up:

- `cargo test -p yune-core --test cantonese_parity m58 -- --nocapture`
- `cargo test -p yune-rime-api --test yune_web m58 -- --nocapture`
- `cargo test -p yune-core --lib typeduck_product_refresh_keeps_profile_page_bounded_until_full_access -- --nocapture`
- `cargo test -p yune-rime-api --test yune_web web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion -- --nocapture`
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep M58 --workers=1`

The browser gate writes the corrective evidence to:

- `browser-corrective/m58-reachability/m58-profile-reachability.json`
- `browser-corrective/m58-reachability/screenshot-m58-profile-reachability-beingo.png`
- `browser-corrective/m58-reachability/screenshot-m58-profile-reachability-zi.png`

This is a TypeDuck/profile product-lane reachability fix, not a canonical
`rime-cantonese` candidate ordering claim.

## Closeout Gates

The corrective closeout also ran:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo test -p yune-core --test upstream_luna_pinyin_parity`
- `cargo test -p yune-core --test cantonese_parity`
- `cargo test -p yune-rime-api --test yune_web`
- `cargo test -p yune-core --test oracle_fixture_provenance m58_canonical_cantonese_capture_has_sanitized_provenance -- --nocapture`
- `scripts/yune-web-wasm-build.sh` through Emscripten
- `npm.cmd --prefix apps/yune-web run check:schema-manifest`
- `npm.cmd --prefix apps/yune-web run typecheck`
- `npm.cmd --prefix apps/yune-web run build`
- `npm.cmd --prefix apps/yune-web run build:public`
- `npm.cmd --prefix packages/yune-web-runtime test`
- `npm.cmd --prefix packages/yune-web-runtime run build`
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M31 PUBLIC" --workers=1`
- M55 product ratchet with `-DeployProductBeforeBenchmark`:
  `m55-product-ratchet-corrective-final-pass2/`
- `git diff --check`

The full Playwright suite was not required; the scoped browser claims are
covered by the focused M58 and M31 PUBLIC gates. The WASM build script skipped
only optional `wasm-opt` post-optimization after module-shape validation failed,
then passed the JS glue scan and browser module smoke.
