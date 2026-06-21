# M18 — Deployment & Processor Depth Implementation Plan

> **Status:** Finished · **Milestone:** M18 (Deployment & processor depth) · **Closed:** 2026-06-21 · **Type:** execution plan
>
> **For agentic workers:** implement task-by-task; steps use checkbox (`- [x]`) syntax. Capture oracle goldens *before* writing the producing module wherever a step is marked **capture-goldens-first**.

**Goal:** Close the deployment-write and punctuation-processor gaps that the M12 parity harness and M20 playground left open: (a) generate a `.prism.bin` from a schema's `speller/algebra` + syllabary, (b) provide a public binary-dictionary **write** API that compiles/serializes the `.table.bin` / `.prism.bin` / `.reverse.bin` artifacts Yune currently only *reads*, and (c) implement the `ascii_punct` processor bypass and immediate-commit punctuation behaviors, un-ignoring the two M12 blockers and wiring the M20-deferred `ascii_punct` toggle.

**Architecture:** Today the `dictionary` subsystem is **read-and-plan only**. It parses source YAML (`source.rs`), parses compiled binaries (`compiled_table.rs`, `compiled_prism.rs`, `compiled_reverse.rs`), and *decides* whether artifacts are stale (`rime_dict_rebuild_plan`, `crates/yune-core/src/dictionary/compiled.rs:259-335`) — but `RimeDictArtifactStatus::Rebuilt` is a status label produced by `artifact_status` (`compiled.rs:337-343`), not an executor; **nothing writes bytes**. M18 adds owning *writer* modules that are the inverse of the existing parsers and round-trip through them, plus a `Prism` builder that finally produces the darts double-array section that `compiled_prism.rs:60-64` presently *rejects* as `UnsupportedSection`. Punctuation today is a pure translator (`punctuation.rs`) that emits candidates and honors `is_full_shape` but nothing else; M18 adds a thin processor layer in the `Engine` key path (`engine.rs:344`) honoring `ascii_punct` (the field lives in `state.rs:221`, is set/read only in `set_option`/`get_option` at `engine.rs:281,296`, and is **never consulted** on the punctuation path) and immediate-commit/confirm/pair definitions, mirroring upstream `Punctuator::ProcessKeyEvent`.

**Tech Stack:** Rust `yune-core` (`crates/yune-core/src/dictionary/**`, `engine.rs`, `punctuation.rs`, `state.rs`); the M12 oracle harness (`scripts/oracle-rime-probe.cs` driving the pinned upstream `rime.dll`, `scripts/capture-upstream-luna-pinyin.ps1`); provenance-guarded fixtures under `crates/yune-core/tests/fixtures/upstream-1.17.0/` enforced by `oracle_fixture_provenance.rs`; parity tests in `upstream_luna_pinyin_parity.rs`. Darts double-array encoding in pure Rust.

> **Capture note (non-circular oracle):** `scripts/oracle-rime-probe.cs` is currently a session/key driver only (`RimeProcessKey`/`RimeGetCommit`/`RimeGetContext`/`RimeSetOption`); it copies no deployer artifacts and captures no `ascii_punct`-on or `commit`/`pair` punctuation scenarios. The existing `luna-pinyin-punctuation.json` fixture has exactly three snapshots (`punctuation_period`, `symbol_fh`, `symbol_no_match`), all with `is_ascii_punct:false`, and its `punctuation_entries` are list-shaped (`half_shape`/`full_shape`/`symbols`) with **no** `commit`/`pair` map forms. Every M18 task that asserts new processor/byte behavior therefore needs a **fresh upstream capture first** — none of the new expectations exist in the current fixtures.

---

## Scope / Non-goals

