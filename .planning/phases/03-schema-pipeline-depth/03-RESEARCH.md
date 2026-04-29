# Phase 3: Schema Pipeline Depth - Research

**Researched:** 2026-04-29
**Domain:** Rust RIME/librime schema-pipeline compatibility: processors, segmentors, translators, filters, and remaining gears
**Confidence:** HIGH for codebase/librime-source mapping; MEDIUM for crate currency because Cargo registry CLI is unavailable in this environment

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
[VERIFIED: /Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md]

#### Processor And Segmentor Depth
- **D-01:** Deeper `speller`, `editor`, `navigator`, `selector`, `chord_composer`, shape, punctuation, and fallback behavior should be proven through ABI-facing tests that drive schema-loaded sessions, not only isolated core tests.
- **D-02:** Processor and segmentor work should prioritize larger-chain interactions where existing focused paths can pass while librime differs: previous-match segment splitting, non-auto-commit composition, segment/selection span changes, navigator fallback, raw/fallback segment exclusion, punctuation segment ordering, shape formatting on commits, and chord raw-sequence lifecycle cleanup.
- **D-03:** Add focused failing comparisons or fixtures before changing dispatch order or state mutation in key-processing paths. Each new behavior slice should name its owning implementation module, owning test module, and librime comparison target before code changes.
- **D-04:** Existing processor modules under `crates/yune-rime-api/src/processors/`, `schema_install.rs`, and schema-selection/session state should remain the ownership anchors. Do not move behavior back into `lib.rs` except for unavoidable ABI export glue.

#### Remaining Gear Policy
- **D-05:** `memory`, `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` must not stay invisible. Each needs either a Phase 3 compatibility increment or a structured deferral that states the observed librime role, why it is out of this phase's implementation slice, and which future phase owns it.
- **D-06:** Prefer small compatibility increments when they can be modeled without compiled dictionary payloads or userdb learning. Examples include schema installation recognition, no-op/diagnostic behavior that preserves chain determinism, or focused candidate weighting/annotation behavior when it can be compared directly against librime.
- **D-07:** Defer behavior that depends on compiled reverse data, UniTE compiled payloads, LevelDB-backed learning, or plugin ecosystems rather than adding incomplete shims that imply compatibility.

#### Distribution Schema Comparison
- **D-08:** Larger real-world schema-chain work should compare Yune directly against librime and then convert differences into focused fixtures or documented findings. Avoid broad snapshot churn that is hard to diagnose.
- **D-09:** Distribution-scale comparisons should emphasize chain semantics before performance: component order, segment tags, generated spellings, OpenCC/filter behavior, punctuation/fallback behavior, and candidate differences that users would see.
- **D-10:** When a comparison reveals a gap outside Phase 3, record it as a structured finding with observed Yune behavior, expected librime behavior when known, scope decision, and target phase. Compiled dictionary payload gaps should point to Phase 4; userdb learning/storage gaps should point to Phase 5.

#### Spelling, OpenCC, Correction, And Tolerance Boundaries
- **D-11:** Broaden spelling algebra only where the current focused `xlit`/`xform`/`erase`/`derive` and generated-spelling penalty coverage is insufficient for schema-loaded lookup compatibility.
- **D-12:** Correction and tolerance-search work should focus on schema-visible lookup/ranking interactions that can be represented without Phase 4 compiled payload consumption. Correction data or tolerance inputs that require compiled prism/table/reverse payloads should be documented for Phase 4.
- **D-13:** OpenCC work should distinguish between filter-chain integration semantics that can be tested now and full OpenCC conversion-data parity, which remains a larger compatibility/data concern. Do not claim full OpenCC compatibility from small built-in maps.

### Claude's Discretion
[VERIFIED: /Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md]

- Exact fixture names, loop counts, selected distribution schemas, comparison script shape, and whether findings live in test notes or docs are left to the planner/executor, provided the decisions above remain true.
- The planner may split Phase 3's four roadmap plans by behavior ownership if that keeps implementation/test modules focused and avoids mixing mechanical test movement with semantic changes.

### Deferred Ideas (OUT OF SCOPE)
[VERIFIED: /Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md]

- Compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payload consumption, rebuild execution, pack checksum chaining, and compiled correction/tolerance data belong to Phase 4.
- LevelDB/userdb storage, learning, frequency updates, predictive lookup, recovery, sync, and transaction behavior belong to Phase 5.
- Full librime C++ plugin ABI compatibility, Lua/octagram/predict/proto plugin ecosystems, and AI-native provider/ranking/context/memory behavior remain outside the current compatibility milestone.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SCHEMA-01 | `speller` behavior covers previous-match segment splitting and non-auto-commit composition behavior beyond current focused auto-commit paths. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Use `crates/yune-rime-api/src/processors/speller.rs` as owner; compare against librime `src/rime/gear/speller.cc` `AutoSelectPreviousMatch` and `FindEarlierMatch`. [VERIFIED: codebase + librime source] |
| SCHEMA-02 | `editor`, `navigator`, and `selector` behavior covers deeper segment/selection span semantics and navigator fallback interactions beyond current focused overrides. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Use `editor.rs`, `navigator.rs`, `selector.rs`, and ABI tests in `schema_processors.rs`; compare against librime `editor.cc`, `navigator.cc`, and `selector.cc`. [VERIFIED: codebase + librime source] |
| SCHEMA-03 | `chord_composer`, `shape_processor`/`shape_formatter`, `punct_segmentor`, and `fallback_segmentor` behavior covers larger-chain and remaining lifecycle edge cases. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Use `chord_composer.rs`, `shape.rs`, `punctuation.rs`, and `schema_install.rs::update_session_segment_tags`; compare against librime `chord_composer.cc`, `punctuator.cc`, and `fallback_segmentor.cc`. [VERIFIED: codebase + librime source] |
| SCHEMA-04 | Remaining librime gear behavior around `memory`, `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` has explicit compatibility increments or documented deferrals. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Add schema-chain recognition/structured findings in `schema_install.rs` rather than invisible ignores; defer LevelDB/userdb, grammar model, reverse compiled data, and UniTE payload behavior where necessary. [VERIFIED: codebase + librime source] |
| SCHEMA-05 | Full spelling algebra, correction/tolerance search interaction, OpenCC conversion data, and distribution-scale processor/segmentor/translator/filter chains are compared directly against librime behavior. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md] | Compare larger schema chains before broadening; extend `yune-core/src/spelling_algebra.rs`, `translator/mod.rs`, and `filter/mod.rs` only for source-dictionary/schema-visible behavior that does not require Phase 4 compiled payloads. [VERIFIED: codebase] |
</phase_requirements>

## Summary

Phase 3 is not a greenfield feature phase; it is a compatibility-depth phase over an existing Rust workspace whose current schema pipeline is installed through `RimeSelectSchema`/`apply_schema_to_session`, then exercised through ABI-visible `RimeProcessKey` and context/status/commit APIs. [VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md] The planner should structure work as focused compatibility slices that start with ABI-facing fixtures or direct librime comparisons, then change owned modules only where the observed user-visible behavior differs. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md]

The highest-risk work is the mismatch between librime's composition/segmentation model and Yune's current flatter core context model. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-core/src/engine.rs and /Users/trenton/Projects/librime/src/rime/gear/navigator.cc] Librime navigation and speller splitting reason over composition segments, phrase spans, selected-candidate spans, and recursive previous-match confirmation; Yune currently stores a single composition input/preedit/caret plus a flat `segment_tags` vector. [VERIFIED: codebase + librime source] Therefore Phase 3 should prefer narrow parity increments backed by tests and structured deferrals for behavior that would require Phase 4 compiled data or Phase 5 userdb. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md]

The remaining-gear policy should be explicit: `memory`, `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` should no longer be silently ignored in component installation. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md] In Phase 3, the safe increment is component recognition plus deterministic no-op/diagnostic or documented finding where behavior depends on LevelDB user dictionaries, grammar models, reverse lookup dictionaries, or compiled data. [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/memory.cc and /Users/trenton/Projects/librime/src/rime/gear/unity_table_encoder.cc]

**Primary recommendation:** Plan Phase 3 as four ownership-based waves: (1) ABI-facing processor/segmentor parity tests and fixes, (2) explicit remaining-gear recognition/deferrals, (3) librime-vs-Yune distribution-chain comparison workflow, and (4) targeted spelling algebra/OpenCC/correction increments that avoid compiled-data and userdb behavior. [VERIFIED: ROADMAP + CONTEXT + codebase]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Schema selection and component installation | RIME Compatibility Layer / ABI Backend | Core Engine Layer | `RimeSelectSchema` calls `apply_schema_to_session`, which resets session state and installs processors, segmentors, translators, and filters. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_selection.rs] |
| Processor key handling (`speller`, `editor`, `navigator`, `selector`, `chord_composer`, punctuation, shape) | RIME Compatibility Layer / ABI Backend | Core Engine Layer | Processor modules under `crates/yune-rime-api/src/processors/` own schema-loaded key behavior before falling through to `Engine`. [VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md] |
| Candidate generation, spelling algebra, correction/tolerance lookup approximations | Core Engine Layer | RIME Compatibility Layer / ABI Backend | `StaticTableTranslator` and `SpellingAlgebra` live in `yune-core`; schema installation feeds them deployed YAML settings. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-core/src/translator/mod.rs and /Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs] |
| OpenCC/simplifier filter behavior | Core Engine Layer | RIME Compatibility Layer / ABI Backend | `SimplifierFilter` is a core filter configured by `schema_install.rs` from schema `engine/filters`. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-core/src/filter/mod.rs and /Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs] |
| Remaining librime gear recognition/deferral (`memory`, `poet`/`grammar`, `contextual_translation`, `unity_table_encoder`) | RIME Compatibility Layer / ABI Backend | Core Engine Layer / Database-Storage future phases | Component prescriptions are parsed in `schema_install.rs`; substantive behavior often depends on future compiled dictionary or userdb storage tiers. [VERIFIED: codebase + librime source] |
| Distribution schema comparison | Test/Tooling Layer | RIME Compatibility Layer / ABI Backend | Comparisons should drive deployed schema sessions through ABI-visible paths and convert differences into focused fixtures/findings. [VERIFIED: /Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md] |
| UserDB learning and LevelDB persistence | Database / Storage | RIME Compatibility Layer / ABI Backend | Current Yune userdb is plain local files; LevelDB-backed learning is deferred to Phase 5. [VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/INTEGRATIONS.md and CONTEXT.md] |
| Compiled dictionary/prism/reverse payload consumption | Database / Storage | Core Engine Layer | Current runtime dictionary loading reads source `.dict.yaml`; compiled payload consumption is Phase 4. [VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/CONCERNS.md and CONTEXT.md] |

