# M12 Upstream Behavioral Parity Closeout Implementation Plan
> **Status:** Active - **Milestone:** M12 (Upstream Behavioral Parity Closeout) - **Updated:** 2026-06-19 - **Type:** execution plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the expanded M12 closeout by turning upstream librime 1.17.0 from a proven binary/header oracle into reproducible behavioral parity gates for the core `luna_pinyin` path. The result should make upstream 1.17.0 the default core oracle for behavior, not only ABI shape, and should prove at least one real dictionary-selection case rather than only ordering preselected oracle winners.

**Architecture:** Keep the official upstream release binary and schema repositories as external oracle inputs. Check in only capture scripts, provenance, JSON fixtures, tests, and docs. Capture expected behavior from the official upstream oracle first; then drive Yune's real parser, dictionary, translator, filter, and engine paths against those bytes. Translator-direct tests are allowed only for translator mechanics. Filter, paging, selection, commit, and option behavior must go through a Yune `Engine` or an equivalent full-pipeline harness. Keep TypeDuck compatibility profile-only and out of the upstream-core completion criteria.

**Tech Stack:** Rust workspace (`yune-core`, focused tests), PowerShell capture scripts, C# P/Invoke oracle probe, official upstream `rime/librime` 1.17.0 Windows MSVC x64 release, pinned upstream schema repositories, JSON fixtures, `cargo fmt`, `cargo clippy`, and workspace tests.

---

## Current State

M12's structural/oracle refresh work is already complete:

- Upstream librime 1.17.0 is the default core oracle in docs.
- TypeDuck librime v1.1.2 is profile-only.
- The C ABI table and provenance rules are documented and tested.
- Official upstream 1.17.0 release binaries have been verified locally:
  - `target/upstream-oracle/1.17.0/extract/dist/lib/rime.dll`
  - `target/upstream-oracle/1.17.0/extract/dist/bin/rime_deployer.exe`
  - `target/upstream-oracle/1.17.0/extract/dist/include/rime_api.h`
- The release header matches upstream source header content at pinned commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- First upstream behavioral fixture is checked in:
  - `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json`
- First active upstream parity test is checked in:
  - `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- Current active parity covers first-page ordering for curated upstream source rows for `ni`, `hao`, `zhong`, and `guo`.

That first active test is real and non-circular, but it is not yet a full dictionary-selection test. The fixture's source rows are selected from the candidates upstream already showed, so Yune is currently asked to order a curated slice rather than choose the same page-one candidates from all competing `luna_pinyin` rows for a code.

What remains is behavioral breadth and test power. The captured `zhongguo` phrase case exists, but it is fixture-locked only; it is not yet an active Yune parity assertion. Reverse lookup, punctuation, paging, selection, commit, schema option toggles, full-dictionary candidate selection, and full-pipeline comparison are not yet covered against upstream 1.17.0.

---

## Completion Criteria

M12 behavioral closeout is complete when all of these are true:

- Every behavior listed in the coverage table below has either:
  - a checked-in upstream 1.17.0 oracle fixture, and
  - an active Yune parity test that drives the real implementation path, or
  - an ignored test with an explicit blocker string and a `panic!()` body.
- Oracle fixtures are non-circular:
  - expected candidates, preedit, commit text, page state, and option effects come from upstream librime capture output;
  - fixture source rows may be extracted from pinned upstream schema data;
  - expected values are never generated from Yune output.
- At least one single-code fixture proves full dictionary selection:
  - start with `ni`;
  - extract every upstream `luna_pinyin.dict.yaml` row whose code is exactly `ni`;
  - extract relevant `essay.txt` vocabulary rows for every in-scope candidate
    character or term so ranking cannot fall back to default or zero essay
    weights;
  - feed that complete competing row set into Yune;
  - assert Yune selects and orders the same page-one candidates captured from upstream.
- Curated source-row fixtures are explicitly labeled as mechanics fixtures, not full behavior-selection fixtures.
- Any behavior affected by filters, menu state, paging, selection, commit, or schema options is compared through Yune's `Engine` or an equivalent full-pipeline harness. Raw `StaticTableTranslator::translate()` output is not sufficient for those behaviors.
- The `zhongguo` phrase case is not treated as complete sentence parity until it is backed by full-data selection evidence or represented by an ignored blocker test that names the missing sentence/lattice feature.
- Capture can be regenerated from:
  - official upstream librime release binary,
  - pinned schema repository commits,
  - checked-in scripts.
- Fixture provenance records:
  - upstream librime version and commit,
  - binary artifact name,
  - schema id,
  - schema repository commits,
  - capture command,
  - input/action sequence,
  - source-row policy, such as `curated_oracle_winners`,
    `all_rows_for_exact_code`, or
    `all_rows_for_exact_code_plus_relevant_essay_rows`,
  - generation timestamp or session date.
- Docs reflect the final state:
  - `docs/roadmap.md` marks the expanded M12 behavioral closeout as finished,
  - `docs/requirements.md` lists the behavioral requirements as complete or explicitly parked,
  - `docs/CONVENTIONS.md` records the fixture and test workflow,
  - this plan is moved to `docs/plans/archive/` after completion.
- Verification passes:
  - `cargo fmt`
  - `cargo test -p yune-core --test upstream_luna_pinyin_parity`
  - `cargo test -p yune-core --test oracle_fixture_provenance`
  - `cargo test -p yune-core`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`
  - `git diff --check`

