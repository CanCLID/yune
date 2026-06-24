# M28 Follow-up Upstream-Style Jyutping Composition Implementation Plan

> **Status:** Complete - **Milestone:** M28 follow-up (default-confirm + upstream-style Jyutping composition) - **Created:** 2026-06-22 - **Type:** execution plan
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Finish the user-visible partial-selection behavior by fixing Space/default-confirm raw-tail commits and moving `jyut6ping3_mobile` long-input composition toward upstream librime engine semantics: sentence candidate first, longest valid fuzzy phrase-prefix candidates next, single-character fallback after that, and invalid fuzzy mis-segmentation ranked low.

**Architecture:** This is correctness work, not performance work. M28 already fixed explicit candidate selection by number/click; this follow-up first makes Space/default-confirm use the same segment-aware consumed-span path, then runs a blocking oracle-feasibility spike before any ranking implementation. TypeDuck v1.1.2 remains the compatibility-profile fixture for existing M14-M28 behavior, but this milestone deliberately evaluates an upstream-librime-engine plus pinned Jyutping source-assets fixture as the product target where TypeDuck v1.1.2 ranking is worse.

**Tech Stack:** Rust (`yune-core` engine/translator/ranking, `cantonese_parity`, `yune-rime-api` `typeduck_web` tests), upstream `rime/librime 1.17.0` oracle release assets under `target/upstream-oracle/1.17.0`, optional local upstream source checkout for inspection only, TypeDuck-Web Playwright evidence, and the existing TypeDuck-Web patch workflow.

---

## Why This Follow-up Exists

Manual dogfooding after M28 found two remaining problems for `caksijathaacoenggeoizi`:

1. With auto-composition off, the first candidate can display as `測sijathaacoenggeoizi`, and pressing Space commits that raw mixed string. Number/click selection was fixed by M28, but Space/default-confirm still bypasses the partial-consumed path because `explicit_partial_consumed_len(...)` returns `None` unless `intent == CommitIntent::ExplicitSelection`.
2. With auto-composition on, Yune now creates a sentence-like candidate, but the follow-up ordering still surfaces single-character candidates too early. The desired product behavior is closer to upstream librime-style script composition: a sentence/lattice candidate first, then longest dictionary phrase-prefix matches such as `測試一下` and `測試`, then single characters, with fuzzy mis-segmentation such as `差距` or `衩裙` ranked below valid syllable-boundary matches.

The user explicitly wants upstream librime behavior to be the behavior oracle for this case. Because upstream librime does not ship a built-in Jyutping schema, this plan treats the upstream `rime/librime 1.17.0` engine as the oracle runtime and uses pinned Jyutping source schema/dictionary assets as data. This is a hybrid oracle: upstream engine semantics plus external Jyutping assets. It must be proven by a source-YAML deploy/capture spike before ranking code starts. If upstream cannot load the assets or produces a result the user does not accept as the oracle of record, execution stops and asks whether to switch to a Yune-authored ranking spec with explicit sign-off.

This hybrid oracle covers composition and ranking only. It does not cover rich dictionary-panel comment payloads because stock upstream librime does not include TypeDuck's `dictionary_lookup_filter` plugin; comment byte parity remains governed by TypeDuck v1.1.2 fixtures and the P2-WIN-02 Windows boundary track.

## Scope

In scope:

- Fix Space/default-confirm for partial candidates so it commits only the consumed span and recomposes the remaining input.
- Add native and TypeDuck-Web tests proving Space does not commit raw `sijathaacoenggeoizi`.
- Capture an upstream-librime-engine Jyutping composition fixture for `caksijathaacoenggeoizi` with the local upstream checkout and documented schema/dictionary assets.
- Add a written ranking spec that is explicit about the upstream-style target and the deliberate departure from TypeDuck v1.1.2 for this ranking case.
- Add a distinct hybrid fixture namespace/provenance guard so this fixture is not mislabeled as pure upstream-core behavior.
- Implement fuzzy-expanded phrase-prefix generation/ranking so valid multi-syllable phrase prefixes rank before single characters.
- Add browser evidence for auto-composition off and on.

Out of scope:

- M29 startup/memory/typing performance work.
- Changing default upstream `luna_pinyin` behavior without existing upstream fixtures.
- Widening `RimeApi`, `RimeCandidate`, or TypeDuck profile ABI slots.
- Treating `typeduck.hk/web`, `my-rime.vercel.app`, or any live site as a hard oracle. They are comparison/feel targets only.
- Treating the hybrid upstream-Jyutping fixture as a dictionary-comment oracle.
- Rewriting the full translator/poet architecture before the fixture proves which behavior is needed.

## Acceptance Gates

- `M28F-UPSTREAM-01`: Space/default-confirm for `caksijathaacoenggeoizi` never commits `測sijathaacoenggeoizi`; it commits only the consumed prefix candidate and keeps the remaining input composing.
- `M28F-UPSTREAM-02`: A checked-in hybrid upstream-librime-engine Jyutping fixture captures `caksijathaacoenggeoizi` composition/ranking with provenance: upstream engine repository/tag/commit, pinned Jyutping schema/dictionary source repository/commit, upstream deploy command, capture command, options, and candidate rows. The fixture contains no local absolute paths.
- `M28F-UPSTREAM-03`: A decision note is added to `docs/decisions.md` stating that this Jyutping composition/ranking slice follows the accepted hybrid upstream-engine fixture over TypeDuck v1.1.2 when they disagree, while TypeDuck v1.1.2 remains the compatibility oracle for profile ABI/comment surfaces.
- `M28F-UPSTREAM-04`: Native tests prove sentence/phrase-prefix/single-character ordering for the captured case, including fuzzy `cak -> caak` phrase-prefix lookup for `測試` and invalid fuzzy mis-segmentation ranked below valid phrase-prefix matches.
- `M28F-UPSTREAM-05`: TypeDuck-Web browser evidence covers auto-composition off plus Space, and auto-composition on plus candidate ordering, without raw-tail commits.
- `M28F-UPSTREAM-06`: Compatibility gates remain green: `cargo fmt --check`, workspace clippy, upstream `luna_pinyin`, `cantonese_parity`, `typeduck_web`, workspace tests, TypeScript runtime tests/build, TypeDuck-Web build/evidence, patch checks if source changes, and `git diff --check`.

