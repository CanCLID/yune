# M41 Phase 1 Owner Summary

Phase 0 identified two blockers before optimization:

- The production build did not package `yune-web.js` and `yune-web.wasm` into
  the built app/public-demo runtime, so the worker could fail at
  `importScripts("./yune-web.js")`.
- After the worker reached `startup:complete`, the React startup path still ran
  the default customize/deploy flow. A focused trace measured that deploy action
  at about `15,538 ms` on the `jyut6ping3_mobile` startup path.

The one-sample baseline was intentionally bounded because the pre-fix path took
about `16-17 s` on Jyutping rows. The final M41 run is the full statistical run.

| Scenario group | Phase 0 top owner | Phase 0 ready median | Planned fix |
| --- | --- | ---: | --- |
| tracked luna cold | Browser/app residual after worker ready | `3,115 ms` | Package the WASM runtime, initialize the worker with the selected schema, and avoid redundant startup deploy work. |
| tracked jyut cold | Default deploy/customize after worker ready | `17,041 ms` | Skip no-op default deploy preferences and keep the initial schema in the worker startup path. |
| public luna cold | Browser/app residual after worker ready | `3,119 ms` | Build public-demo into its own output directory and package the same runtime artifacts. |
| public jyut cold | Default deploy/customize after worker ready | `16,872 ms` | Same as tracked Jyutping, with public-demo runtime packaging evidence. |

Decision: optimize runtime packaging and startup deploy/schema reuse first
because they were the repeated multi-second blockers. Asset scoping was then
applied so `luna_pinyin` startup does not eagerly load the full Jyutping asset
set.
