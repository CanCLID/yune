# M31 yune-web Public Demo Readiness Implementation Plan

> **Status:** Complete - **Milestone:** M31 (`yune-web` public demo readiness) - **Created:** 2026-06-22 - **Closed:** 2026-06-24 - **Type:** execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

## Closeout Summary

M31 is complete. The public demo is deployed as `yune-web` at
<https://yune-web.pages.dev> through Cloudflare Pages Direct Upload.
The public surface exposes only the supported
`jyut6ping3_mobile` schema with Hong Kong Traditional and `hk2s` Simplified
output-standard controls, records owner-approved TypeDuck provenance, builds
from checked-in Yune state, and keeps AI default-off/local-only with no remote
calls, telemetry, or committed secrets.

Evidence is under `apps/yune-web/e2e/results/m31-yune-web-public-demo/`.
Local preview, deployed Playwright smoke, Rust/runtime/build gates, patch
reverse/forward checks, and `git diff --check` passed. The repo-owned harness
migrated from `third_party/typeduck-web/` to `apps/yune-web/`; the ignored
upstream-derived checkout now lives at `apps/yune-web/source/`.

Measured caveat: M31 proves public delivery pruning and content-addressed warm
cache behavior, not a browser startup or typing speed win. The public boot path
excludes non-public schema families but still loads Luna and scholar lookup
dependencies because current TypeDuck schema behavior requires them. Full
reverse-lookup dependency deferral is a future scoped follow-up, not an M31
claim.

**Goal:** Rename the internal TypeDuck-Web-derived dogfood harness into `yune-web`, then make it ready to publish as a public Yune engine demo with owner-approved TypeDuck assets, payload pruning, browser-honest output-standard support only where needed, a `my_rime`-style lazy delivery model for public assets, and a reproducible Cloudflare deployment path.

**Architecture:** M31 is a deterministic `yune-web` readiness milestone, not a new AI product milestone and not the primary Windows product track. This repo's harness is now `apps/yune-web/`: a Yune engine playground/stress surface derived from TypeDuck-Web, while the real TypeDuck-Web product belongs in a separate product repo. TypeDuck-Web shell/dictionary redistribution is owner-approved by the project owner, who is also the TypeDuck author, so this plan does not carry a separate licensing blocker. Public UI, deployment, docs, and the repo-owned harness path use `yune-web`; a short public provenance note explains that the page exercises the Yune engine through an upstream-derived shell. Then add only the OpenCC output standards Yune can prove and that the public demo actually needs.

The `LibreService/my_rime` comparison changes M31's delivery bar. `my_rime` feels fast in the browser because it separates the app shell from schema payloads: the worker fetches only the selected schema's prebuilt `.table.bin`/`.prism.bin`/`.reverse.bin` dependencies, caches them by content hash, keeps engine work off the main thread, and relies on PWA/CDN caching for warm visits. Yune already has a worker, Emscripten FS/IDBFS, and prebuilt schema assets; the M31 public demo must therefore stop treating the internal dogfood harness's eager all-schema asset load as acceptable public-demo behavior. Finally make the patched TypeDuck-Web source reproducible from checked-in Yune state, pin and prune the WASM/schema assets, add active-schema-only loading plus warm-cache evidence, and publish through a Cloudflare Pages Direct Upload deployment with smoke evidence.

**Tech Stack:** Rust (`yune-core`, `yune-rime-api`), OpenCC source dictionaries/config chains, `packages/yune-typeduck-runtime`, TypeDuck-Web Vite + React + Tailwind source under `apps/yune-web/source/`, Playwright, Wrangler, and Cloudflare Pages Direct Upload deployment.

---

## Status

Complete. P2-WIN-02 was already complete before deployment. P2-WIN-01 remains
the primary Windows product track, and M31 did not widen Yune's default ABI or
touch Windows frontend work. Public deployment used the `yune-web` identity,
owner-approved TypeDuck provenance, measured payload pruning, local preview,
deployed Cloudflare smoke, and a documented delivery caveat for remaining
Luna/scholar boot dependencies.

## Scope

In scope:

- OpenCC output-standard support that is real in Yune, not only a browser label.
- Runtime and TypeDuck-Web UI for supported output standards.
- Rename/rebrand the public harness from TypeDuck-Web dogfood to **`yune-web`** before deployment. The public app title, deployment name, docs, provenance, smoke evidence, public copy, and repo-owned app path must use the `yune-web` name. M31 completed the mechanical path migration from `third_party/typeduck-web/` to `apps/yune-web/`.
- Public provenance note stating that TypeDuck-Web shell/dictionary use is owner-approved by the TypeDuck author and that the page is a Yune engine demo.
- Public payload pruning so the deployed demo ships only the assets needed for the public surface.
- Active-schema-only runtime asset loading for the public demo. The first paint/ready path must not fetch every schema, source dictionary, and compiled artifact in `apps/yune-web/source/public/schema`.
- Lazy reverse-lookup dependency loading. Assets needed only for reverse lookup, such as `luna_pinyin`/`cangjie5`/`stroke`-style dependencies or `luna_pinyin_yune_reverse.dict.yaml`, belong in a distinct dependency tier and should load on first reverse lookup rather than at boot.
- Content-addressed schema/runtime asset metadata and warm-cache behavior, using IndexedDB, Cache Storage, or an equivalent browser cache. Relying only on a hand-bumped `?v=` query string is not enough for the public-readiness gate.
- PWA/service-worker or Cloudflare immutable-cache behavior for repeat visits, with cold/warm evidence and explicit offline/near-offline limits.
- Reproducible reconstruction of the patched `apps/yune-web/source/` checkout from checked-in lock/patch/bridge state.
- Release WASM and schema asset packaging for a public demo.
- Cloudflare deployment config, local preview, deployed smoke evidence, and docs.
- AI posture gate: default-off, local-only, no remote calls, no telemetry, no secrets, and AI-off byte identity.

