param(
    [string]$OracleRoot,
    [string]$Output,
    [string]$ScenarioInput
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot "crates\yune-core\tests\fixtures\upstream-1.17.0\luna-pinyin-basic.json"
}
$UseDefaultScenarioInput = [string]::IsNullOrWhiteSpace($ScenarioInput)
if ($UseDefaultScenarioInput) {
    $ScenarioInput = Join-Path $OracleRoot "luna-pinyin-scenarios.json"
}

$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
$ScenarioInput = [System.IO.Path]::GetFullPath($ScenarioInput)
$FixtureRoot = Split-Path -Parent $Output
$SelectionOutput = Join-Path $FixtureRoot "luna-pinyin-selection.json"
$ActionsOutput = Join-Path $FixtureRoot "luna-pinyin-actions.json"
$ReverseOutput = Join-Path $FixtureRoot "luna-pinyin-reverse-lookup.json"
$PunctuationOutput = Join-Path $FixtureRoot "luna-pinyin-punctuation.json"
$OptionsOutput = Join-Path $FixtureRoot "luna-pinyin-options.json"
$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot "rime-shared"
$User = Join-Path $OracleRoot "rime-user"
$Build = Join-Path $User "build"
$SchemaRoot = Join-Path $OracleRoot "schema-src"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"

$RequiredPaths = @(
    (Join-Path $Extract "dist\lib\rime.dll"),
    (Join-Path $Extract "dist\bin\rime_deployer.exe"),
    (Join-Path $Extract "dist\include\rime_api.h"),
    (Join-Path $SchemaRoot "rime-prelude"),
    (Join-Path $SchemaRoot "rime-essay"),
    (Join-Path $SchemaRoot "rime-luna-pinyin"),
    (Join-Path $SchemaRoot "rime-stroke"),
    $ProbeSource
)
foreach ($Path in $RequiredPaths) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing required upstream oracle input: $Path"
    }
}
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    throw "Node.js is required to write deterministic UTF-8 fixture JSON."
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

foreach ($Repo in @("rime-prelude", "rime-essay", "rime-luna-pinyin", "rime-stroke")) {
    $Source = Join-Path $SchemaRoot $Repo
    Get-ChildItem -LiteralPath $Source -File |
        Where-Object { $_.Name -like "*.yaml" -or $_.Name -eq "essay.txt" } |
        ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Shared $_.Name) -Force
        }
}

$OpenCcDest = Join-Path $Shared "opencc"
New-Item -ItemType Directory -Force -Path $OpenCcDest | Out-Null
$OpenCcSource = Join-Path $Extract "share\opencc"
Get-ChildItem -LiteralPath $OpenCcSource | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDest -Recurse -Force
}
@"
patch:
  schema_list:
    - schema: luna_pinyin
"@ | Set-Content -LiteralPath (Join-Path $Shared "default.custom.yaml") -Encoding UTF8

New-Item -ItemType Directory -Force -Path $Build | Out-Null
$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
& (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}

Add-Type -Path $ProbeSource
$Inputs = [string[]]@("ni", "hao", "zhong", "guo", "zhongguo")
$Modules = [string[]]@("default")
$Cases = [RimeProbe]::Capture($Shared, $User, $Build, "luna_pinyin", $Modules, $Inputs)
$CasesJson = Join-Path $OracleRoot "luna-pinyin-capture-cases.json"
$Cases | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $CasesJson -Encoding UTF8

