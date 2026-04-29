# Phase 04: compiled-dictionary-data - Research

**Researched:** 2026-04-29
**Domain:** RIME compiled dictionary payloads, Rust binary readers, runtime fallback, rebuild execution
**Confidence:** HIGH for codebase/librime-derived findings; MEDIUM for implementation decomposition where exact byte support must be proven by fixtures

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
## Implementation Decisions

### Payload Scope
- **D-01:** Phase 4 should implement minimum usable readers for librime `.table.bin`, `.prism.bin`, and `.reverse.bin` payloads, not stop at header/checksum metadata.
- **D-02:** Reader work should be behavior-driven against schema-loaded lookup fixtures and librime-derived payload observations. Unsupported binary sections should become structured findings with exact observed format/role and target follow-up, not silent no-ops that imply compatibility.
- **D-03:** Compiled payload parsing belongs in `crates/yune-core/src/dictionary/` or focused submodules owned by the dictionary layer. RIME ABI/schema code should choose resources and install dictionaries, not own binary parsing.

### Runtime Fallback Policy
- **D-04:** Runtime schema installation should prefer valid fresh compiled payloads when available, then fall back to source `.dict.yaml` parsing when compiled payloads are missing, stale, unsupported, or fail validation and source data is available.
- **D-05:** If neither a usable compiled payload nor source dictionary is available, the failure should be explicit and test-covered rather than silently installing an empty dictionary.
- **D-06:** Source and compiled paths must produce the same user-visible candidate ordering for focused fixtures before performance-oriented shortcuts are accepted.

### Rebuild Semantics
- **D-07:** Rebuild execution should be deterministic and local to Yune's Rust implementation, built around existing checksum/rebuild-plan primitives, runtime paths, staging/prebuilt directories, and deployed schema/dictionary resources.
- **D-08:** Do not shell out to librime compilers or depend on external generated artifacts during normal tests. Librime remains the oracle for comparison, not an implementation dependency.
- **D-09:** Freshness decisions should cover table, prism, reverse, source-vs-prebuilt fallback, pack checksum chaining, and forced rebuild flags where librime behavior is observable. Partial rebuild support should be explicit about which artifacts were rebuilt or reused.

### Advanced Compiled Data
- **D-10:** Stem-column data, reverse-db `dict_settings`, preset vocabulary phrase injection, and UniTE-style encoder payloads should be consumed where existing schemas rely on them and where they can be represented from compiled/source data without pulling Phase 5 userdb behavior forward.
- **D-11:** Correction data and tolerance-search inputs should move from Phase 3 schema-visible boundaries into the compiled-data path when the data is present in table/prism/reverse artifacts and can be compared through schema-loaded lookup tests.
- **D-12:** LevelDB/userdb learning, predictive frequency updates, plugin-backed translators, and AI-native ranking/memory remain out of Phase 4 even when compiled-data findings point toward them; record those findings for Phase 5 or future milestones.

### Security And Resource Boundaries
- **D-13:** Compiled-data readers must treat schema-provided dictionary IDs as logical resource IDs and preserve the resource-ID validation established in Phase 2 and Phase 3.
- **D-14:** Binary payload parsing must be bounded and fail closed on malformed lengths, offsets, counts, or unsupported versions. Tests should include malformed local fixture bytes without reading outside the payload or panicking.

### Claude's Discretion
- Exact parser module layout, fixture byte construction strategy, selected distribution dictionaries, and whether findings live in summary sections or focused comparison tests are left to planning/execution, provided ownership and phase boundaries above remain true.
- The planner may split Phase 4 plans by artifact type or by runtime flow if that keeps tests focused and avoids mixing binary parser work with schema-install/rebuild orchestration.

### Deferred Ideas (OUT OF SCOPE)
## Deferred Ideas

