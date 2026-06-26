# M41 Final Gates

M41 is a browser-harness milestone for `apps/yune-web/`. It does not claim
native engine performance, product delivery, or platform frontend performance.

## Environment

| Field | Value |
| --- | --- |
| Branch | `codex/m41-yune-web-startup-optimization` |
| Pre-commit benchmark HEAD | `0a4011a7612863774b55968702bc73605520900e` |
| Dirty state during benchmark | Dirty with M41 implementation/evidence changes |
| Browser | Chromium `149.0.7827.55` through Playwright |
| Build mode | Production Vite build and production public-demo build |
| Evidence root | `apps/yune-web/e2e/results/m41-yune-web-startup-optimization/` |
| Baseline evidence | `phase-0-one-sample/` |
| Final evidence | `phase-7-final-full/` |
| Final samples | `150` total samples across `10` scenarios |

## What Changed

- Added a production runtime packaging step so `yune-web.js` and
  `yune-web.wasm` are copied into the app/public-demo runtime before build.
- Fixed `build:public` so the public-demo build no longer overwrites the
  tracked app's `dist/` output while preparing its own package.
- Passed the selected schema to `worker.js` at worker construction time.
- Skipped the first React schema-select action when the worker already started
  with the requested schema.
- Skipped the initial customize/deploy call when the preference set is exactly
  the default deploy state.
- Scoped startup asset loading by schema so `luna_pinyin` startup avoids the
  full Jyutping payload.
- Added a focused Playwright startup benchmark with real-worker, mock-worker,
  tracked build, public-demo build, memory, resource, cache, startup-owner, and
  first-key metrics.

## Before And After

| Scenario | Phase 0 ready median | Final ready median | Improvement |
| --- | ---: | ---: | ---: |
| tracked luna cold | `3,115 ms` | `846 ms` | `72.8%` |
| tracked luna warm reload | `2,399 ms` | `266 ms` | `88.9%` |
| tracked luna warm new page | `2,438 ms` | `306 ms` | `87.4%` |
| tracked jyut cold | `17,041 ms` | `1,254 ms` | `92.6%` |
| tracked jyut warm reload | `15,783 ms` | `654 ms` | `95.9%` |
| tracked jyut warm new page | `16,081 ms` | `704 ms` | `95.6%` |
| public luna cold | `3,119 ms` | `867 ms` | `72.2%` |
| public jyut cold | `16,872 ms` | `1,291 ms` | `92.3%` |

## Final Startup Rows

| Scenario | Samples | Ready median | Ready p95 | Worker startup median | First-key median | Transfer bytes | Encoded bytes | Cache h/m/e | Top remaining owner |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| tracked-luna-cold | `10` | `846 ms` | `932 ms` | `250 ms` | `64 ms` | `5,451,094` | `5,449,894` | `0/0/0` | Browser/React residual, `601 ms` |
| tracked-luna-warm-reload | `20` | `266 ms` | `268 ms` | `203 ms` | `86 ms` | `0` | `5,449,894` | `0/0/0` | Worker initialized, `203 ms` |
| tracked-luna-warm-new-page | `20` | `306 ms` | `322 ms` | `205 ms` | `86 ms` | `0` | `5,449,894` | `0/0/0` | Worker initialized, `205 ms` |
| tracked-jyut-cold | `10` | `1,254 ms` | `1,330 ms` | `646 ms` | `26 ms` | `34,862,027` | `34,860,827` | `0/0/0` | Worker initialized, `646 ms` |
| tracked-jyut-warm-reload | `20` | `654 ms` | `691 ms` | `592 ms` | `26 ms` | `0` | `35,142,383` | `0/0/0` | Worker initialized, `592 ms` |
| tracked-jyut-warm-new-page | `20` | `704 ms` | `737 ms` | `601 ms` | `27 ms` | `0` | `34,860,827` | `0/0/0` | Worker initialized, `601 ms` |
| tracked-mock-cold | `10` | `609 ms` | `646 ms` | N/A | `25 ms` | `743,347` | `742,147` | `0/0/0` | Browser/React residual, `609 ms` |
| tracked-mock-warm | `20` | `383 ms` | `393 ms` | N/A | `25 ms` | `743,347` | `742,147` | `0/0/0` | Browser/React residual, `383 ms` |
| public-luna-cold | `10` | `867 ms` | `883 ms` | `263 ms` | `40 ms` | `5,451,034` | `5,449,834` | `2/20/0` | Browser/React residual, `600 ms` |
| public-jyut-cold | `10` | `1,291 ms` | `1,349 ms` | `680 ms` | `26 ms` | `34,861,967` | `34,860,767` | `1/36/0` | Worker initialized, `680 ms` |

## First-Key Cold Rows

Final cold rows remain interactive after ready. The phase-0 baseline was a
single-owner sample, so the raw before/after percent comparison is not treated
as statistically meaningful for first-key timing.