---

## Scope

### In Scope

- Upstream `luna_pinyin` behavioral parity for the core engine.
- Official upstream librime 1.17.0 release binary as the behavior oracle.
- Pinned upstream schema data for:
  - `rime-luna-pinyin`
  - `rime-prelude`
  - `rime-essay`
  - `rime-stroke`
- Yune's real implementation paths:
  - schema parsing,
  - table dictionary loading,
  - static table translation,
  - full-dictionary selection from all matching source rows and relevant essay
    vocabulary rows for at least one code,
  - full engine/context execution for menu-dependent behavior,
  - reverse lookup translation,
  - punctuation translation,
  - simplifier/filter behavior,
  - paging, selection, and commit paths where those are already modeled by `yune-core`.
- Checked-in fixtures under `crates/yune-core/tests/fixtures/upstream-1.17.0/`.

### Out of Scope

- Reproducing every upstream librime plugin.
- TypeDuck-specific Cantonese or Windows compatibility behavior.
- Web runtime and TypeScript package behavior unless a core API change forces it.
- Lua, octagram, prediction, custom user dictionary learning, and deployer plugin behavior beyond what is required to compile upstream schema data for capture.
- Local rebuilding of librime as the canonical path; the official upstream release binary remains the behavioral oracle unless the release binary is proven unusable for a specific captured behavior.

---

## Coverage Table

| Behavior | Current State | Required Closeout State |
| --- | --- | --- |
| Curated single-code ordering | Active test for `ni`, `hao`, `zhong`, `guo` using source rows selected from upstream winners | Keep active and label as mechanics coverage |
| Full-dictionary single-code selection | Not covered | Add at least one case, starting with `ni`, using every upstream source row for the exact tested code and assert page-one selection plus order |
| Phrase/sentence candidate for `zhongguo` | Captured in fixture, not actively compared through Yune | Add active mechanics parity, but do not claim full sentence parity until backed by full-data evidence or an ignored blocker |
| Full-pipeline Yune comparison | Not covered; current active test drives `StaticTableTranslator` directly | Add reusable Engine/full-pipeline helper before filter, paging, selection, commit, and option tests |
| Reverse lookup | Not captured | Capture upstream stroke reverse lookup and compare Yune reverse lookup output |
| Punctuation and symbols | Not captured | Capture punctuation/symbol cases and compare Yune punctuation path |
| Paging | Not captured | Capture first page, next page, previous page snapshots and compare Yune page state/candidates |
| Selection and commit | Not captured | Capture numeric/space selection and compare committed text where Yune models commit |
| Schema option toggles | Not captured | Capture `zh_hans`, `ascii_punct`, and `full_shape` effects; activate tests where Yune supports them |
| Fixture provenance | Manifest exists and first fixture is provenance-rich | Enforce provenance checks across all upstream fixtures |
| Docs and requirements | Roadmap names first fixture as done and behavioral expansion as next | Mark expanded M12 closeout complete only after active tests or explicit parked blockers |

---

## Task 1 - Audit The Remaining Behavioral Surface

**Purpose:** Establish the exact baseline before expanding fixtures or tests.

**Files:**