- LevelDB/userdb storage, learning, frequency updates, predictive lookup, and backdated scan behavior remain Phase 5.
- Full plugin ABI, Lua/octagram/predict/proto ecosystems, and AI-native input behavior remain future milestone scope.
- Full OpenCC conversion-data chain parity remains outside Phase 4 unless a compiled dictionary lookup fixture requires a narrow integration boundary.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DATA-01 | Runtime dictionary loading can consume compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` payloads beyond the current metadata slice. | Use `yune-core::dictionary` readers for bounded librime `MappedFile`/`OffsetPtr` structures, then have schema install select compiled readers before source fallback. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; `/Users/trenton/Projects/librime/src/rime/dict/table.h`; `/Users/trenton/Projects/librime/src/rime/dict/prism.h`; `/Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.h`] |
| DATA-02 | Dictionary rebuild execution handles source-vs-prebuilt fallback, table/prism/reverse checksum decisions, pack checksum chaining, and compiled output freshness. | Extend existing `RimeDictRebuildInput`/`RimeDictRebuildPlan` and deployment runtime path helpers to mirror librime `DictCompiler::Compile`, including prebuilt reuse when source is absent and pack checksum chaining from primary checksum. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; `/Users/trenton/Projects/yune/crates/yune-rime-api/src/deployment.rs`; `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`] |
| DATA-03 | Stem-column data, reverse-db `dict_settings`, preset-vocabulary phrase injection, and UniTE-style encoder payloads are consumed where librime schemas rely on them. | Source path already preserves stems, preset vocabulary, and encoder rules; compiled reverse reader must expose reverse lookup values, stem keys using `\x1fstem`, and YAML `dict_settings` for UniTE-compatible encoder metadata. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/source.rs`; `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/encoder.rs`; `/Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.cc`; `/Users/trenton/Projects/librime/src/rime/gear/unity_table_encoder.cc`] |
| DATA-04 | Correction data and tolerance search inputs are represented in the compiled-data path sufficiently for schema-loaded lookup compatibility. | Prism spelling-map descriptors include spelling type, correction bit, credibility, and tips; librime’s active corrector currently returns `NearSearchCorrector` and has edit-distance `.correction.bin` build/load disabled behind `#if 0`, so Phase 4 should model prism spelling-map correction/tolerance inputs first and record `.correction.bin` as a finding unless a fixture demands it. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/prism.h`; `/Users/trenton/Projects/librime/src/rime/dict/prism.cc`; `/Users/trenton/Projects/librime/src/rime/dict/corrector.cc`] |
</phase_requirements>

## Summary

Phase 4 should be planned as a data-compatibility phase centered on `yune-core/src/dictionary`, not as an ABI rewrite. The current code already has source `.dict.yaml` parsing, stems, preset vocabulary, table encoder primitives, and checksum/rebuild-plan metadata, but compiled `.table.bin`, `.prism.bin`, and `.reverse.bin` support stops at header/checksum parsing. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/source.rs`; `/Users/trenton/Projects/yune/docs/analysis.md`] Runtime schema installation currently reads `{dictionary}.dict.yaml` directly from selected runtime data and returns `None` silently if load fails; Phase 4 must replace that with an explicit resource-selection result that prefers fresh compiled artifacts, falls back to source YAML, and records an explicit failure when neither path is usable. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`]

The librime oracle shows that compiled data is not one flat table file. `.table.bin` is a mapped structure with metadata, syllabary array, multi-level index, entries, weights, and a MARISA string table; `.prism.bin` is a Darts double-array plus spelling-map descriptors carrying spelling type/correction/tips; `.reverse.bin` is metadata plus MARISA key/value tries, index array, optional dict-settings YAML, and special stem keys ending in `\x1fstem`. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/table.h`; `/Users/trenton/Projects/librime/src/rime/dict/table.cc`; `/Users/trenton/Projects/librime/src/rime/dict/prism.h`; `/Users/trenton/Projects/librime/src/rime/dict/prism.cc`; `/Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.h`; `/Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.cc`] Because Yune must not shell out to librime in normal tests, plans should build small local binary fixtures or Rust-generated compiled fixtures and compare user-visible ordering against source-backed fixtures. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`]

**Primary recommendation:** Plan four slices in this order: (1) bounded core readers plus schema-install compiled/source fallback, (2) deterministic Rust rebuild execution/freshness/pack checksum behavior, (3) reverse/stem/dict-settings/preset/UniTE-compatible compiled data exposure, and (4) prism correction/tolerance data integration with schema-loaded lookup tests. [VERIFIED: `/Users/trenton/Projects/yune/.planning/ROADMAP.md`; `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Bounded `.table.bin`/`.prism.bin`/`.reverse.bin` parsing | Core dictionary layer (`crates/yune-core/src/dictionary/`) | — | Phase decision D-03 assigns compiled payload parsing to dictionary modules; ABI/schema code only selects resources and installs dictionaries. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |
| Runtime compiled-vs-source dictionary selection | RIME compatibility/schema install layer (`crates/yune-rime-api/src/schema_install.rs`) | Core dictionary layer | Schema install already owns deployed schema translator/filter setup and dictionary resource lookup, while parsing remains in core. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`; `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md`] |
| Rebuild/freshness execution | RIME deployment/runtime layer (`crates/yune-rime-api/src/deployment.rs`, `runtime.rs`) | Core dictionary checksum/compiler primitives | Deployment owns maintenance/workspace update and runtime roots; core owns reusable checksum/rebuild data structures and binary/source dictionary build logic. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/deployment.rs`; `/Users/trenton/Projects/yune/crates/yune-rime-api/src/runtime.rs`; `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`] |
| Candidate ordering from compiled data | Core translator layer (`crates/yune-core/src/translator/mod.rs`) | Core dictionary layer | Translators own user-visible candidate ordering; compiled data must feed existing translator semantics rather than duplicating translation behavior in ABI modules. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`; `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md`] |
| Resource-ID validation for dictionary names | RIME compatibility/resource boundary (`resource_id.rs`, `schema_install.rs`, `deployment.rs`) | Core receives validated logical IDs or byte slices only | Phase 2/3 logical resource validation must be preserved before filesystem joins; core binary parsers should not accept schema strings as paths. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs`; `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`; `/Users/trenton/Projects/yune/crates/yune-rime-api/src/deployment.rs`] |

## Project Constraints (from CLAUDE.md)

No `CLAUDE.md` exists at `/Users/trenton/Projects/yune/CLAUDE.md` or in the agent worktree, so there are no additional CLAUDE.md directives to enforce. [VERIFIED: Read tool file-not-found result]

Project skill directories `.claude/skills/` and `.agents/skills/` were not present in the main project; `.planning/codebase/ARCHITECTURE.md` also records that project skills were not detected. [VERIFIED: filesystem listing; `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md`]

## Standard Stack

### Core

