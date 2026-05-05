# AI-Native Frontend Exposure Readiness Recommendation

**Generated**: 2026-05-05T16:40:00Z
**Phase**: 10 (TypeDuck-Web App Integration And E2E)
**Evidence Source**: docs/typeduck-web-integration-findings.md

---

Recommendation: NO-GO

---

## Basis

Browser validation of TypeDuck-Web + Yune seam cannot run at all due to environment/tooling blockers preventing WASM artifact generation. Composition/candidate/commit flows have not been validated in real browser execution. The seam patch is structurally sound and adapter/runtime mismatches are handled, but without browser validation we cannot demonstrate core classic input behavior through TypeDuck-Web.

**Evidence from Phase 10** (see docs/typeduck-web-integration-findings.md):

**Upstream seam inspection** (Plan 10-01):
- TypeDuck-Web upstream source pinned to revision 03f9afd2cf6ca75653197f2193f24d1cd0adbd83
- Seam files identified: src/worker.ts (primary replacement), src/rime.ts (preserve facade), src/types.ts (preserve Actions interface)
- Original librime/WASM call path documented: UI → Worker → Module.ccall → api.cpp → librime
- Contract mismatches identified: string input vs keycode/mask, RimeResult vs TypeDuckResponse, missing setOption, persistence timing

**Yune seam patch** (Plan 10-02):
- Minimal patch generated: package.json (Yune alias), src/worker.ts (adapter imports), src/yune-integration/ (adapter.ts, assets.ts)
- Contract mismatches addressed: adapter parses key sequences, translates response shapes, uses Yune persistence helpers
- Build gates passed: repository runtime build PASSED, upstream package install PASSED, worker build PASSED, TypeScript typecheck PASSED
- TypeDuck-Web app/source blockers documented: asset configuration TODO, WASM artifact path
- Yune adapter/runtime mismatches documented: TypeDuckContext properties missing, setOption gap, customize bitmap incomplete
- Environment/tooling blockers: none at build stage

**Browser E2E execution attempt** (Plan 10-03):
- Browser E2E spec created covering all D-08/TYPEDUCK-E2E-03 flows (composition, candidates, paging, selection, deletion, deploy, customize, persistence)
- Upstream has NO browser test framework, standalone Playwright spec created
- **Execution BLOCKED**: cargo/rustup/emcc missing, WASM artifact cannot be built
- **All flows BLOCKED**: Composition, candidate list, paging, selection, deletion, deploy, customize, persistence sync/reload cannot run without WASM artifact
- Evidence captured: blocker.md documents exact commands, missing dependencies, install hints per D-09

**Blocker taxonomy** (Final findings):
- **TypeDuck-Web app/source blockers**: 2 (asset configuration TODO, WASM artifact path) — **Do NOT block AI-native frontend** (deployment/setup requirements)
- **Yune adapter/runtime mismatches**: 3 (TypeDuckContext properties, setOption gap, customize bitmap) — **Do NOT block AI-native frontend** (compatibility layers handle mismatches)
- **Environment/tooling blockers**: 4 (cargo missing, rustup missing, emcc missing, WASM artifact not built) — **BLOCK AI-native frontend** (browser validation cannot run)

**Rubric application** (from Plan 10-04 context):

NO-GO applies when:
- "Browser validation cannot run at all without unbounded blockers" ✓ **TRUE** — Browser validation cannot run at all
- OR "composition/candidate/commit fails due to runtime mismatch or upstream seam incompatibility" ✗ **FALSE** — Seam patch is structurally sound, no upstream incompatibility detected

**Analysis of "unbounded blockers"**:
- Blockers are bounded (cargo/rustup/emcc have documented install paths at rustup.rs, emscripten.org)
- Blockers are standard tooling dependencies, not unknown/unfixable issues
- Blockers are reproducible with exact commands and fallback evidence per D-09
- However, rubric states "cannot run at all" without qualification about boundedness

**Strict interpretation**: Without browser validation demonstrating core composition/candidate/commit works, we cannot satisfy GO or GO WITH CONDITIONS requirements. NO-GO is the principled recommendation acknowledging we lack browser evidence.

**Pragmatic assessment**: Seam is structurally sound, blockers are bounded with clear resolution path, but browser validation prerequisite not met. Recommendation reflects inability to demonstrate working browser behavior, not fundamental seam incompatibility.