- `docs/plans/m12-upstream-behavioral-parity-closeout.md`
- `scripts/capture-upstream-luna-pinyin.ps1`
- `scripts/oracle-rime-probe.cs`
- `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json`

**Steps:**

- [ ] Confirm the working tree is clean before starting implementation:

```powershell
git status --short --branch --untracked-files=all
```

- [ ] Confirm the current first fixture test still passes:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity
```

- [ ] Search for current implementation entry points:

```powershell
rg -n "zhongguo|reverse_lookup|punct|Page|page|select_candidate|commit|set_option|zh_hans|ascii_punct|full_shape|Simplifier|PunctuationTranslator|ReverseLookupTranslator" crates/yune-core scripts docs
```

- [ ] Record the implementation paths to use for each behavior:
  - phrase/sentence: `TableDictionary` plus `StaticTableTranslator`;
  - reverse lookup: existing reverse lookup translator path if present;
  - punctuation: existing punctuation translator path if present;
  - paging/selection/commit: existing engine or context page APIs;
  - option toggles: existing option state and filter/translator paths.
- [ ] Classify every planned test as one of:
  - `mechanics`: proves Yune ordering or module behavior from a curated fixture slice;
  - `selection`: proves Yune selects the same candidates from all competing source rows for the tested code;
  - `full_pipeline`: proves behavior after engine, filters, menu state, paging, selection, or commit handling.
- [ ] Do not let a `mechanics` test satisfy a `selection` or `full_pipeline` completion criterion.
- [ ] If a behavior has no usable implementation path, add an ignored test with:
  - a blocker string naming the missing owning module/API;
  - a `panic!()` body;
  - a fixture or capture case proving upstream behavior where capture is possible.

**Expected Result:** The coverage table in this plan stays accurate, and each remaining behavior has an implementation path or a named blocker test.

---

## Task 2 - Extend The Oracle Probe To Capture Action Sequences

**Purpose:** The current capture path is enough for final candidate snapshots after plain text input. Paging, selection, commit, and options require multi-step action capture.

**Files:**

- `scripts/oracle-rime-probe.cs`
- `scripts/capture-upstream-luna-pinyin.ps1`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/README.md`
- `crates/yune-core/tests/oracle_fixture_provenance.rs`

**Required Probe Capabilities:**

- [ ] Preserve the existing text-input capture behavior for `luna-pinyin-basic.json`.
- [ ] Add JSON scenario input support with ordered actions:

```json
{
  "schema": "luna_pinyin",
  "scenarios": [
    {
      "name": "paging_ni",
      "actions": [
        { "type": "input", "text": "ni" },
        { "type": "snapshot", "label": "page_1" },
        { "type": "key", "keycode": 65366, "mask": 0 },
        { "type": "snapshot", "label": "page_2" },
        { "type": "key", "keycode": 65365, "mask": 0 },
        { "type": "snapshot", "label": "page_1_again" }
      ]
    }
  ]
}
```

- [ ] Use `RimeProcessKey(session, keycode, mask)` for key actions.
- [ ] Use `RimeSetOption(session, option_name, value)` for option actions.
- [ ] Use `RimeClearComposition(session)` between scenarios.
- [ ] Capture snapshots after explicit `snapshot` actions and after commit-producing actions.
- [ ] Include these fields in each snapshot when upstream exposes them:
  - composition length,
  - cursor position,
  - preedit,
  - candidate page number,
  - page size,
  - highlighted candidate index,
  - candidate list text/comment pairs,
  - commit text when a commit happened.
- [ ] Keep action capture output deterministic by running each scenario in a fresh session or clearing composition and options before each scenario.
- [ ] Update fixture provenance tests so scenario fixtures must declare:
  - action list,
  - schema id,
  - oracle version/commit,
  - schema repository commits,
  - capture command.

**Keycode Notes:**

- `65366` is X11 `Page_Down`.
- `65365` is X11 `Page_Up`.
- Numeric key selection should use ASCII keycodes for `1` through `9`.
- Space commit should use ASCII `32`.
- Confirm these against Yune's key model before writing active Yune assertions.

**Expected Result:** The probe can produce multi-snapshot fixtures without disturbing the existing `luna-pinyin-basic.json` format or test.

---

## Task 3 - Add Full-Dictionary Selection And Full-Pipeline Test Foundations

**Purpose:** Close the two test-power gaps before expanding behavioral breadth: curated source rows only prove ordering, and translator-direct tests cannot prove filter or menu behavior.