## Project Constraints (from CLAUDE.md)

No `CLAUDE.md` file exists at `/Users/trenton/Projects/yune/CLAUDE.md`; no project-specific CLAUDE directives were available to apply. [VERIFIED: filesystem check]

Project skill directories `.claude/skills/` and `.agents/skills/` do not exist in `/Users/trenton/Projects/yune`; no project skill `SKILL.md` files were available to apply. [VERIFIED: filesystem check]

## Standard Stack

### Core

| Library / Component | Version | Purpose | Why Standard |
|---------------------|---------|---------|--------------|
| Rust workspace | MSRV 1.76, edition 2021 [VERIFIED: /Users/trenton/Projects/yune/Cargo.toml] | Compile and test `yune-core`, `yune-schema`, `yune-rime-api`, and `yune-cli`. | Existing project standard; all Phase 3 code belongs in these crates. [VERIFIED: Cargo manifests] |
| `yune-rime-api` | 0.1.0 [VERIFIED: /Users/trenton/Projects/yune/Cargo.lock] | Librime-shaped C ABI, schema installation, session state, and schema-loaded processor routing. | Owns ABI-visible compatibility contract and processor modules. [VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md] |
| `yune-core` | 0.1.0 [VERIFIED: /Users/trenton/Projects/yune/Cargo.lock] | Core `Engine`, translators, filters, state, spelling algebra, dictionaries, and candidate ranking hooks. | Keeps reusable deterministic engine behavior outside ABI glue. [VERIFIED: ARCHITECTURE.md] |
| `regex` crate | 1.12.3 locked [VERIFIED: /Users/trenton/Projects/yune/Cargo.lock] | Recognizer patterns and spelling-algebra transforms. | Existing dependency used by `yune-core` and `yune-rime-api`; Rust regex avoids backtracking blowups but differs from librime/Boost regex in some constructs. [VERIFIED: codebase + CONCERNS.md] |
| `serde_yaml` crate | 0.9.34+deprecated locked [VERIFIED: /Users/trenton/Projects/yune/Cargo.lock] | YAML config/schema parsing and deployed config access. | Existing parser for RIME YAML subset; compatibility shims must preserve librime/yaml-cpp behavior where tests define it. [VERIFIED: Cargo.lock + CONCERNS.md] |
| `libc` crate | 0.2.186 locked [VERIFIED: /Users/trenton/Projects/yune/Cargo.lock] | Unix ABI/platform helpers. | Existing dependency in `yune-rime-api`; keep unsafe/platform work at ABI boundary. [VERIFIED: Cargo.lock + ARCHITECTURE.md] |
| Local librime source | `/Users/trenton/Projects/librime` present [VERIFIED: filesystem check] | External oracle for gear behavior. | Project constraint says librime is the compatibility oracle. [VERIFIED: /Users/trenton/Projects/yune/.planning/PROJECT.md] |

### Supporting

| Library / Component | Version | Purpose | When to Use |
|---------------------|---------|---------|-------------|
| `libloading` crate | 0.8.9 locked [VERIFIED: /Users/trenton/Projects/yune/Cargo.lock] | Dynamic loader integration tests for the cdylib. | Use for frontend/native ABI validation tests, not for ordinary schema processor unit tests. [VERIFIED: Cargo.lock + Phase 2 context] |
| Cargo test harness | unavailable in current shell; required by project [VERIFIED: environment audit + TESTING.md] | Run focused and workspace Rust tests. | Planner must include commands, but executor cannot run them in this environment until Rust toolchain is available. [VERIFIED: environment audit] |
| `rg` / ripgrep | 14.1.1 [VERIFIED: environment audit] | Search codebase and fixture references. | Useful for locating component ownership and existing tests. [VERIFIED: environment audit] |
| Git | 2.50.1 Apple Git [VERIFIED: environment audit] | Version-control inspection/commits. | Available for docs commit workflow if requested by GSD. [VERIFIED: environment audit] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Extending existing processor modules | New generic librime gear framework | Do not build a generic framework in Phase 3; existing ownership anchors are locked decisions and lower risk. [VERIFIED: CONTEXT.md] |
| `serde_yaml` compatibility shims | Swap YAML parser | Parser replacement is outside Phase 3; schema pipeline work should add focused compatibility tests around current helpers. [VERIFIED: CONCERNS.md] |
| Built-in `SimplifierFilter` maps | Full OpenCC data/runtime | Full conversion-data parity is explicitly larger than Phase 3 unless a focused filter-chain integration slice can be tested now. [VERIFIED: CONTEXT.md] |
| Source `.dict.yaml` loading | Compiled `.table.bin`/`.prism.bin`/`.reverse.bin` payloads | Compiled payload consumption belongs to Phase 4; Phase 3 may document gaps but should not add incomplete shims. [VERIFIED: CONTEXT.md] |
| Plain local file userdb shim | LevelDB-compatible user dictionary | LevelDB/userdb learning/storage belongs to Phase 5; Phase 3 `memory` behavior should be deferred or limited to safe recognition. [VERIFIED: CONTEXT.md + INTEGRATIONS.md] |