| Library / Facility | Version | Purpose | Why Standard |
|--------------------|---------|---------|--------------|
| Rust workspace / Cargo | Rust MSRV 1.76 in workspace; local `cargo` not available in this shell | Primary implementation and test harness | Project is a Rust workspace with `yune-core`, `yune-schema`, `yune-rime-api`, and `yune-cli`; planning must assume Rust code changes but include environment gap handling for local execution. [VERIFIED: `/Users/trenton/Projects/yune/Cargo.toml`; environment audit] |
| `regex` | 1.12.3 in `Cargo.lock` | Existing recognizer/spelling algebra/encoder regex support | Already used in `yune-core` and `yune-rime-api`; avoid introducing another regex engine for Phase 4 unless a librime fixture proves current semantics insufficient. [VERIFIED: `/Users/trenton/Projects/yune/Cargo.lock`; `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/encoder.rs`; `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`] |
| `serde_yaml` | 0.9.34+deprecated in `Cargo.lock` | Existing YAML config/dictionary metadata parsing | Already used for runtime config and deployment; use it for reverse `dict_settings` YAML ingestion only behind existing config helper semantics where possible. [VERIFIED: `/Users/trenton/Projects/yune/Cargo.lock`; `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`; `/Users/trenton/Projects/yune/crates/yune-rime-api/src/deployment.rs`] |
| `libc` | 0.2.186 in `Cargo.lock` | ABI crate dependency | Do not expand ABI unsafe scope for binary parsing; compiled readers should remain safe Rust in `yune-core`. [VERIFIED: `/Users/trenton/Projects/yune/Cargo.lock`; `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md`] |
| Local bounded byte readers | New in `yune-core::dictionary` | Parse mapped-file offsets/lists/arrays without unsafe memory-mapping | Librime’s compiled formats rely on C++ `MappedFile`, relative `OffsetPtr`, flexible arrays, and external trie images; Rust should parse from `&[u8]` with checked offsets/counts rather than transmuting. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/mapped_file.h`; `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |

### Supporting

| Library / Facility | Version | Purpose | When to Use |
|--------------------|---------|---------|-------------|
| `libloading` | 0.8.9 in `Cargo.lock` | Existing dynamic-loader tests | Not central to Phase 4 parser work; use only for ABI/frontend-style integration tests that already drive `yune-rime-api` through exported APIs. [VERIFIED: `/Users/trenton/Projects/yune/Cargo.lock`; `/Users/trenton/Projects/yune/crates/yune-rime-api/tests/dynamic_loader.rs`] |
| Handwritten binary fixtures | In-repo test bytes | Malformed length/offset/count/version tests and minimum valid payload observations | Required by D-14 and avoids shelling out to librime in normal tests. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |
| `/Users/trenton/Projects/librime` oracle source | Local checkout | Format/behavior oracle | Use for comparison and fixture interpretation, not as a normal test dependency. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`; `/Users/trenton/Projects/yune/.planning/PROJECT.md`] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Safe checked byte readers | `unsafe` struct casts into mapped bytes | Unsafe casts would imitate librime’s memory layout but conflict with D-14 fail-closed parsing and risk UB on malformed payloads; use checked little-endian readers and bounded relative-offset resolution. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`; `/Users/trenton/Projects/librime/src/rime/dict/mapped_file.h`] |
| Rust rebuild execution | Shelling out to librime `rime_dict_manager`/compiler | D-08 forbids shelling out during normal tests and normal implementation; librime remains oracle only. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |
| Runtime source-only dictionary loading | Compiled-first with source fallback | Source-only remains current behavior but fails DATA-01 and distribution-scale data compatibility. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`; `/Users/trenton/Projects/yune/.planning/REQUIREMENTS.md`] |
| Full MARISA/Darts dependency clone | Minimum behavior-driven payload readers | The current project has no marisa/darts Rust dependency and D-02 allows unsupported sections to become structured findings; plan minimum usable readers first, not full binary compatibility theatre. [VERIFIED: `/Users/trenton/Projects/yune/Cargo.toml`; `/Users/trenton/Projects/librime/src/rime/dict/string_table.h`; `/Users/trenton/Projects/librime/src/rime/dict/prism.h`; `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |

**Installation:**

No new package should be assumed for Phase 4 planning. [VERIFIED: current `Cargo.toml` dependency set] If a later implementation chooses a Rust MARISA or Darts decoder crate, it must be separately researched and version-verified before adding it. [ASSUMED]

**Version verification:** Dependency versions were verified from `Cargo.lock` because `cargo` is unavailable in this shell and this is a Rust/Cargo project rather than npm. [VERIFIED: environment audit; `/Users/trenton/Projects/yune/Cargo.lock`]

## Architecture Patterns

### System Architecture Diagram

```text
Schema selection / deployment maintenance
        |
        v
schema_install.rs reads deployed schema YAML
        |
        v
validate dictionary/prism/pack logical resource IDs
        |
        v
resolve runtime roots: staging first, then prebuilt/shared fallback
        |
        v
compiled artifact candidate set
(.table.bin + .prism.bin + .reverse.bin, plus packs)
        |
        v
freshness + bounded parse validation decision
        |------------------------------|
        | valid fresh compiled payloads | missing/stale/unsupported/malformed
        v                              v
yune-core compiled dictionary model     source .dict.yaml parser
        |                              |
        | candidate-order parity check |
candidate dictionary installed into StaticTableTranslator / reverse filters
        |
        v
Engine refresh -> existing translator/filter ordering -> ABI-visible candidates
```

### Recommended Project Structure

```text
crates/yune-core/src/dictionary/
├── compiled.rs          # public compiled metadata/rebuild facade plus parse result types
├── compiled_table.rs    # .table.bin bounded reader and table-entry materialization
├── compiled_prism.rs    # .prism.bin bounded reader, spelling-map/correction metadata
├── compiled_reverse.rs  # .reverse.bin bounded reader, stems, reverse values, dict_settings
├── source.rs            # existing source fallback and rebuild source-of-truth behavior
├── encoder.rs           # existing rule-based encoder primitives
└── mod.rs               # exports only

crates/yune-rime-api/src/
├── schema_install.rs    # select compiled/source resource path, install translators/filters
├── deployment.rs        # rebuild execution/freshness hooks during workspace update
├── runtime.rs           # runtime path roots only, no parser logic
└── tests/
    ├── schema_selection.rs or split dictionary_data.rs
    ├── distribution_schema_comparison.rs
    └── deployment.rs
```