**In scope (each item names a target):**
- **Prism generation from algebra** — produce a `.prism.bin` whose bytes the existing `parse_rime_prism_bin_payload` (and a new darts reader) accept, from a schema's `speller/algebra` projection over the table syllabary. Target: round-trip against Yune's own reader **and** a byte-level golden captured from the upstream `1.17.0` deployer for a curated small syllabary.
- **Binary-dictionary write API** — public `dictionary` functions that serialize a `TableDictionary` (+ corrections/tolerance/encoder advanced data) to `.table.bin` and a reverse index to `.reverse.bin`, plus the prism above. Target: every writer round-trips through its existing parser; a `rime_dict_rebuild_plan` decision can now be *executed*.
- **`ascii_punct` processor bypass** — un-ignore `ascii_punct_option_processor_bypass_parity_is_blocked` (`upstream_luna_pinyin_parity.rs:383`) and wire the M20-deferred toggle (the M20 plan explicitly punts `ascii_punct` "until M18 implements the real processor-level behavior", `docs/plans/archive/m20-plan-web-demo-showcase-controls.md:48`). **Requires a new oracle capture of an `ascii_punct`-on scenario** (the current fixture has none).
- **Immediate-commit punctuation** — un-ignore `punctuation_immediate_commit_processor_parity_is_blocked` (`upstream_luna_pinyin_parity.rs:389`); implement ConfirmUnique / AutoCommit(`commit`) / Pair / alternating semantics from upstream `punctuator.cc`. **Requires a new oracle capture of a schema exercising `commit`/`pair` definitions** (luna_pinyin default punctuation has neither shape).

**Non-goals (no named target):**
- A `marisa` string-table table format, or matching upstream's exact darts node packing **beyond what Yune's own reader requires** — Yune's table reader already round-trips a Yune-native `YUNE-*` advanced payload (`compiled_table.rs:57`) and rejects marisa (`compiled_table.rs:49-53`); the write API targets *that* format, not bit-for-bit upstream table internals.
- Full `DictCompiler` orchestration parity (pack iteration, memory-release ordering) beyond what is needed to emit the three artifacts.
- `use_space`/auto-commit config surfaces for any schema **no fixture exercises** — implement only the punctuation definition shapes present in the captured fixtures (after the Task 5 capture lands).
- The M17 language-model / lattice work (the other blocked test `full_sentence_lattice_parity_for_zhongguo_is_blocked` stays ignored).
- General deployment lifecycle (maintenance threads, user-config sync) — out of scope; M18 is artifact *bytes* + punctuation *processor*, not the deployer runtime.

---

## Tasks

### Task 1 — Pure-Rust darts double-array writer + reader (prism substrate)

Yune's prism reader **refuses** any double-array section (`compiled_prism.rs:60-64`: `UnsupportedSection { role: "darts double_array" }`). Prism generation is impossible without one. Own this slice first.

- [x] Add `crates/yune-core/src/dictionary/double_array.rs` implementing a Darts-compatible double-array build (`build(keys: &[(key, value)])`) and `common_prefix_search` / `exact_match` lookups over the serialized `Vec<i32>` base/check arrays.
- [x] Add a `double_array` reader path to `compiled_prism.rs` so a non-empty double_array section parses into the trie instead of erroring; keep the existing reject for *malformed* sections.
- [x] **capture-goldens-first:** extend `scripts/oracle-rime-probe.cs` with an artifact-extraction mode that points the deployer at a curated build dir and copies the compiled `<build>/<schema>.prism.bin` out into a fixture (the probe today only drives sessions/keys — verify the upstream `rime.dll` deployer actually emits the file for the curated schema). Capture a byte golden of a real upstream `.prism.bin` double_array for a tiny curated syllabary (e.g. 5–8 luna_pinyin syllables). Store under `tests/fixtures/upstream-1.17.0/` with provenance.
- [x] Unit test: `build` then `common_prefix_search` recovers all inserted keys; the upstream golden double_array parses and resolves the same syllables.
- **Acceptance:** `compiled_prism.rs` parses an upstream `.prism.bin` containing a real darts double_array (the section it formerly rejected); the pure-Rust builder reproduces equivalent lookup results; new fixture has machine-readable provenance accepted by `oracle_fixture_provenance.rs`.

### Task 2 — Prism generation from `speller/algebra`