## Files And Responsibilities

- `docs/decisions.md`: ratify the upstream-style Jyutping composition/ranking target for this slice.
- `crates/yune-core/tests/oracle_fixture_provenance.rs`: add a distinct hybrid upstream-Jyutping fixture family and local-path guard.
- `crates/yune-core/src/engine.rs`: Space/default-confirm partial consumed-span behavior.
- `crates/yune-core/src/state.rs`: candidate source/commit preview behavior if the raw-tail preview needs tightening.
- `crates/yune-core/src/translator/mod.rs`: fuzzy phrase-prefix candidate generation and ranking, only after oracle capture.
- `crates/yune-core/src/poet.rs` or existing sentence-ranking module: reuse upstream-style sentence/phrase scoring if the fixture proves it is the right owner.
- `crates/yune-core/tests/cantonese_parity.rs`: native engine tests for default-confirm and upstream-style Jyutping ranking.
- `crates/yune-core/tests/fixtures/upstream-jyutping/`: new hybrid upstream-engine plus Jyutping source-assets fixture family.
- `crates/yune-rime-api/tests/typeduck_web.rs`: TypeDuck-Web/native API regression tests.
- `scripts/`: capture script for local upstream librime + Jyutping assets if no existing script can be reused.
- `apps/yune-web/e2e/yune-typeduck.spec.ts`: browser evidence.
- `apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/`: evidence folder.
- `apps/yune-web/patches/yune-web-runtime.patch`: regenerate only if TypeDuck-Web source changes.

## Implementation Tasks

### Task 0 - Ratify The Behavior Target

**Files:**

- Modify: `docs/decisions.md`
- Create: `apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/target-decision.md`

- [x] Step 0.1: Add the decision text before code changes.

Add a new project-wide decision after D-30:

```markdown
### Upstream-style Jyutping composition target (project-wide D-31)

**D-31 / JYUTPING-UPSTREAM-COMPOSITION - For long Jyutping composition in the TypeDuck-Web/Yune product surface, prefer an accepted upstream-librime-engine Jyutping fixture over TypeDuck v1.1.2 ranking when TypeDuck v1.1.2 produces worse segmentation.** Upstream `rime/librime 1.17.0` remains the engine oracle, but because upstream does not ship a built-in Jyutping schema, fixtures for this slice run the upstream engine against pinned Jyutping schema/dictionary source YAML and upstream's deployer. The intended ordering is sentence/lattice candidate first, longest valid fuzzy phrase-prefix candidates next, single-character fallback after that, and invalid fuzzy mis-segmentation ranked low. The hybrid upstream-engine fixture is the oracle of record for this named composition/ranking case once captured and accepted, even if it diverges from `typeduck.hk/web`, `my-rime.vercel.app`, or TypeDuck v1.1.2. TypeDuck v1.1.2 remains the compatibility-profile oracle for fork-only ABI/comment/profile behavior and existing M14-M28 fixtures; this hybrid fixture is not a dictionary-comment oracle because stock upstream lacks TypeDuck's `dictionary_lookup_filter` plugin. If the hybrid upstream-engine fixture cannot be captured or accepted, this milestone stops and requires explicit user sign-off for a Yune-authored ranking spec before ranking code changes.
```

Expected:

- The decision names the scope narrowly.
- It does not change the default upstream `RimeApi`.
- It does not erase existing TypeDuck v1.1.2 fixtures.
- It states that live websites are feel targets only, not oracles.
- It excludes dictionary-comment payloads from the hybrid oracle.

- [x] Step 0.2: Record the decision evidence.

Create `target-decision.md`:

```markdown
# M28 Follow-up Target Decision

The user selected upstream librime engine behavior as the product target for `caksijathaacoenggeoizi` long Jyutping composition. TypeDuck v1.1.2 remains the compatibility oracle for existing TypeDuck profile ABI/comment behavior, but its ranking for this case is not the desired product behavior.

Oracle constraints:
- Upstream `rime/librime 1.17.0` ships no built-in Jyutping schema.
- The M28 follow-up ranking fixture is therefore a hybrid: upstream engine plus pinned Jyutping source YAML deployed by upstream's `rime_deployer.exe`.
- `typeduck.hk/web` and `my-rime.vercel.app` are feel/comparison targets only.
- Dictionary comment payloads remain TypeDuck v1.1.2 / P2-WIN-02 scope because stock upstream lacks the TypeDuck `dictionary_lookup_filter` plugin.

Execution order:
1. Fix Space/default-confirm raw-tail commit.
2. Prove the hybrid upstream-engine Jyutping oracle can be captured from source YAML.
3. If the fixture is captured and accepted, implement only the fixture-backed ranking/generation gaps.
4. If the fixture cannot be captured or accepted, stop and request explicit sign-off for a Yune-authored ranking spec.
```

Expected:

- Evidence file exists before implementation.
- Later tasks can link to it.

### Task 1 - Fix Space/Default-Confirm Raw-Tail Commit

**Files:**

- Modify: `crates/yune-core/tests/cantonese_parity.rs`
- Modify: `crates/yune-rime-api/tests/typeduck_web.rs`
- Modify: `crates/yune-core/src/engine.rs`

- [x] Step 1.1: Add a failing `cantonese_parity` test.

First split the existing helper so tests can construct the same TypeDuck mobile engine with sentence generation disabled:

```rust
fn typeduck_jyut6ping3_mobile_engine_with_sentence(
    enable_correction: bool,
    enable_sentence: bool,
) -> Engine {
    // Move the current typeduck_jyut6ping3_mobile_engine body here and replace
    // .with_sentence(true) with .with_sentence(enable_sentence).
}

fn typeduck_jyut6ping3_mobile_engine(enable_correction: bool) -> Engine {
    typeduck_jyut6ping3_mobile_engine_with_sentence(enable_correction, true)
}
```

Add a focused test named `m28_followup_default_confirm_partial_candidate_recomposes`:

```rust
#[test]
fn m28_followup_default_confirm_partial_candidate_recomposes() {
    let input = "caksijathaacoenggeoizi";
    let remaining_input = "sijathaacoenggeoizi";
    let mut engine = typeduck_jyut6ping3_mobile_engine_with_sentence(false, false);
    engine.set_input(input);

    let selected = engine.context().candidates[0].clone();
    assert_eq!(selected.text, "測");
    assert_eq!(engine.process_char(' ').as_deref(), Some("測"));

    assert_eq!(engine.context().composition.input, remaining_input);
    assert_eq!(engine.context().composition.preedit, remaining_input);
    assert!(!engine
        .context()
        .last_commit
        .as_deref()
        .is_some_and(|commit| commit.contains(remaining_input)));

    let event = engine
        .take_pending_userdb_learning()
        .expect("default partial selection should stage consumed-span userdb learning");
    assert_eq!(event.input, "cak");
    assert_eq!(event.selected_text, selected.text);
    assert_eq!(event.segment_start, 0);
    assert_eq!(event.segment_end, "cak".len());
    assert_eq!(event.code, "cak1");
}
```

If the local helper shape changes before implementation, use the existing M28 partial-selection helper style in `cantonese_parity.rs`; preserve the same assertions and test name. Do not use a runtime `set_option("translator/enable_sentence", false)` here: sentence enablement is baked into the test translator construction path.

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_followup_default_confirm_partial_candidate_recomposes
```

Expected before implementation:

- The test fails because Space/default-confirm commits `測sijathaacoenggeoizi` or clears composition.

- [x] Step 1.2: Add a failing `typeduck_web` API test.

Add a test named `typeduck_adapter_m28_followup_space_partial_candidate_recomposes`:

```rust
#[test]
fn typeduck_adapter_m28_followup_space_partial_candidate_recomposes() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("m28-followup-space-partial");
    runtime.write_browser_real_assets();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    for ch in "caksijathaacoenggeoizi".chars() {
        drop(response_json(unsafe { yune_typeduck_process_key(state, ch as i32, 0) }));
    }

    let space = response_json(unsafe { yune_typeduck_process_key(state, ' ' as i32, 0) });
    assert_eq!(space["commits"], serde_json::json!(["測"]));
    assert_eq!(space["context"]["input"], serde_json::json!("sijathaacoenggeoizi"));
    assert_ne!(space["commits"], serde_json::json!(["測sijathaacoenggeoizi"]));

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}
```

If `write_browser_real_assets()` has a different helper name in the current file, use the helper used by the existing M28 `typeduck_web` partial-selection test.

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_web -- typeduck_adapter_m28_followup_space_partial_candidate_recomposes
```

Expected before implementation:

- The test fails on the commit or remaining input assertion.

- [x] Step 1.3: Add a sentence-on default-confirm guard test.

Add a focused test named `m28_followup_default_confirm_whole_sentence_keeps_full_primary_code_learning` in `cantonese_parity.rs`. Keep it close to the existing `m28_whole_sentence_selection_keeps_full_primary_code_learning` test and reuse that test's code-extraction pattern:

```rust
#[test]
fn m28_followup_default_confirm_whole_sentence_keeps_full_primary_code_learning() {
    let fixture = m28_partial_selection_fixture();
    let input = fixture["input"]
        .as_str()
        .expect("M28 fixture should capture input");

    let mut engine = typeduck_jyut6ping3_mobile_engine(false);
    engine.set_input(input);

    let selected_sentence = engine.context().candidates[0].clone();
    let expected_code = selected_sentence
        .comment
        .split('\r')
        .filter_map(|record| {
            let fields = record.strip_prefix("1,")?.split(',').collect::<Vec<_>>();
            let is_composition = fields.get(7).is_some_and(|field| *field == "composition");
            (!is_composition).then(|| fields.get(1).map(|code| (*code).to_owned()))?
        })
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        !expected_code.is_empty(),
        "sentence candidate should carry primary component codes"
    );
    assert!(
        selected_sentence.text.chars().count() > 1,
        "test should exercise a whole-input sentence/composition candidate"
    );

    assert_eq!(
        engine.process_char(' ').as_deref(),
        Some(selected_sentence.text.as_str())
    );
    let event = engine
        .take_pending_userdb_learning()
        .expect("whole-sentence default confirm should stage userdb learning");
    assert_eq!(event.input, input);
    assert_eq!(event.selected_text, selected_sentence.text);
    assert_eq!(event.segment_start, 0);
    assert_eq!(event.segment_end, input.len());
    assert_eq!(event.code, expected_code);

    assert!(engine.context().composition.input.is_empty());
}
```