Out of scope:

- Richer AI-native UX or remote-provider work. That belongs to M32.
- Windows TSF/frontend work. That belongs to P2-WIN-01 after P2-WIN-02.
- Candidate ranking, Jyutping composition, TypeDuck comment-byte compatibility, `RimeApi`, or `RimeCandidate` changes unless a focused OpenCC fixture proves the need.
- Treating `typeduck.hk/web` as an oracle. It can be a comparison target only.
- Treating `my_rime` as a behavior oracle or copying its source wholesale. It is a delivery and librime-WASM architecture reference; Yune behavior remains fixture/oracle driven by upstream librime and TypeDuck profile captures.
- Splitting `apps/yune-web/` into a separate repository or submodule.
- Renaming or restructuring the separate real TypeDuck-Web product repo. M31 only renames this repo's Yune-owned harness and public deployment identity.
- Blocking P2-WIN-01 on Cloudflare/demo polish.

## Preconditions

- M30 is complete and the current branch is synced with `origin/main`.
- P2-WIN-02 is complete before public deployment. Engine/UI work may be prepared earlier only if the plan record says deployment is blocked pending P2-WIN-02.
- P2-WIN-01 keeps product priority after P2-WIN-02. If M31 conflicts with Windows product work, pause M31 unless the user explicitly chooses public web visibility first.
- If M34 queryable table+prism lookup performance is active, M31 may proceed only with Cloudflare/devops/provenance/payload/UI-only work in a separate worktree. Do not run M31 Task 2 engine OpenCC breadth in parallel with M34; either choose `opencc_scope = current_simplification_only` for the first public demo or queue broader engine OpenCC work after M34 lands.
- TypeDuck-Web shell and dictionary use is owner-approved by the project owner, who is also the TypeDuck author. No separate M31 licensing clearance gate is required.
- Public deployment must use the **`yune-web`** identity. M31 resolved this by moving the repo-owned harness from `third_party/typeduck-web/` to `apps/yune-web/`; no public app or Cloudflare deployment is named TypeDuck-Web.
- Cloudflare deployment docs are checked at execution time. As of the M31 plan date, Cloudflare recommends Workers Static Assets for new static/full-stack Worker apps, React + Vite examples use `wrangler.jsonc`, and Wrangler JSON config is recommended for new projects.

## Acceptance Gates

- `M31-PUBLIC-00`: Public identity and provenance are recorded before deployment. The public app is named **`yune-web`**, not TypeDuck-Web; the milestone notes that TypeDuck-Web shell/dictionary use is owner-approved by the TypeDuck author; and the public page identifies that it is exercising the Yune engine through a TypeDuck-Web-derived harness.
- `M31-PUBLIC-01`: P2-WIN-02 status and P2-WIN-01 priority are checked before deployment; public deploy is blocked or explicitly risk-noted if P2-WIN-02 is still active, and M31 pauses if it would pull effort away from the Windows product track.
- `M31-PUBLIC-02`: Every exposed OpenCC output standard has engine/runtime/browser evidence. Unsupported standards are absent from the UI or shown as unavailable with a reason.
- `M31-PUBLIC-03`: The output-standard control changes candidate/commit output through Yune, not a browser-only postprocessor.
- `M31-PUBLIC-04`: The public demo can be rebuilt from checked-in Yune state: lock file, patch, Yune bridge, WASM, schema assets, and build commands.
- `M31-PUBLIC-05`: Cloudflare local preview and deployed smoke cover app boot, WASM load, schema asset load, `jyut6ping3_mobile` typing, OpenCC standard toggle, and route/base-path behavior.
- `M31-PUBLIC-06`: AI stays default-off/local-only; no remote calls, telemetry, secrets, or third-party model keys are present in the public build. AI-off output is byte-identical to the classic path.
- `M31-PUBLIC-07`: Public payload is measured, pruned, and documented. The deployed bundle ships only the schemas, compiled assets, dictionaries, fonts, and static files needed by the public surface; source dictionaries and unused schemas are excluded unless the runtime requires them.
- `M31-PUBLIC-08`: Public runtime asset loading is active-schema-only. The default public boot fetches only the app/runtime assets plus the selected schema's boot dependencies, not all schemas, all source dictionaries, reverse-lookup-only dependencies, or every compiled side artifact.
- `M31-PUBLIC-09`: Schema/runtime assets have content-addressed metadata and warm-cache evidence. Repeat visits skip unchanged schema payload downloads through IndexedDB, Cache Storage, Cloudflare/browser immutable caching, or an equivalent measured cache. Reverse-lookup dependency assets have separate cold-first-use and warm-reuse evidence.
- `M31-PUBLIC-10`: Public delivery has PWA/service-worker or Cloudflare cache evidence for the runtime shell and WASM/schema assets. Cold and warm startup are measured separately, and no browser-startup claim is made without real browser evidence.
- `M31-PUBLIC-11`: WASM/download-size optimization is measured as a delivery win only. Rust engine latency claims stay in M34 or later engine milestones.
- `M31-PUBLIC-12`: Rust, runtime, TypeDuck-Web build, Playwright, patch reverse/forward, and `git diff --check` gates pass.
- `M31-PUBLIC-13`: Harness rename is complete before deployment. Public-facing strings, deployment config, docs, evidence folder labels, route copy, and the repo-owned app path use the `yune-web` identity; the old TypeDuck-Web name remains only in provenance/history.

