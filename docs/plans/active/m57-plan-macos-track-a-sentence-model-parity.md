# M57 macOS Track A Sentence-Model Parity And Verification Repair Plan

> **For agentic workers:** If a plan-execution sub-skill is available, use it;
> otherwise execute the checkboxes directly, one phase at a time. This plan is
> intentionally written as a review packet: before implementing, have another
> reviewer challenge the root-cause chain, the proposed fix, and the evidence
> gates.

> **Status:** Draft for review. - **Track:** Engine performance correctness and
> cross-platform verification. - **Created:** 2026-07-04. - **Type:** bug-fix
> milestone plus verification repair. No ABI widening, no product behavior
> expansion, no browser live-site comparison.

**Goal:** Make native Track A `luna_pinyin` sentence-model behavior and
performance evidence platform-honest on macOS by fixing the Yune-side
sentence-model construction/abbreviation path exposed by the independent macOS
verification run, then re-running the same macOS evidence bundle against the
fixed code.

This milestone does **not** aim to create a new performance claim. It repairs a
specific portability/comparability failure:

- macOS local librime `1.17.0` candidate snapshots match the Windows
  corrective oracle shape for the two abbreviation rows.
- macOS Yune candidate snapshots do **not** match the Windows corrective Yune
  row or the local librime oracle for those rows.
- The slow rows are not dominated by `process_key`, ABI allocation,
  `get_context/free_context`, Darwin memory sampling, raw table lookup, or raw
  prism lookup. They are dominated by Yune's upstream sentence-model graph
  rebuild/extend path.

Until this is fixed, the macOS verification bundle is useful diagnostic
evidence but **not** a valid contradiction of the Windows M55 corrective claim.

---

## Problem Statement

The macOS verification run created under
`docs/reports/evidence/macos-performance-verification-2026-07-04/` used the
same native benchmark shape as M55: every keypress runs `process_key` and then
`get_context/free_context`. Most rows were understandable, but four Track A
rows were obviously wrong:

| Row | Windows corrective Yune/librime | macOS Yune/librime | Why suspicious |
| --- | ---: | ---: | --- |
| 37-char `ceshiyixiachangjushuruxingnengzenyang` | `1.913x` slower | `~60.8x` slower | graph work exploded |
| 59-char `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `1.528x` slower | `~43.2x` slower | graph work exploded |
| `cszysmsrsd` | `0.381x` faster | `~5.05x` slower | abbreviation path skipped |
| `zybfshmsru` | `0.564x` faster | `~7.18x` slower | abbreviation path skipped |

The same rows also show candidate mismatch:

| Input | Expected Yune/librime shape | macOS Yune shape |
| --- | --- | --- |
| `cszysmsrsd` | `重商主義什麼是認識到`, `重商主義`, `催生作用`, `產生爭議`, `測試資源` | `重商主義什麼少女時代`, `造山作用什麼少女時代`, ... |
| `zybfshmsru` | `自有辦法什麼收入`, `自有辦法`, `重要部分`, `晝夜不分`, `主要部分` | `專業並不是美少女`, `只有並不是美少女`, ... |

The local macOS librime candidate snapshots still produce the expected
abbreviation rows. The defect is Yune-side.

## Diagnostic Evidence

The `m37_metrics.csv` counters point at the sentence model:

| Row | Windows vocab considered | macOS vocab considered | Windows graph edges | macOS graph edges |
| --- | ---: | ---: | ---: | ---: |
| 37-char | `168` | `9,741` | `401` | `10,211` |
| 59-char | `314` | `15,051` | `667` | `15,899` |
| `cszysmsrsd` | `151` | `9,020` | `11,156` | `9,090` |
| `zybfshmsru` | `89` | `6,993` | `8,833` | `7,070` |

For the abbreviation rows, the decisive counter is that the Windows corrective
run reaches the abbreviation-specific path while macOS does not:

| Row | Windows `abbreviation_span_discovery_calls` | macOS `abbreviation_span_discovery_calls` |
| --- | ---: | ---: |
| `cszysmsrsd` | `9` | `0` |
| `zybfshmsru` | `9` | `0` |

The memory-owner profile exposes a model-shape mismatch:

| Owner | Windows owned default (`run-c` / `gate-run-d`) | macOS run 2 |
| --- | ---: | ---: |
| `poet.entries_by_code` | `513,353` | `191,984` |
| `poet.lookup_index` | `332,604` | `31,262` |
| `poet.vocabulary` | `421,966` | `421,966` |
| `poet.abbreviation_vocabulary` | `11` | `421,966` |

That last row is the smoking gun. The M42 abbreviation vocabulary is supposed
to be the 11 target phrases in
`crates/yune-rime-api/src/schema_install.rs`:

- `重商主義`
- `什麼`
- `認識到`
- `催生作用`
- `產生爭議`
- `測試資源`
- `自有辦法`
- `收入`
- `重要部分`
- `晝夜不分`
- `主要部分`

On this Mac, those 11 rows exist in `essay.txt`, and an external parser
reproduces the 11-row filter. The runtime model nevertheless reports the full
`421,966` essay rows as the abbreviation vocabulary, which means the
abbreviation-specific model was not constructed or was bypassed.

There is also a compiled-data mismatch:

| Field | Windows corrective Track A | macOS Track A |
| --- | --- | --- |
| `source_checksum` | `0x16ad0e3e` | `0xb3d4e98e` |
| `table_checksum` | `0xb967cfef` | `0x29d56c89` |
| `byte_source_len` | `13,013,460` | `13,013,460` |
| `stored_entries` | `498,564` | `498,564` |

The byte size and stored-entry count match, but checksum and sentence-model
owner shape do not. Yune exact lookup remains usable from the compact table,
but the sentence model is built from `TableStorage::Compact::table_entry_iter`,
which enumerates `store.all_codes()` and then exact candidates. That makes the
sentence model sensitive to compact/MARISA full-code enumeration.

Two isolation checks are required before implementation:

1. Run the focused macOS rows with `YUNE_POET_BYTE_BACKED=1` and capture
   candidate snapshots, owner counts, and product-path status. If the rows
   remain wrong, byte-backed poet storage is not a workaround and the defect is
   upstream of the owned/byte-backed poet choice.
2. Count `table_entry_iter()` output against the compiled table's
   `stored_entries=498,564`, and record the distinct `all_codes()` count
   separately. `all_codes()` itself is a distinct-code count, not an
   entry-count; the entry-count invariant is the `all_codes()` plus
   `exact_candidates()` expansion.

Relevant code paths:

- `crates/yune-core/src/translator/mod.rs`
  - `TableStorage::table_entry_iter`
  - `TableStorage::all_codes` / `TableStorage::exact_candidates` delegation
  - `StaticTableTranslator::with_upstream_sentence_model`
  - `StaticTableTranslator::abbreviation_sentence_spans`
  - bounded sentence-model path that returns normal sentence candidates before
    trying abbreviation fallback
- `crates/yune-core/src/dictionary/compiled_table.rs`
  - `CompactTableStore::all_codes` and `exact_candidates`
  - MARISA traversal and tail-code decoding
  - `marisa_initial_prefix_frames` / `CompactAllCodesInner::Marisa`
- `crates/yune-rime-api/src/schema_install.rs`
  - `spelling_algebra_for_dictionary`
  - `load_luna_pinyin_preset_vocabularies`
  - `load_m42_luna_pinyin_abbreviation_vocabulary`

## Current Root-Cause Hypothesis

The defect is a Yune runtime construction issue, not a macOS librime issue.
The likely chain is:

1. The macOS oracle/deploy step produces a valid librime `luna_pinyin` bundle
   whose user-visible librime results are correct.
2. Yune's compact table lookup path can answer common exact/prefix rows, but
   the full-entry enumeration used to build the upstream sentence model is not
   equivalent to the Windows corrective path.
3. The model constructed from that enumeration has far fewer
   `poet.entries_by_code` and `poet.lookup_index` rows.
4. The M42 abbreviation-only model is not active in that constructed model, so
   `poet.abbreviation_vocabulary` falls back to the full essay vocabulary.
5. For abbreviation inputs, the normal sentence model now returns bad full
   vocabulary candidates, so the bounded path returns early and never reaches
   `abbreviation_sentence_candidates`.
6. For long full-pinyin rows, the same model shape creates thousands more
   vocabulary-derived graph edges, causing the 40x-60x macOS slowdown.

The plan must verify or falsify each step before changing behavior.

A reviewer challenged the owned-vs-byte-backed confound and reported that
`YUNE_POET_BYTE_BACKED=1` did not rescue the macOS rows. M57 must reproduce
that isolation locally and should not assume byte-backed poet storage is a
workaround. The decisive experiment is to run the same Yune binary over
Windows-built and macOS-built table bytes, then diff `all_codes()`,
`table_entry_iter()` expansion, sentence-model owner counts, and the four
candidate snapshots.

Existing `upstream_luna_pinyin_parity` tests are not sufficient coverage for
this defect because they build the sentence model from source YAML. The failing
benchmark path builds from the `rime_deployer`-compiled table bytes, so M57
needs a real compiled-table regression fixture or generated real-path test.

## Proposed Fix

The intended fix is construction-first. Fix A is the primary repair. Fix B is a
conditional safety valve only if a correct model still permits bad
abbreviation-row preemption.

### Fix A: Make sentence-model construction platform-stable

Yune must not build the Luna upstream sentence model from a compact table
enumeration that can silently drop or reshape the model across platform-built
MARISA payloads.

Preferred repair:

1. Add an internal diagnostic/assertion path that records, for the selected
   `luna_pinyin` profile:
   - spelling-algebra formula count
   - whether any formula is abbreviation-marked
   - selected storage, checksum status, source-fallback status, table format,
     `stored_entries`, and `YUNE_POET_BYTE_BACKED` state
   - compact all-code count
   - `table_entry_iter` model-entry count
   - sentence-model `entries_by_code`, `lookup_index`, `vocabulary`, and
     `abbreviation_vocabulary` counts
   - whether `from_table_entries_with_abbreviation_vocabulary` was used
   - whether the M42 abbreviation model was built
2. Fix `CompactTableStore::all_codes` / MARISA traversal so full-code
   enumeration and `exact_candidates` reproduce the expected sentence-model
   entry set on both Windows-derived and macOS-derived table payloads.
3. Add a same-binary dual-table diagnostic, when Windows table bytes are
   available, that runs the macOS Yune binary over both the Windows-built and
   macOS-built `luna_pinyin.table.bin` payloads and diffs enumeration/model
   shape before any ranking code changes.
4. Add a real-path regression test that fails if macOS-style table bytes build
   a model with only `~191k` entries or full-size abbreviation vocabulary.
   The test must exercise the compiled-table construction path, not only the
   source-YAML model path used by existing `upstream_luna_pinyin_parity` tests.

Fallback repair if the compact MARISA traversal cannot be made reliable within
this milestone:

1. For upstream `luna_pinyin` sentence-model construction only, build the
   sentence model from the parsed source dictionary/import payload when source
   rows are available, while keeping compact table lookup for runtime exact and
   prefix lookup.
2. Record the startup and memory cost explicitly.
3. Close partial/no-go if the fallback violates M55 startup/session or memory
   guardrails.

### Conditional Fix B: Protect The M42 Abbreviation Route

The abbreviation rows are behavior rows, not just speed rows. If
single-letter abbreviation support is configured, an abbreviation-shaped input
must not be preempted by unrelated normal sentence candidates. However, the
current evidence says the abbreviation model is not wired correctly
(`abbreviation_span_discovery_calls = 0`,
`poet.abbreviation_vocabulary = 421,966`), so ranking/preemption changes are
not the first fix.

Do not implement this section unless Fix A restores the expected model shape
and the named abbreviation rows are still wrong.

Candidate repair:

1. Preserve the existing normal sentence path for full pinyin rows.
2. Before returning normal sentence candidates for an ASCII input that is
   coverable by abbreviation spans, verify the M42 abbreviation path either:
   - produces candidates that should rank before the normal sentence output, or
   - is explicitly empty.
3. For the named M42 rows, return the abbreviation candidate set first and keep
   the spaced preedit (`c s z y s m s r s d`, `z y b f sh m s ru`).
4. Add tests proving the normal sentence path cannot return the `少女時代` /
   `美少女` false positives ahead of the M42 abbreviation candidates.

This part should be reviewed carefully: the safest implementation may be to
fix the model construction first and add a narrow invariant test, rather than
changing ranking order broadly.

## Decided Calls

- **No ABI widening.** Do not change `RimeApi`, TypeDuck profile slots,
  `yune_web_*` exports, or public runtime contracts.
- **No schema behavior expansion.** The target is existing upstream
  `luna_pinyin` behavior for the named rows.
- **No browser claim.** Browser rows remain carried evidence unless a separate
  browser plan runs Playwright.
- **No new performance claim.** The closeout may say the macOS verification
  defect is repaired and classify claims honestly; it must not say Yune is
  generally faster than librime.
- **Oracle first.** Candidate expectations for the four suspicious rows come
  from local upstream librime `1.17.0`, not from Yune's current output.
- **Cross-platform evidence required.** At minimum, rerun the macOS native
  verification bundle. If Windows is not available in this milestone, state
  that the standing Windows M55 ratchet was not re-run.

## Win Bars

M57 closes complete when:

1. The macOS Track A Yune memory-owner model shape is no longer the outlier:
   `poet.abbreviation_vocabulary` is the 11-row M42 target set for owned
   default Luna, and the sentence entry/lookup-index counts are explained and
   stable.
2. The macOS Yune candidate snapshots for `cszysmsrsd` and `zybfshmsru` match
   the local librime oracle text/ranking for the first page.
3. The macOS long rows no longer show the graph explosion:
   `upstream_sentence_model_vocabulary_entries_considered` and
   `upstream_sentence_model_graph_edges` must return to the same order of
   magnitude as the Windows corrective evidence, or any remaining difference
   must be explained by a byte-identifiable oracle-data delta.
4. The four suspicious rows are no longer false contradictions of the M55
   claim shape:
   - 37-char and 59-char rows can remain bounded slower rows.
   - `cszysmsrsd` and `zybfshmsru` must return to abbreviation win-row shape
     or be explicitly reclassified with candidate-parity evidence.
5. `cargo fmt --check`, targeted `cargo clippy` for touched crates/benches, and
   focused Rust tests pass.
6. A fresh macOS evidence bundle is written under
   `docs/reports/evidence/m57-macos-track-a-sentence-model-parity/` with raw
   CSVs, candidate snapshots, memory-owner profiles, and a short verdict.
7. The closeout includes the byte-backed isolation result and, if Windows-built
   table bytes are available, the same-binary dual-table enumeration diff.

Close partial/no-go if:

- the macOS table payload is valid but intentionally different from Windows in
  a way that changes Yune's owned sentence model and cannot be fixed without a
  larger dictionary/storage redesign;
- abbreviation-first repair creates broader ranking regressions; or
- the only viable repair regresses M55 startup/session or Track A memory beyond
  the standing thresholds.

## Scope

In scope:

- diagnostics for sentence-model construction and benchmark evidence
- compact MARISA all-code/table-entry enumeration correctness
- M42 abbreviation-model activation and conditional preemption protection
- macOS verification script fixes needed to capture the right evidence
- focused tests for the four suspicious Track A rows
- recording whether the existing WEB-03 TypeDuck byte-backed long-input
  expansion guard can run on this Mac after local web assets are present
- docs/report updates that classify the macOS run honestly

Out of scope:

- replacing M55 as the standing Windows native gate
- byte-backed poet default flip
- `YUNE-POET/2` scratch port
- browser live-site or Playwright comparisons
- TypeDuck profile performance work or new TypeDuck speed claims
- ABI changes
- broad refactors of `schema_install.rs` or compact table storage

Adjacent caveat: `cargo test --workspace --no-fail-fast` on this checkout can
leave `cantonese_parity` and two `yune_web` tests red when the gitignored local
asset directories (`apps/yune-web/source/`,
`apps/yune-web/public-demo/dist/schema`) are missing. That is fixture setup,
not evidence that M57 affects TypeDuck. However, the existing
`web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion`
guard cannot prove the TypeDuck byte-backed product lane on macOS until those
assets exist.

## Phase 0: Reproduce And Freeze The Failure

- [ ] Record the current git head, local dirty status, macOS version, Rust
      version, and librime commit in
      `docs/reports/evidence/m57-macos-track-a-sentence-model-parity/phase-0/`.
- [ ] Copy or regenerate the macOS run-2 focused raw rows for:
      `ceshiyixiachangjushuruxingnengzenyang`,
      `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`,
      `cszysmsrsd`, and `zybfshmsru`.
- [ ] Add a focused diagnostic extractor that reports, per input:
      `translator_ns`, `upstream_sentence_model_ns`,
      `upstream_sentence_model_graph_rebuild_ns`,
      `upstream_sentence_model_incremental_extend_ns`,
      `upstream_sentence_model_vocabulary_entries_considered`,
      `upstream_sentence_model_graph_edges`,
      `abbreviation_span_discovery_calls`, and candidate texts.
- [ ] Run the same focused rows with `YUNE_POET_BYTE_BACKED=1` and capture
      candidate snapshots, owner counts, and product-path status. Treat this as
      a confound check, not a proposed workaround.
- [ ] Capture `selected_storage`, `checksum_status`, `source_fallback`,
      `table_format`, `stored_entries`, `rsmarisa_num_keys`, source checksum,
      and table checksum for each focused run.
- [ ] Compare Windows owned-default evidence (`run-c-owned-default`, plus
      `gate-run-d/e`) to macOS `run-1/run-2` in a checked-in Markdown note.
      Include the tables from this plan and keep byte-backed contrast rows
      separate.
- [ ] Confirm local macOS librime still emits the expected abbreviation
      candidates. If not, stop and fix the oracle bundle first.

## Phase 1: Instrument The Model Construction Path

- [ ] Add dev-only diagnostics to the native benchmark or a narrow test helper
      that records:
      - schema id and dictionary id
      - source/table/prism checksums
      - spelling algebra formula count and abbreviation-formula count
      - whether `single_letter_sentence_guard_enabled` is true
      - whether `YUNE_POET_BYTE_BACKED=1` is active
      - compact `all_codes()` count
      - `table_entry_iter()` count after expanding exact candidates
      - compiled-table `stored_entries`
      - sentence-model owner counts
      - whether `from_table_entries_with_abbreviation_vocabulary` was used
- [ ] Do not expose these diagnostics through public ABI.
- [ ] Add a test or bench-time assertion that `luna_pinyin` with abbrev
      formulas and non-empty M42 vocabulary cannot produce a full-size
      `poet.abbreviation_vocabulary`.
- [ ] Add a candidate snapshot assertion for the two M42 abbreviation rows
      against local librime-captured expected text/ranking.

## Phase 2: Repair Compact Enumeration Or Select A Safe Source

- [ ] Audit `CompactTableStore::all_codes`, MARISA traversal frames, and
      tail-code decoding against the macOS-generated table payload.
- [ ] Prove whether `all_codes()` misses codes, emits non-canonical code
      strings, or pairs codes with incomplete exact candidates.
- [ ] If a Windows-built `luna_pinyin.table.bin` is available, run the same
      macOS Yune diagnostic binary over both Windows-built and macOS-built table
      bytes. Diff `all_codes()` count, `table_entry_iter()` count,
      sentence-model owner counts, and candidate snapshots.
- [ ] Confirm whether the Windows and macOS tables have identical
      `stored_entries` and `byte_source_len` but different MARISA bytes; if so,
      the enumeration path must be robust to platform-serialized MARISA layout.
- [ ] Fix the compact enumeration path if the defect is local to traversal or
      exact-candidate pairing.
- [ ] Add a minimal regression fixture or real-path generated fixture that
      exercises the MARISA layout that failed on macOS.
- [ ] If compact enumeration cannot be fixed safely, implement the
      source-dictionary fallback only for upstream Luna sentence-model
      construction and measure startup/session/memory impact.
- [ ] Preserve compact table/prism lookup for normal runtime lookup.

## Phase 3: Repair Abbreviation Preemption

- [ ] Decide, with evidence, whether model-construction repair alone restores
      M42 rows.
- [ ] If, and only if, construction repair restores the expected model shape but
      the named abbreviation rows are still wrong, add a narrow
      abbreviation-preemption guard so abbreviation-shaped ASCII rows cannot
      return unrelated normal sentence candidates before M42 abbreviation
      candidates.
- [ ] Keep full-pinyin sentence rows on the normal sentence path unless a test
      proves the abbreviation guard is needed there.
- [ ] Add focused tests for:
      - `cszysmsrsd` candidate text/ranking
      - `zybfshmsru` candidate text/ranking
      - no regression on 37-char and 59-char full-pinyin rows
      - no regression on `zhongguo`, `ni`, and `hao`

## Phase 4: Re-Run macOS Verification

- [ ] Run:

```bash
cargo fmt --check
cargo build --release -p yune-rime-api
cargo clippy -p yune-rime-api --bench native_inprocess_benchmark -- -D warnings
```

- [ ] Run focused tests added by this milestone.
- [ ] If local web assets are available, run the existing TypeDuck/web guard:

```bash
cargo test -p yune-rime-api --test yune_web \
  web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion \
  -- --exact