**Installation:**
```bash
# No new dependencies recommended for Phase 3.
# Existing workspace manifests already declare regex, serde_yaml, libc, and libloading.
```

**Version verification:** Cargo registry version lookup could not be completed because `cargo` is unavailable in the current environment and `cargo search` produced no output. [VERIFIED: environment audit] The table uses `Cargo.lock` locked versions as the authoritative project-current versions. [VERIFIED: /Users/trenton/Projects/yune/Cargo.lock]

## Architecture Patterns

### System Architecture Diagram

```text
Deployed RIME YAML + dict YAML
        |
        v
RimeSelectSchema / apply_schema_to_session
        |
        +--> schema_install.rs installs segment tags, translators, filters, segmentors
        |        |
        |        +--> yune-core translators/filters/spelling algebra
        |        +--> session segmentor state: punct, affix, matcher, fallback flags
        |
        +--> processors installed into SessionState
                 |
Frontend/ABI key event -> RimeProcessKey
                 |
                 v
   ascii/key-binder/selector/navigator pre-dispatch
                 |
                 v
   process_session_key_event
                 |
                 +--> chord -> key_binder -> recognizer -> punctuation
                 +--> alternative select -> speller -> editor
                 +--> yune-core Engine fallback
                 |
                 v
   update_session_segment_tags + shape formatting + commit buffering
                 |
                 v
RimeGetContext / RimeGetStatus / RimeGetCommit user-visible output
                 |
                 v
Focused comparison: Yune ABI output vs librime oracle output
```
[VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md and codebase reads]

### Recommended Project Structure

```text
crates/
├── yune-rime-api/
│   ├── src/processors/        # Own processor key semantics: speller/editor/navigator/selector/chord/shape/punctuation
│   ├── src/schema_install.rs  # Own component recognition, translator/filter/segmentor install, remaining-gear deferrals
│   ├── src/schema_selection.rs# Own schema reset/install ordering
│   └── src/tests/            # ABI-facing schema processor/selection tests and focused comparison fixtures
├── yune-core/
│   └── src/                  # Own engine, translator, filter, spelling algebra, dictionary behavior
└── yune-cli/                 # Optional frontend-surrogate comparison path if planner chooses it
```
[VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md]

### Pattern 1: ABI-Facing Schema Fixture Before Behavior Change

**What:** Create a deployed schema fixture in a unique temp runtime, initialize/select it via RIME ABI calls, drive `RimeProcessKey`, then assert context/status/commit/candidate output. [VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/TESTING.md]

**When to use:** Every Phase 3 processor/segmentor compatibility slice, especially before changing `RimeProcessKey`, `process_session_key_event`, `update_session_segment_tags`, or commit buffering. [VERIFIED: CONTEXT.md + CONCERNS.md]

**Example:**
```rust
// Source: /Users/trenton/Projects/yune/crates/yune-rime-api/src/tests/mod.rs and TESTING.md
let _guard = test_guard();
let shared_dir = unique_temp_dir("schema-depth");
// write default.yaml, schema YAML, dict YAML into shared/user/staging dirs
// call RimeSetup/RimeInitialize/RimeDeployWorkspace/RimeCreateSession/RimeSelectSchema
// call RimeProcessKey for the target key sequence
// assert RimeGetContext/RimeGetStatus/RimeGetCommit values
```
[VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/tests/mod.rs]

### Pattern 2: Behavior Slice Ownership Header

**What:** Before implementation, name the owning implementation module, owning test module, and librime comparison target. [VERIFIED: /Users/trenton/Projects/yune/.planning/REQUIREMENTS.md]

**When to use:** Every SCHEMA-01 through SCHEMA-05 increment. [VERIFIED: REQUIREMENTS.md]

**Example:**
```text
Slice: speller non-auto previous-match splitting
Implementation owner: crates/yune-rime-api/src/processors/speller.rs
Test owner: crates/yune-rime-api/src/tests/schema_processors.rs
Oracle: /Users/trenton/Projects/librime/src/rime/gear/speller.cc::AutoSelectPreviousMatch + FindEarlierMatch
```
[VERIFIED: codebase + librime source]

### Pattern 3: Explicit Deferral Record for Remaining Gears

**What:** For unsupported components, record observed librime role, current Yune behavior, why Phase 3 cannot safely implement full behavior, and future owner phase. [VERIFIED: CONTEXT.md]

**When to use:** `memory`, `poet`/`grammar`, `contextual_translation`, and `unity_table_encoder` when implementation depends on userdb learning, grammar models, reverse dictionaries, or compiled payloads. [VERIFIED: librime source]