## File Responsibilities

- `crates/yune-core/src/filter/mod.rs`: owns OpenCC-backed conversion behavior and filter wiring.
- `crates/yune-rime-api/src/schema_install.rs`: owns schema option/config installation into browser/runtime deploy state.
- `crates/yune-rime-api/tests/typeduck_web.rs`: owns native runtime contract tests for browser-facing options.
- `packages/yune-typeduck-runtime/`: owns TypeScript wrapper and response/control contract.
- `apps/yune-web/`: canonical repo-owned yune-web harness path after M31 migration.
- `apps/yune-web/yune-integration/`: owns the Yune bridge layered over the TypeDuck-Web-derived shell.
- `apps/yune-web/source/`: owns the patched demo app UI; changes here require patch regeneration.
- `apps/yune-web/patches/yune-web-runtime.patch`: committed representation of source changes.
- `apps/yune-web/e2e/`: owns browser evidence and public-demo smoke tests.
- `apps/yune-web/public-demo/`: owns Cloudflare config, static public package, and deployment notes.
- `apps/yune-web/public-demo/PROVENANCE.md`: records owner-approved TypeDuck-Web shell/dictionary use and the Yune engine demo positioning.
- `apps/yune-web/public-demo/asset-manifest.md`: owns public payload inventory and pruning decisions.
- `apps/yune-web/public-demo/schema-asset-manifest.json`: owns content-addressed schema/runtime asset metadata for active-schema-only loading.
- `apps/yune-web/source/src/worker.ts`: owns worker-side schema asset fetching and must not eagerly load unrelated public-demo schemas.
- `apps/yune-web/source/src/yune-integration/adapter.ts`: owns the runtime bridge and deploy-cache interaction with preloaded assets.
- `docs/roadmap.md` and `docs/requirements.md`: own milestone status and requirement traceability.

---

## Task 0 - Product Priority, Provenance, And Payload Checkpoint

**Files:**

- Read: `docs/roadmap.md`
- Read: `docs/plans/p2-win02-plan-typeduck-boundary-compat.md`
- Read: `docs/plans/archive/m30-plan-engine-representation-performance.md`
- Create: `apps/yune-web/public-demo/PROVENANCE.md`
- Create: `apps/yune-web/public-demo/asset-manifest.md`
- Create evidence: `apps/yune-web/e2e/results/m31-yune-web-public-demo/public-readiness-gate.md`
- Create evidence: `apps/yune-web/e2e/results/m31-yune-web-public-demo/yune-web-rename.md` or migrated equivalent

- [x] **Step 0.1: Confirm repository and dependency state**

Run:

```powershell
git fetch origin --prune
git status --short --branch
git log --oneline -5 --decorate
```

Expected:

- Worktree is clean or unrelated active changes are identified before editing.
- `origin/main` contains the completed M30 closeout.

- [x] **Step 0.2: Check Windows product priority and P2-WIN-02 deployment gate**

Run:

```powershell
rg -n "P2-WIN-01|P2-WIN-02|TypeDuck Windows boundary compatibility|M31|M32|primary product" docs\roadmap.md docs\plans\p2-win01-plan-typeduck-windows-next.md docs\plans\p2-win02-plan-typeduck-boundary-compat.md
```

Expected:

- If P2-WIN-02 is complete, M31 can proceed through deployment.
- If P2-WIN-02 is active, M31 may complete local engine/UI/build work, but Task 6 deployed smoke must remain blocked and the closeout must say why.
- If P2-WIN-01 is ready and actively staffed, pause M31 unless the user explicitly chooses public web visibility first.

- [x] **Step 0.3: Record owner-approved TypeDuck provenance**

Run:

```powershell
git status --short --branch
rg -n "TypeDuck|Yune engine|public demo|owner-approved" docs\roadmap.md docs\plans\archive\m31-plan-yune-web-public-demo-readiness.md
```

Expected:

- `PROVENANCE.md` states that TypeDuck-Web shell and TypeDuck dictionary assets are owner-approved for this Yune-hosted demo because the project owner is the TypeDuck author.
- The public UI can use a TypeDuck/TypeDuck-Web-derived shell, but the milestone record still labels the technical purpose clearly: test and demonstrate the Yune engine.
- No courtesy-contact or license-clearance task is required for TypeDuck-owned assets in this milestone.