Upstream `BuildPrism()` (confirmed in `dict_compiler.cc`): load the primary table's syllabary (`GetSyllabary`), read `config.GetList("speller/algebra")`, `for (x : syllabary) script.AddSyllable(x)`, apply each projection (`p.Apply(&script)`), then `prism_->Build(syllabary, script.empty() ? nullptr : &script, dict_file_checksum, schema_file_checksum)`. Yune has the runtime algebra engine (`spelling_algebra.rs` — but it is `pub(crate)` runtime *expansion*, not a serializer) and the syllabary (from a parsed table). No build path exists.

- [x] Add `dictionary::build_prism_bin(syllabary, algebra_formulas, dict_file_checksum, schema_file_checksum) -> Vec<u8>` (new `prism_writer.rs`). Reuse `SpellingAlgebra::parse` to project each syllable into spelled forms, build the spelling_map (`syllable_id`, `spelling_type`, `is_correction` bit-30, `credibility`) and the Task 1 double_array, and serialize the header offsets matching `parse_rime_prism_bin_payload` (size at 48, offsets at 52/56/60/64, version `Rime::Prism/4.0`).
- [x] Wire fuzzy/abbrev/correction spelling types and credibility from algebra formula kinds, reusing the penalty constants already in `spelling_algebra.rs:10-12` (`SPELLING_ALGEBRA_FUZZY_PENALTY` etc.) for the credibility values where upstream stores them.
- [x] Round-trip test: `build_prism_bin(...)` → `parse_rime_prism_bin_payload(...)` recovers the same `num_syllables`, `spelling_map`, checksums.
- [x] **capture-goldens-first:** byte-level parity test comparing Yune's generated spelling_map/metadata against the Task 1 upstream golden for the curated syllabary + a small algebra (`derive`, `abbrev`, `fuzz` one rule each, captured from upstream). Compare *parsed structures*, not raw bytes, where Yune's offset layout legitimately differs from upstream's (document the divergence in the test as a named non-goal).
- **Acceptance:** Generated prism round-trips through Yune's reader; spelling descriptors (type/correction/credibility) match the upstream oracle golden for the curated algebra; test names the algebra rules it covers.

### Task 3 — Public binary-dictionary write API (`.table.bin`, `.reverse.bin`)

`dictionary/mod.rs:8-28` exposes only `parse_*` readers + a decision-only `rime_dict_rebuild_plan` (plus `RimeDictRebuildExecutionReport`/`RimeDictArtifactStatus`, which are *reporting* types, not writers). Add the inverse writers so a rebuild plan can be *executed*.

- [x] Add `table_writer.rs`: `build_table_bin(dict: &TableDictionary, dict_file_checksum) -> Vec<u8>` emitting the syllabary, head index, and the Yune-native `YUNE-*` advanced payload (corrections/tolerance/encoder/lookup) that `compiled_table.rs:57` already reads. Do **not** emit marisa/darts table sections (Non-goal).
- [x] Add `reverse_writer.rs`: `build_reverse_bin(dict, dict_file_checksum) -> Vec<u8>` as the inverse of `parse_rime_reverse_bin_dictionary`.
- [x] Add a thin `execute_rebuild_plan(plan, sources, out_dir)` helper that, given a `RimeDictRebuildPlan` with `rebuild_table/rebuild_prism/rebuild_reverse` set, calls Task 2/Task 3 writers and persists files — turning `RimeDictArtifactStatus::Rebuilt` (today only a label from `artifact_status`) into an action.
- [x] Re-export the new `build_*_bin` and the executor from `dictionary/mod.rs`.
- [x] Round-trip tests: `parse(build(dict)) == dict` for table and reverse (modulo documented lossy fields); checksum field at offset 32 matches the input.
- **Acceptance:** `build_table_bin`/`build_reverse_bin` round-trip through their existing parsers; `execute_rebuild_plan` writes exactly the artifacts the plan marks `Rebuilt` and leaves `ReusedFresh`/`ReusedPrebuilt` untouched; all three writers exercised by tests.