This test is intentionally about the FORK-PARITY-03 invariant, not only the user-visible symptom. It must prove that default confirm for a whole-input sentence candidate keeps the learning event on the full input span (`segment_start == 0`, `segment_end == input.len()`) and preserves the expected primary component code.

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_followup_default_confirm_whole_sentence_keeps_full_primary_code_learning
```

Expected:

- The test passes before and after the minimal Space fix.
- If it fails before the fix, stop and inspect why the default-confirm path is changing whole-sentence learning semantics before editing `engine.rs`.

- [x] Step 1.4: Implement the minimal engine fix.

In `crates/yune-core/src/engine.rs`, replace `explicit_partial_consumed_len(...)` with a helper that allows both explicit selection and default confirm:

```rust
fn partial_consumed_len_for_commit(
    input: &str,
    candidate: &Candidate,
    intent: CommitIntent,
) -> Option<usize> {
    if !matches!(
        intent,
        CommitIntent::ExplicitSelection | CommitIntent::DefaultConfirm
    ) {
        return None;
    }
    let consumed = candidate.source.partial_consumed_len()?;
    if consumed == 0 || consumed >= input.len() || !input.is_char_boundary(consumed) {
        return None;
    }
    Some(consumed)
}
```

Then update `commit_candidate(...)`:

```rust
let partial_consumed = partial_consumed_len_for_commit(&input, &candidate, intent);
```

Expected:

- AI candidates still cannot be default-confirmed because the existing `if intent == CommitIntent::DefaultConfirm && candidate_source.is_ai()` guard remains before the partial path.
- Whole-input sentence candidates continue to fall through unchanged because they do not carry a shorter `PartialTable` consumed span.

- [x] Step 1.5: Run focused gates.

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_followup_default_confirm_partial_candidate_recomposes
cargo test -p yune-core --test cantonese_parity -- m28_followup_default_confirm_whole_sentence_keeps_full_primary_code_learning
cargo test -p yune-rime-api --test typeduck_web -- typeduck_adapter_m28_followup_space_partial_candidate_recomposes
cargo test -p yune-core --test cantonese_parity -- m28_partial_selection
```

Expected:

- New tests pass.
- The sentence-on guard test still passes after default-confirm starts using partial consumed spans.
- Existing M28 partial-selection tests still pass.

### Task 2 - Run Hybrid Upstream-Jyutping Oracle De-risking Spike

**Files:**

- Create: `scripts/capture-upstream-jyutping-composition.ps1`
- Create: `crates/yune-core/tests/fixtures/upstream-jyutping/README.md`
- Create: `crates/yune-core/tests/fixtures/upstream-jyutping/oracle-manifest.json`
- Create when capture succeeds: `crates/yune-core/tests/fixtures/upstream-jyutping/jyutping-m28-followup-composition.json`
- Modify: `crates/yune-core/tests/oracle_fixture_provenance.rs`
- Create: `apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/oracle-capture.md`

- [x] Step 2.1: Stage upstream 1.17.0 release assets if missing.

The capture script expects the official upstream Windows MSVC x64 release archive to be extracted under `target/upstream-oracle/1.17.0/extract`. Use the upstream release page and asset:

- Release: `https://github.com/rime/librime/releases/tag/1.17.0`
- Asset: `https://github.com/rime/librime/releases/download/1.17.0/rime-33e7814-Windows-msvc-x64.7z`

Run this check before capture:

```powershell
$OracleRoot = "target\upstream-oracle\1.17.0"
$Extract = Join-Path $OracleRoot "extract"
$Required = @(
  "dist\lib\rime.dll",
  "dist\bin\rime_deployer.exe",
  "dist\include\rime_api.h"
) | ForEach-Object { Join-Path $Extract $_ }
$Missing = $Required | Where-Object { -not (Test-Path -LiteralPath $_) }
if ($Missing) {
  New-Item -ItemType Directory -Force -Path $OracleRoot | Out-Null
  throw "Missing upstream librime 1.17.0 extract files. Download rime-33e7814-Windows-msvc-x64.7z from https://github.com/rime/librime/releases/tag/1.17.0 and extract it so dist/lib, dist/bin, and dist/include sit under $Extract."
}
```

Expected:

- `target/upstream-oracle/1.17.0/extract/dist/lib/rime.dll` exists.
- `target/upstream-oracle/1.17.0/extract/dist/bin/rime_deployer.exe` exists.
- `target/upstream-oracle/1.17.0/extract/dist/include/rime_api.h` exists.
- If any are missing, record the blocker in `oracle-capture.md` and do not attempt to fabricate expected candidates.

- [x] Step 2.2: Locate pinned Jyutping source YAML, not browser compiled assets.

Run:

```powershell
$Candidates = Get-ChildItem -Path target -Recurse -Filter jyut6ping3.dict.yaml -ErrorAction SilentlyContinue |
  Where-Object {
    Test-Path (Join-Path $_.DirectoryName "jyut6ping3_mobile.schema.yaml") -and
    Test-Path (Join-Path $_.DirectoryName "jyut6ping3.schema.yaml")
  }
$Candidates | Select-Object -ExpandProperty DirectoryName
```

Expected:

- At least one source-YAML directory is found.
- The chosen directory contains `jyut6ping3_mobile.schema.yaml`, `jyut6ping3.schema.yaml`, and `jyut6ping3.dict.yaml`.
- Do not use `apps/yune-web/source/public/schema` for this spike; that tree contains browser/deployed compiled assets, not the source YAML upstream deployer consumes.

If no directory is found, prepare the source under the upstream oracle cache and stop after recording the missing-input blocker in `oracle-capture.md`:

```powershell
New-Item -ItemType Directory -Force -Path target\upstream-oracle\1.17.0\schema-src | Out-Null
git clone https://github.com/TypeDuck-HK/schema target\upstream-oracle\1.17.0\schema-src\typeduck-schema
git -C target\upstream-oracle\1.17.0\schema-src\typeduck-schema rev-parse HEAD
```

- [x] Step 2.3: Write the capture script using upstream deployer plus `oracle-rime-probe.cs`.

Create `scripts/capture-upstream-jyutping-composition.ps1`:

```powershell
param(
  [string]$OracleRoot = "target\upstream-oracle\1.17.0",
  [Parameter(Mandatory = $true)]
  [string]$JyutpingSchemaSource,
  [string[]]$DependencySource = @(),
  [string]$Output = "crates\yune-core\tests\fixtures\upstream-jyutping\jyutping-m28-followup-composition.json",
  [string]$Evidence = "apps\yune-web\e2e\results\m28-follow-up-upstream-jyutping\oracle-capture.md"
)

$ErrorActionPreference = "Stop"

function FullPath([string]$Path) {
  [System.IO.Path]::GetFullPath((Join-Path $RepoRoot $Path))
}

function Write-Utf8NoBom([string]$Path, [string]$Text) {
  $Dir = Split-Path -Parent $Path
  if (-not [string]::IsNullOrWhiteSpace($Dir)) {
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
  }
  $Encoding = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($Path, $Text, $Encoding)
}

function Write-JsonFile([string]$Path, $Value) {
  Write-Utf8NoBom $Path (($Value | ConvertTo-Json -Depth 64) + "`n")
}

function Copy-YamlSources([string]$SourceDir, [string]$DestDir) {
  Get-ChildItem -LiteralPath $SourceDir -File |
    Where-Object { $_.Name -like "*.yaml" -or $_.Name -eq "essay.txt" } |
    ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $DestDir $_.Name) -Force }
}

function SourceCommit([string]$SourceDir) {
  try {
    return (git -C $SourceDir rev-parse HEAD 2>$null).Trim()
  } catch {
    $Match = [regex]::Match($SourceDir, "schema-([0-9a-fA-F]{40})")
    if ($Match.Success) { return $Match.Groups[1].Value.ToLowerInvariant() }
    throw "Cannot determine Jyutping source commit from $SourceDir; pass a git checkout or schema-<commit> directory"
  }
}

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$OracleRoot = FullPath $OracleRoot
$JyutpingSchemaSource = [System.IO.Path]::GetFullPath($JyutpingSchemaSource)
$Output = FullPath $Output
$Evidence = FullPath $Evidence
$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot "m28f-jyutping-shared"
$User = Join-Path $OracleRoot "m28f-jyutping-user"
$Build = Join-Path $User "build"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"

foreach ($Path in @(
  (Join-Path $Extract "dist\lib\rime.dll"),
  (Join-Path $Extract "dist\bin\rime_deployer.exe"),
  (Join-Path $Extract "dist\include\rime_api.h"),
  (Join-Path $JyutpingSchemaSource "jyut6ping3_mobile.schema.yaml"),
  (Join-Path $JyutpingSchemaSource "jyut6ping3.schema.yaml"),
  (Join-Path $JyutpingSchemaSource "jyut6ping3.dict.yaml"),
  $ProbeSource
)) {
  if (-not (Test-Path -LiteralPath $Path)) {
    throw "Missing required upstream Jyutping oracle input: $Path"
  }
}

foreach ($Dir in @($Shared, $User)) {
  $ResolvedRoot = [System.IO.Path]::GetFullPath($OracleRoot)
  $ResolvedDir = [System.IO.Path]::GetFullPath($Dir)
  if (-not $ResolvedDir.StartsWith($ResolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to recreate outside oracle root: $ResolvedDir"
  }
  if (Test-Path -LiteralPath $Dir) {
    Remove-Item -LiteralPath $Dir -Recurse -Force
  }
  New-Item -ItemType Directory -Force -Path $Dir | Out-Null
}

Copy-YamlSources $JyutpingSchemaSource $Shared
foreach ($Source in $DependencySource) {
  Copy-YamlSources ([System.IO.Path]::GetFullPath($Source)) $Shared
}

@"
patch:
  schema_list:
    - schema: jyut6ping3_mobile
"@ | Set-Content -LiteralPath (Join-Path $Shared "default.custom.yaml") -Encoding UTF8

$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
New-Item -ItemType Directory -Force -Path $Build | Out-Null
& (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
  throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}

Add-Type -Path $ProbeSource
$Modules = [string[]]@("default")

function New-ProbeAction([string]$Type, [int]$Keycode, [string]$Label) {
  $Action = [RimeProbe+ProbeAction]::new()
  $Action.type = $Type
  $Action.keycode = $Keycode
  $Action.mask = 0
  $Action.label = $Label
  $Action
}

function New-InputAction([string]$Text) {
  $Action = [RimeProbe+ProbeAction]::new()
  $Action.type = "input"
  $Action.text = $Text
  $Action
}

function New-SnapshotAction([string]$Label) {
  $Action = [RimeProbe+ProbeAction]::new()
  $Action.type = "snapshot"
  $Action.label = $Label
  $Action
}

function New-Scenario([string]$Name, $Actions) {
  $Scenario = [RimeProbe+ProbeScenario]::new()
  $Scenario.name = $Name
  $Scenario.actions = [RimeProbe+ProbeAction[]]$Actions
  $Scenario
}

$Scenarios = [RimeProbe+ProbeScenario[]]@(
  (New-Scenario "auto_composition_default_before_space" @(
    (New-InputAction "caksijathaacoenggeoizi"),
    (New-SnapshotAction "before_space"),
    (New-ProbeAction "key" 32 "after_space")
  ))
)

$Snapshots = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "jyut6ping3_mobile", $Modules, $Scenarios)
$BeforeSpace = $Snapshots | Where-Object { $_["scenario"] -eq "auto_composition_default_before_space" -and $_["label"] -eq "before_space" } | Select-Object -First 1
$AfterSpace = $Snapshots | Where-Object { $_["scenario"] -eq "auto_composition_default_before_space" -and $_["label"] -eq "after_space" } | Select-Object -First 1
if ($null -eq $BeforeSpace -or $null -eq $AfterSpace) {
  throw "oracle capture did not include before_space and after_space snapshots"
}