| Scenario | Input | Median | p95 | Median candidates |
| --- | --- | ---: | ---: | ---: |
| tracked luna cold | `hao` | `22.5 ms` | `27 ms` | `5` |
| tracked luna cold | `ni` | `23.5 ms` | `34 ms` | `5` |
| tracked luna cold | `zhongguo` | `23 ms` | `49 ms` | `5` |
| tracked luna cold | `ceshiyixiachangjushuruxingnengzenyang` | `77 ms` | `98 ms` | `0` |
| tracked luna cold | `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `113 ms` | `182 ms` | `0` |
| tracked luna cold | `cszysmsrsd` | `134 ms` | `231 ms` | `0` |
| tracked luna cold | `zybfshmsru` | `169.5 ms` | `235 ms` | `0` |
| tracked jyut cold | `hai` | `26 ms` | `28 ms` | `5` |
| tracked jyut cold | `ngo` | `22.5 ms` | `24 ms` | `5` |
| tracked jyut cold | `caksi` | `22.5 ms` | `24 ms` | `0` |
| tracked jyut cold | `sihaacoenggeoisyujapgecukdou` | `42 ms` | `49 ms` | `5` |
| tracked jyut cold | `taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng` | `39.5 ms` | `63 ms` | `0` |

## Memory

| Scenario | JS heap used | JS heap total | DOM nodes | Windows working set |
| --- | ---: | ---: | ---: | ---: |
| tracked-luna-cold | `5,672,764` | `11,444,224` | `1,069` | `497,397,760 B` |
| tracked-luna-warm-reload | `9,708,900` | `15,048,704` | `1,700` | `533,327,872 B` |
| tracked-luna-warm-new-page | `5,928,248` | `11,706,368` | `1,087` | `506,036,224 B` |
| tracked-jyut-cold | `6,051,784` | `38,789,120` | `1,189` | `718,516,224 B` |
| tracked-jyut-warm-reload | `6,528,840` | `16,359,424` | `802` | `756,490,240 B` |
| tracked-jyut-warm-new-page | `6,116,020` | `38,526,976` | `1,071` | `708,665,344 B` |
| tracked-mock-cold | `4,399,168` | `8,495,104` | `609` | `360,001,536 B` |
| tracked-mock-warm | `7,342,420` | `11,902,976` | `1,713` | `412,348,416 B` |
| public-luna-cold | `5,319,260` | `11,706,368` | `1,069` | `500,723,712 B` |
| public-jyut-cold | `6,078,336` | `38,789,120` | `1,189` | `724,533,248 B` |

## Verdict

M41 closes the web-harness startup optimization target. The old multi-second
startup owner was not native lookup; it was browser/runtime packaging plus a
redundant startup deploy path. Final real-worker cold medians are below the
M41 budget: `846 ms` for `luna_pinyin` and `1,254 ms` for
`jyut6ping3_mobile` in the tracked app, with public-demo cold medians of
`867 ms` and `1,291 ms`.

Remaining owners are lower-level browser/application costs:

- cold `luna_pinyin` is now mostly Browser/React ready residual around
  `600 ms`, matching the mock-worker shell floor;
- cold `jyut6ping3_mobile` is now mostly worker initialization around
  `600-700 ms`;
- Jyutping still carries a large local encoded asset footprint around
  `34.9 MB`, but it no longer creates a 15 second startup delay in the local
  production harness.

## Verification

- `npm.cmd --prefix apps/yune-web run build`
- `npm.cmd --prefix apps/yune-web run build:public`
- `npm.cmd --prefix apps/yune-web run typecheck`
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "M41 STARTUP" --workers=1`
- `npm.cmd --prefix apps/yune-web/e2e run test:e2e -- --grep "Composition after typing schema-valid keys|Candidate list visible|M41 default startup preserves deploy-time engine defaults" --workers=1`
- `git diff --check`

Rust gates were not required because M41 did not touch Rust files.

Review follow-up: the first M41 implementation skipped the default startup
deploy while the shipped schema defaults still disabled some app-default
engine features. The follow-up fix bakes the default deploy preference set into
the shipped schema assets and adds the `M41 default startup preserves
deploy-time engine defaults` browser guard. That guard proves the fast path
starts with completion on, correction off, sentence on, user dictionary on,
combined candidates on, prediction-never-first on, threshold `0`, no
dictionary exclude, and Cangjie 5 selected without a startup deploy marker.

Broad historical smoke caveat: the older full `@smoke` tag is still not the
M41 closeout claim. After the review fix, the current focused composition,
candidate visibility, and M41 deploy-default guard pass. The legacy M16
sentence/combine rows remain separate browser-behavior failures and were not
proven green on the pre-M41 base during this review pass, so they should be
handled by a future browser-parity cleanup rather than folded into this startup
optimization closeout.