---

## Conditions or blockers

**Primary blockers** (environment/tooling):

1. **cargo missing** — Rust build tool unavailable
   - Evidence: `./scripts/typeduck-wasm-build.sh` → `cargo: command not found`
   - Impact: WASM artifact cannot be built, browser runtime cannot initialize
   - Resolution: Install Rust toolchain from https://rustup.rs
   - Blocks: All TYPEDUCK-E2E-03 flows (composition, candidates, paging, selection, deletion, deploy, customize, persistence)

2. **rustup missing** — Cannot install wasm32-unknown-emscripten target
   - Evidence: `rustup target list --installed` → `command not found: rustup`
   - Impact: WASM target not available, Emscripten compilation blocked
   - Resolution: Install rustup from https://rustup.rs, add WASM target

3. **emcc missing** — Emscripten compiler unavailable
   - Evidence: `emcc --version` → `emcc not found`
   - Impact: Cannot compile Rust to WASM/JS glue, browser runtime cannot load
   - Resolution: Install Emscripten SDK from https://emscripten.org/docs/getting_started/downloads.html

4. **WASM artifact not built** — No yune-typeduck.js/yune-typeduck.wasm generated
   - Evidence: blocker.md documents missing Phase 7 artifact
   - Impact: Browser runtime cannot initialize, classic input through TypeDuck-Web cannot be tested
   - Resolution: Resolve tooling blockers, run ./scripts/typeduck-wasm-build.sh

**Secondary blockers** (TypeDuck-Web app/source):

5. **Asset configuration TODO** — Patched worker has placeholder YAML content
   - Evidence: src/worker.ts lines 246-251 `content: ""`
   - Impact: Runtime init may fail without explicit assets
   - Resolution: Provide explicit default.yaml, schema.yaml, dictionary.yaml from TypeDuck-Web source or CDN
   - Severity: Non-critical, does NOT block AI-native frontend (deployment requirement)

**Tertiary blockers** (Yune adapter/runtime):

6. **TypeDuckContext properties missing** — comments/highlighted_candidate_index not in interface
   - Evidence: adapter.ts maps to undefined/0
   - Impact: Candidate comments may differ, highlight behavior may vary
   - Severity: Accepted, does NOT block AI-native frontend (compatibility layer handles)

7. **setOption API gap** — Yune wrapper lacks setOption method
   - Evidence: adapter.ts throws error documenting gap per D-07
   - Impact: setOption calls fail, customize/deploy paths unaffected
   - Severity: Accepted, does NOT block AI-native frontend (not required for core flows)

8. **customize options bitmap incomplete** — pageSize mapped, options bitmap partial
   - Evidence: adapter.ts customize handling
   - Impact: Customize may have partial behavior
   - Severity: Accepted, does NOT block AI-native frontend (non-critical edge case)

**Total blockers**: 8 (4 environment/tooling, 2 TypeDuck-Web source, 2 Yune adapter/runtime)

**Blocking AI-native frontend exposure**: 4 environment/tooling blockers prevent browser validation

---

## What remains deferred

The following AI-native product features remain explicitly deferred per D-14 and are NOT part of Phase 10 or this recommendation:

1. **AI-native provider calls** — Engine exposing AiCandidateProvider interface for LLM-assisted candidates
2. **AI-native candidate generation** — Generating candidates from local models or remote inference without replacing classic translators
3. **AI-native ranking** — Local model and rule-backed ranking implementations with timeout/fallback behavior
4. **AI-native context capture** — Context providers defining app/field/text/cursor/schema data sharing with AI providers
5. **AI-native memory** — Memory store recording user vocabulary, phrase preferences, domain terms through inspectable policy
6. **AI-native privacy controls** — Privacy policy disabling learning and remote calls for sensitive contexts, keeping classic input functional when AI disabled
7. **New first-party Yune frontend** — Yune-owned frontend exposing AI-native features and Yune-specific UI controls (FRONTEND-01, FRONTEND-02)

**Current scope**: Classic RIME input through existing frontends (TypeDuck-Web, Squirrel, ibus-rime, fcitx-rime) with Yune compatibility foundation. AI-native layer is a separate future milestone requiring its own planning, provider contracts, ranking policy, context/memory architecture, and privacy controls before new first-party Yune frontend exposure.