- [x] **Step 0.4: Rename the harness identity before any deployment work**

Use **`yune-web`** as the public name.

Audit current naming:

```powershell
rg -n "TypeDuck-Web|typeduck-web|TypeDuck Web|dogfood|dogfooding" docs packages apps\yune-web -g "!apps/yune-web/source/node_modules/**"
```

Required result:

- Public UI title, document title, footer/source links, public-demo README, Cloudflare Pages/config names, evidence titles, and deployed smoke labels use `yune-web`.
- The public page may say it is "derived from TypeDuck-Web" only in provenance/history text.
- `TypeDuck-Web` remains valid only for the separate real product repo, the upstream-derived shell provenance, historical archived evidence, or a temporary internal path note.
- Deployment names must not be `typeduck-web`, `typeduck_web`, or `TypeDuck-Web`.

M31 completed the mechanical migration of this repo-owned harness from
`third_party\typeduck-web\` to `apps\yune-web\`:

```powershell
New-Item -ItemType Directory -Force -Path apps | Out-Null
git mv third_party\typeduck-web apps\yune-web
rg -n "third_party[/\\]typeduck-web|third_party\\\\typeduck-web|typeduck-web" docs AGENTS.md README.md packages crates apps -g "!apps/yune-web/source/node_modules/**"
```

Path references in docs, scripts, Playwright config, patch commands, package
commands, evidence paths, and build scripts were updated to the new
`apps\yune-web\` path.

Expected:

- `yune-web-rename.md` records `path_migration = complete` with the new path.
- Public deployment remains blocked unless public identity and app path both use `yune-web`.

- [x] **Step 0.5: Measure the asset payload before choosing the deployment surface**

Run:

```powershell
Get-ChildItem -Recurse apps\yune-web\source\public\schema -File | Measure-Object Length -Sum
Get-ChildItem -Recurse apps\yune-web\source\public -File | Sort-Object Length -Descending | Select-Object -First 20 FullName,Length
```

Expected:

- `asset-manifest.md` records total public asset bytes, largest files, and which schema/dictionary/font/assets are intended for deployment.
- The public bundle plan prefers the minimum viable surface: `jyut6ping3_mobile` plus required boot compiled assets, public WASM/JS/CSS, and only dictionaries/assets needed at runtime.
- The manifest records the current internal eager-load behavior as a baseline and states which assets must move to active-schema-only loading before public deployment.
- The manifest distinguishes boot assets from reverse-lookup dependencies. Reverse-lookup-only assets are preserved and lazy-loaded on first reverse lookup instead of being dropped or fetched at boot.
- Source dictionaries, unused schemas, and dev/test-only assets are excluded unless the runtime requires them.

## Task 1 - Scope OpenCC Output Standards From Real Demo Need

**Files:**

- Inspect: `crates/yune-core/src/filter/mod.rs`
- Inspect: `crates/yune-rime-api/src/schema_install.rs`
- Inspect: `crates/yune-core/tests/cantonese_parity.rs`
- Inspect: OpenCC data under the Yune source tree
- Create: `apps/yune-web/e2e/results/m31-yune-web-public-demo/opencc-support-audit.md`

- [x] **Step 1.1: Decide whether OpenCC breadth is required for the first public demo**

Record one of these decisions in `opencc-support-audit.md`:

- `opencc_scope = current_simplification_only`: the first public demo needs only the already-proven `simplification`/`hk2s` path. Do not add broad OpenCC matrix work in M31; proceed to packaging/deployment with the existing real selector.
- `opencc_scope = add_limited_supported_standards`: add only the output standards backed by checked-in Yune data and required for the public demo.
- `opencc_scope = split_follow_up`: broader OpenCC output standards are valuable but too large for M31; create a follow-up plan and do not block the public demo on them.

Expected:

- M31 does not implement broad OpenCC support just because a UI label would be nice.
- No unsupported standard is shown as an enabled live option.

- [x] **Step 1.2: Inventory available OpenCC data/config chains**

Run:

```powershell
rg -n "hk2s|opencc|simplification|traditional|tw|hk|s2t|t2s" crates docs packages apps\yune-web -g "!apps/yune-web/source/node_modules/**"
rg --files | rg "opencc|hk2s|s2t|t2s|tw|hk"
```

Expected:

- The audit lists which conversion chains are backed by checked-in data.
- The audit distinguishes proven support from desired UI labels.

- [x] **Step 1.3: Choose the M31 exposed standard set**

Record the supported standards in `opencc-support-audit.md` using this table:

```markdown
| Standard id | User label | Engine support | Data/config chain | M31 UI status | Evidence owner |
| --- | --- | --- | --- | --- | --- |
| keep_original | Original output | no conversion | none | enabled | browser smoke |
| simplified | Simplified Chinese | supported only if chain is present | exact checked-in chain | enabled or blocked | native + browser |
| hong_kong_traditional | Hong Kong Traditional | supported only if chain is present | exact checked-in chain | enabled or blocked | native + browser |
| taiwan_traditional | Taiwan Traditional | supported only if chain is present | exact checked-in chain | enabled or blocked | native + browser |
```

Expected:

- No standard is enabled unless Yune can apply it through engine/runtime code.
- Labels are browser-facing, but conversion stays engine-owned.
- If only the existing simplification path is used, the audit states that M31's OpenCC work is satisfied by preserving and documenting the existing real control rather than adding a larger selector.

## Task 2 - Add Engine And Runtime OpenCC Support

**Files:**

- Modify: `crates/yune-core/src/filter/mod.rs`
- Modify: `crates/yune-rime-api/src/schema_install.rs`
- Modify: `crates/yune-rime-api/tests/typeduck_web.rs`
- Modify only if needed: `crates/yune-core/tests/cantonese_parity.rs`
- Create evidence: `apps/yune-web/e2e/results/m31-yune-web-public-demo/opencc-native-gates.md`

Skip code changes in this task if Task 1 records `opencc_scope = current_simplification_only`. In that case, record the existing native/browser evidence owner and proceed to Task 3 without adding a broader output-standard selector.

- [x] **Step 2.1: Add failing native tests for each enabled standard**

Add focused tests whose names encode the exposed standards, for example:

```rust
#[test]
fn m31_typeduck_web_output_standard_simplified_changes_candidate_output() {
    // Drive the real TypeDuck-Web runtime path.
    // Assert the selected output standard changes candidate or commit text.
    // Assert disabling the standard returns to the original output.
}