This structure preserves D-03 by keeping binary parsing in `yune-core::dictionary` and keeps ABI/runtime modules responsible only for resource selection, deployment orchestration, and schema-loaded behavior. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`; `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md`]

### Pattern 1: Safe Relative-Offset Reader

**What:** Parse librime `MappedFile` payloads from `&[u8]` with helpers for little-endian primitives, relative `OffsetPtr` resolution, arrays, lists, strings, and NUL-terminated format fields. Do not transmute bytes into Rust structs. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/mapped_file.h`]

**When to use:** Every compiled `.bin` reader, especially sections using `OffsetPtr<T>`, `List<T>`, `Array<T>`, and `String`. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/table.h`; `/Users/trenton/Projects/librime/src/rime/dict/prism.h`; `/Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.h`]

**Example:**

```rust
// Source: /Users/trenton/Projects/librime/src/rime/dict/mapped_file.h
// OffsetPtr<T>::get() resolves from the address of its own offset field.
fn checked_offset_ptr(bytes: &[u8], field_offset: usize) -> Result<Option<usize>, ParseError> {
    let raw = read_i32_le(bytes, field_offset)?;
    if raw == 0 {
        return Ok(None);
    }
    let target = field_offset
        .checked_add_signed(raw as isize)
        .ok_or(ParseError::OutOfBounds)?;
    if target >= bytes.len() {
        return Err(ParseError::OutOfBounds);
    }
    Ok(Some(target))
}
```

### Pattern 2: Selection Result, Not Silent `Option`

**What:** Replace `load_schema_table_dictionary(...) -> Option<TableDictionary>` planning target with an internal result enum that distinguishes `Compiled`, `SourceFallback`, `NoUsablePath`, and `UnsupportedCompiledFindings`. [VERIFIED: current function returns `Option` at `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`]

**When to use:** Runtime dictionary installation, reverse lookup filter/translator installation, and deployment tests where missing/stale/malformed compiled data must be visible. [VERIFIED: D-04/D-05/D-02 in `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`]

**Example:**

```rust
// Source: Phase 04 D-04/D-05 and existing schema_install.rs load_schema_table_dictionary.
enum DictionaryLoadOutcome {
    Compiled(TableDictionary),
    SourceFallback { dictionary: TableDictionary, reason: CompiledRejectReason },
    NoUsablePath { dictionary_id: String, reason: DictionaryLoadFailure },
}
```

### Pattern 3: Behavior Parity Before Optimization

**What:** Compiled readers should materialize or adapt into the same `TableDictionary`/translator path as source dictionaries until focused fixtures prove equivalent ordering. [VERIFIED: D-06 in `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`; current `StaticTableTranslator::from_dictionary` path in `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`]

**When to use:** 04-01 and 04-03. Optimized prefix/trie lookup can be deferred until compiled-vs-source candidate ordering is locked. [VERIFIED: `/Users/trenton/Projects/yune/.planning/codebase/CONCERNS.md` documents linear dictionary lookup as a scaling limit, not the immediate compatibility gate]

### Pattern 4: Rebuild Decision Followed by Explicit Actions

**What:** Split rebuild planning from rebuild execution: first compute checksums/freshness, then run deterministic Rust builders for table, reverse, prism, and packs, reporting which artifacts were rebuilt or reused. [VERIFIED: existing plan primitive in `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; D-09 in context]

**When to use:** 04-02 deployment/workspace update. [VERIFIED: `/Users/trenton/Projects/yune/.planning/ROADMAP.md`]

**Example:**

```rust
// Source: /Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc
// librime rebuilds/reuses primary table, checks prism checksums, forces table rebuild when reverse checksum mismatches, then chains pack checksums.
struct DictRebuildExecution {
    dict_file_checksum: u32,
    table: ArtifactAction,
    reverse: ArtifactAction,
    prism: ArtifactAction,
    packs: Vec<PackArtifactAction>,
}
```

### Anti-Patterns to Avoid

