# AI-Native Frontend Exposure Readiness Recommendation

> **Status:** Superseded by HR-7 evidence | **Milestone:** M9 / TypeDuck-Web browser validation | **Closed:** 2026-06-18 | **Type:** decision record (archived)

**Generated**: 2026-06-18 **Phase**: 17 / M9 (TypeDuck-Web Browser Validation) **Evidence Source**: docs/plans/archive/m09-findings-typeduck-web-integration.md

This record supersedes the Phase 10 tooling-blocked recommendation from 2026-05-05. That earlier result was a principled NO-GO because browser validation could not run without a WASM artifact. HR-5 and HR-6 replace it with real-browser and oracle-backed evidence.

---

Recommendation: GO WITH CONDITIONS

---

## Basis

The TypeDuck-Web path now runs Yune in a real browser against real TypeDuck `jyut6ping3_mobile` assets. The HR-5 matrix covers composition, candidate list, paging, selection, deletion, Space commit, phrase commit, deploy, customize, persistence sync, reload survival, and dictionary-panel comment rendering. The final browser capture records zero warning/error console entries.

The shared comment path is also covered: HR-6 adds a TypeDuck-HK/librime v1.1.2 fixture for the reverse-lookup `"; "` joiner and schema-prompt preedit bytes, with core and RIME C ABI tests asserting Yune against that oracle.

## Conditions

- AI-native behavior remains disabled by default in real frontends until the separate M11 provider, ranking, privacy, and fallback contracts are proven through the CLI slice and explicitly enabled.
- The five broader Cantonese/Jyutping parity cases remain explicit ignored tests pending dedicated TypeDuck v1.1.2 oracle captures. They do not block the TypeDuck-Web browser matrix, but they do block claiming full Cantonese parity.
- Future browser claims still require committed real-browser, real-asset artifacts under `apps/yune-web/e2e/results/`.

## Evidence

- `apps/yune-web/e2e/results/hr5-real-assets-matrix.json`
- `apps/yune-web/e2e/results/screenshot-hr5-dictionary-panel.png`
- `apps/yune-web/e2e/results/screenshot-hr5-after-delete.png`
- `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json`
- `crates/yune-core/tests/fixtures/typeduck-v1.1.2/reverse-lookup-prompt.json`
- `docs/plans/archive/m09-findings-typeduck-web-integration.md`

## Deferred Scope

AI-native provider calls, candidate generation, model/rule ranking, context capture, memory, privacy controls, and a first-party Yune frontend remain M11+ work. This recommendation only says the real TypeDuck-Web compatibility gate no longer blocks gated AI-native frontend exposure planning.

---

_Recommendation updated: 2026-06-18_ _Decision: D-P10-13 - HR-7 closes M9 with GO WITH CONDITIONS_
