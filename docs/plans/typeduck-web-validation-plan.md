# Yune → TypeDuck-Web: Browser Validation Plan (M9)

> **Status:** Active · **Milestone:** M9 (TypeDuck-Web browser validation) · **Created:** 2026-06-17 · **Type:** execution plan

> **Audience.** An autonomous coding agent (e.g. GPT) executing in the `yune` repo.
> Each work item is independently committable, names exact files, and ends with
> copy-pasteable acceptance gates.
>
> **Goal.** Actually **run Yune in a real web browser** through the TypeDuck-Web
> seam and turn the Phase-10 *NO-GO* into an **evidence-based GO/NO-GO**. The web
> build-out already exists; the engine has never been *observed* working in a
> browser because the WASM artifact was never built locally.
>
> **Line anchors** are accurate as of 2026-06-17 but *will drift* — re-`grep` the
> named symbol/file before editing. Trust names over line numbers.

This is the M9 counterpart to the parked
[`yune-windows-contract-implementation-plan.md`](./yune-windows-contract-implementation-plan.md).
Per web-first sequencing ([`../roadmap.md`](../roadmap.md)), finish this **before**
resuming the TypeDuck-Windows native work.

---

## 0. What already exists (do NOT rebuild)

Verified present on `main`:

| Piece | Location | State |
|---|---|---|
| Rust C/WASM adapter | `crates/yune-rime-api/src/typeduck_web.rs` | Exports `yune_typeduck_*`; emits JSON `{highlighted, candidates:[{text,comment}], …}`. Native contract tests pass. |
| TypeScript runtime | `packages/yune-typeduck-runtime/src/` (`response.ts`, `keys.ts`) | Parses per-candidate `comment`; `TypeDuckContext` exposes `highlighted` + `candidates`; key/mask mapping (incl. the recent `BackSpace` alias). |
| WASM build script | `scripts/typeduck-wasm-build.sh` | Emscripten / `wasm32-unknown-emscripten`; export list in `scripts/typeduck-exports.txt`. |
| Upstream app seam | tracked source: `third_party/typeduck-web/yune-integration/adapter.ts`; patch: `third_party/typeduck-web/patches/yune-typeduck-runtime.patch`; ignored checkout: `third_party/typeduck-web/source/src/yune-integration/adapter.ts` | Wires TypeDuck-Web's input engine to the Yune bridge. The tracked source/patch are the versions to fix in WI-2; the ignored checkout may be hot-patched locally but will not land in git. |
| Findings + blockers | [`typeduck-web-integration-findings.md`](./typeduck-web-integration-findings.md) | WI-4 browser run recorded PASS/FAIL evidence; core composition/selection pass, but paging/deletion/deploy/persistence/dictionary-comment parity fail. |
| Superseded recommendation | [`archive/ai-native-frontend-readiness.md`](./archive/ai-native-frontend-readiness.md) | The tooling-blocked NO-GO this plan replaces. |

The single thing that blocked Phase 10 was **no Emscripten toolchain** → no WASM
artifact → browser validation could not run. WI-1 removes that block.

---

## Execution order

```
1. Provision toolchain + build the WASM artifact   ← unblocks everything
2. Fix the TypeDuck-Web adapter shape mismatches    ← cheap; needed before E2E is meaningful
3. Browser filesystem: assets + persistence         ← init prerequisites
4. Run the real-browser E2E                          ← the actual validation
5. Record the evidence-based GO/NO-GO               ← supersedes the Phase-10 NO-GO
6. (parallel) Shared engine parity                   ← benefits web AND parked Windows
```