- **Adding parser logic to `schema_install.rs`:** Violates D-03 and makes ABI/schema code own binary safety details; keep parsing in `yune-core::dictionary`. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`]
- **Accepting a compiled artifact because metadata parses:** Current metadata checks are insufficient for DATA-01; reader must validate and consume usable payload sections or return structured unsupported findings. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; D-01/D-02]
- **Silently installing no translator when dictionary load fails:** Current `Option` behavior is insufficient for D-05; tests must assert explicit failure/no usable path when no compiled/source data exists. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`; D-05]
- **Using filesystem paths from schema dictionary IDs:** Resource IDs must remain logical and pass `validate_data_resource_id` before joins. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs`; D-13]
- **Implementing Phase 5 userdb learning while touching UniTE:** UniTE encoder payload support can consume reverse `dict_settings` and stems, but LevelDB/userdb learning remains out of scope. [VERIFIED: D-10/D-12; `/Users/trenton/Projects/librime/src/rime/gear/unity_table_encoder.cc`]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML config path and scalar coercion | New slash-path YAML traversal helpers | Existing `find_config_value`, `config_scalar_*`, `schema_string_list` helpers | Existing helpers encode project compatibility behavior and are already used by schema install/deployment. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`; `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md`] |
| Resource path validation | Ad hoc `Path::join` with schema values | `validate_data_resource_id` before any compiled/source dictionary path resolution | D-13 requires logical resource IDs and current validator rejects traversal, separators, NULs, and drive prefixes. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs`] |
| Table phrase encoder rules | New formula parser | Existing `TableEncoder` in `dictionary/encoder.rs` | Source path already matches librime formula/tail-anchor behavior for focused cases. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/encoder.rs`; `/Users/trenton/Projects/librime/src/rime/algo/encoder.cc`] |
| Rebuild checksum primitive | New CRC implementation | Existing `RimeChecksumComputer`, `rime_dict_source_checksum`, `rime_dict_rebuild_plan` | Current code already implements librime-compatible checksum and initial rebuild-plan slice. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`] |
| ABI/front-end state access | Direct core shortcuts in schema-loaded tests | Existing ABI test helpers and schema-loaded selection path | Compatibility behavior must be visible through deployed schema/session paths. [VERIFIED: `/Users/trenton/Projects/yune/.planning/codebase/TESTING.md`; `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |
| MARISA/Darts full clone | Full trie codec from scratch without fixtures | Minimum behavior-driven readers and structured findings for unsupported sections | `.table.bin` and `.reverse.bin` string payloads are MARISA trie images and `.prism.bin` contains Darts double-array; full codec work is broad and should only be planned if a focused fixture requires it. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/string_table.h`; `/Users/trenton/Projects/librime/src/rime/dict/prism.h`; D-02] |

**Key insight:** The hardest part is not header offsets; it is preserving candidate behavior while safely crossing from librime’s memory-mapped relative-pointer file layout to Rust’s checked byte slices. Plan for explicit parse results and parity tests before optimizing lookup. [VERIFIED: librime mapped-file/table/prism/reverse sources; D-02/D-06/D-14]

## Common Pitfalls

### Pitfall 1: Relative OffsetPtr Base Is the Field Address

**What goes wrong:** Treating librime offsets as file-absolute offsets instead of offsets relative to the offset field address. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/mapped_file.h`]
**Why it happens:** `OffsetPtr<T>::get()` computes `(char*)&offset_ + offset_`, not `file_base + offset_`. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/mapped_file.h`]
**How to avoid:** Implement one shared checked offset resolver and test with nonzero offset fields at multiple struct positions. [ASSUMED]
**Warning signs:** Metadata fields parse but syllabary/index/trie sections fail or point inside the metadata header. [ASSUMED]

### Pitfall 2: Flexible Array Size Arithmetic Overflows

**What goes wrong:** Reading `Array<T>` or `List<T>` count and multiplying by element size can overflow or pass end-of-payload. [VERIFIED: `Array`/`List` definitions in `/Users/trenton/Projects/librime/src/rime/dict/mapped_file.h`; D-14]
**Why it happens:** Librime reads memory-mapped C++ structs; malformed local bytes can set arbitrary counts. [VERIFIED: D-14]
**How to avoid:** Use `checked_mul`, `checked_add`, maximum fixture-informed bounds, and return typed parse errors. [ASSUMED]
**Warning signs:** Tests panic on tiny/malformed byte arrays or allocator attempts huge vectors. [ASSUMED]

### Pitfall 3: Metadata Freshness Is Not Payload Usability

**What goes wrong:** A table/prism/reverse header has matching checksum/version but required payload sections are unsupported or malformed. [VERIFIED: current metadata parser only reads header/checksum fields in `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; D-01/D-02]
**Why it happens:** Phase 3 added metadata parsing for rebuild decisions, not full payload consumption. [VERIFIED: `/Users/trenton/Projects/yune/docs/analysis.md`]
**How to avoid:** Separate metadata parse, freshness decision, payload validation, and materialization into distinct result stages. [ASSUMED]
**Warning signs:** Runtime claims compiled path but candidates are empty or differ from source fixtures. [ASSUMED]

### Pitfall 4: Rebuild Reverse Mismatch Forces Table Rebuild

**What goes wrong:** Planning only table/prism freshness misses librime behavior where missing/stale reverse db forces table rebuild because reverse is built from table source/vocabulary during table build. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`; existing reverse checksum check in `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`]
**Why it happens:** Reverse db is not independently checked in the initial table/prism mental model. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`]
**How to avoid:** Rebuild execution result must include table, reverse, and prism actions; when reverse checksum mismatches, plan table/reverse rebuild together. [ASSUMED]
**Warning signs:** `.table.bin` reused while `.reverse.bin` remains stale or absent. [ASSUMED]

### Pitfall 5: Pack Checksum Chaining Uses Prior Primary Checksum

**What goes wrong:** Pack dictionaries are rebuilt with independent checksums instead of checksums initialized from the primary dictionary checksum. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc` lines 186-188]
**Why it happens:** Packs look like separate dictionaries but librime chains their checksum from the primary checksum. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`]
**How to avoid:** Model packs as ordered artifacts with `pack_file_checksum = checksum(initial=dict_file_checksum, pack_dict_files, settings)`. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`]
**Warning signs:** Pack table appears stale/rebuilt differently from librime for unchanged primary+pack source sets. [ASSUMED]

### Pitfall 6: UniTE Touches UserDB But UserDB Is Out of Scope

**What goes wrong:** Implementing UniTE by adding LevelDB/userdb learning and commit-history updates in Phase 4. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/gear/unity_table_encoder.cc`; D-12]
**Why it happens:** Librime `UnityTableEncoder` writes encoded phrases to user dictionary with prefix `\x7fenc\x1f`. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/gear/unity_table_encoder.cc`]
**How to avoid:** In Phase 4, consume compiled reverse `dict_settings`, stem lookup, and encoder payload data where representable; record userdb-backed phrase storage/learning for Phase 5. [VERIFIED: D-10/D-12]
**Warning signs:** Plan tasks mention LevelDB, user frequency updates, or predictive learning. [VERIFIED: D-12 says out of scope]

### Pitfall 7: Correction Data Has Disabled Librime Build Path

**What goes wrong:** Planning a `.correction.bin` implementation as mandatory without observing that librime’s edit-distance corrector creation/build path is disabled with `#if 0` and active creation returns `NearSearchCorrector`. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`; `/Users/trenton/Projects/librime/src/rime/dict/corrector.cc`]
**Why it happens:** Corrector source files exist and can distract from active compiled data path. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/corrector.cc`]
**How to avoid:** Plan prism spelling-map correction/tips support first and make `.correction.bin` a structured finding unless a schema-loaded fixture proves current librime build emits it. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/prism.cc`; `/Users/trenton/Projects/librime/src/rime/dict/corrector.cc`]
**Warning signs:** Plan depends on external `.correction.bin` artifacts or tries to load them in normal tests. [ASSUMED]