**Example:**
```text
Gear: memory
Librime role: listens to commit/delete/unhandled-key notifications and updates user dictionary transactions.
Current Phase 3 action: recognize component in schema-chain comparison and document no-op/deferral.
Deferred because: Yune current userdb is plain local files, and LevelDB/userdb learning belongs to Phase 5.
Future owner: Phase 5 USERDB-01..USERDB-03.
```
[VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/memory.cc and /Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md]

### Anti-Patterns to Avoid

- **Adding owned behavior to `crates/yune-rime-api/src/lib.rs`:** `lib.rs` is already fragile dispatch/export glue; put behavior in processor modules or schema installers. [VERIFIED: ARCHITECTURE.md + CONCERNS.md]
- **Bypassing schema installation:** Do not mutate translators, filters, segment tags, or processor state from unrelated API paths; extend `apply_schema_to_session` installers. [VERIFIED: ARCHITECTURE.md]
- **Broad snapshot churn:** Distribution-scale differences must become focused fixtures/findings; broad snapshots are hard to diagnose. [VERIFIED: CONTEXT.md]
- **Claiming full OpenCC parity from built-in maps:** `SimplifierFilter` currently maps a small built-in character set; full OpenCC data parity remains larger than Phase 3. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-core/src/filter/mod.rs + CONTEXT.md]
- **Implementing fake userdb/compiled-data shims:** Do not imply compatibility for LevelDB learning or compiled dictionary payload behavior before Phase 4/5 foundations. [VERIFIED: CONTEXT.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Schema component lookup | New ad hoc path parser | Existing `schema_component_prescription`, `find_config_value`, and scalar helpers | Existing helpers encode current librime-style path/scalar behavior. [VERIFIED: schema_install.rs + ARCHITECTURE.md] |
| ABI/frontend behavior tests | Direct `Engine` setup pretending to be frontend behavior | RIME ABI path with `RimeProcessKey`, `RimeGetContext`, `RimeGetStatus`, `RimeGetCommit` | Phase 3 behavior must be visible through schema-loaded sessions. [VERIFIED: CONTEXT.md + TESTING.md] |
| Regex engine replacement | Custom parser or new regex engine | Existing `regex` crate with focused compatibility findings | Rust regex is already used; unsupported Boost/librime regex differences should become findings unless a narrow fix is required. [VERIFIED: CONCERNS.md] |
| OpenCC conversion database | Handwritten large conversion tables | Existing `SimplifierFilter` for focused tests; document full OpenCC data gap | Full OpenCC parity is a data concern, not a small Phase 3 map. [VERIFIED: filter/mod.rs + CONTEXT.md] |
| User dictionary learning | New Phase 3 storage shim | Structured deferral to Phase 5 | Librime `memory` depends on user dictionary transactions and learning semantics. [VERIFIED: memory.cc + CONTEXT.md] |
| Compiled dictionary/prism/reverse behavior | Fake compiled payload support | Structured deferral to Phase 4 | Phase 4 owns `.table.bin`, `.prism.bin`, `.reverse.bin` payload consumption. [VERIFIED: CONTEXT.md] |
| Generic gear framework | Abstract framework for all librime gears | Focused recognition/increments in `schema_install.rs` and existing modules | Locked decisions require ownership anchors and small measurable slices. [VERIFIED: CONTEXT.md] |

**Key insight:** In this domain, custom abstractions are less risky than librime internals only when they preserve the external contract; hand-rolled compatibility shims that skip librime comparison create false confidence. [VERIFIED: /Users/trenton/Projects/yune/.planning/PROJECT.md]

## Common Pitfalls

### Pitfall 1: Treating Yune's Flat Composition Model as Equivalent to Librime Segmentation

**What goes wrong:** Planner assumes caret/span behavior can be fixed by delimiter-based movement only. [VERIFIED: Yune navigator.rs + librime navigator.cc]

**Why it happens:** Yune's core context has a single input/preedit/caret plus flat tags, while librime's navigator builds spans from selected phrase spans and segment start/end boundaries. [VERIFIED: /Users/trenton/Projects/librime/src/rime/gear/navigator.cc]

**How to avoid:** Start SCHEMA-02 with small librime comparison cases: selected phrase span, half-selected candidate, caret not at end, linear selector fallback. If full multi-segment modeling is required, document the exact gap rather than overfitting delimiter movement. [VERIFIED: CONTEXT.md]

**Warning signs:** Tests assert only caret numeric movement for simple delimiter input; no candidate/segment selection state is asserted. [VERIFIED: TESTING.md patterns]

### Pitfall 2: Changing Dispatch Order Without Guard Fixtures

**What goes wrong:** Fixing one processor changes whether selector, navigator, chord, punctuation, speller, editor, or core engine consumes a key. [VERIFIED: CONCERNS.md]

**Why it happens:** `RimeProcessKey` and `process_session_key_event` contain ordered early-return logic for schema processors. [VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md]

**How to avoid:** Add focused failing ABI tests before touching dispatch order or state mutation; keep implementation in processor modules and only touch `lib.rs` for unavoidable routing glue. [VERIFIED: CONTEXT.md]

**Warning signs:** A change edits `lib.rs` and multiple processor modules with no new test showing previous behavior. [VERIFIED: CONCERNS.md]

### Pitfall 3: Silent Ignore of Remaining Gears

**What goes wrong:** Distribution schema comparison reports unexplained candidate differences because `memory`, `poet`, `contextual_translation`, or `unity_table_encoder` are silently ignored. [VERIFIED: CONTEXT.md]

**Why it happens:** `schema_install.rs` currently ignores unknown translator/filter/component names in match defaults. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs]