**Decision D-14 enforcement**: This recommendation does not implement, plan implementation, or document implementation steps for any deferred AI-native features. Phase 10 scope ends at TypeDuck-Web browser integration readiness assessment.

---

## Next allowed work

Following NO-GO recommendation, next work is bounded to resolving blockers and obtaining browser evidence, NOT implementing deferred AI-native features:

**Phase 10 follow-up** (resolving blockers):

1. **Environment setup for WASM artifact generation**
   - Install Rust toolchain (cargo, rustup) from https://rustup.rs
   - Add wasm32-unknown-emscripten target via `rustup target add wasm32-unknown-emscripten`
   - Install Emscripten SDK (emcc, emar) from https://emscripten.org
   - Configure PATH so cargo/rustup/emcc are available
   - Run ./scripts/typeduck-wasm-build.sh to generate yune-typeduck.js/yune-typeduck.wasm

2. **Asset configuration for browser E2E**
   - Obtain explicit TypeDuck-Web-owned YAML assets (default.yaml, schema.yaml, dictionary.yaml)
   - Configure assets in patched worker or E2E runner
   - Document asset sources/paths per e2e/assets/README.md

3. **Browser validation execution**
   - Install Playwright OR use manual browser smoke procedure from e2e/yune-browser-smoke.md
   - Run browser E2E spec or manual smoke covering all D-08/TYPEDUCK-E2E-03 flows
   - Capture evidence: browser-run.log, screenshots, persistence-sync.log, blocker.md updates
   - Verify composition, candidate paging, selection, deletion, deploy, customize, persistence flows work

4. **Re-assess recommendation with browser evidence**
   - If composition/candidate/commit passes in real browser → upgrade to GO WITH CONDITIONS or GO
   - Document runtime mismatches discovered during browser execution
   - Update blocker taxonomy with observed behavior, not theoretical compatibility

**Future milestone planning** (NOT Phase 10 follow-up):

5. **AI-native input layer roadmap planning** (requires separate milestone definition)
   - Define AI-native provider contracts (AiCandidateProvider interface design)
   - Define ranking policy and fallback behavior (timeout bounds, classic fallback priority)
   - Define context capture architecture (what app/field/text data may be shared)
   - Define memory store architecture (vocabulary recording, preference tracking, domain terms)
   - Define privacy controls (sensitive context detection, learning disable, remote call disable)
   - Define frontend integration points (how existing frontends expose AI-native controls)
   - Plan implementation phases AFTER TypeDuck-Web browser validation succeeds

6. **Yune-specific frontend research** (NOT implementation in Phase 10)
   - Research UI/UX patterns for AI-native candidate presentation (source labels, confidence indicators, fallback visibility)
   - Research context capture UI controls (per-app/field opt-in, context preview, data inspectability)
   - Research memory/privacy controls UI (vocabulary inspection, preference editing, learning toggle, context blacklist)
   - Document research findings for future frontend implementation milestone

**Scope boundary**: Phase 10 ends at browser validation readiness recommendation. AI-native implementation planning begins AFTER browser validation succeeds and separate milestone is defined. NO-GO recommendation does not trigger AI-native implementation work.

---

## Recommendation Summary

**Status**: NO-GO for AI-native frontend exposure

**Reason**: Browser validation cannot run at all due to environment/tooling blockers. Without browser evidence demonstrating core composition/candidate/commit behavior through TypeDuck-Web, we cannot satisfy GO or GO WITH CONDITIONS requirements.

**Structural assessment**: TypeDuck-Web seam patch is sound, adapter handles contract mismatches, blockers are bounded (not fundamental seam incompatibility). Resolution path is clear: install tooling, build WASM artifact, run browser validation.

**Deferred scope**: AI-native provider calls, candidate generation, ranking, context, memory, privacy controls, and new first-party frontend remain deferred per D-14. This recommendation does not plan or implement any AI-native features.

**Next milestone**: Resolve environment/tooling blockers, obtain browser validation evidence, re-assess recommendation. AI-native input layer planning requires separate milestone definition after TypeDuck-Web integration succeeds.

---

*Recommendation generated: 2026-05-05T16:40:00Z*
*Evidence source: docs/typeduck-web-integration-findings.md*
*Phase: 10 (TypeDuck-Web App Integration And E2E)*
*Decision: D-13 — GO, GO WITH CONDITIONS, or NO-GO for AI-native frontend exposure*