## Code Examples

Verified patterns from local source/oracle references:

### Current Runtime Source Dictionary Loader To Replace

```rust
// Source: /Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs
fn load_schema_table_dictionary(
    schema_config: &Value,
    name_space: &str,
) -> Option<TableDictionary> {
    let dictionary_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
        .and_then(config_scalar_string)
        .and_then(|dictionary_name| validate_data_resource_id(&dictionary_name))?;
    let dictionary_path = selected_runtime_data_path(&format!("{dictionary_name}.dict.yaml"))?;
    let dictionary_yaml = fs::read_to_string(dictionary_path).ok()?;
    // ... source imports, packs, vocabulary ...
}
```

Planning implication: keep validation and source fallback, but introduce compiled candidate resolution before `*.dict.yaml` reading and return a structured outcome instead of raw `Option`. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`; D-04/D-05]

### Librime Table Metadata Layout

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/dict/table.h
struct Metadata {
  static const int kFormatMaxLength = 32;
  char format[kFormatMaxLength];
  uint32_t dict_file_checksum;
  uint32_t num_syllables;
  uint32_t num_entries;
  OffsetPtr<Syllabary> syllabary;
  OffsetPtr<Index> index;
  int32_t reserved_1;
  int32_t reserved_2;
  OffsetPtr<char> string_table;
  uint32_t string_table_size;
};
```

Planning implication: `.table.bin` reader needs metadata, syllabary, index, entries, weights, and string-table handling or structured unsupported findings. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/table.h`; `/Users/trenton/Projects/librime/src/rime/dict/table.cc`]

### Librime Prism Correction Bit In Spelling Descriptor

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/dict/prism.cc
const int32_t kTypeIsCorrectionMask = 1 << 30;
const int32_t kSpellingTypeMask = ~kTypeIsCorrectionMask;
props.type = static_cast<SpellingType>(packed_type & kSpellingTypeMask);
props.is_correction = (packed_type & kTypeIsCorrectionMask) != 0;
```

Planning implication: DATA-04 should parse spelling descriptors sufficiently to expose normal/generated/correction spelling properties and compare tolerance/correction lookup behavior. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/prism.cc`; `/Users/trenton/Projects/librime/src/rime/dict/prism.h`]

### Reverse Stem Key Suffix and Dict Settings

```cpp
// Source: /Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.cc
static const char* kStemKeySuffix = "\x1fstem";
// dict settings required by UniTE
if (settings && settings->use_rule_based_encoder()) {
  std::ostringstream yaml;
  settings->SaveToStream(yaml);
  dict_settings = yaml.str();
}
```

Planning implication: reverse reader must expose both ordinary reverse lookup entries and stem lookup entries, plus optional `dict_settings` YAML for UniTE-compatible encoder behavior. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.cc`]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Source `.dict.yaml` runtime parsing only | Compiled-first, source-fallback runtime selection is required for Phase 4 | Phase 4 context, 2026-04-29 | Plans must add compiled readers and selection/failure semantics, not only source parser improvements. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |
| Metadata-only compiled checks | Payload readers beyond checksum metadata | Phase 4 requirement DATA-01 | Metadata parser must become or be accompanied by usable readers/finding outputs. [VERIFIED: `/Users/trenton/Projects/yune/.planning/REQUIREMENTS.md`; `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`] |
| Shelling out to librime compiler for artifacts | Deterministic local Rust rebuild execution | Phase 4 decision D-07/D-08 | Normal tests must not depend on external generated artifacts; use local fixtures and Rust builders. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |
| Phase 3 deferrals for UniTE/correction/contextual data | Phase 4 consumes representable compiled data and records out-of-scope findings | Phase 3 to Phase 4 boundary | UniTE reverse `dict_settings`, stems, prism spelling-map data, and correction/tolerance inputs now belong in planning; LevelDB/userdb/plugin remains out. [VERIFIED: `/Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md`; `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md`] |