**How to avoid:** Add explicit recognition or structured findings for each gear, with future owner phase where full behavior is out of scope. [VERIFIED: CONTEXT.md]

**Warning signs:** No test or documentation mentions the four named remaining gears after Phase 3. [VERIFIED: REQUIREMENTS.md]

### Pitfall 4: OpenCC Overclaiming

**What goes wrong:** A test passes for a small traditional/simplified mapping and the project claims full OpenCC compatibility. [VERIFIED: filter/mod.rs + CONTEXT.md]

**Why it happens:** Current `SimplifierFilter` maps selected characters and config names, but does not consume OpenCC conversion data files. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-core/src/filter/mod.rs]

**How to avoid:** Phrase OpenCC increments as filter-chain integration or focused conversion cases only; document full OpenCC data parity as a larger compatibility/data concern. [VERIFIED: CONTEXT.md]

**Warning signs:** Research/plans say "implement OpenCC" without naming exact `opencc_config`, candidate input, expected output, and source of conversion data. [VERIFIED: CONTEXT.md]

### Pitfall 5: Environment Assumes Cargo Exists

**What goes wrong:** Plan requires `cargo test` execution, but current shell has no `cargo` or `rustc`. [VERIFIED: environment audit]

**Why it happens:** The repo is Rust, but the target environment lacks Rust toolchain binaries in PATH. [VERIFIED: environment audit]

**How to avoid:** Include a Wave 0 environment gate: install/activate Rust toolchain or mark implementation verification blocked until Cargo is available. [VERIFIED: environment audit]

**Warning signs:** `command -v cargo` returns no output. [VERIFIED: environment audit]

## Code Examples

Verified patterns from repository and librime source:

### Librime Speller Previous-Match Oracle

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/gear/speller.cc
if (AutoSelectPreviousMatch(ctx, &previous_segment)) {
  if (!is_initial && ctx->composition().GetCurrentSegmentLength() == 1) {
    ctx->PopInput(1);
    return kNoop;
  }
}
```
[VERIFIED: librime source]

Planning implication: SCHEMA-01 fixtures must cover both accepted splitting and the case where a non-initial leftover key is returned for other processors. [VERIFIED: librime source]

### Librime Recursive Earlier-Match Splitting

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/gear/speller.cc
if (ctx->get_option("_auto_commit")) {
  ctx->Commit();
  string rest = input.substr(end);
  ctx->set_input(rest);
  end = 0;
} else {
  ctx->ConfirmCurrentSelection();
  ctx->set_input(input);
}
if (!ctx->HasMenu()) {
  size_t next_start = ctx->composition().GetCurrentStartPosition();
  size_t next_end = ctx->composition().GetCurrentEndPosition();
  if (next_start == end) {
    FindEarlierMatch(ctx, next_start, next_end);
  }
}
```
[VERIFIED: librime source]

Planning implication: Yune's current `speller_auto_select_previous_match` only commits previous match when `_auto_commit` is true and current conversion fails; non-auto confirmation and recursive earlier-match splitting need focused comparison before implementation. [VERIFIED: /Users/trenton/Projects/yune/crates/yune-rime-api/src/processors/speller.rs]

### Yune Schema Install Chain Owner

```rust
// Source: /Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_selection.rs
install_schema_segment_tags(session, schema_id);
install_schema_editor_processor(session, schema_id);
install_schema_chord_composer_processor(session, schema_id);
install_schema_ascii_composer_processor(session, schema_id);
install_schema_speller_processor(session, schema_id);
install_schema_recognizer_processor(session, schema_id);
install_schema_selector_bindings(session, schema_id);
install_schema_navigator_bindings(session, schema_id);
install_schema_key_binder_processor(session, schema_id);
install_schema_punctuation_processor(session, schema_id);
install_schema_translator_chain(session, schema_id);
install_schema_filter_chain(session, schema_id);
```
[VERIFIED: codebase]

Planning implication: New component recognition and processor behavior should be installed through this flow, not ad hoc session mutation. [VERIFIED: ARCHITECTURE.md]

### Librime Selector/Navigator Fallback Oracle

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/gear/selector.cc
if (is_linear_layout(ctx) && !caret_at_end_of_input(ctx)) {
  // let navigator handle the arrow key.
  return false;
}
```
[VERIFIED: librime source]

Planning implication: SCHEMA-02 should assert when selector declines and navigator consumes the same key in linear layout with caret inside input. [VERIFIED: librime source]

### Remaining Gear Deferral Evidence

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/gear/memory.cc
commit_connection_ =
    ctx->commit_notifier().connect([this](Context* ctx) { OnCommit(ctx); });
delete_connection_ = ctx->delete_notifier().connect(
    [this](Context* ctx) { OnDeleteEntry(ctx); });
unhandled_key_connection_ = ctx->unhandled_key_notifier().connect(
    [this](Context* ctx, const KeyEvent& key) { OnUnhandledKey(ctx, key); });
```
[VERIFIED: librime source]

