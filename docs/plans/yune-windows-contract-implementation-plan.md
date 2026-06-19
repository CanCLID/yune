# Yune → TypeDuck-Windows: Concrete Implementation Plan

> **Status:** Parked · **Milestone:** M10 (TypeDuck-Windows native backend; deferred behind M9) · **Updated:** 2026-06-17 · **Type:** execution plan

> **Audience.** An autonomous coding agent (e.g. GPT) executing directly in the
> `yune` repo. Every work item is independently committable, names exact files,
> and ends with copy-pasteable verification commands.
>
> **Goal.** Make Yune satisfy the four-item *graduation contract* in
> [`typeduck-windows-backend-requirements.md`](../typeduck-windows-backend-requirements.md)
> so the parked `TypeDuck-Windows` (weasel fork) can swap `librime → Yune` behind
> the RIME C ABI.
>
> **Line anchors** in this doc are accurate as of 2026-06-17 but *will drift* —
> re-`grep` the named symbol before editing. Trust symbol names over line numbers.

---

## 0. What was verified before writing this plan

Five contested claims were checked against the code by paired investigate+skeptic
agents. Results that shaped the ordering below:

| Claim | Verdict | Consequence for the plan |
|---|---|---|
| `RimeCandidate.comment` transport already exists in the C ABI | **Confirmed present** (`abi.rs:54`, populated `context_api.rs:167-187` & `candidate_api.rs:104-127`, E2E-tested `frontend_client.rs:3260-3267`) | Item #2 is **not** missing plumbing. Do **not** rebuild transport. |
| TS runtime parses `candidate.comment`; `TypeDuckContext` lacks context-level `comments`/`highlighted_candidate_index` | **Confirmed; web adapter resolved in M9** | Web-only adapter key-name mismatch; no Windows C ABI work required. |
| `config_list_append_string` (+ siblings) absent from RimeApi table | **Confirmed absent** (`abi.rs:324-336`, `api_table.rs:124-134`, `config_api.rs`) | Cleanest first feature slice → **Item 2**. |
| Real item-#2 gap is fork comment **semantics** (`"; "` join, reverse-code+comment, schema-in-prompt) | **Confirmed** (`filter/mod.rs:541` & `translator/mod.rs:~585` join with `" "`; no schema-in-prompt anywhere) | Needs a v1.1.2 **oracle** before coding → **Items 3 + 4**. |
| Windows test fragility around timestamps/line-endings | **Timestamps: confirmed & baseline-breaking. Line-endings: not substantiated.** | **Item 1 is a hard blocker — fix first.** |

### The blocker, precisely

`librime_signature_modified_time()` ([`crates/yune-rime-api/src/lib.rs:1833-1856`](../../crates/yune-rime-api/src/lib.rs)) returns a
ctime(3)-shaped string on Unix but a **bare Unix-seconds integer on Windows**.
`assert_librime_ctime_shape()` ([`tests/mod.rs:317-332`](../../crates/yune-rime-api/src/tests/mod.rs)) unconditionally asserts
`parts.len() == 5`. On Windows it panics `left: 1, right: 5`. The panic fires while
holding the shared `TEST_LOCK` in `test_guard()` ([`tests/mod.rs:98-103`](../../crates/yune-rime-api/src/tests/mod.rs), `.expect("test lock should not be poisoned")`),
so the mutex **poisons** and ~230 downstream tests fail with `PoisonError`.

Reproduce:
```sh
cargo test -p yune-rime-api --lib -- --test-threads=1
# Today on Windows: 233 failed; 36 passed. Root failure pinned at tests/mod.rs:319.
```

---

## Execution order (and why)

```
0. Housekeeping & planning reconciliation      (no code risk; do immediately)
1. Fix the Windows test baseline               ← BLOCKER: nothing is trustworthy until green
2. config_list_append_string (+ siblings)      ← Contract #1; cleanest, oracle-free slice
3. Establish the v1.1.2 oracle                 ← shared prerequisite for #4 and #6
4. Fork-compatible candidate comment semantics ← Contract #2; driven by #3 goldens
5. Native Windows engine artifact              ← Contract #4; build/packaging
6. Cantonese/Jyutping parity regression suite  ← Contract #3; driven by #3 oracle
```