**Deprecated/outdated:**
- Treating `.table.bin`/`.prism.bin`/`.reverse.bin` parse success as “metadata parsed”: insufficient for DATA-01. [VERIFIED: `/Users/trenton/Projects/yune/.planning/REQUIREMENTS.md`]
- Treating correction as an active `.correction.bin` dependency: local librime source has edit-distance corrector build/load paths disabled under `#if 0`; active corrector creation returns `NearSearchCorrector`. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`; `/Users/trenton/Projects/librime/src/rime/dict/corrector.cc`]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | If a later implementation chooses a Rust MARISA or Darts decoder crate, it must be separately researched and version-verified before adding it. | Standard Stack | Planner may under-scope dependency research if full trie compatibility is chosen. |
| A2 | Implement one shared checked offset resolver and test with nonzero offset fields at multiple struct positions. | Common Pitfalls | Could miss a valid alternative parser abstraction, but safety requirement remains. |
| A3 | Warning signs for offset/count/freshness bugs. | Common Pitfalls | Low impact; these are planning heuristics, not locked technical facts. |
| A4 | Use `checked_mul`, `checked_add`, maximum fixture-informed bounds, and return typed parse errors. | Common Pitfalls | Exact error type names may differ, but bounded parsing requirement is locked. |
| A5 | Separate metadata parse, freshness decision, payload validation, and materialization into distinct result stages. | Common Pitfalls | Planner could choose a different internal split if tests remain explicit. |
| A6 | Rebuild execution result must include table, reverse, and prism actions. | Common Pitfalls | Exact struct shape can differ, but action visibility is required by D-09. |
| A7 | Warning signs involving `.correction.bin`/pack/source behavior. | Common Pitfalls | Low impact; these are validation heuristics. |

## Open Questions

1. **How far should minimum usable `.table.bin` support go without a MARISA decoder?**
   - What we know: Librime table entries store text as string-table IDs and table/reverse string tables are MARISA trie images. [VERIFIED: `/Users/trenton/Projects/librime/src/rime/dict/table.h`; `/Users/trenton/Projects/librime/src/rime/dict/string_table.h`]
   - What's unclear: Whether Phase 4 fixtures can use a minimal locally generated payload variant that avoids full MARISA decoding, or whether a Rust MARISA reader dependency becomes necessary. [ASSUMED]
   - Recommendation: Plan 04-01 with a spike/test gate that either proves a minimum supported payload class or creates a structured finding naming MARISA string-table decoding as the blocker for broader table/reverse parity. [VERIFIED: D-02]

2. **Where should explicit dictionary load failure surface in ABI-visible behavior?**
   - What we know: Current installer silently skips translator install on `None`; D-05 requires explicit/test-covered failure. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs`; D-05]
   - What's unclear: The existing ABI has no obvious diagnostic channel for schema-install dictionary load failure beyond absent candidates/status. [VERIFIED: codebase architecture/testing docs]
   - Recommendation: Plan a focused internal outcome and test-visible assertion first; if no ABI error API exists, record structured deferral rather than silently installing empty dictionaries. [ASSUMED]

3. **Should rebuild write exact librime binary layout or a Yune-readable compatible subset first?**
   - What we know: D-07 requires deterministic local Rust rebuild and DATA-02 freshness behavior; D-01 requires readers for librime payload compatibility. [VERIFIED: context/requirements]
   - What's unclear: Whether rebuild output must be byte-consumable by librime in Phase 4 or only by Yune with librime-observable behavior parity. [ASSUMED]
   - Recommendation: Plan rebuild output around Yune reader + candidate behavior parity first, and explicitly record byte-for-byte/librime-consumable output as not proven unless tests compare it. [ASSUMED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust `cargo` | Build/test implementation | ✗ | — | Human/environment must provide Cargo before executing tests locally. [VERIFIED: environment audit] |
| `rustc` | Build/test implementation | ✗ | — | Usually installed with Cargo; same blocker. [VERIFIED: environment audit] |
| Git | Source inspection/version control | ✓ | git version 2.50.1 (Apple Git-155) | — [VERIFIED: environment audit] |
| `/Users/trenton/Projects/librime` local checkout | Oracle source comparison | ✓ | local source tree present | Continue using source as oracle; do not require during normal tests. [VERIFIED: filesystem/source reads] |
| CMake/Ninja/librime build tools | Optional oracle artifact generation only | ✗/unknown because shell stopped after missing cargo in audit command | Not required for normal Phase 4 tests per D-08. [VERIFIED: D-08; environment audit limitation] |

**Missing dependencies with no fallback:**
- `cargo`/`rustc` are missing in this shell; implementation execution and Cargo test validation are blocked until the Rust toolchain is available in the executor environment. [VERIFIED: environment audit]

**Missing dependencies with fallback:**
- External librime compiler/build tooling is not required for normal tests because D-08 forbids shelling out to librime compilers during normal tests. [VERIFIED: D-08]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No authentication layer exists in this local library/CLI project. [VERIFIED: `/Users/trenton/Projects/yune/.planning/codebase/INTEGRATIONS.md`] |
| V3 Session Management | yes | Preserve existing deterministic RIME session lifecycle tests; Phase 4 should not mutate process-wide session state outside schema selection/deployment paths. [VERIFIED: `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md`; `/Users/trenton/Projects/yune/.planning/codebase/TESTING.md`] |
| V4 Access Control | no | No multi-user access-control boundary exists; filesystem resource IDs are the relevant boundary. [VERIFIED: `/Users/trenton/Projects/yune/.planning/codebase/INTEGRATIONS.md`] |
| V5 Input Validation | yes | `validate_data_resource_id` for schema-provided dictionary IDs; bounded compiled binary readers fail closed on malformed offsets/counts/lengths/versions. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs`; D-13/D-14] |
| V6 Cryptography | no | Checksums are compatibility/freshness CRC-style values, not security cryptography; do not treat them as tamper-proof integrity controls. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc`] |