Planning implication: Full `memory` support is not just a translator/filter; it listens to commit/delete/unhandled-key events and updates user dictionary transactions, so Phase 3 should not fake it. [VERIFIED: librime source + CONTEXT.md]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Direct core fixture setup for compatibility behavior | ABI-facing schema-loaded tests through `yune-rime-api` | Established by Phase 1/2 planning and Phase 3 context [VERIFIED: STATE.md + CONTEXT.md] | Phase 3 tests should use deployed schema sessions, not only `Engine` shortcuts. |
| `lib.rs` accumulating behavior | Owned modules under processors/schema/core with `lib.rs` as facade/glue | Current codebase map, 2026-04-28 [VERIFIED: ARCHITECTURE.md] | Planner should assign tasks by module ownership and avoid facade growth. |
| Silent unknown component ignore | Explicit compatibility increment or deferral for named remaining gears | Required in Phase 3 context, 2026-04-29 [VERIFIED: CONTEXT.md] | `memory`, `poet`/`grammar`, `contextual_translation`, `unity_table_encoder` need visible outcomes. |
| Focused schema subset only | Larger distribution schema-chain comparisons before broad changes | Required in Phase 3 context, 2026-04-29 [VERIFIED: CONTEXT.md] | Differences should become focused fixtures/findings rather than broad snapshots. |
| Built-in OpenCC-like maps as enough | Distinguish filter integration from full conversion-data parity | Required in Phase 3 context, 2026-04-29 [VERIFIED: CONTEXT.md] | Avoid overclaiming OpenCC; document data gaps. |