$Fixture = [ordered]@{
  fixture = "jyutping-m28-followup-composition"
  oracle = [ordered]@{
    engine = "rime/librime"
    engine_tag = "1.17.0"
    engine_commit = "33e78140250125871856cdc5b42ddc6a5fcd3cd4"
    canonical_repository = "https://github.com/rime/librime"
    jyutping_schema_repository = "https://github.com/TypeDuck-HK/schema"
    jyutping_schema_commit = SourceCommit $JyutpingSchemaSource
    capture_command = "powershell -NoProfile -ExecutionPolicy Bypass -File scripts\capture-upstream-jyutping-composition.ps1 -OracleRoot target\upstream-oracle\1.17.0 -JyutpingSchemaSource target\upstream-oracle\1.17.0\schema-src\typeduck-schema"
  }
  schema = "jyut6ping3_mobile"
  module_list = $Modules
  input = "caksijathaacoenggeoizi"
  user_feel_target = "測試一下長句子"
  oracle_scope = "composition_and_ranking_only_not_comment_payloads"
  ranking_contract = @(
    "sentence_candidate_first",
    "longest_valid_fuzzy_phrase_prefix_before_single_character",
    "single_character_after_phrase_prefix",
    "invalid_fuzzy_missegmentation_ranked_low"
  )
  auto_composition_on = [ordered]@{
    candidate_rows = $BeforeSpace["selected_candidates"]
    space_commit = $AfterSpace["commit_text"]
    remaining_input_after_space = $AfterSpace["rime_get_input"]
  }
  snapshots = $Snapshots
}

Write-JsonFile $Output $Fixture

$EvidenceText = @"
# M28 Follow-up Hybrid Upstream Jyutping Oracle Capture

Command:

````powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\capture-upstream-jyutping-composition.ps1 -OracleRoot target\upstream-oracle\1.17.0 -JyutpingSchemaSource target\upstream-oracle\1.17.0\schema-src\typeduck-schema
````