**Files:**

- `scripts/capture-upstream-luna-pinyin.ps1`
- `scripts/oracle-rime-probe.cs`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-selection.json`
- `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- `crates/yune-core/tests/oracle_fixture_provenance.rs`
- `crates/yune-core/src/engine.rs`

**Full-Dictionary Selection Steps:**

- [ ] Add a fixture case named `single_code_ni_full_dictionary_selection`.
- [ ] Capture upstream's page-one candidates for input `ni` from the official upstream oracle.
- [ ] Extract source rows from pinned `rime-luna-pinyin/luna_pinyin.dict.yaml` by code, not by candidate text.
- [ ] Extract vocabulary rows from pinned `rime-essay/essay.txt` for every
  in-scope candidate character or term represented by the exact-code dictionary
  row set.
- [ ] The extraction rule is exact:

```javascript
const rowsForExactCode = (file, code) => readUtf8(file)
  .split(/\r?\n/)
  .filter(Boolean)
  .filter((line) => {
    const fields = line.split('\t');
    return fields.length >= 2 && fields[1] === code;
  });
```

- [ ] Store those rows under a fixture field named `source_dictionary_rows_all_for_code`.
- [ ] Store the essay rows under a fixture field named
  `essay_vocabulary_rows_for_candidates`.
- [ ] Record `"source_row_policy": "all_rows_for_exact_code_plus_relevant_essay_rows"` in fixture provenance.
- [ ] Record the exact tested code, source dictionary file, essay vocabulary
  file, dictionary source row count, essay source row count, and the
  in-scope candidate text set used for essay extraction.
- [ ] Add a provenance assertion that this fixture has more source rows than upstream's page size, so it cannot silently collapse back into a curated-winner slice.
- [ ] Add a provenance assertion that this fixture has non-empty essay rows
  and that every upstream page-one candidate text has either a matching essay
  row or an explicit fixture-local `essay_row_absent` explanation.
- [ ] Add an active Yune test that builds `TableDictionary` from all exact-code rows and asserts:
  - actual top page candidate text equals upstream captured page-one text;
  - actual top page length equals upstream captured page-one length;
  - rows outside the upstream first page are present in the source input but not selected into page one.
  - candidate ranking consumes the fixture essay weights rather than implicit
    default or zero weights.
- [ ] Keep the existing `luna-pinyin-basic.json` test, but label it as `"source_row_policy": "curated_oracle_winners"` and treat it as ordering/mechanics coverage.

**Full-Pipeline Harness Steps:**

- [ ] Add a shared test helper in `upstream_luna_pinyin_parity.rs` that can construct a Yune `Engine` from fixture-backed upstream source data.
- [ ] The helper must support:
  - adding a `StaticTableTranslator`;
  - adding filters required by the fixture scenario;
  - processing text input through `Engine::process_key_sequence` or `Engine::process_key_event`;
  - taking menu snapshots from `engine.context()`;
  - reading `last_commit` or returned commit text after selection.
- [ ] Use translator-direct assertions only for `mechanics` tests.
- [ ] Use the full-pipeline helper for:
  - paging,
  - numeric selection,
  - space commit,
  - simplifier behavior,
  - full-shape/ascii punctuation option behavior,
  - any test whose expected upstream output has passed through a filter.
