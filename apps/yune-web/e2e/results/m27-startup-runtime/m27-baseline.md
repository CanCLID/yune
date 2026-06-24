# M27 Baseline

> **Status:** Captured from M26 - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

## M26 Startup Numbers

- Browser fresh startup after M26: `startup:complete.totalMs` about `10.7s`.
- Browser reload startup after M26: about `10.4s`.
- Native `startup_real_jyut6ping3_mobile_runtime_ready`: median about `15.1s`, p95 about `15.6s`.
- Native `startup_real_luna_pinyin_runtime_ready`: median about `0.79s`, p95 about `0.80s`.

## Rule

M27 must not close until native startup rows include a real Windows process memory metric.