### Known Threat Patterns for compiled dictionary parsing

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via schema dictionary/prism/pack IDs | Tampering / Information Disclosure | Validate logical resource IDs before path joins and reject separators, `..`, NUL, absolute/drive-like IDs. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs`; D-13] |
| Malformed binary offset points outside payload | Denial of Service / Tampering | Resolve `OffsetPtr` with checked arithmetic against byte length; return parse error, never panic/transmute. [VERIFIED: D-14; `/Users/trenton/Projects/librime/src/rime/dict/mapped_file.h`] |
| Malformed array/list count causes allocation explosion | Denial of Service | Bound counts by payload size and fixture-informed maxima; avoid allocating before validating byte ranges. [VERIFIED: D-14] |
| Unsupported format version accepted as compatible | Tampering / Reliability | Enforce table >= 4.0, prism >= 4.0 current behavior, reverse compatible 3.0..4.0 or updated exact oracle range with structured unsupported errors. [VERIFIED: `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; `/Users/trenton/Projects/librime/src/rime/dict/table.cc`; `/Users/trenton/Projects/librime/src/rime/dict/prism.cc`; `/Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.cc`] |
| Checksum spoofing treated as security | Spoofing / Tampering | Use checksum only for freshness compatibility; payload parser must still validate structure and fail closed. [VERIFIED: `RimeChecksumComputer` in `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs`; D-14] |

## Sources

### Primary (HIGH confidence)
- `/Users/trenton/Projects/yune/.planning/phases/04-compiled-dictionary-data/04-CONTEXT.md` — locked Phase 4 decisions D-01 through D-14, scope, deferred ideas.
- `/Users/trenton/Projects/yune/.planning/REQUIREMENTS.md` — DATA-01 through DATA-04.
- `/Users/trenton/Projects/yune/.planning/ROADMAP.md` — Phase 4 goal, success criteria, and 04-01 through 04-04 plan slices.
- `/Users/trenton/Projects/yune/.planning/PROJECT.md` — compatibility oracle, architecture/security/testing constraints.
- `/Users/trenton/Projects/yune/.planning/phases/03-schema-pipeline-depth/03-CONTEXT.md` — Phase 4 deferrals from Phase 3.
- `/Users/trenton/Projects/yune/.planning/phases/02-native-abi-validation-and-runtime-safety/02-CONTEXT.md` — resource-ID and ABI safety constraints.
- `/Users/trenton/Projects/yune/.planning/codebase/ARCHITECTURE.md` — ownership boundaries and data flow.
- `/Users/trenton/Projects/yune/.planning/codebase/INTEGRATIONS.md` — runtime files, local storage, dependencies.
- `/Users/trenton/Projects/yune/.planning/codebase/CONCERNS.md` — compiled data gaps, scaling/security concerns.
- `/Users/trenton/Projects/yune/.planning/codebase/TESTING.md` — test framework and ABI/schema-loaded test patterns.
- `/Users/trenton/Projects/yune/docs/analysis.md` and `/Users/trenton/Projects/yune/docs/roadmap.md` — current compatibility state and compiled-data gaps.
- `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/compiled.rs` — existing checksum/metadata/rebuild primitives.
- `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/source.rs` — source dictionary parser, stems, preset vocabulary, phrase injection.
- `/Users/trenton/Projects/yune/crates/yune-core/src/dictionary/encoder.rs` — table encoder primitives.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/schema_install.rs` — current source dictionary loading and schema install ownership.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/runtime.rs` — runtime roots.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/deployment.rs` — maintenance/workspace update/deployment behavior.
- `/Users/trenton/Projects/yune/crates/yune-rime-api/src/resource_id.rs` — logical resource-ID validation.
- `/Users/trenton/Projects/librime/src/rime/dict/mapped_file.h` — `OffsetPtr`, `Array`, `List`, mapped-file allocation layout.
- `/Users/trenton/Projects/librime/src/rime/dict/table.h` and `table.cc` — `.table.bin` format and query behavior.
- `/Users/trenton/Projects/librime/src/rime/dict/prism.h` and `prism.cc` — `.prism.bin` format, Darts array, spelling map, correction bit.
- `/Users/trenton/Projects/librime/src/rime/dict/reverse_lookup_dictionary.h` and `.cc` — `.reverse.bin` format, stems, dict settings, MARISA key/value tries.
- `/Users/trenton/Projects/librime/src/rime/dict/dict_compiler.cc` — source-vs-prebuilt fallback, checksum/freshness, pack chaining, rebuild execution.
- `/Users/trenton/Projects/librime/src/rime/dict/entry_collector.cc` and `dict_settings.cc` — stems, preset vocabulary, source collection behavior.
- `/Users/trenton/Projects/librime/src/rime/algo/encoder.cc` — encoder formulas and phrase encoding behavior.
- `/Users/trenton/Projects/librime/src/rime/gear/unity_table_encoder.cc` — UniTE reverse dict settings/stem usage and userdb boundary.
- `/Users/trenton/Projects/librime/src/rime/dict/corrector.cc` and `.h` — correction/tolerance behavior and disabled edit-distance artifact path.

### Secondary (MEDIUM confidence)
- None. No web search was needed because authoritative local project and local librime oracle sources were available. [VERIFIED: local source availability]

### Tertiary (LOW confidence)
- Assumptions listed in the Assumptions Log only.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — dependency versions and project structure verified from `Cargo.toml`/`Cargo.lock`; toolchain availability verified by shell audit. [VERIFIED]
- Architecture: HIGH — ownership and flow verified from Phase 4 context and codebase architecture maps. [VERIFIED]
- Payload format details: HIGH for metadata/layout fields from local librime source; MEDIUM for minimum implementable reader scope because exact fixture payload support remains a planning/execution choice. [VERIFIED + ASSUMED]
- Rebuild semantics: HIGH — librime `DictCompiler::Compile` and existing Yune rebuild primitives were read directly. [VERIFIED]
- Pitfalls: MEDIUM-HIGH — core pitfalls are directly grounded in source; some warning signs are planning heuristics. [VERIFIED + ASSUMED]

**Research date:** 2026-04-29
**Valid until:** 2026-05-29 for local codebase/oracle-source findings; re-check if librime checkout or Yune Phase 3/4 implementation changes before planning.