### Task 4 — `ascii_punct` processor bypass

The field is tracked (`state.rs:221`; `engine.rs:281` sets, `engine.rs:296` reads in `set_option`/`get_option`) but **never consulted** by the punctuation path; `punctuation.rs` translates regardless (it honors only `is_full_shape`, `punctuation.rs:72-114`). Upstream `Punctuator::ProcessKeyEvent` returns `kNoop` when `ascii_punct` is on (`if (ctx->get_option("ascii_punct")) return kNoop;`), letting the raw ASCII key through.

- [x] **capture-goldens-first:** the current `luna-pinyin-punctuation.json` has no `ascii_punct`-on scenario (all three snapshots are `is_ascii_punct:false`, lines 333/355/403). Extend the probe / capture script to set `ascii_punct=1` and capture the upstream behavior for `.` (and `/`): expect raw-ASCII commit and an empty/absent punctuation menu. Add the new snapshot(s) to the fixture with provenance.
- [x] Add a `Status.is_ascii_punct` check on the punctuation path: when `ascii_punct` is on, the Engine must **not** route a punctuation key into the punctuation translator/composition and must instead pass the literal ASCII character through to commit (mirroring `kNoop` → raw key).
- [x] Decide the seam: prefer the existing `process_key_event` / `translate_with_context` boundary over a new processor trait unless the bypass cannot be expressed there (own-each-slice; minimal new surface).
- [x] Wire the M20-deferred `ascii_punct` UI toggle through the existing `setOption()` path (no new export) — but only after the engine behavior is proven; update the M20 control table note that previously deferred it.
- [x] Replace the `panic!` body of `ascii_punct_option_processor_bypass_parity_is_blocked` (`upstream_luna_pinyin_parity.rs:383-386`) with a real assertion driven by the **newly captured** `ascii_punct`-on snapshot.
- **Acceptance:** with `ascii_punct` on, typing `.` commits/produces `.` (not `。`) and produces no punctuation candidate menu, matching the newly captured upstream snapshot; with it off, behavior is unchanged; the formerly-ignored test is enabled and passes against the upstream fixture (not against Yune-derived expectations).

### Task 5 — Immediate-commit / confirm-unique / pair punctuation

Upstream distinguishes (confirmed in `punctuator.cc`): ConfigValue → `ConfirmUniquePunct` (immediate `ConfirmCurrentSelection`, no menu); ConfigMap`{commit:}` → `AutoCommitPunct` (`ctx->Commit()` directly); ConfigMap`{pair:}` → `PairPunct` (alternating index via `oddness_`: `(selected_index += oddness) %= 2; oddness = 1 - oddness;`); ConfigList → alternating menu (`TranslateAlternatingPunct`). Yune's punctuation always yields a candidate list with no immediate-commit path in the Engine.

- [x] **capture-goldens-first:** the current fixture's `punctuation_entries` are list-shaped only (`half_shape`/`full_shape`/`symbols`, lines 277-320) with **no** `commit` or `pair` map forms, and luna_pinyin's default punctuation has neither. Capture from upstream a curated schema (or curated `punctuation:` config) that exercises one `unique` (ConfigValue), one `commit` (ConfigMap), and one `pair` (ConfigMap) definition; record the per-keystroke commit/menu/oddness behavior. Add the fixture with provenance.
- [x] Extend the punctuation entry model to carry a definition *kind* (unique / commit / pair / list), parsed from the punctuation source data shapes (list forms already present; `commit`/`pair` map forms from the new capture).
- [x] In the Engine key path, after punctuation translation: a `unique` or `commit` definition commits immediately (one keystroke → committed text, no lingering composition); a `pair` definition toggles between its two values on repeat; a `list` definition shows the alternating menu (existing behavior).
- [x] Reuse `set_punctuation_composition` / `commit_candidate` (`engine.rs:696`, `:906`) rather than adding parallel commit machinery.
- [x] Replace the `panic!` body of `punctuation_immediate_commit_processor_parity_is_blocked` (`upstream_luna_pinyin_parity.rs:389-393`) with assertions over the **newly captured** fixture covering each kind present.
- **Acceptance:** one-key punctuation whose definition is unique/commit commits immediately with empty residual composition; pair punctuation alternates on repeat; list punctuation still menus; the formerly-ignored immediate-commit test is enabled and matches the upstream fixture.