Rationale: you cannot trust TDD on Windows while 233 tests are red (Item 1).
Item 2 is self-contained and needs no external oracle, so it is the first *feature*.
Items 4 and 6 both depend on golden outputs from the real fork binary, so that
oracle work (Item 3) is pulled forward and shared. Item 5 is independent build work.

### Cross-cutting rules (apply to every item)
- **Module/test ownership (QUAL-01/02):** each slice gets an owning impl module +
  owning test module; keep `lib.rs`/`main.rs` as facades.
- **Quality gate per slice:** `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`,
  the slice's focused tests, then `cargo test --workspace` when shared behavior changed.
- **One commit per work item** (or per sub-task where noted).
- **Push every completed step:** after each commit, push the feature branch so the
  remote records the same step boundary as the local history.
- **Update tracking** at the end of each item: tick the box in
  `docs/typeduck-windows-backend-requirements.md` §"Status checklist" and the
  relevant row in [`../requirements.md`](../requirements.md) / [`../roadmap.md`](../roadmap.md).

---

## Work Item 1 — Fix the Windows test baseline  *(BLOCKER — do first)*

**Why first:** every later item is verified with `cargo test`. The baseline is
red on Windows for a reason unrelated to those items, so fix it before building
anything on top.

### 1a. Make the signature timestamp consistent across platforms *(recommended)*

The Unix arm emits the librime ctime(3) shape; the Windows/emscripten arm emits a
bare integer. librime itself produces a ctime-style string on all platforms, so
**make the Windows arm match the shape** rather than hiding the divergence in the test.

Replace the non-Unix arm of `librime_signature_modified_time()` in
[`crates/yune-rime-api/src/lib.rs:1848-1856`](../../crates/yune-rime-api/src/lib.rs) with a dependency-free ctime-shaped
formatter (illustrative — verify edge cases):

```rust
#[cfg(any(not(unix), target_os = "emscripten"))]
pub(crate) fn librime_signature_modified_time() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_ctime_utc(secs)
}

// "Www Mmm D HH:MM:SS YYYY" — day NOT zero-padded so split_whitespace() yields
// exactly 5 tokens, matching assert_librime_ctime_shape().
#[cfg(any(not(unix), target_os = "emscripten"))]
fn format_ctime_utc(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let sod = epoch_secs.rem_euclid(86_400);
    let (hh, mm, ss) = (sod / 3600, (sod % 3600) / 60, sod % 60);
    let dow = ((days.rem_euclid(7)) + 4) % 7;          // 1970-01-01 = Thursday(4)
    let (y, m, d) = civil_from_days(days);              // Howard Hinnant's algorithm
    const WD: [&str; 7] = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"];
    const MO: [&str; 12] = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
    format!("{} {} {} {:02}:{:02}:{:02} {}",
            WD[dow as usize], MO[(m - 1) as usize], d, hh, mm, ss, y)
}

#[cfg(any(not(unix), target_os = "emscripten"))]
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
```

> **Known limitation to note in the commit message:** Unix uses *local* time (via
> `ctime_r`); this Windows formatter uses UTC. Only the *shape* is contractually
> asserted and no consumer parses the value, so this is acceptable. If a consumer
> later needs exact local time, swap in the Win32 `GetLocalTime`/`localtime_s` path.

**Fallback (lower effort, if the formatter is judged risky):** keep the integer
format and make the assertion platform-aware — add a `#[cfg(...)]` branch in
`assert_librime_ctime_shape` (`tests/mod.rs:317`) that asserts a single
integer-seconds token on non-Unix. This unblocks the baseline but enshrines a
real cross-platform ABI divergence, so prefer 1a.

### 1b. Stop one assertion from poisoning the whole suite *(do regardless of 1a)*

In `test_guard()` ([`tests/mod.rs:100-103`](../../crates/yune-rime-api/src/tests/mod.rs)) replace the poison-panicking lock with
poison-tolerant recovery, so a single failing test no longer masks ~230 others:

```rust
let guard = TEST_LOCK
    .get_or_init(|| Mutex::new(()))
    .lock()
    .unwrap_or_else(std::sync::PoisonError::into_inner);
```