if ($UseDefaultScenarioInput -or -not (Test-Path -LiteralPath $ScenarioInput)) {
@'
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
    },
    {
      "name": "select_ni_second",
      "actions": [
        { "type": "input", "text": "ni" },
        { "type": "snapshot", "label": "before_select" },
        { "type": "key", "keycode": 50, "mask": 0, "label": "after_select_2" }
      ]
    },
    {
      "name": "commit_ni_space",
      "actions": [
        { "type": "input", "text": "ni" },
        { "type": "snapshot", "label": "before_space" },
        { "type": "key", "keycode": 32, "mask": 0, "label": "after_space" }
      ]
    },
    {
      "name": "reverse_lookup_h",
      "actions": [
        { "type": "input", "text": "`h" },
        { "type": "snapshot", "label": "prefix_h" }
      ]
    },
    {
      "name": "reverse_lookup_hs",
      "actions": [
        { "type": "input", "text": "`hs" },
        { "type": "snapshot", "label": "prefix_hs" }
      ]
    },
    {
      "name": "reverse_lookup_no_result",
      "actions": [
        { "type": "input", "text": "`q" },
        { "type": "snapshot", "label": "no_result" }
      ]
    },
    {
      "name": "punctuation_period",
      "actions": [
        { "type": "key", "keycode": 46, "mask": 0, "label": "period_commit" }
      ]
    },
    {
      "name": "symbol_fh",
      "actions": [
        { "type": "input", "text": "/fh" },
        { "type": "snapshot", "label": "symbols" }
      ]
    },
    {
      "name": "symbol_no_match",
      "actions": [
        { "type": "input", "text": "/notasymbol" },
        { "type": "snapshot", "label": "no_match" }
      ]
    },
    {
      "name": "option_zh_hans_off",
      "actions": [
        { "type": "set_option", "option": "zh_hans", "value": 0 },
        { "type": "input", "text": "zhongguo" },
        { "type": "snapshot", "label": "traditional" }
      ]
    },
    {
      "name": "option_zh_hans_on",
      "actions": [
        { "type": "set_option", "option": "zh_hans", "value": 1 },
        { "type": "input", "text": "zhongguo" },
        { "type": "snapshot", "label": "simplified" }
      ]
    },
    {
      "name": "option_zh_hans_single_off",
      "actions": [
        { "type": "set_option", "option": "zh_hans", "value": 0 },
        { "type": "input", "text": "guo" },
        { "type": "snapshot", "label": "traditional_single" }
      ]
    },
    {
      "name": "option_zh_hans_single_on",
      "actions": [
        { "type": "set_option", "option": "zh_hans", "value": 1 },
        { "type": "input", "text": "guo" },
        { "type": "snapshot", "label": "simplified_single" }
      ]
    },
    {
      "name": "option_ascii_punct_on",
      "actions": [
        { "type": "set_option", "option": "ascii_punct", "value": 1 },
        { "type": "key", "keycode": 46, "mask": 0, "label": "ascii_period" },
        { "type": "snapshot", "label": "ascii_period_snapshot" }
      ]
    },
    {
      "name": "option_full_shape_on",
      "actions": [
        { "type": "set_option", "option": "full_shape", "value": 1 },
        { "type": "key", "keycode": 47, "mask": 0, "label": "full_shape_slash" },
        { "type": "snapshot", "label": "full_shape_slash_snapshot" }
      ]
    }
  ]
}
'@ | Set-Content -LiteralPath $ScenarioInput -Encoding UTF8
}

$ScenarioDoc = Get-Content -LiteralPath $ScenarioInput -Raw | ConvertFrom-Json
$ScenarioObjects = foreach ($Scenario in $ScenarioDoc.scenarios) {
    $ScenarioObject = [RimeProbe+ProbeScenario]::new()
    $ScenarioObject.name = [string]$Scenario.name
    $ActionObjects = foreach ($Action in $Scenario.actions) {
        $ActionObject = [RimeProbe+ProbeAction]::new()
        $ActionObject.type = [string]$Action.type
        $ActionObject.text = [string]$Action.text
        $ActionObject.keycode = [int]$Action.keycode
        $ActionObject.mask = [int]$Action.mask
        $ActionObject.option = [string]$Action.option
        $ActionObject.value = [int]$Action.value
        $ActionObject.label = [string]$Action.label
        $ActionObject
    }
    $ScenarioObject.actions = [RimeProbe+ProbeAction[]]$ActionObjects
    $ScenarioObject
}
$ScenarioCases = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "luna_pinyin", $Modules, [RimeProbe+ProbeScenario[]]$ScenarioObjects)
$ScenarioCasesJson = Join-Path $OracleRoot "luna-pinyin-scenario-snapshots.json"
$ScenarioCases | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $ScenarioCasesJson -Encoding UTF8

$Composer = Join-Path $OracleRoot "compose-luna-pinyin-fixture.js"
@'
const fs = require('fs');
const path = require('path');
const cp = require('child_process');

