# M27 Optimization Choice

> **Status:** Captured with M27 native attribution - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** decision evidence

## Chosen Owner

Optimize `spelling_algebra_expand` in `crates/yune-core/src/spelling_algebra.rs`.

## Evidence

- Native browser-matching startup was dominated by `select_schema_total`.
- `translator_install` accounted for nearly all `select_schema_total`.
- `spelling_algebra_expand` accounted for nearly all `translator_install`: median `14,820,345us` before optimization.
- Compiled table, prism, and reverse payload loading were millisecond-scale and not the startup bottleneck.

## Implementation

The TypeDuck dictionary has many candidate rows per original code. Before M27, the spelling-algebra pipeline ran every regex formula against every candidate row. M27 caches the expanded code variants per original code, then applies those variants to the candidates for that code while preserving the existing final dedupe and candidate-ranking model.

The change is intentionally local to the core spelling-algebra expansion path. It does not widen the C ABI, change resource path handling, or make TypeDuck-Web deploy behavior the optimization target.

## Rejected Options

- Reusing compiled prism descriptors directly for startup lookup construction was rejected for M27 because the current table parser discards the table syllabary after expanding table entries. A prism fast path would require new metadata plumbing and a larger equivalence proof.
- Optimizing `deploy_real_jyut6ping3_mobile_cache_hit` was rejected because M26 browser diagnostics show deploy is short and the expensive wait is post-deploy RIME init/schema selection.