### Cross-cutting rules
- **Ownership (QUAL-01/02):** new behavior gets an owning module + owning test; keep facades thin.
- **Quality gate:** `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, focused tests, then `cargo test --workspace` if shared Rust changed; for TS, the package's typecheck + unit tests.
- **One commit per work item.** Update this plan's checklist plus `docs/roadmap.md` / `docs/requirements.md` as each lands.
- **Native fallback stays green:** `crates/yune-rime-api/tests/typeduck_web.rs` is the deterministic fallback when a browser isn't available — keep it passing throughout.

---

## Work Item 1 — Provision toolchain and build the WASM artifact

**Why first:** this is the exact gap that produced the Phase-10 NO-GO.

### Steps
1. Install the **Emscripten SDK** (`emsdk install latest && emsdk activate latest`) and add the Rust target:
   ```sh
   rustup target add wasm32-unknown-emscripten
   ```
2. Run the documented build:
   ```sh
   ./scripts/typeduck-wasm-build.sh        # -> target/wasm32-unknown-emscripten/debug/yune-typeduck.{js,wasm}
   ```
3. Verify **every** symbol in `scripts/typeduck-exports.txt` is present in the build's `EXPORTED_FUNCTIONS` (the script should fail loudly if not).
4. Verify the emitted Emscripten module instantiates and exposes `cwrap`, `UTF8ToString`, `FS`, and `IDBFS`; call one `yune_typeduck_*` export and perform one `FS` write/read.
5. If the toolchain genuinely cannot be installed in this environment, record a *reproducible* blocker (same discipline as the findings doc) and stop — but note this is an environment fix, not a design gap.

### Acceptance
- `yune-typeduck.js` + `yune-typeduck.wasm` are produced under `target/wasm32-unknown-emscripten/debug/`; all `typeduck-exports.txt` symbols are exported and the module smoke proves one `yune_typeduck_*` call plus one `FS` operation.
- `cargo test -p yune-rime-api --test typeduck_web` still green (native fallback intact).

---

## Work Item 2 — Fix and reapply the TypeDuck-Web adapter shape mismatches

The upstream seam reads a shape the runtime does not emit. Fix
the tracked source `third_party/typeduck-web/yune-integration/adapter.ts`
(re-grep; ~lines 177-184), then regenerate/apply the TypeDuck-Web patch so the
ignored checkout receives the same change before browser E2E:

| Bug | Current | Correct |
|---|---|---|
| candidate text | `text: candidate` (whole `{text,comment}` object) | `text: candidate.text` |
| candidate comment | `comment: response.context.comments?.[index]` (no such context-level array) | `comment: candidate.comment` (per-candidate) |
| highlight index | `response.context.highlighted_candidate_index ?? 0` (wrong key) | `response.context.highlighted ?? 0` |

Align against the runtime types in `packages/yune-typeduck-runtime/src/response.ts`
(`TypeDuckCandidate {text, comment}`, `TypeDuckContext {highlighted, candidates}`).
Keep the existing `BackSpace`/`Backspace` behavior covered: the Rust/runtime
alias should stay green, and the TypeDuck-Web adapter may normalize the upstream
`{BackSpace}` spelling as a frontend convenience.

### Acceptance
- The tracked adapter and generated/applied patch no longer contain
  `text: candidate`, `context.comments`, or `highlighted_candidate_index`.
- A focused TypeDuck-Web adapter smoke/typecheck proves a representative
  `yune_typeduck_*` JSON response maps candidate `text`, candidate `comment`,
  and `context.highlighted` into the app-facing candidate panel shape.
- `npm --prefix packages/yune-typeduck-runtime test` and
  `npm --prefix packages/yune-typeduck-runtime run build` still pass.

---

## Work Item 3 — Browser filesystem: assets and persistence

Resolve the "asset configuration TODO" the findings flagged.

### Steps
1. Host setup creates `shared_data_dir`, `user_data_dir`, and `user_data_dir/build` (TYPEDUCK-FS-01).
2. Preload the TypeDuck **schema + dictionary assets** into MEMFS **before** `yune_typeduck` init (TYPEDUCK-FS-02). Treat missing assets as an init-time failure.
3. IDBFS (or equivalent) syncs **before init** and **after** deploy / customize / userdb mutations (TYPEDUCK-FS-03).

### Acceptance
- Init succeeds with assets present; a deploy followed by reload **persists** (smoke).
- Missing-asset and failed-sync paths surface a clear error (not a silent hang).

---

## Work Item 4 — Run the real-browser E2E

Serve the TypeDuck-Web app wired to the Yune bridge and drive it in a real browser
(Playwright / headless Chromium preferred; a documented manual smoke is an
acceptable fallback). Move every row of the findings matrix from **BLOCKED** to
**PASS/FAIL with captured evidence**:

1. composition (preedit builds from keys)
2. candidate paging
3. candidate selection
4. candidate deletion
5. commit output
6. deploy
7. customize
8. persistence sync
9. persistence reload
10. **dictionary-panel comment rendering** — assert the `RimeCandidate.comment`
    bytes against the v1.1.2 oracle (`crates/yune-core/tests/fixtures/typeduck-v1.1.2/`) where applicable.

### Acceptance
- Each of the 10 flows has a recorded PASS/FAIL with evidence (screenshot / console / trace).
- Core composition → candidate → commit demonstrably works in-browser, or the exact failing flow is captured reproducibly.

---

## Work Item 5 — Record the evidence-based GO/NO-GO

1. Update [`typeduck-web-integration-findings.md`](./typeduck-web-integration-findings.md):
   replace the BLOCKED matrix with the WI-4 results.
2. Write the recommendation that **supersedes** the Phase-10 NO-GO in
   [`archive/ai-native-frontend-readiness.md`](./archive/ai-native-frontend-readiness.md):
   exactly one `GO` / `GO WITH CONDITIONS` / `NO-GO` line, grounded in browser evidence.
3. Update [`../roadmap.md`](../roadmap.md) (M9), [`../requirements.md`](../requirements.md) (`TYPEDUCK-E2E-03`), and [`../decisions.md`](../decisions.md).

### Acceptance
- One recommendation line, evidence-referenced; tracking docs reflect the real result.

---

## Work Item 6 — Shared engine parity *(parallel; benefits web and parked Windows)*

Not strictly required for the browser run, but it hardens the comment path both
frontends share. Drive from the v1.1.2 oracle:
- Extend the (already non-circular) comment byte-parity test with the `"; "`
  reverse-lookup joiner and schema-name-in-prompt oracle cases; ideally feed real
  shipped `.dict.yaml` rows rather than in-test authored rows.
- Capture the v1.1.2 goldens for the 5 ignored Cantonese/Jyutping cases in
  `crates/yune-core/tests/cantonese_parity.rs` and activate them.

### Acceptance
- `cargo test -p yune-core --test cantonese_parity` runs the previously-ignored cases (or documents any still-blocked golden).

---

## GSD Phase 17 mapping

- **17-01** = WI-1 + WI-2 (build the artifact; fix/reapply the tracked adapter shapes)
- **17-02** = WI-3 + WI-4 + WI-5 (assets/persistence; real-browser E2E; GO/NO-GO)
- **17-03** = WI-6 (shared engine parity)

## Known risks / blockers
- **Emscripten availability** — cleared locally by WI-1/WI-1b; keep the reproducible build green.
- **Headless browser availability** for automated E2E (else documented manual smoke).
- **Upstream TypeDuck-Web build** — the app must build and serve with the Yune bridge wired in.

## Summary checklist
- [x] **WI-1** — Emscripten + loadable WASM/JS artifact built; exports verified; native fallback green
- [x] **WI-2** — `adapter.ts` text/comment/highlight shapes fixed + unit-tested
- [x] **WI-3** — browser FS layout, asset preload, and IDBFS sync wired into the patched app seam
- [x] **WI-4** — 10 E2E flows run in a real browser with captured PASS/FAIL evidence
- [ ] **WI-5** — evidence-based GO/NO-GO recorded; tracking docs updated
- [ ] **WI-6** — shared engine parity (`"; "` joiner + schema-prompt; Cantonese goldens)