const root = process.env.ORACLE_ROOT;
const output = process.env.OUTPUT;
const selectionOutput = process.env.OUTPUT_SELECTION;
const actionsOutput = process.env.OUTPUT_ACTIONS;
const reverseOutput = process.env.OUTPUT_REVERSE;
const punctuationOutput = process.env.OUTPUT_PUNCTUATION;
const optionsOutput = process.env.OUTPUT_OPTIONS;
const readUtf8 = (file) => fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
const gitHead = (rel) => cp.execFileSync('git', ['-C', path.join(root, rel), 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const rowsForTerms = (file, terms) => readUtf8(file)
  .split(/\r?\n/)
  .filter(Boolean)
  .filter((line) => terms.has(line.split('\t')[0]));
const rowsForExactCode = (file, code) => readUtf8(file)
  .split(/\r?\n/)
  .filter(Boolean)
  .filter((line) => {
    const fields = line.split('\t');
    return fields.length >= 2 && fields[1] === code;
  });
const termsFromRows = (rows) => new Set(rows.map((line) => line.split('\t')[0]));
const termsFromSnapshots = (snapshots) => {
  const terms = new Set();
  for (const snapshot of snapshots) {
    for (const candidate of snapshot.selected_candidates || []) {
      if (candidate.text) terms.add(candidate.text);
    }
    if (snapshot.commit_text) terms.add(snapshot.commit_text);
  }
  return terms;
};
const writeJson = (file, value) => {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
};
const oracle = {
  engine: 'rime/librime',
  engine_tag: '1.17.0',
  engine_commit: '33e78140250125871856cdc5b42ddc6a5fcd3cd4',
  release_url: 'https://github.com/rime/librime/releases/tag/1.17.0',
  binary_assets: [
    'rime-33e7814-Windows-msvc-x64.7z',
    'rime-deps-33e7814-Windows-msvc-x64.7z'
  ],
  capture_date: '2026-06-19',
  capture_command: 'powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-luna-pinyin.ps1 -OracleRoot target/upstream-oracle/1.17.0 -Output crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-basic.json'
};
const dependencyRepositories = {
  'rime/rime-prelude': gitHead('schema-src/rime-prelude'),
  'rime/rime-essay': gitHead('schema-src/rime-essay'),
  'rime/rime-stroke': gitHead('schema-src/rime-stroke')
};
const lunaDict = path.join(root, 'schema-src/rime-luna-pinyin/luna_pinyin.dict.yaml');
const essayTxt = path.join(root, 'schema-src/rime-essay/essay.txt');
const strokeDict = path.join(root, 'schema-src/rime-stroke/stroke.dict.yaml');
const symbolsYaml = path.join(root, 'schema-src/rime-prelude/symbols.yaml');

const cases = JSON.parse(readUtf8(path.join(root, 'luna-pinyin-capture-cases.json')));
const scenarioInput = JSON.parse(readUtf8(path.join(root, 'luna-pinyin-scenarios.json')));
const scenarioSnapshots = JSON.parse(readUtf8(path.join(root, 'luna-pinyin-scenario-snapshots.json')));
const terms = new Set();
for (const testCase of cases) {
  for (const candidate of testCase.selected_candidates) {
    terms.add(candidate.text);
  }
}

const fixture = {
  oracle,
  schema: 'luna_pinyin',
  module_list: ['default'],
  input_sequence: ['ni', 'hao', 'zhong', 'guo', 'zhongguo'],
  capture: {
    schema_data: 'rime/rime-luna-pinyin',
    schema_data_commit: gitHead('schema-src/rime-luna-pinyin'),
    dependency_repositories: dependencyRepositories,
    dictionary: 'luna_pinyin.dict.yaml',
    vocabulary: 'essay.txt',
    source_row_policy: 'curated_oracle_winners',
    source_dictionary_rows: rowsForTerms(lunaDict, terms),
    source_vocabulary_rows: rowsForTerms(essayTxt, terms)
  },
  cases
};

writeJson(output, fixture);

const niCase = cases.find((testCase) => testCase.input === 'ni');
const dictionaryRowsForNi = rowsForExactCode(lunaDict, 'ni');
const selectionTerms = termsFromRows(dictionaryRowsForNi);
const essayRowsForSelection = rowsForTerms(essayTxt, selectionTerms);
const essayTerms = termsFromRows(essayRowsForSelection);
const pageOneTermsWithoutEssay = (niCase.selected_candidates || [])
  .map((candidate) => candidate.text)
  .filter((text) => !essayTerms.has(text))
  .map((text) => ({ text, reason: 'not present in pinned rime-essay/essay.txt' }));
writeJson(selectionOutput, {
  oracle,
  schema: 'luna_pinyin',
  module_list: ['default'],
  case_name: 'single_code_ni_full_dictionary_selection',
  input_sequence: ['ni'],
  capture: {
    schema_data: 'rime/rime-luna-pinyin',
    schema_data_commit: gitHead('schema-src/rime-luna-pinyin'),
    dependency_repositories: dependencyRepositories,
    dictionary: 'luna_pinyin.dict.yaml',
    vocabulary: 'essay.txt',
    source_row_policy: 'all_rows_for_exact_code_plus_relevant_essay_rows',
    tested_code: 'ni',
    source_dictionary_file: 'rime-luna-pinyin/luna_pinyin.dict.yaml',
    essay_vocabulary_file: 'rime-essay/essay.txt',
    in_scope_candidate_texts: Array.from(selectionTerms).sort(),
    source_row_counts: {
      dictionary: dictionaryRowsForNi.length,
      essay: essayRowsForSelection.length
    },
    source_dictionary_rows_all_for_code: dictionaryRowsForNi,
    essay_vocabulary_rows_for_candidates: essayRowsForSelection,
    essay_row_absent: pageOneTermsWithoutEssay
  },
  cases: [niCase]
});

const fixtureForSnapshots = (snapshots, sourceRowPolicy, extraCapture = {}) => ({
  oracle,
  schema: 'luna_pinyin',
  module_list: ['default'],
  scenarios: scenarioInput.scenarios.filter((scenario) =>
    snapshots.some((snapshot) => snapshot.scenario === scenario.name)
  ),
  capture: {
    schema_data: 'rime/rime-luna-pinyin',
    schema_data_commit: gitHead('schema-src/rime-luna-pinyin'),
    dependency_repositories: dependencyRepositories,
    source_row_policy: sourceRowPolicy,
    ...extraCapture
  },
  snapshots
});

const actionSnapshots = scenarioSnapshots.filter((snapshot) =>
  ['paging_ni', 'select_ni_second', 'commit_ni_space'].includes(snapshot.scenario)
);
writeJson(actionsOutput, fixtureForSnapshots(actionSnapshots, 'action_sequence_oracle_snapshots', {
  dictionary: 'luna_pinyin.dict.yaml',
  vocabulary: 'essay.txt',
  source_dictionary_rows_all_for_code: dictionaryRowsForNi,
  essay_vocabulary_rows_for_candidates: essayRowsForSelection
}));

const reverseSnapshots = scenarioSnapshots.filter((snapshot) =>
  snapshot.scenario.startsWith('reverse_lookup_')
);
const reverseTerms = termsFromSnapshots(reverseSnapshots);
writeJson(reverseOutput, fixtureForSnapshots(reverseSnapshots, 'curated_reverse_lookup_rows', {
  reverse_lookup_dictionary: 'stroke.dict.yaml',
  reverse_lookup_prefix: '`',
  reverse_lookup_suffix: "'",
  source_stroke_rows: rowsForTerms(strokeDict, reverseTerms),
  source_stroke_vocabulary_rows: rowsForTerms(essayTxt, reverseTerms),
  source_reverse_comment_rows: rowsForTerms(lunaDict, reverseTerms)
}));

const punctuationSnapshots = scenarioSnapshots.filter((snapshot) =>
  snapshot.scenario.startsWith('punctuation_') ||
  snapshot.scenario.startsWith('symbol_')
);
const symbolFhSnapshot = punctuationSnapshots.find((snapshot) =>
  snapshot.scenario === 'symbol_fh' && snapshot.label === 'symbols'
);
const symbolFhEntries = (symbolFhSnapshot?.selected_candidates || [])
  .map((candidate) => ['/fh', candidate.text])
  .filter((entry) => entry[1]);
writeJson(punctuationOutput, fixtureForSnapshots(punctuationSnapshots, 'curated_symbols_from_pinned_prelude', {
  symbols_file: 'rime-prelude/symbols.yaml',
  source_symbol_lines: readUtf8(symbolsYaml)
    .split(/\r?\n/)
    .filter((line) =>
      line.includes("'.'") ||
      line.includes("'/") ||
      line.includes('/fh') ||
      line.includes('half_shape:') ||
      line.includes('full_shape:') ||
      line.includes('symbols:')
    ),
  punctuation_entries: {
    half_shape: [['.', '\u3002'], ['/', '\u3001']],
    full_shape: [['.', '\u3002'], ['/', '\uff0f']],
    symbols: symbolFhEntries
  }
}));

const optionSnapshots = scenarioSnapshots.filter((snapshot) =>
  snapshot.scenario.startsWith('option_')
);
writeJson(optionsOutput, fixtureForSnapshots(optionSnapshots, 'option_action_sequence_oracle_snapshots', {
  dictionary: 'luna_pinyin.dict.yaml',
  vocabulary: 'essay.txt',
  source_dictionary_rows: rowsForTerms(lunaDict, terms),
  source_vocabulary_rows: rowsForTerms(essayTxt, terms),
  punctuation_entries: {
    half_shape: [['.', '\u3002'], ['/', '\u3001']],
    full_shape: [['.', '\u3002'], ['/', '\uff0f']],
    symbols: symbolFhEntries
  }
}));
'@ | Set-Content -LiteralPath $Composer -Encoding UTF8

$env:ORACLE_ROOT = $OracleRoot
$env:OUTPUT = $Output
$env:OUTPUT_SELECTION = $SelectionOutput
$env:OUTPUT_ACTIONS = $ActionsOutput
$env:OUTPUT_REVERSE = $ReverseOutput
$env:OUTPUT_PUNCTUATION = $PunctuationOutput
$env:OUTPUT_OPTIONS = $OptionsOutput
node $Composer
if ($LASTEXITCODE -ne 0) {
    throw "fixture composer failed with exit code $LASTEXITCODE"
}
Write-Host "Wrote $Output"