```

      If the assets are absent, record it as not-run because of fixture setup.
      Do not infer whether the M57-class defect does or does not affect the
      byte-backed TypeDuck product lane from a setup panic.
- [ ] Run two full macOS native verification passes using the same input and
      iteration shape as the M55 corrective gate:

```bash
scripts/benchmark-native-rime-inprocess-macos.sh \
  --output-root docs/reports/evidence/m57-macos-track-a-sentence-model-parity/run-1 \
  --iterations 9 \
  --session-iterations 60 \
  --key-iterations 80

scripts/benchmark-native-rime-inprocess-macos.sh \
  --output-root docs/reports/evidence/m57-macos-track-a-sentence-model-parity/run-2 \
  --iterations 9 \
  --session-iterations 60 \
  --key-iterations 80
```

- [ ] Recompute `summary-comparison.csv` from raw CSVs.
- [ ] Write `macos-verdict.md` classifying each report claim as confirmed,
      platform-specific, contradicted, or not re-run.
- [ ] Explicitly state whether Windows M55 standing ratchet was re-run. If not,
      do not claim Windows guardrails passed.

## Phase 5: Documentation And Closeout

- [ ] Update the macOS verification report/evidence README with:
      - the original failing symptom
      - root cause
      - fix
      - before/after candidate snapshots
      - before/after graph counters
      - remaining platform-specific caveats
- [ ] Update `docs/reports/yune-vs-librime-performance.md` only if the public
      claim text needs qualification.
- [ ] Update `docs/reports/yune-vs-librime-root-cause-analysis.md` only if the
      owner story changes.
- [ ] Update `docs/roadmap.md`, `docs/requirements.md`, and
      `docs/ledgers/milestone-history.md` on closeout.
- [ ] Move this plan to `docs/plans/completed/` only when evidence is complete.

## Review Checklist For Claude

Ask the reviewer to challenge these points specifically:

- Is the model-shape mismatch (`513,353` vs `191,984` entries, `11` vs
  `421,966` abbreviation vocabulary) sufficient to classify the macOS run as
  non-comparable?
- Is `CompactTableStore::all_codes` the right first code owner, or should the
  investigation start in schema deployment/checksum generation?
- Does the byte-backed isolation check prove the defect is upstream of poet
  storage, or does it reveal a separate `poet.bin` consumption bug?
- Is the `table_entry_iter()`-versus-`stored_entries` invariant the right direct
  repro, and what distinct-code count should `all_codes()` have for this
  payload?
- Is the same-binary dual-table test strong enough to separate a macOS Yune
  binary issue from a macOS-built MARISA byte-layout issue?
- Is the proposed source-dictionary fallback acceptable as a temporary repair,
  or would it hide a compact-table correctness bug?
- Should abbreviation preemption stay out of the implementation unless correct
  model construction still leaves candidate parity broken?
- What tests are needed to prevent this exact bug from recurring on Linux or a
  future macOS/librime build?
- Does any proposed change risk widening the default upstream ABI or changing
  TypeDuck-profile behavior?

## Non-Goals

- Do not make byte-backed poet the default.
- Do not optimize unrelated short-key rows.
- Do not refactor all dictionary storage.
- Do not update public "Yune faster than librime" claims beyond the evidence.
- Do not use browser evidence to validate this native fix.