### Task 6 — Wire-up, docs, and ledger

- [x] Update `docs/roadmap.md` M18 row from planned to in-progress/done as tasks land; move prism-generation and binary-dict/deployment writing from the **Deferred** column (`docs/roadmap.md:432`) into the **In scope** column of the scope ledger.
- [x] Add a `docs/fork-parity-ledger.md` note for the table/prism write-format decision (Yune-native `YUNE-*` payload + darts prism; not marisa) as an explicit own-each-slice divergence from upstream internals (and note generated `.table.bin` is not consumable by upstream librime).
- [x] Ensure `oracle_fixture_provenance.rs` covers any new fixtures (the Task 1 prism golden, the Task 4 `ascii_punct`-on snapshot, and the Task 5 commit/pair capture): manifest entries, non-circular source.
- **Acceptance:** roadmap + ledger reflect shipped behavior; no fixture lacks provenance; `cargo test -p yune-core` green with the two formerly-ignored punctuation tests now enabled.

---

## Completion criteria

- [x] Tasks 1–6 acceptance lines all met.
- [x] `parse_rime_prism_bin_payload` accepts a real upstream darts double_array (former hard reject removed for valid sections).
- [x] `build_prism_bin`, `build_table_bin`, `build_reverse_bin` each round-trip through their existing parsers; `execute_rebuild_plan` writes only the artifacts a plan marks `Rebuilt`.
- [x] Prism spelling-map descriptors match the upstream `1.17.0` oracle golden for a curated syllabary + small algebra; divergences (offset layout vs marisa/upstream packing) are documented as named non-goals, not silent.
- [x] `ascii_punct_option_processor_bypass_parity_is_blocked` and `punctuation_immediate_commit_processor_parity_is_blocked` are no longer `#[ignore]`d and pass against **newly captured** upstream fixtures (an `ascii_punct`-on snapshot and a commit/pair capture — not against the prior `is_ascii_punct:false`, list-only fixture).
- [x] M20 `ascii_punct` toggle flows through `setOption()` with no new `RimeApi`/`RimeCandidate`/export change.

## Review checklist

- [x] **Non-circular oracle:** every "expected bytes"/expected-behavior comes from a captured upstream `1.17.0` artifact or snapshot (Tasks 1, 2, 4, 5 each add a fresh capture), never re-derived from Yune; the two un-ignored tests assert captured upstream snapshots, not the prior all-false fixture.
- [x] **Own-each-slice:** double-array, prism writer, table writer, reverse writer, ascii_punct bypass, and immediate-commit each have a dedicated module + test + named oracle/round-trip target.
- [x] **Upstream-first ABI:** no `RimeApi` table or `RimeCandidate` change; the ascii_punct toggle reuses existing option transport.
- [x] **Fixtures-before-module-replacement:** Task 1, 2, 4, and 5 goldens/snapshots captured before the producing modules/assertions are trusted.
- [x] **Name-the-behavior:** marisa string-table, full `DictCompiler` orchestration, and `use_space`/auto-commit shapes no fixture uses remain unimplemented (explicit non-goals), not half-built.
- [x] Heavy/risky flags acknowledged: the darts double-array writer (Task 1) is the highest-risk slice and gates Task 2 — if upstream byte parity proves out of reach for a small syllabary, fall back to Yune-native prism bytes that round-trip Yune's reader and record the divergence in the ledger rather than chasing bit-for-bit darts internals. Secondary risk: the probe must be extended to extract deployer artifacts (Task 1) and to capture `ascii_punct`-on and `commit`/`pair` scenarios (Tasks 4/5), none of which the current session/key-only probe supports.