Engine: rime/librime 1.17.0 (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`)
Jyutping source repository: TypeDuck-HK/schema
Jyutping source commit: $($Fixture.oracle.jyutping_schema_commit)
Schema: jyut6ping3_mobile
Input: caksijathaacoenggeoizi

Scope:
- composition/ranking oracle: yes
- dictionary-comment oracle: no, stock upstream lacks TypeDuck dictionary_lookup_filter
- live website oracle: no

Result:
- before_space first page: captured in $Output
- after_space commit: captured in $Output
- user acceptance: pending review of captured candidate rows
"@
Write-Utf8NoBom $Evidence $EvidenceText
```

Expected:

- The script uses `target/upstream-oracle/1.17.0/extract/dist/bin/rime_deployer.exe --build` and `scripts/oracle-rime-probe.cs`.
- The script consumes source `*.schema.yaml` / `*.dict.yaml` files, not browser compiled `.bin` assets.
- The checked-in fixture records repositories and commits, not `C:\...` local checkout paths.
- The capture uses module list `["default"]`; `dictionary_lookup` remains out of scope.

- [x] Step 2.4: Add a hybrid fixture root and provenance guard.

Create `crates/yune-core/tests/fixtures/upstream-jyutping/README.md`:

````markdown
# Upstream Engine Jyutping Oracle Fixtures

This fixture family is intentionally hybrid:

- Engine oracle: upstream `rime/librime` `1.17.0`
  (`33e78140250125871856cdc5b42ddc6a5fcd3cd4`)
- Schema assets: pinned Jyutping source YAML from `https://github.com/TypeDuck-HK/schema`

The fixtures cover composition and ranking behavior only. They do not cover
TypeDuck rich dictionary-panel comment bytes because stock upstream librime does
not include TypeDuck's `dictionary_lookup_filter` plugin.

Capture command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\capture-upstream-jyutping-composition.ps1 -OracleRoot target\upstream-oracle\1.17.0 -JyutpingSchemaSource target\upstream-oracle\1.17.0\schema-src\typeduck-schema
```

Fixtures must record source repositories and commits, not local checkout paths.
````

Create `crates/yune-core/tests/fixtures/upstream-jyutping/oracle-manifest.json`:

```json
{
  "fixture_family": "upstream-engine-jyutping-assets",
  "oracle": {
    "engine": "rime/librime",
    "engine_tag": "1.17.0",
    "engine_commit": "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
    "canonical_repository": "https://github.com/rime/librime",
    "jyutping_schema_repository": "https://github.com/TypeDuck-HK/schema"
  },
  "profile_only": false,
  "hybrid_oracle": true,
  "scope": "composition_and_ranking_only_not_comment_payloads"
}
```

In `crates/yune-core/tests/oracle_fixture_provenance.rs`, extend `oracle_fixture_roots_have_machine_readable_provenance()`:

```rust
assert_manifest(
    "upstream-jyutping",
    "upstream-engine-jyutping-assets",
    "rime/librime",
    "1.17.0",
    "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
    false,
);
```

Add a new test:

```rust
#[test]
fn upstream_jyutping_fixture_has_hybrid_provenance() {
    let root = fixture_root("upstream-jyutping");
    let path = root.join("jyutping-m28-followup-composition.json");
    assert!(path.is_file(), "M28 follow-up should check in hybrid Jyutping fixture");
    let fixture = read_json(&path);
    assert_eq!(fixture["oracle"]["engine"], "rime/librime", "{path:?}");
    assert_eq!(fixture["oracle"]["engine_tag"], "1.17.0", "{path:?}");
    assert_eq!(
        fixture["oracle"]["engine_commit"],
        "33e78140250125871856cdc5b42ddc6a5fcd3cd4",
        "{path:?}"
    );
    assert_eq!(
        fixture["oracle"]["jyutping_schema_repository"],
        "https://github.com/TypeDuck-HK/schema",
        "{path:?}"
    );
    assert!(
        fixture["oracle"]["jyutping_schema_commit"]
            .as_str()
            .is_some_and(|commit| commit.len() == 40),
        "{path:?} must include a pinned Jyutping schema commit"
    );
    assert_eq!(
        fixture["oracle_scope"],
        "composition_and_ranking_only_not_comment_payloads",
        "{path:?}"
    );
    assert_eq!(
        fixture["module_list"],
        serde_json::json!(["default"]),
        "{path:?}"
    );
    assert_no_local_absolute_paths(&path, &fixture);
}
```

Run:

```powershell
cargo test -p yune-core --test oracle_fixture_provenance -- upstream_jyutping_fixture_has_hybrid_provenance
```

Expected:

- The test fails until the fixture is captured.
- Once the fixture exists, any local absolute path in the fixture fails the guard.

- [x] Step 2.5: Run the de-risking capture and decide whether ranking can proceed.

Run with the chosen source-YAML root:

```powershell
$JyutpingSchemaSource = (Get-ChildItem -Path target -Recurse -Filter jyut6ping3.dict.yaml -ErrorAction Stop |
  Where-Object {
    Test-Path (Join-Path $_.DirectoryName "jyut6ping3_mobile.schema.yaml") -and
    Test-Path (Join-Path $_.DirectoryName "jyut6ping3.schema.yaml")
  } |
  Select-Object -First 1).DirectoryName
if ([string]::IsNullOrWhiteSpace($JyutpingSchemaSource)) {
  throw "No Jyutping source YAML root found under target"
}
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\capture-upstream-jyutping-composition.ps1 -OracleRoot target\upstream-oracle\1.17.0 -JyutpingSchemaSource $JyutpingSchemaSource
```

Then review:

- `crates/yune-core/tests/fixtures/upstream-jyutping/jyutping-m28-followup-composition.json`
- `apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/oracle-capture.md`

Acceptance:

- If capture succeeds and the candidate rows are accepted as the oracle of record, continue to Task 3.
- If upstream fails to deploy/load the Jyutping source assets, record the exact unsupported file/gear in `oracle-capture.md`, do not write the fixture, and stop for user review.
- If capture succeeds but the ordering is not accepted as the oracle of record, stop and ask whether to replace Tasks 3-4 with a Yune-authored ranking spec. Do not implement ranking from TypeDuck v1.1.2, `typeduck.hk/web`, or `my-rime.vercel.app`.

### Task 3 - Add Ranking Tests Before Implementation

**Files:**

- Modify: `crates/yune-core/tests/cantonese_parity.rs`
- Modify: `crates/yune-rime-api/tests/typeduck_web.rs`
- Read: `crates/yune-core/tests/fixtures/upstream-jyutping/jyutping-m28-followup-composition.json`

- [x] Step 3.1: Add a native ranking test.

Add a test named `m28_followup_upstream_style_phrase_prefix_ranking` that:

- Loads `jyut6ping3_mobile`.
- Enables auto-composition/sentence behavior.
- Types `caksijathaacoenggeoizi`.
- Asserts candidate 0 is a sentence/composition candidate from the fixture.
- Asserts the first page contains a valid phrase-prefix candidate for `測試` or a longer fixture-captured phrase prefix before single-character `測`.
- Asserts invalid fuzzy mis-segmentation candidates such as `差距` / `衩裙`, if present, rank after valid syllable-boundary phrase prefixes and single-character fallback.

Use fixture-driven assertions:

```rust
let fixture: serde_json::Value =
    serde_json::from_str(include_str!("fixtures/upstream-jyutping/jyutping-m28-followup-composition.json"))
        .expect("fixture should parse");
let expected_rows = fixture["auto_composition_on"]["candidate_rows"]
    .as_array()
    .expect("fixture should capture candidate rows");
```

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_followup_upstream_style_phrase_prefix_ranking
```

Expected before implementation:

- The test fails on missing/ranked-too-low phrase-prefix candidates or wrong Space/default behavior.

- [x] Step 3.2: Add a TypeDuck-Web/native API ranking test.

Add a test named `typeduck_adapter_m28_followup_upstream_style_phrase_prefix_ranking`:

```rust
#[test]
fn typeduck_adapter_m28_followup_upstream_style_phrase_prefix_ranking() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("m28-followup-upstream-ranking");
    runtime.write_browser_real_assets();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    for ch in "caksijathaacoenggeoizi".chars() {
        drop(response_json(unsafe { yune_typeduck_process_key(state, ch as i32, 0) }));
    }
    let response = response_json(unsafe { yune_typeduck_process_key(state, 0, 0) });
    let candidates = response["context"]["candidates"]
        .as_array()
        .expect("candidates should be present");
    assert!(
        candidates.iter().any(|row| row["text"].as_str().is_some_and(|text| text.starts_with("測試"))),
        "valid fuzzy phrase-prefix candidates should be present on the first page"
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}
```

If `yune_typeduck_process_key(state, 0, 0)` is not a no-op refresh in current tests, use the response from the final typed character instead.

Run:

```powershell
cargo test -p yune-rime-api --test typeduck_web -- typeduck_adapter_m28_followup_upstream_style_phrase_prefix_ranking
```

Expected before implementation:

- The test fails if the first page lacks fixture-required phrase-prefix candidates.

### Task 4 - Implement Upstream-Style Phrase-Prefix Ranking

**Files:**

- Modify: `crates/yune-core/src/translator/mod.rs`
- Modify if needed: `crates/yune-core/src/poet.rs`
- Modify if needed: `crates/yune-core/src/state.rs`

- [x] Step 4.1: Diagnose where `測試` is lost.

Add temporary debug output only inside a local test or evidence helper, not production logs, to answer:

- Does the spelling algebra expand `cak` to `caak` for the first syllable of a multi-syllable lookup?
- Does lookup produce `測試` for `cak si` / `caak1 si3`?
- If produced, what source and quality put it behind single characters?

Record the result in:

```text
apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/phrase-prefix-diagnosis.md
```

Expected:

- The evidence identifies generation gap, ranking gap, or both.

- [x] Step 4.2: Generate valid fuzzy phrase-prefix candidates.

Implement the smallest behavior that satisfies the fixture:

- Preserve syllable boundaries from the speller/translator path.
- Apply fuzzy spelling expansion across phrase-prefix lookup, not only single-syllable lookup.
- Generate phrase-prefix candidates only for valid dictionary phrase rows such as `測試`.
- Do not create cross-boundary fuzzy splits like treating `cak` as `ca` + `k` when syllable segmentation already has `cak`.

Implementation guard:

```rust
if profile.is_typeduck_jyutping() && candidate_matches_valid_phrase_prefix {
    candidate.source = CandidateSource::PartialTable { consumed: consumed_input_len };
    candidate.quality = phrase_prefix_quality(base_quality, phrase_len, fuzzy_penalty);
}
```

Use existing profile/config predicates instead of introducing a shared unconditional TypeDuck constant.

- [x] Step 4.3: Rank sentence, phrase prefixes, single characters, and mis-segmentation.

Expected ranking policy:

1. Sentence/composition candidate, if enabled and fixture-backed.
2. Valid phrase-prefix candidates, longest prefix first, then dictionary quality/frequency.
3. Single-character candidates.
4. Fuzzy mis-segmentation candidates and low-confidence fallbacks.

Do not hardcode `測試` or `caksijathaacoenggeoizi`; the implementation must use candidate source, consumed span, phrase length, fuzzy penalty, and dictionary quality.

- [x] Step 4.4: Run focused tests.

Run:

```powershell
cargo test -p yune-core --test cantonese_parity -- m28_followup
cargo test -p yune-rime-api --test typeduck_web -- m28_followup
```

Expected:

- Space/default-confirm tests pass.
- Ranking tests pass against the captured upstream-librime-engine fixture.
- Existing M28 partial-selection tests still pass.

### Task 5 - Add Browser Evidence

**Files:**

- Modify: `apps/yune-web/e2e/yune-typeduck.spec.ts`
- Modify if source changed: `apps/yune-web/patches/yune-web-runtime.patch`
- Create: `apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/browser-space-default-confirm.json`
- Create: `apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/browser-upstream-ranking.json`

- [x] Step 5.1: Add Space/default-confirm browser smoke.

The test must:

- Load `/web/`.
- Turn auto-composition off.
- Type `caksijathaacoenggeoizi`.
- Press Space.
- Assert the text area receives `測`, not `測sijathaacoenggeoizi`.
- Assert the composition panel remains active with the remaining input beginning `sijathaa`.

Save `browser-space-default-confirm.json` with:

```json
{
  "input": "caksijathaacoenggeoizi",
  "action": "Space",
  "committed": "測",
  "raw_tail_committed": false,
  "remaining_input_prefix": "sijathaa"
}
```

- [x] Step 5.2: Add upstream-style ranking browser smoke.

The test must:

- Load `/web/`.
- Turn auto-composition on.
- Type `caksijathaacoenggeoizi`.
- Save the first candidate page.
- Assert candidate 0 is the fixture-backed sentence/composition candidate.
- Assert a valid `測試` phrase-prefix candidate appears before single-character fallback rows.

Save `browser-upstream-ranking.json` with candidate rows and option state.

Run:

```powershell
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M28 FOLLOW-UP" --workers=1
```

Expected:

- Both browser tests pass.
- If TypeDuck-Web source changed, regenerate and reverse/forward-check the patch.

### Task 6 - Close With Gates And Docs

**Files:**

- Modify: `docs/roadmap.md`
- Modify: `docs/requirements.md`
- Archive when complete: `docs/plans/archive/m28-follow-up-plan-upstream-jyutping-composition.md`
- Create: `apps/yune-web/e2e/results/m28-follow-up-upstream-jyutping/task-6-gates.md`

- [x] Step 6.1: Run full verification.

Run:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test typeduck_web
cargo test --workspace
npm.cmd --prefix packages/yune-typeduck-runtime test
npm.cmd --prefix packages/yune-typeduck-runtime run build
npm.cmd --prefix apps/yune-web/source run build
$env:YUNE_WEB_APP_URL='http://localhost:5173/web/'; npm.cmd --prefix apps\yune-web\e2e run test:e2e -- --grep "M28 FOLLOW-UP" --workers=1
git diff --check
```

If TypeDuck-Web source changed, also run the patch reverse/forward checks used by M24-M28.

Expected:

- All gates pass.
- `task-6-gates.md` records exact command results.
- Roadmap and requirements mark this follow-up complete with fixture/evidence paths.
- The plan moves to `docs/plans/archive/` only after evidence and gates pass.

## Handoff Message

Use this if starting the work in a fresh session:

```text
Please execute the M28 follow-up plan in C:\Users\laubonghaudoi\Documents\GitHub\yune:

Read AGENTS.md, docs/CONVENTIONS.md, docs/roadmap.md, docs/requirements.md, docs/plans/m28-follow-up-plan-upstream-jyutping-composition.md, and the archived M28 plan/evidence first.

Goal: fix Space/default-confirm raw-tail partial commits, then run the hybrid upstream-engine Jyutping oracle spike for caksijathaacoenggeoizi before any ranking implementation. Use the existing upstream oracle pattern: upstream rime.dll/rime_deployer.exe from target/upstream-oracle/1.17.0, source Jyutping YAML, and scripts/oracle-rime-probe.cs. Do not use apps/yune-web/source/public/schema compiled browser assets, and do not create a standalone probe executable. TypeDuck v1.1.2 remains the existing compatibility oracle for ABI/comment/profile surfaces, but this follow-up may prefer an accepted hybrid upstream-engine fixture for this named ranking case.

Do the Space/default-confirm fix first with failing native/API tests, including the sentence-on whole-input guard. Then run Task 2 as a blocking de-risking spike. Do not implement ranking before the hybrid fixture is captured and accepted, or before the user explicitly signs off on a Yune-authored ranking spec fallback. Preserve default upstream ABI and TypeDuck profile ABI. If TypeDuck-Web source changes, regenerate and reverse/forward-check the patch. Stage only intended files and push directly to origin/main when all gates pass.
```