#[test]
fn m31_typeduck_web_output_standard_unsupported_values_are_rejected() {
    // Drive the same runtime option/customize path.
    // Assert unknown standard ids fail or are ignored without mutating output.
}
```

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_web m31_typeduck_web_output_standard -- --nocapture
```

Expected before implementation:

- New tests fail because the standard selector is not yet wired.

- [x] **Step 2.2: Implement the smallest engine-owned selector**

Wire output-standard selection so the runtime controls Yune conversion state instead of transforming text in React. Preserve existing `simplification` behavior and add a normalized setting such as `output_standard` only if the existing option cannot express the supported standards clearly.

Required behavior:

- Unknown standard ids do not panic.
- Switching standards while composing refreshes candidates through the existing engine path.
- Candidate text, commit text, and dictionary comments remain valid UTF-8.
- Classic behavior is unchanged when the standard is `keep_original`.

- [x] **Step 2.3: Run focused OpenCC gates**

Run:

```powershell
cargo fmt --check
cargo test -p yune-rime-api --test typeduck_web m31_typeduck_web_output_standard -- --nocapture
cargo test -p yune-core --test cantonese_parity -- opencc --nocapture
```

Expected:

- Focused tests pass.
- If `cantonese_parity` has no matching `opencc` tests, record `no matching cantonese_parity opencc tests` in `opencc-native-gates.md` and rely on the new `typeduck_web` runtime tests.

## Task 3 - Expose Output Standards In TypeDuck-Web

**Files:**

- Modify: `packages/yune-typeduck-runtime/src/`
- Modify: `apps/yune-web/yune-integration/`
- Modify: `apps/yune-web/source/src/`
- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Regenerate: `apps/yune-web/patches/yune-web-runtime.patch`
- Create evidence: `apps/yune-web/e2e/results/m31-yune-web-public-demo/opencc-browser-evidence.md`

Only add a new output-standard selector if Task 1 chooses `opencc_scope = add_limited_supported_standards`. If Task 1 chooses `current_simplification_only`, this task preserves the existing real simplification control, adds public-demo documentation/evidence for it, and does not invent disabled placeholder standards.

- [x] **Step 3.1: Add runtime wrapper support**

Add a typed API for the output-standard setting in `packages/yune-typeduck-runtime` and the Yune integration bridge only if the existing `simplification` option cannot express the Task 1 standard set. Use string ids from `opencc-support-audit.md`; do not expose unimplemented ids.

Run:

```powershell
npm.cmd --prefix packages\yune-typeduck-runtime test
npm.cmd --prefix packages\yune-typeduck-runtime run build
```

Expected:

- Runtime tests/build pass.

- [x] **Step 3.2: Add a minimal UI control**

Add an output-standard control under display/browser-facing controls. Requirements:

- Label explains that it changes Chinese output standard.
- Enabled options are exactly the supported standard ids from Task 1.
- Unsupported standards are absent, or disabled with a short reason.
- The control does not trigger a full engine reload unless the runtime evidence proves deploy-time state is required.

- [x] **Step 3.3: Add browser evidence**

Add Playwright coverage that:

- loads the app from a fresh build
- types a stable input with visible candidates
- toggles each enabled output standard
- verifies candidate or commit output changes where expected
- verifies `keep_original` returns to baseline
- verifies no literal OpenCC control/debug marker appears in the UI

Run:

```powershell
npm.cmd --prefix apps\yune-web\source run build
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M31 OpenCC" --workers=1
```

Expected:

- Focused browser test passes and writes screenshots/state JSON under `apps/yune-web/e2e/results/m31-yune-web-public-demo/`.