**Deprecated/outdated:**
- `serde_yaml` is locked as `0.9.34+deprecated` in `Cargo.lock`; do not expand dependency reliance without acknowledging parser compatibility risk. [VERIFIED: /Users/trenton/Projects/yune/Cargo.lock]
- `.planning/codebase/INTEGRATIONS.md` still says `yune-rime-api` does not declare dynamic-library `crate-type`, but current `crates/yune-rime-api/Cargo.toml` declares `crate-type = ["rlib", "cdylib"]`; treat the codebase map line as stale. [VERIFIED: Cargo.toml + INTEGRATIONS.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Cargo registry latest versions are not necessary to change for Phase 3 because no new dependencies are recommended. [ASSUMED] | Standard Stack | If a security or compatibility issue requires dependency updates, planner may need an explicit dependency-upgrade task. |
| A2 | Distribution comparisons can be implemented with local librime source/build or existing user-provided librime tooling. [ASSUMED] | Environment Availability / Open Questions | If no runnable librime binary/API is available, comparison tasks need a Wave 0 setup step. |

## Open Questions

1. **How should distribution comparison results be stored?**
   - What we know: Context allows findings in test notes or docs at planner discretion. [VERIFIED: CONTEXT.md]
   - What's unclear: Exact path/format for comparison findings is not locked.
   - Recommendation: Use a small checked-in notes file or focused test comments only when tied to a fixture; avoid broad generated snapshots. [ASSUMED]

2. **Is a runnable librime comparison harness available, or only source?**
   - What we know: `/Users/trenton/Projects/librime` source exists. [VERIFIED: environment audit]
   - What's unclear: Whether librime is built and callable from scripts in this environment.
   - Recommendation: Planner should include an environment gate to verify/build the librime comparison target before SCHEMA-05 comparisons. [ASSUMED]

3. **How far should span modeling go in Phase 3?**
   - What we know: Librime uses segment and phrase spans for navigator/editor behavior; Yune's core model is flatter. [VERIFIED: codebase + librime source]
   - What's unclear: Whether targeted approximations can cover selected Phase 3 fixtures without a larger composition model change.
   - Recommendation: Start with focused ABI tests for user-visible behavior and defer full segment model redesign unless tests force it. [VERIFIED: CONTEXT.md]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust `cargo` | Build/test all Phase 3 changes | ✗ | — | Install/activate Rust toolchain before implementation verification. [VERIFIED: environment audit] |
| `rustc` | Compile workspace | ✗ | — | Install/activate Rust toolchain. [VERIFIED: environment audit] |
| Git | Docs commit / source inspection | ✓ | 2.50.1 Apple Git | — [VERIFIED: environment audit] |
| ripgrep (`rg`) | Code/test discovery | ✓ | 14.1.1 | POSIX grep, but rg is available. [VERIFIED: environment audit] |
| Local librime source | Behavior oracle source references | ✓ | source tree at `/Users/trenton/Projects/librime` | If not buildable, use source-level oracle plus documented manual comparison setup. [VERIFIED: environment audit] |
| Built librime executable/library | Direct SCHEMA-05 runtime comparison | ? | — | Planner should add Wave 0 probe/build step. [ASSUMED] |

**Missing dependencies with no fallback:**
- `cargo` and `rustc` are blocking for implementation verification in this environment. [VERIFIED: environment audit]

**Missing dependencies with fallback:**
- Built/runnable librime comparison target is unverified; source-level research can proceed, but runtime comparisons need a setup/probe step. [ASSUMED]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No authentication layer exists; all APIs are local CLI/FFI/library calls. [VERIFIED: /Users/trenton/Projects/yune/.planning/codebase/INTEGRATIONS.md] |
| V3 Session Management | yes | RIME session IDs are process-local and guarded by `SessionRegistry`; validate session IDs and avoid leaking global state across tests. [VERIFIED: ARCHITECTURE.md] |
| V4 Access Control | limited | No user authorization, but runtime resource IDs must remain logical IDs under configured roots. [VERIFIED: PROJECT.md + CONCERNS.md] |
| V5 Input Validation | yes | Use resource ID validators, config scalar helpers, and bounded/checked regex compilation for schema-provided inputs. [VERIFIED: schema_install.rs + CONCERNS.md] |
| V6 Cryptography | no | No cryptographic operations in Phase 3 scope. [VERIFIED: INTEGRATIONS.md] |

### Known Threat Patterns for Rust RIME Schema Pipeline

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Schema or API resource ID path traversal | Tampering / Information Disclosure | Keep `validate_data_resource_id` and logical-ID checks before filesystem joins; do not reintroduce raw path joins. [VERIFIED: PROJECT.md + schema_install.rs] |
| Untrusted schema regex resource exhaustion | Denial of Service | Rust `regex` avoids catastrophic backtracking, but planner should avoid unbounded pattern count/length expansion and document skipped invalid patterns. [VERIFIED: CONCERNS.md] |
| FFI pointer/lifetime misuse while exposing context/candidates | Tampering / Denial of Service | Keep C allocation/free ownership in ABI modules and tests; Phase 3 should not move unsafe pointer behavior into core. [VERIFIED: ARCHITECTURE.md + CONCERNS.md] |
| Process-wide state contamination across tests | Tampering / Reliability | Use `test_guard()`, unique temp runtime dirs, and explicit initialization/finalization patterns. [VERIFIED: TESTING.md] |

## Sources

### Primary (HIGH confidence)
- `/Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md` - locked Phase 3 decisions, discretion, deferred scope. [VERIFIED]
- `/Users/trenton/Projects/yune/.planning/REQUIREMENTS.md` - SCHEMA-01 through SCHEMA-05 and quality requirements. [VERIFIED]
- `/Users/trenton/Projects/yune/.planning/STATE.md` - Phase 2 complete, Phase 3 current focus, deferred schema/data/userdb boundaries. [VERIFIED]
- `/Users/trenton/Projects/yune/.planning/ROADMAP.md` - Phase 3 goal, success criteria, and four plan slices. [VERIFIED]
- `/Users/trenton/Projects/yune/.planning/PROJECT.md` - compatibility oracle, architecture, testing, security, data compatibility constraints. [VERIFIED]
- `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md` - layer responsibilities, key path, schema selection/install flow, anti-patterns. [VERIFIED]
- `/Users/trenton/Projects/yune/.planning/codebase/TESTING.md` - Rust test framework and ABI test patterns. [VERIFIED]
- `/Users/trenton/Projects/yune/.planning/codebase/INTEGRATIONS.md` - local filesystem/userdb/runtime integration boundaries. [VERIFIED, with noted stale crate-type line]
- `/Users/trenton/Projects/yune/.planning/codebase/CONCERNS.md` - fragile dispatch, security risks, scaling gaps, missing OpenCC/compiled/userdb features. [VERIFIED]
- `/Users/trenton/Projects/yune/Cargo.toml`, crate manifests, and `Cargo.lock` - workspace, crate types, dependencies, locked versions. [VERIFIED]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_selection.rs` - schema reset/install order. [VERIFIED]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs` - translator/filter/segmentor install, component prescription, segment tag updates. [VERIFIED]
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/processors/*.rs` - processor ownership and current behavior. [VERIFIED]
- `/Users/trenton/Projects/yune/crates/yune-core/src/spelling_algebra.rs`, `translator/mod.rs`, `filter/mod.rs`, `engine.rs` - core behavior owners. [VERIFIED]
- `/Users/trenton/Projects/librime/src/rime/gear/speller.cc`, `navigator.cc`, `selector.cc`, `fallback_segmentor.cc`, `memory.cc`, `contextual_translation.cc`, `unity_table_encoder.cc` - oracle source for Phase 3 semantics. [VERIFIED]

### Secondary (MEDIUM confidence)
- Existing docs `docs/analysis.md`, `docs/roadmap.md`, `docs/refactor-plan.md` were summarized from prior session context and align with planning docs. [VERIFIED in prior conversation summary]

### Tertiary (LOW confidence)
- No web-only findings were used. [VERIFIED]

## Metadata

**Confidence breakdown:**
- Standard stack: MEDIUM - locked versions verified from `Cargo.lock`, but Cargo/crates.io version lookup is unavailable in this environment. [VERIFIED: Cargo.lock + environment audit]
- Architecture: HIGH - verified from planning codebase maps and direct source reads. [VERIFIED]
- Pitfalls: HIGH - derived from project concerns, locked Phase 3 decisions, and direct librime/Yune source comparison. [VERIFIED]

**Research date:** 2026-04-29
**Valid until:** 2026-05-29 for architecture/source ownership; 2026-05-06 for dependency/tool availability because environment may change quickly.