- [ ] Add a guard comment near the translator-direct helper explaining that it must not be used to validate filter, paging, selection, commit, or option behavior.
- [ ] Run:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity full_dictionary
```

**Expected Result:** M12 now has one high-power selection test and a test harness boundary that prevents filter/menu behavior from being falsely validated at the raw translator stage.

---

## Task 4 - Activate Phrase/Sentence Parity For `zhongguo`

**Purpose:** The first fixture already captures the upstream phrase case, but Yune does not yet actively compare it. This is useful phrase mechanics coverage, but it must not be treated as full sentence/lattice parity unless backed by full-data source evidence.

**Files:**

- `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json`
- `scripts/capture-upstream-luna-pinyin.ps1`
- `crates/yune-core/src/dictionary/*`
- `crates/yune-core/src/translator/*`

**Steps:**

- [ ] Add a failing active mechanics test that reads the existing `zhongguo` case from `luna-pinyin-basic.json` and compares Yune's top translated candidate text to the oracle top candidate text.
- [ ] Do not hard-code the expected Chinese candidate in the Rust test; read it from the fixture.
- [ ] Build Yune input data only from source rows or upstream schema data recorded in the fixture.
- [ ] Label this test as phrase mechanics coverage if it uses curated source rows.
- [ ] If the current fixture does not contain enough source data to build the phrase path, extend the capture script to include the minimal upstream source rows and metadata needed by Yune:
  - dictionary name,
  - imports used,
  - relevant source rows for each syllable and phrase,
  - encoder settings if the phrase path depends on them.
- [ ] Drive the real dictionary and translator APIs. Do not add a phrase-only test helper that bypasses production parsing.
- [ ] Keep page-wide comparison narrow at first:
  - active assertion: top candidate text and comment for `zhongguo`;
  - fixture-lock assertion: full upstream candidate list remains in JSON;
  - ignored blocker test for full-page phrase parity if ordering diverges for a known missing Yune feature.
- [ ] Add an ignored blocker test for full sentence parity unless this task also introduces enough complete source data to prove phrase selection from the real competing dictionary/language-model surface.
- [ ] Run:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity phrase
```

**Expected Result:** `zhongguo` becomes an active upstream phrase mechanics case through Yune's real dictionary/translator path, and full sentence parity is either proven with full-data evidence or represented by an ignored blocker test naming the exact missing phrase/sentence feature.

---

## Task 5 - Add Reverse Lookup Parity

**Purpose:** Reverse lookup is part of real upstream `luna_pinyin` behavior through the `stroke` dictionary. It needs its own oracle fixture and active Yune parity test.

**Files:**

- `scripts/capture-upstream-luna-pinyin.ps1`
- `scripts/oracle-rime-probe.cs`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-reverse-lookup.json`
- `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- `crates/yune-core/src/translator/*`
- `crates/yune-core/src/schema/*`

**Steps:**

- [ ] Select reverse lookup inputs from the pinned upstream `rime-stroke` data, not from memory.
- [ ] Use a script step to locate stable source rows for common characters in `stroke.dict.yaml`, then derive the reverse lookup input by applying the upstream `luna_pinyin` reverse lookup prefix and suffix.
- [ ] Record the exact source rows used in the fixture provenance.
- [ ] Capture at least three reverse lookup scenarios:
  - one single-character result with a short stroke code;
  - one input with multiple candidates;
  - one no-result input that proves empty behavior.
- [ ] Add active Yune tests that compare:
  - candidate text,
  - candidate comment,
  - empty/no-candidate behavior.
- [ ] If Yune's reverse lookup path is incomplete, add ignored tests with the captured upstream fixture and blocker strings naming the missing API or module.
- [ ] Run:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity reverse
```

**Expected Result:** Reverse lookup behavior is captured from upstream and either actively matched or explicitly blocked with reproducible upstream evidence.

---

## Task 6 - Add Punctuation And Symbol Parity

**Purpose:** Punctuation and symbol expansion are user-visible `luna_pinyin` behavior and are already represented in upstream schema data.

**Files:**

- `scripts/capture-upstream-luna-pinyin.ps1`
- `scripts/oracle-rime-probe.cs`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-punctuation.json`
- `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- `crates/yune-core/src/translator/*`
- `crates/yune-core/src/schema/*`

**Steps:**

- [ ] Capture punctuation scenarios for:
  - ordinary punctuation key behavior,
  - symbol recognizer behavior from slash-prefixed symbol codes,
  - empty/no-match symbol behavior.
- [ ] Include the exact upstream punctuation/symbol source entries used by the Yune test fixture.
- [ ] Add active Yune tests through the real punctuation translator path.
- [ ] If an upstream punctuation scenario depends on recognizer state, menu state, or schema options, route the Yune side through the full-pipeline helper from Task 3 instead of calling the punctuation translator directly.
- [ ] Assert:
  - candidate text,
  - comments where upstream provides comments,
  - no-candidate behavior for no-match input.
- [ ] Keep full-shape/ascii punctuation toggles for Task 8 so base punctuation remains separable.
- [ ] Run:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity punctuation
```

**Expected Result:** Base punctuation and symbol behavior have upstream fixtures and active Yune parity coverage.

---

## Task 7 - Add Paging, Selection, And Commit Parity

**Purpose:** Candidate content parity is incomplete without page movement and selection behavior.

**Files:**

- `scripts/oracle-rime-probe.cs`
- `scripts/capture-upstream-luna-pinyin.ps1`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-actions.json`
- `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- `crates/yune-core/src/engine/*`
- `crates/yune-core/src/key.rs`
- `crates/yune-core/src/context.rs`

**Steps:**

- [ ] Capture action scenarios:
  - input `ni`, snapshot page 1;
  - page down, snapshot page 2;
  - page up, snapshot page 1 again;
  - numeric selection on page 1;
  - space commit for the highlighted candidate.
- [ ] Record keycodes and masks in fixture action provenance.
- [ ] Add active Yune tests that use the full-pipeline helper from Task 3 and the real engine/context key path.
- [ ] Compare:
  - visible candidate page text/comment pairs,
  - selected/highlighted index,
  - committed text after selection/space.
- [ ] If Yune currently exposes translator paging but not full key-driven commit, split tests:
  - active translator/context page assertions;
  - ignored engine commit assertions with captured upstream commit text.
- [ ] Run:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity paging
cargo test -p yune-core --test upstream_luna_pinyin_parity commit
```

**Expected Result:** Page movement and commit behavior are either active parity tests or explicitly blocked with upstream action fixtures.

---

## Task 8 - Add Schema Option Toggle Parity

**Purpose:** Upstream `luna_pinyin` behavior changes under schema options. The closeout should prove the core toggles Yune claims to support.

**Files:**

- `scripts/oracle-rime-probe.cs`
- `scripts/capture-upstream-luna-pinyin.ps1`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-options.json`
- `crates/yune-core/tests/upstream_luna_pinyin_parity.rs`
- `crates/yune-core/src/filter/*`
- `crates/yune-core/src/translator/*`
- `crates/yune-core/src/schema/*`

**Scenarios:**

- [ ] `zh_hans` disabled and enabled for a phrase input already present in upstream capture.
- [ ] `ascii_punct` disabled and enabled for punctuation behavior.
- [ ] `full_shape` disabled and enabled for punctuation behavior if Yune has an owning implementation path.

**Steps:**

- [ ] Capture each option scenario with explicit `set_option` actions in the fixture.
- [ ] Reset option state between scenarios.
- [ ] Add active Yune tests for option paths that are implemented.
- [ ] These tests must use the full-pipeline helper from Task 3; translator-direct output cannot prove simplifier, ascii punctuation, or full-shape behavior.
- [ ] Compare before/after candidate or commit text from the fixture.
- [ ] For an option Yune does not implement yet, keep the fixture and add an ignored blocker test naming the missing module.
- [ ] Run:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity option
```

**Expected Result:** Supported schema option toggles are tested against upstream, and unsupported toggles are visible as explicit blocked tests rather than undocumented gaps.

---

## Task 9 - Strengthen Fixture Provenance Tests

**Purpose:** As fixture count grows, provenance has to be enforced automatically.

**Files:**

- `crates/yune-core/tests/oracle_fixture_provenance.rs`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/*.json`
- `crates/yune-core/tests/fixtures/upstream-1.17.0/README.md`

**Steps:**

- [ ] Extend provenance tests to scan every JSON fixture under `crates/yune-core/tests/fixtures/upstream-1.17.0/`.
- [ ] Require every fixture to declare:
  - oracle name,
  - oracle version,
  - oracle commit,
  - binary artifact name,
  - schema id,
  - schema repository commits,
  - capture command,
  - input or action sequence,
  - source-row policy.
- [ ] Reject fixture paths or provenance fields that contain local absolute `target/` extraction paths.
- [ ] Reject fixtures that omit source rows when an active Yune parity test depends on source data from that fixture.
- [ ] Reject any fixture labeled `all_rows_for_exact_code` unless it records:
  - the exact tested code,
  - the exact source dictionary file,
  - a source row count greater than the captured upstream page size.
- [ ] Reject any fixture labeled
  `all_rows_for_exact_code_plus_relevant_essay_rows` unless it records:
  - the exact tested code,
  - the exact source dictionary file,
  - the exact essay vocabulary file,
  - the in-scope candidate text set,
  - dictionary source row count greater than the captured upstream page size,
  - non-empty essay source rows or per-candidate `essay_row_absent`
    explanations.
- [ ] Reject any test metadata that uses a `curated_oracle_winners` fixture to satisfy a `selection` completion criterion.
- [ ] Preserve the existing manifest checks.
- [ ] Run:

```powershell
cargo test -p yune-core --test oracle_fixture_provenance
```

**Expected Result:** Upstream fixture hygiene is enforced by tests, not reviewer memory.

---

## Task 10 - Update Canonical Docs And Requirements

**Purpose:** Once behavior parity gates exist, canonical docs must state the new baseline accurately.

**Files:**

- `docs/roadmap.md`
- `docs/requirements.md`
- `docs/CONVENTIONS.md`
- `docs/decisions.md`
- `docs/plans/m12-upstream-behavioral-parity-closeout.md`

**Steps:**

- [ ] Add or update requirement rows for upstream behavioral parity:
  - single-code candidates,
  - full-dictionary single-code selection,
  - phrase/sentence candidate,
  - full-pipeline comparison harness,
  - reverse lookup,
  - punctuation/symbols,
  - paging/selection/commit,
  - option toggles,
  - fixture provenance.
- [ ] Mark only actually passing active tests as complete.
- [ ] Mark intentionally deferred behavior as parked with the owning ignored test name and blocker.
- [ ] Update roadmap current baseline to say M12 behavioral closeout is complete only after this plan's tests pass.
- [ ] Update conventions with:
  - how to regenerate upstream fixtures,
  - where official release binaries are expected locally,
  - which files are intentionally checked in,
  - which files must remain ignored.
- [ ] Add a decision log entry if the team decides that official release binaries, not local source builds, are the canonical behavior oracle.

**Expected Result:** The docs distinguish completed parity, captured-but-blocked parity, and out-of-scope TypeDuck behavior.

---

## Task 11 - Final Verification, Commit, Push, And Archive

**Purpose:** Close M12 from a clean, reviewable state.

**Steps:**

- [ ] Regenerate fixtures from the official upstream release binary and pinned schema repositories:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-luna-pinyin.ps1
```

- [ ] Run formatting:

```powershell
cargo fmt
```

- [ ] Run focused upstream parity tests:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test oracle_fixture_provenance
```

- [ ] Run core tests:

```powershell
cargo test -p yune-core
```

- [ ] Run workspace quality gates:

```powershell
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
```

- [ ] If TypeScript or web-facing files were changed, also run:

```powershell
npm --prefix packages/yune-typeduck-runtime test
npm --prefix packages/yune-typeduck-runtime run build
```

- [ ] Review changed files explicitly:

```powershell
git status --short --branch --untracked-files=all
git diff --stat
git diff -- docs crates scripts
```

- [ ] Stage only intended files:

```powershell
git add -- docs crates scripts
```

- [ ] Commit with a message that names behavioral parity:

```powershell
git commit -m "test: expand upstream behavioral parity"
```

- [ ] Push to `origin/main` after confirming the branch is still `main`:

```powershell
git branch --show-current
git push origin main
```

- [ ] Move this plan to `docs/plans/archive/` only after the completion criteria pass:

```powershell
git mv docs/plans/m12-upstream-behavioral-parity-closeout.md docs/plans/archive/m12-upstream-behavioral-parity-closeout.md
```

- [ ] Commit and push the archive move:

```powershell
git commit -m "docs: archive M12 upstream behavioral parity plan"
git push origin main
```

**Expected Result:** M12's expanded behavioral closeout is verified, published, and archived. Any remaining unsupported upstream behavior is represented by ignored blocker tests with upstream fixtures, not by undocumented gaps.

---

## Review Checklist For Claude

- [ ] The plan does not treat local librime source builds as the default behavior oracle.
- [ ] The plan keeps official upstream 1.17.0 release binaries out of git.
- [ ] The plan prevents circular tests by separating upstream capture from Yune assertions.
- [ ] The plan requires at least one full-dictionary selection case, not only curated-winner ordering.
- [ ] The plan requires Engine/full-pipeline comparison for filter, paging, selection, commit, and option behavior.
- [ ] The plan distinguishes active parity, fixture-locked evidence, and blocked behavior.
- [ ] The plan keeps TypeDuck behavior out of upstream M12 completion criteria.
- [ ] The plan has enough detail for an implementation agent to execute without inventing scope.
- [ ] The verification gate is strong enough for closing M12.