## Task 4 - Make The Public Demo Build Reproducible

**Files:**

- Modify or create: `apps/yune-web/yune-web.lock.json`
- Modify: `apps/yune-web/patches/yune-web-runtime.patch`
- Create: `apps/yune-web/public-demo/README.md`
- Create or update: `apps/yune-web/public-demo/PROVENANCE.md`
- Create or update: `apps/yune-web/public-demo/asset-manifest.md`
- Create: `apps/yune-web/public-demo/build.ps1`
- Create evidence: `apps/yune-web/e2e/results/m31-yune-web-public-demo/reproducible-build.md`

- [x] **Step 4.1: Capture the source reconstruction contract**

`public-demo/README.md` must state:

- which TypeDuck-Web upstream revision the source checkout is based on
- which patch file is applied
- which Yune integration files are copied or referenced
- where release WASM and schema assets come from
- how `/web/` versus root deployment is configured
- how the public page describes the relationship between TypeDuck-Web and the Yune engine
- where the TypeDuck owner-approved provenance note lives

- [x] **Step 4.2: Add a reproducible build script**

Create `public-demo/build.ps1` that performs these actions:

```powershell
param(
  [string] $OutputDir = "apps\yune-web\public-demo\dist"
)

$ErrorActionPreference = "Stop"
npm.cmd --prefix packages\yune-typeduck-runtime run build
npm.cmd --prefix apps\yune-web\source run build
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
Copy-Item -Recurse -Force apps\yune-web\source\dist\* $OutputDir
```

After writing the initial script, run `rg --files apps\yune-web\source\dist | rg "wasm|schema|js|css"` and add explicit `Copy-Item` lines for any release WASM or schema assets that are not already present in `source\dist`. Prune the deployed asset set before preview:

- Ship only the public schemas exposed by the UI, starting with `jyut6ping3_mobile` unless Task 1 deliberately exposes more.
- Prefer compiled runtime assets required by the browser. Do not ship source `*.dict.yaml` files unless the runtime requires them.
- Do not eagerly fetch `cangjie`, `luna_pinyin`, `loengfan`, test fixtures, or dev-only source dictionaries at boot unless the public UI exposes them as selected schemas. If any of those assets are reverse-lookup dependencies of the active public schema, keep them in the reverse-lookup dependency tier and lazy-load them on first use.
- Keep `PROVENANCE.md` and `asset-manifest.md` in the public-demo source folder; include them in the deployed static assets if the public page links to them.

Do not copy build output into git unless the project already tracks that exact artifact type.

- [x] **Step 4.3: Record public asset manifest**

Run after the public-demo build:

```powershell
Get-ChildItem -Recurse apps\yune-web\public-demo\dist -File | Measure-Object Length -Sum
Get-ChildItem -Recurse apps\yune-web\public-demo\dist -File | Sort-Object Length -Descending | Select-Object -First 30 FullName,Length
```

Expected:

- `asset-manifest.md` records total deployed bytes, largest files, and why each schema/dictionary asset is present.
- The manifest explicitly states which potentially large source dictionaries were excluded.
- The evidence notes the current Cloudflare file-size and asset limits checked in Task 6.1 before claiming deployment feasibility.

- [x] **Step 4.4: Verify patch discipline**

Run:

```powershell
git -C apps\yune-web\source diff --binary --submodule=diff > apps\yune-web\patches\yune-web-runtime.patch
git -C apps\yune-web\source apply --reverse --check ..\patches\yune-web-runtime.patch
```

Then forward-check the patch in a clean detached checkout at the revision named by `yune-web.lock.json`.

Expected:

- Reverse and forward checks pass.
- `reproducible-build.md` records the exact commands and source revision.

## Task 5 - Add Active-Schema Delivery And Warm Cache

**Files:**

- Modify: `apps/yune-web/source/src/worker.ts`
- Modify: `apps/yune-web/source/src/yune-integration/adapter.ts`
- Modify or create: `apps/yune-web/public-demo/schema-asset-manifest.json`
- Modify or create: `apps/yune-web/public-demo/cache-policy.md`
- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Create evidence: `apps/yune-web/e2e/results/m31-yune-web-public-demo/delivery-cache-evidence.md`

This task is the `my_rime` delivery lesson adapted to Yune. It is not an engine rewrite and must not change candidate behavior. The worker may still use Yune's own runtime bridge and deploy-cache model; the required behavior is that public boot fetches only the selected schema's boot assets, reverse-lookup dependencies are loaded on first reverse lookup, and unchanged payloads are reused on warm visits.

- [x] **Step 5.1: Build a selected-schema asset manifest**

Create or generate a manifest that maps each public schema id to exactly the files it needs at runtime:

```json
{
  "schemas": {
    "jyut6ping3_mobile": {
      "schema": ["jyut6ping3.schema.yaml"],
      "sourceDictionaries": [],
      "compiled": ["jyut6ping3.table.bin", "jyut6ping3.reverse.bin", "jyut6ping3_mobile.prism.bin"],
      "opencc": ["opencc/hk2s.json", "opencc/HKVariantsRev.ocd2", "opencc/HKVariantsRevPhrases.ocd2", "opencc/TSCharacters.ocd2", "opencc/TSPhrases.ocd2"],
      "reverseLookupDependencies": {
        "luna_pinyin": {
          "load": "onFirstReverseLookup",
          "schema": ["luna_pinyin.schema.yaml"],
          "sourceDictionaries": ["luna_pinyin_yune_reverse.dict.yaml"],
          "compiled": ["luna_pinyin.table.bin", "luna_pinyin.reverse.bin", "luna_pinyin.prism.bin"]
        }
      }
    }
  }
}
```

Adjust the exact paths after auditing what Yune actually requires. The audit must classify candidate reverse-lookup dependencies such as `luna_pinyin`, `cangjie5`, `stroke`, and `luna_pinyin_yune_reverse.dict.yaml` as one of: boot-required, lazy reverse-lookup dependency, exposed selectable schema, or unused. If a source `*.dict.yaml` is still required despite compiled assets being present, record the reason in `asset-manifest.md` and treat it as a follow-up optimization candidate.

- [x] **Step 5.2: Replace eager all-schema public loading**

Refactor the public path so startup does not fetch unrelated schemas such as `loengfan`, `jyut6ping3_scolar`, or their source dictionaries unless the public UI exposes and selects them. For `cangjie5`, `luna_pinyin`, `stroke`, and related reverse-lookup assets, first determine whether they are reverse-lookup dependencies of the active schema; if so, lazy-fetch them on first reverse lookup instead of boot-fetching or dropping them.

Expected:

- Default public boot fetches only app shell, runtime/WASM, shared defaults, selected schema dependencies, and required OpenCC files.
- Schema switch, if exposed, fetches the target schema's dependency set on demand.
- Reverse lookup fetches its dependency tier on first use, then reuses it through the same content-addressed warm cache.
- Internal dogfood-only diagnostics may keep broader fixtures behind an explicit non-public mode, but public deployment must use the pruned path.

- [x] **Step 5.3: Add content-addressed warm cache**

Add per-asset hash/version metadata and cache lookup before network fetch. Acceptable storage mechanisms include IndexedDB, Cache Storage, or a narrow existing Emscripten IDBFS marker if it can prove unchanged payload reuse.

Expected:

- Warm reload reuses unchanged schema payloads without network refetch.
- Cache invalidation is keyed by content hash or generated version, not only by a hand-edited `?v=` string.
- Deploy/config markers remain compatible with Yune's existing runtime deploy-cache checks.

- [x] **Step 5.4: Add delivery evidence**

Run real-browser evidence that records network requests and startup markers for:

- cold first load
- warm reload
- first reverse lookup that requires deferred dependencies, for example the public Jyutping reverse-lookup trigger
- warm reverse lookup after dependency caching
- schema switch, if the public UI exposes more than one schema

Expected:

- `delivery-cache-evidence.md` lists fetched URLs, transferred bytes, startup markers, cache hit/miss counts, and largest remaining payloads.
- The evidence proves reverse-lookup dependencies are absent from the default boot network set, fetched on first reverse lookup, and reused on warm reverse lookup.
- The evidence explicitly separates browser delivery wins from engine typing-latency wins.
- If a service worker/PWA is added here instead of Task 6, the evidence records its cache version and update behavior.

## Task 6 - Add Cloudflare Deployment

**Files:**

- Create: `apps/yune-web/public-demo/wrangler.jsonc`
- Create or modify: `apps/yune-web/public-demo/worker/index.ts`
- Create evidence: `apps/yune-web/e2e/results/m31-yune-web-public-demo/cloudflare-smoke.md`
- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`

- [x] **Step 6.1: Verify Cloudflare docs before writing config**

Open current Cloudflare docs and confirm:

- Workers Static Assets or Pages is the intended deployment target.
- `wrangler.jsonc` is still recommended for new Wrangler config.
- SPA fallback and static asset directory syntax are current.

Record the documentation URLs and access date in `cloudflare-smoke.md`.
Also record the current maximum static-asset/file-size limits and compare them with `asset-manifest.md`.

- [x] **Step 6.2: Add local preview config**

Create `wrangler.jsonc` with:

```jsonc
{
  "$schema": "../../source/node_modules/wrangler/config-schema.json",
  "name": "yune-engine-web-demo",
  "compatibility_date": "2026-06-22",
  "assets": {
    "directory": "./dist",
    "not_found_handling": "single-page-application"
  }
}
```

Adjust paths only after verifying the actual public-demo folder layout. Keep the config committed; do not store account ids, tokens, or secrets.

- [x] **Step 6.3: Run local Cloudflare preview**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File apps\yune-web\public-demo\build.ps1
npx.cmd wrangler dev --config apps\yune-web\public-demo\wrangler.jsonc --local
```

Expected:

- Preview serves the app.
- WASM loads with the correct MIME behavior.
- Schema assets load.
- `/web/` and root routing follow the decision recorded in `public-demo/README.md`.

- [x] **Step 6.4: Deploy only after P2-WIN-02, provenance, payload, and active-schema delivery gates are satisfied**

If P2-WIN-02 is complete or explicitly waived, TypeDuck owner-approved provenance is recorded, the asset manifest is below the verified Cloudflare limits, and Task 5 proves active-schema delivery plus warm-cache behavior, run:

```powershell
npx.cmd wrangler pages deploy apps\yune-web\public-demo\dist --project-name yune-web --branch main
```

Expected:

- Deployed URL is recorded in `cloudflare-smoke.md`.
- Deployed URL is also written as plain text to `apps\yune-web\e2e\results\m31-yune-web-public-demo\deployed-url.txt`.
- No Cloudflare token, account id, or secret is committed.
- If any gate is not satisfied, do not deploy. Record the blocker in `cloudflare-smoke.md` and keep M31 open or partial.

- [x] **Step 6.5: Run deployed smoke**

Run the focused Playwright smoke against the deployed URL:

```powershell
$DeployedUrl = Get-Content apps\yune-web\e2e\results\m31-yune-web-public-demo\deployed-url.txt -Raw
$env:YUNE_WEB_APP_URL = ($DeployedUrl.TrimEnd("/") + "/web/")
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M31 public demo" --workers=1
```

Expected:

- App boots.
- Typing `ngohaig` or the current canonical Jyutping smoke input produces candidates.
- Output-standard selector works.
- AI remains off by default.
- Network capture shows no remote AI/model/telemetry calls.
- The public page or footer links to the provenance note, or the evidence records why the in-repo provenance file is sufficient.

## Task 7 - AI Posture And Privacy Gate

**Files:**

- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Create evidence: `apps/yune-web/e2e/results/m31-yune-web-public-demo/ai-posture.md`

- [x] **Step 7.1: Add AI-off identity smoke**

Add a browser test that captures candidate state with AI disabled and compares it to the same input after toggling unrelated display controls. The AI option must remain disabled by default on fresh load. For the first public demo, the AI control should be hidden or developer-only unless a separate M32 decision explicitly exposes it.

- [x] **Step 7.2: Add network posture check**

In Playwright, collect requests during startup and a short typing session. Assert no requests go to:

- AI/model provider domains
- telemetry or analytics endpoints
- secret-bearing URLs

The app may fetch its own WASM, JS, CSS, schema, and font/static assets.

- [x] **Step 7.3: Record posture result**

`ai-posture.md` must state:

- default AI setting
- whether local AI is hidden, developer-only, or manually enable-able in this demo
- whether remote calls exist
- whether any secrets or telemetry are configured
- whether AI-off output stayed byte-identical through the tested path

## Task 8 - Closeout

**Files:**

- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Archived plan: `docs/plans/archive/m31-plan-yune-web-public-demo-readiness.md`

- [x] **Step 8.1: Run full gates**

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
npm.cmd --prefix packages\yune-typeduck-runtime test
npm.cmd --prefix packages\yune-typeduck-runtime run build
npm.cmd --prefix apps\yune-web\source run build
npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M31" --workers=1
git diff --check
```

Expected:

- All gates pass.
- If P2-WIN-02 blocked public deployment, local preview evidence may close only the engine/UI/build portion; the roadmap must keep deployment active.

- [x] **Step 8.2: Update docs and archive**

Update:

- `docs/requirements.md` with `M31-PUBLIC-REQ-*` rows matching the acceptance gates, including active-schema delivery and warm-cache gates.
- `docs/roadmap.md` with either complete status and deployed URL, or partial/blocker status if public deployment is blocked.
- `apps/yune-web/public-demo/PROVENANCE.md` and `asset-manifest.md` with the final public-readiness state.
- Archive this plan only when all acceptance gates close or the remaining deployment blocker is recorded as a new active plan.

- [x] **Step 8.3: Commit scoped changes**

Stage only M31 files:

```powershell
git status --short
git add -- docs\roadmap.md docs\requirements.md docs\decisions.md docs\plans\archive\m31-plan-yune-web-public-demo-readiness.md apps\yune-web packages crates
git commit -m "Prepare TypeDuck-Web public demo"
git push origin main
```

Only include `apps\yune-web`, `packages`, and `crates` paths that actually changed in M31.

## Review Questions

- Which OpenCC standards are truly backed by checked-in Yune data today, and which should remain absent until data/config support lands?
- Is OpenCC breadth truly required for the first public demo, or should M31 keep only the existing proven simplification path and split broader output standards later?
- Should the public URL serve from root or `/web/`, and how should redirects behave?
- Is Workers Static Assets still the preferred Cloudflare target at execution time, or should the worker choose Pages based on current docs and project needs?
- Can public deployment proceed before P2-WIN-02, or should the raw TypeDuck comment-boundary risk block sharing even though the web UI sanitizes display output?
- What exact public copy should explain that the demo exercises Yune through a TypeDuck-Web-derived harness?
- Which schema/dictionary assets are actually needed by the first public bundle, and which should be excluded for payload size?
- Can the public worker avoid loading source dictionaries entirely when compiled `.table.bin`/`.reverse.bin`/`.prism.bin` assets are present, or does Yune still require source YAML for a specific public feature?
- Which cache mechanism should own per-asset schema payload reuse: IndexedDB, Cache Storage/service worker, or the existing Emscripten IDBFS/deploy-cache marker?