Apply the same `unwrap_or_else(PoisonError::into_inner)` pattern to other shared
`static ... Mutex` locks used by tests if they `.expect("...not be poisoned")`
(e.g. `notification_events()`), so the suite degrades to *one* failure, not a cascade.
Keep this recovery test-only; do not change production runtime mutex behavior as
part of this baseline fix.

### Acceptance (Item 1)
```sh
cargo test -p yune-rime-api --lib -- --test-threads=1   # 0 failed
cargo test --workspace                                  # green on Windows
cargo clippy --workspace --all-targets -- -D warnings
```
Add a focused unit test asserting `librime_signature_modified_time()` satisfies
`assert_librime_ctime_shape` on the host platform (so the regression can't return).

---

## Work Item 2 — `config_list_append_string` (+ bool/int/double)  *(Contract #1)*

**Contract:** the TypeDuck-Windows deployer calls `config_list_append_string` at
7 sites in `WeaselDeployer/TypeDuckSettings.cpp`, **via the `RimeApi` function
table** (struct-pointer style, *not* a flat exported symbol). Siblings
`config_list_append_{bool,int,double}` are declared for symmetry. No upstream
substitute exists; the YAML `__append`/`/+` patch syntax in `config_compiler.rs`
is **not** an equivalent and must not be confused with this C API.

**Semantics to implement** (confirm against the librime fork if any doubt): append
a scalar value to the sequence at `key`; if no list exists at `key`, create one
and append (mirrors librime's `Config::AppendToList` "create-on-missing" behavior).
Adjacent primitives already exist to build on: `RimeConfigCreateList`
([`config_api.rs:504`](../../crates/yune-rime-api/src/config_api.rs)), `RimeConfigListSize` ([`config_api.rs:524`](../../crates/yune-rime-api/src/config_api.rs)),
`RimeConfigSetString`/`SetItem`, and the internal `config_lookup` / `config_set` /
`find_config_value` / `set_config_value` helpers.

### Steps
1. **Implement four `extern "C"` functions** in
   [`crates/yune-rime-api/src/config_api.rs`](../../crates/yune-rime-api/src/config_api.rs) next to the other config writers
   (after `RimeConfigSetString`, ~line 417). Each: validate non-null pointers,
   resolve the key, load-or-create the `Value::Sequence` at the key, push the new
   scalar using the same string-backed representation as the existing
   `RimeConfigSet*` writers, write it back, return `TRUE`/`FALSE`. Mark
   `#[no_mangle] pub unsafe extern "C"`. Sketch:

   ```rust
   /// Appends a string to the list at `key`, creating the list if absent.
   /// # Safety
   /// `config`, `key`, and `value` must be valid pointers.
   #[no_mangle]
   pub unsafe extern "C" fn RimeConfigListAppendString(
       config: *mut RimeConfig, key: *const c_char, value: *const c_char,
   ) -> Bool {
       if value.is_null() { return FALSE; }
       let value = unsafe { CStr::from_ptr(value) }.to_string_lossy().into_owned();
       unsafe { config_list_append(config, key, Value::String(value)) }
   }
   // ...Bool/Int/Double variants delegating to the same helper...
   ```
   Add one private helper `unsafe fn config_list_append(config, key, item: Value) -> Bool`
   that does the load-or-create-sequence-and-push, so the four entry points stay thin.

2. **Add struct fields** to the `RimeApi` table in
   [`crates/yune-rime-api/src/abi.rs`](../../crates/yune-rime-api/src/abi.rs), immediately after `config_list_size`
   and before `config_begin_list`. Define fn-pointer type aliases consistent with the existing
   `ConfigSet*Fn` aliases, e.g.:
   ```rust
   pub config_list_append_bool:   Option<ConfigListAppendBoolFn>,
   pub config_list_append_int:    Option<ConfigListAppendIntFn>,
   pub config_list_append_double: Option<ConfigListAppendDoubleFn>,
   pub config_list_append_string: Option<ConfigListAppendStringFn>,
   ```
   > **ABI ordering caveat:** append the new fields *after* the existing ones, at
   > the position librime's real `RimeApi` places them (check the fork's
   > `rime_api.h` field order). Field order *is* the ABI for struct-pointer access.

3. **Wire the table** in [`crates/yune-rime-api/src/api_table.rs`](../../crates/yune-rime-api/src/api_table.rs) after
   `config_list_size: Some(RimeConfigListSize),`:
   ```rust
   config_list_append_bool:   Some(RimeConfigListAppendBool),
   config_list_append_int:    Some(RimeConfigListAppendInt),
   config_list_append_double: Some(RimeConfigListAppendDouble),
   config_list_append_string: Some(RimeConfigListAppendString),
   ```

4. **Tests** — new test module section in
   [`crates/yune-rime-api/src/tests/config_api.rs`](../../crates/yune-rime-api/src/tests/config_api.rs):
   - append to a **missing** key creates a list of size 1;
   - append twice → `RimeConfigListSize == 2`, values readable in order;
   - each of the 4 type variants round-trips;
   - null `config`/`key`/`value` → `FALSE`, no panic;
   - a **deployer-shaped** test: build a display-language list + a few toggle
     entries the way `TypeDuckSettings.cpp` does, then read them back.
   - call through `rime_get_api()`’s table (not the raw fn) in at least one test,
     to prove the field is wired.

### Acceptance (Item 2)
```sh
cargo test -p yune-rime-api config_list_append
cargo clippy --workspace --all-targets -- -D warnings
```
Grep proves wiring on all three layers:
```sh
grep -n "config_list_append" crates/yune-rime-api/src/abi.rs \
  crates/yune-rime-api/src/api_table.rs crates/yune-rime-api/src/config_api.rs
```

---

## Work Item 3 — Establish the v1.1.2 comment/behavior oracle  *(prerequisite for 4 & 6)*

Items 4 and 6 require **byte-level parity** with the librime fork
`TypeDuck-HK/librime @ v1.1.2` + the pinned TypeDuck schema. You cannot implement
"join pronunciations with `; `" correctly by guessing the exact wrapping/spacing —
capture goldens from the real binary first.

### Steps
1. Acquire the **v1.1.2** fork artifact + pinned TypeDuck schema (the Windows CI
   pulls `rime-TypeDuck-{x86,x64}` release archives via `github.install.bat`,
   keyed on the release tag = `git describe`). Record the exact source + revision,
   matching the evidence discipline used in
   [`typeduck-web-integration-findings.md`](./archive/typeduck-web-integration-findings.md).
2. Run a fixed input transcript through the fork and capture, per candidate:
   `text`, `comment`, the menu `highlighted_candidate_index`, and the prompt
   string — for cases that exercise: single reverse-lookup pronunciation,
   **multiple** reverse-lookup pronunciations (the `"; "` case), reverse-code +
   original-comment co-display, and schema-name-in-prompt.
3. Store as deterministic fixtures under a new owned location, e.g.
   `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` (goldens) +
   a small README documenting capture method and revision.
4. **If the fork binary/schema cannot be obtained locally**, document a
   reproducible blocker (exactly as Phase 7/10 did for Emscripten) and stop here;
   Items 4 and 6 are then *blocked*, not *failed*. Do not fabricate goldens.

### Acceptance (Item 3)
Goldens committed (or a documented, reproducible blocker). No production code change.

---

## Work Item 4 — Fork-compatible candidate comment semantics  *(Contract #2)*

**Do not touch transport** — `RimeCandidate.comment` already crosses the ABI and
is tested. The gap is *what the comment string contains*. Drive every change from
the Item 3 goldens; the separators/wrapping below are the *current* Yune behavior,
not assumptions about the fork.

### Verified target from Item 3
The v1.1.2 fixture showed that TypeDuck-Windows' dictionary panel is powered by
the fork module `dictionary_lookup_filter`, not by a plain context-level comment
array. Its `RimeCandidate.comment` payload is a raw dictionary-panel record:

- comment starts with form-feed `\f`;
- each record starts with carriage-return + primary marker: `\r1,` for the
  candidate's own pronunciation and `\r0,` for alternate pronunciations;
- the rest of each record is the source dictionary row fields joined with commas.

The fixture also preserves `schema_id`, `schema_name`, and
`highlighted_candidate_index` through existing status/menu fields. No separate
schema prompt byte string was captured for the Windows C ABI, so this item should
not invent one.

### Implemented behavior
1. Preserve raw source dictionary row fields as `DictionaryLookupRecord`s on
   `TableDictionary`.
2. Add `DictionaryLookupFilter`, which emits the TypeDuck `\f\r1,...\r0,...`
   comment payload for table/completion/sentence candidates.
3. Wire `engine/filters: - dictionary_lookup_filter` in `schema_install.rs`,
   loading its dictionary from source YAML so the raw row columns are available
   even when normal translators keep preferring compiled table/prism/reverse data.
4. Change normal `reverse_lookup_filter` and `reverse_lookup_translator`
   multi-pronunciation joins from `" "` to the fork-compatible `"; "`.
5. Cover the behavior with core filter/translator tests and an ABI-level
   schema-selection test that reads `RimeCandidate.comment`.

### Acceptance (Item 4)
```sh
cargo test -p yune-core dictionary_lookup_filter
cargo test -p yune-core reverse_lookup_translator_uses_target_dictionary_comments
cargo test -p yune-rime-api select_schema_loads_typeduck_dictionary_lookup_filter
cargo test --workspace
```

---

## Work Item 5 — Native (non-WASM) Windows engine artifact  *(Contract #4)*

**Contract:** weasel's MSBuild release path consumes `rime.dll` + `rime.lib` +
`dist/include/rime_*.h`, today shipped as `rime-TypeDuck-{x86,x64}` archives.
Must expose the deployment / levers / config-compile (`__include`/`__patch`/
list-append) APIs the deployer drives — including Item 2's new append functions.

### Steps
1. Confirm `crates/yune-rime-api/Cargo.toml` declares `crate-type = ["cdylib"]`
   (and `staticlib` if a static variant is wanted). On MSVC, a `cdylib` build
   produces both the `.dll` and an import `.lib`.
2. Produce a documented build command for `x86_64-pc-windows-msvc` (and `i686-` if
   x86 is still required), e.g.:
   ```sh
   cargo build -p yune-rime-api --release --target x86_64-pc-windows-msvc
   # -> target/x86_64-pc-windows-msvc/release/yune_rime_api.dll (+ .dll.lib)
   ```
3. Define the rename/packaging step to `rime.dll` / `rime.lib` and assemble a
   `dist/include/` header set compatible with what weasel includes
   (`rime_api.h`, `rime_levers_api.h`). Decide: hand-maintain a Yune header that
   matches the `RimeApi` struct field order, or vendor the fork's headers and
   verify field-order parity (tie this back to the Item 2 ABI-ordering caveat).
4. Document required linker/export flags, the artifact layout, and the
   release-tag (`git describe`) keying that `github.install.bat` relies on, in a
   new `docs/plans/yune-windows-native-build.md`.
5. If the MSVC toolchain isn't available in this environment, record a
   reproducible blocker (Phase 7 pattern) and keep the native adapter contract
   tests as the fallback validation path.

### Implemented result

`scripts/package-typeduck-windows.ps1` builds `yune-rime-api` for
`x86_64-pc-windows-msvc`, copies the Cargo output into the TypeDuck/weasel
layout as `dist/lib/rime.dll` and `dist/lib/rime.lib`, copies
`rime_api.h`/`rime_levers_api.h` from the v1.1.2 oracle headers, and smoke-checks
that the packaged DLL exports `rime_get_api` with a non-null
`config_list_append_string` table slot. The reproducible steps are documented in
[`yune-windows-native-build.md`](./yune-windows-native-build.md).

### Acceptance (Item 5)
A documented, reproducible build producing `rime.dll`/`.lib`/headers (or a
documented blocker). A smoke check that `rime_get_api()` from the built DLL
returns a table whose `config_list_append_string` slot is non-null.

---

## Work Item 6 — Cantonese/Jyutping parity regression suite  *(Contract #3)*

Snapshot goldens from the **v1.1.2** binary + pinned schema (Item 3) and assert
Yune parity for the genuinely fork-only behaviors:
- options `combine_candidates`, `show_full_code`, `enable_sentence` (disable toggle);
- completion + prediction (freq-threshold tuned) and the **completion-enable
  option** — ⚠️ the fork uses `enable_completion` while upstream librime renamed
  it `enable_word_completion`. **Pick one name and keep the TypeDuck schema YAML +
  the deployer's `DISABLE_COMPLETION_VALUE` patch consistent**, or the toggle
  silently no-ops. Add a test that fails if the option name drifts.
- correction (minimal-distance, monosyllabic, `m`-abbreviation penalty);
- reverse-lookup pronunciation formatting (overlaps Item 4 — reuse those goldens);
- schema-menu hiding (`hide lone schema`, `hide caret`);
- per-entry user-dictionary pronunciations.

### Steps
1. Build the suite as an owned test module/crate (e.g.
   `crates/yune-core/tests/cantonese_parity.rs`) reading the Item 3 fixtures.
2. One assertion group per behavior above; mark any behavior whose golden could
   not be captured as an explicit `#[ignore]` with a documented reason (no silent
   gaps).

### Implemented result

`crates/yune-core/tests/cantonese_parity.rs` reads the v1.1.2 fixture, locks the
captured `jyut6ping3_mobile` schema/menu/comment shape, and replays sampled
TypeDuck dictionary-panel comment payloads through Yune's `DictionaryLookupFilter`.
The behaviors not captured by `jyut6ping3-mobile-comments.json` are present as
explicit ignored tests with the missing oracle called out in the ignore reason:
option toggles, completion/prediction, correction penalties, schema-menu hiding,
and per-entry userdb pronunciations.

### Acceptance (Item 6)
```sh
cargo test -p yune-core --test cantonese_parity   # all green (or documented ignores)
```
Then revisit `TypeDuck-Windows/INTEGRATION_PLAN.md`: with #1–#4 met and E2E green,
the `librime → Yune` swap behind the RIME C ABI is a contained change.

---

## Work Item 0 — Housekeeping & planning reconciliation  *(do immediately; no code risk)*

1. **Commit the untracked baseline** so the new direction is recorded:
   `.gitattributes`, `.editorconfig`, and
   `docs/typeduck-windows-backend-requirements.md` (plus this plan).
   `.gitattributes` should define the future repository line-ending policy
   (`* text=auto eol=lf`). Do **not** run `git add --renormalize .` in this
   housekeeping commit: the current checkout has many CRLF working-tree files,
   and staging them would create the large meaningless diff this plan is meant
   to avoid. If a full tracked-file renormalization is ever needed, make it a
   separate audited PR/commit after previewing the exact file list.
2. **Tracking lives in `docs/`** — the GSD `.planning/` system has been retired.
   Update [`../roadmap.md`](../roadmap.md), [`../requirements.md`](../requirements.md),
   and [`../decisions.md`](../decisions.md) as work lands.
3. No source changes; pure docs/tracking. Commit separately.

---

## Appendix A — Web-path adapter mismatches  *(RESOLVED IN M9 — web only, not the Windows contract)*

The Windows frontend is C++ (weasel) talking to the RIME C ABI directly; it does
**not** use the TypeScript bridge. These are recorded so they aren't re-discovered,
but they are **out of scope** for the Windows contract. They were resolved in the
TypeDuck-Web adapter during M9 by mapping the runtime-owned response shape:

- candidate text comes from `candidate.text`
- candidate comments come from per-candidate `candidate.comment`
- highlight comes from `context.highlighted`

No engine or Windows C ABI change was required.

---

## Summary checklist

- [x] **Item 1** — Windows test baseline green (`librime_signature_modified_time` shape + poison-tolerant lock)
- [x] **Item 2** — `config_list_append_{string,bool,int,double}` on struct + table + impl + tests *(Contract #1)*
- [x] **Item 3** — v1.1.2 goldens captured (or reproducible blocker) *(prereq)*
- [x] **Item 4** — comment semantics: TypeDuck dictionary lookup payload + `"; "` reverse-lookup joins, golden-tested *(Contract #2)*
- [x] **Item 5** — native `rime.dll`/`.lib`/headers build documented + produced *(Contract #4)*
- [x] **Item 6** — Cantonese/Jyutping parity suite added with documented ignored oracle gaps *(Contract #3 regression path)*
- [x] **Item 0** — untracked files committed, EOL policy recorded, planning state reconciled, Windows milestone tracked
