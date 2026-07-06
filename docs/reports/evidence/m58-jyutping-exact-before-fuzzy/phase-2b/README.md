# M58 Phase 2b TypeDuck/Profile `beingo` Disposition

Status: complete.

This phase records the shipped/profile lane separately from canonical
`rime-cantonese` behavior. TypeDuck-HK/librime v1.1.2 is used here only as a
profile-lane source, not as the canonical `jyut6ping3` candidate oracle.

## Profile Oracle Capture

- Capture file: `typeduck-profile-beingo-capture.json`
- Command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\capture-typeduck-jyutping.ps1 -Fixture Smoke -Inputs be,bei,bein,being,beingo,beix,beixngoxx -Output docs\reports\evidence\m58-jyutping-exact-before-fuzzy\phase-2b\typeduck-profile-beingo-capture.json
```

The captured TypeDuck/profile fixture reports `captured_all_pages: true` for all
inputs. Relevant profile facts:

| Input | Page size | Candidate count | First candidates | Relevant reachability |
| --- | ---: | ---: | --- | --- |
| `beingo` | 50 | 47 | `俾我`, `比`, `被`, `備`, `俾`, `避`, `畀`, `鼻` | `俾我` at index 0; standalone `畀` at index 6. |
| `beixngoxx` | 50 | 12 | `俾我`, `比`, `俾`, `畀`, `彼`, `髀`, `吡`, `媲` | `俾我` at index 0; standalone `畀` at index 3. |

## Current Yune Product Path

Current `apps/yune-web/public/schema` differs from the raw TypeDuck shared
profile: it has `translator/combine_candidates: true`, byte-backed compiled
launch assets, and page size 6. Before the fix, the current product first page
for `beingo` was:

```text
俾我, 比, 被, 備, 俾, 避
```

That made standalone `畀` unreachable on the first page even though the
TypeDuck/profile capture had it at index 6 under page size 50 and canonical
`rime-cantonese` had it at index 3.

## Fix

M58 applies a scoped product-lane fix to
`apps/yune-web/public/schema/jyut6ping3.dict.yaml`:

- reweight `畀	bei2` from `200000` to `600000`;
- move that public row immediately after `比	bei2` so the compiled
  prefix-fallback path sees `畀` within the existing two-candidates-per-fetch
  bounded expansion cap.

No TypeDuck source snapshot, TypeDuck v1.1.2 fixture, schema id, userdb key, or
profile predicate was renamed. The compiled public assets were regenerated with
the existing WEB-03 harness:

```powershell
$env:YUNE_WEB03_EVIDENCE_DIR = "docs\reports\evidence\m58-jyutping-exact-before-fuzzy\phase-2b\web03-rebuild"
$env:YUNE_WEB03_APPLY_ASSETS = "1"
cargo test -p yune-rime-api --test yune_web web03_regenerates_public_schema_compiled_assets_from_clean_rebuild -- --ignored --nocapture
```

Evidence files:

- `web03-rebuild/task2-native-regeneration/workspace-rebuild-reports.json`
- `web03-rebuild/task2-native-regeneration/workspace-rebuild-reports.csv`
- `web03-rebuild/task2-native-regeneration/compiled-asset-inventory.csv`
- `yune-web-beingo-disposition.json`
- `browser/worker-0-M58-yune-web-TypeDuck-profile-surfaces-beingo-standalone-bei-first-page-smoke/m58-beingo-first-page.json`
- `browser/worker-0-M58-yune-web-TypeDuck-profile-surfaces-beingo-standalone-bei-first-page-smoke/screenshot-m58-beingo-first-page.png`

Focused guards:

- `cargo test -p yune-core --test cantonese_parity m58_current_yune_web_profile_surfaces_beingo_report_candidates -- --nocapture`
- `cargo test -p yune-rime-api --test yune_web m58_yune_web_browser_app_assets_surface_beingo_standalone_bei_first_page -- --nocapture`
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep M58 --workers=1`

The source-backed product guard now keeps `俾我` first and includes standalone
`畀` on the page-size-6 first page. The byte-backed browser-app guard now sees:

```text
俾我, 比, 被, 畀, 備, 悲
```

This is a TypeDuck/profile product-lane reachability fix, not a canonical
`rime-cantonese` candidate ordering claim.

Blast-radius guard note: the focused Rust product guards also cover the nearby
`bei`, `being`, `beix`, and `beixngoxx` inputs. `being` is intentionally
snapshotted separately because the source-backed product path keeps its phrase
prediction first page (`俾我`, `悲哀`, `備案`, `彼岸`, `被愛`, `彼岸花`), while
the byte-backed browser-app path now sees (`比五`, `比`, `被`, `畀`, `備`, `悲`).
The other nearby forms assert standalone `畀` remains reachable on the
page-size-6 first page, and `beingo` / `beixngoxx` keep `俾我` first.

Verification note: the generated, ignored `target/typeduck-oracle/v1.1.2`
profile-capture tree was moved aside before final `yune_web` verification
because the legacy `write_browser_real_assets` helper auto-selects that local
path when present. The committed TypeDuck/profile evidence for M58 is
`typeduck-profile-beingo-capture.json`; the product-lane guards use the tracked
browser-app assets.
