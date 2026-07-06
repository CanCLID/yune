# M58 Long-Composition Reachability Corrective

Status: corrective follow-up complete, with a repeated M55 ratchet residual.

This note records the post-M58 shipped-product gap reported for current
`yune-web`: typing `zijiguk` did not allow selecting standalone `諮`, even
though the canonical upstream `rime-cantonese` lane already returns `諮議局`
first for the reported case.

This is not a new canonical-oracle claim. It is a TypeDuck/profile product-lane
composition reachability bug: when a phrase candidate is not suitable, a user
must be able to page to a standalone character for the leading complete
syllable, select it, and recompose the remaining input.

## Root Cause

The M58 product-lane corrective still had fixed-depth behavior in the
Jyutping prefix fallback and page-turn paths:

- the engine retained bounded `jyut6ping3_mobile` refreshes only within the
  original short-input scope;
- compact prefix fallback admitted only a small fixed number of rows per fetch
  code on bounded requests;
- the explicit Rime ABI `change_page` path completed a provisional list before
  moving forward, but the browser's physical `PageDown` key used selector
  layout handling and stayed on the bounded list until too late.

Together those behaviors allowed `beingo` / 畀 and bare `zi` / 諮 to pass while
longer composition input `zijiguk` still could not reach standalone `諮`.

## Corrective Behavior

The follow-up fix:

- removes the input-length gate from `jyut6ping3_mobile` bounded reachability
  surplus;
- makes bounded prefix-fallback work derive from the requested candidate limit
  instead of a fixed input-length cap;
- adds full-list leading-syllable standalone-character expansion so a
  multi-syllable composition can still page to the first syllable's single
  characters;
- keeps short-input TypeDuck/profile page guards intact for `beingo` / 畀 and
  bare `zi` / 諮;
- updates `RimeChangePage` and selector physical-key page handling so
  multi-character Jyutping page turns complete the provisional list before
  moving to the next page;
- keeps early full-list completion out of the short `zi` profile guard so
  `zi` / 諮 remains at the grandfathered TypeDuck/profile page position.

The focused regressions use real `yune-web` browser-app assets and prove:

1. typing `zijiguk` can page to standalone `諮`;
2. selecting that row commits only `諮`;
3. the remaining composition is `jiguk`, preserving the existing M28 partial
   selection/recomposition semantics.

## Focused Verification

Run during implementation:

```powershell
cargo test -p yune-rime-api --test yune_web m58 -- --nocapture
cargo test -p yune-rime-api --test yune_web web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion -- --nocapture
npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep M58 --workers=1
```

Observed result:

- `m58_yune_web_browser_app_assets_reach_profile_ranked_report_candidates`:
  passed; `beingo` / 畀 and bare `zi` / 諮 page positions remain guarded.
- `m58_yune_web_browser_app_assets_reach_prefix_candidate_for_long_reported_input`:
  passed; explicit `flip_page` reaches standalone `諮`, selection commits `諮`,
  and recomposes `jiguk`.
- `m58_yune_web_page_down_key_reaches_prefix_candidate_for_long_reported_input`:
  passed; the physical `PageDown` key path used by `yune-web` reaches
  standalone `諮`.
- M58 Playwright smoke: passed; current `yune-web` reaches `beingo` / 畀,
  `zi` / 諮, and `zijiguk` / 諮, then selection commits `諮` and
  recomposes `jiguk`.
- `web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion`:
  passed; the long-input prefix-fallback tripwire remains under its existing
  limits.

## Ratchet Residual

The standing M55/Track B ratchet was run twice with `-DeployProductBeforeBenchmark`
and `-FailOnRegression`:

- `long-composition-corrective-ratchet/`
- `long-composition-corrective-ratchet-rerun/`

Both runs failed the same guarded rows. Track A unrelated Luna rows failed
`ni`, `hao`, and `zhongguo`; the Track B Jyutping product row also missed its
latency ceiling narrowly:

| Run | Track B observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| `long-composition-corrective-ratchet/` | 362.039 us | 347.975 us | fail |
| `long-composition-corrective-ratchet-rerun/` | 359.349 us | 347.975 us | fail |

Memory guard rows passed in both runs. This is recorded as a live performance
guard residual, not as a hidden pass. The browser/user-visible reachability
fix remains verified by the Rust and Playwright gates above.
