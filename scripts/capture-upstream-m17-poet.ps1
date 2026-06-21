param(
    [string]$OracleRoot,
    [string]$SentenceOutput,
    [string]$LatticeOutput
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)

if ([string]::IsNullOrWhiteSpace($SentenceOutput)) {
    $SentenceOutput = Join-Path $RepoRoot "crates\yune-core\tests\fixtures\upstream-1.17.0\luna-pinyin-sentence.json"
}
if ([string]::IsNullOrWhiteSpace($LatticeOutput)) {
    $LatticeOutput = Join-Path $RepoRoot "crates\yune-core\tests\fixtures\upstream-1.17.0\luna-pinyin-lattice.json"
}
$SentenceOutput = [System.IO.Path]::GetFullPath($SentenceOutput)
$LatticeOutput = [System.IO.Path]::GetFullPath($LatticeOutput)

$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot "rime-shared"
$User = Join-Path $OracleRoot "rime-user"
$Build = Join-Path $User "build"
$SchemaRoot = Join-Path $OracleRoot "schema-src"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"

$RequiredPaths = @(
    (Join-Path $Extract "dist\lib\rime.dll"),
    (Join-Path $Extract "dist\bin\rime_deployer.exe"),
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
    $ResolvedDir = [System.IO.Path]::GetFullPath($Dir)
    if (-not $ResolvedDir.StartsWith($OracleRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
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
Get-ChildItem -LiteralPath (Join-Path $Extract "share\opencc") | ForEach-Object {
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
$Modules = [string[]]@("default")
$Inputs = [string[]]@("zhongguo", "nihao", "woshi", "tiantian", "renmin")
$Cases = [RimeProbe]::Capture($Shared, $User, $Build, "luna_pinyin", $Modules, $Inputs)
$CasesJson = Join-Path $OracleRoot "m17-luna-pinyin-sentence-cases.json"
$Cases | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $CasesJson -Encoding UTF8

$Scenario = [RimeProbe+ProbeScenario]::new()
$Scenario.name = "sentence_lattice_zhongguo"
$Actions = @()
$Action = [RimeProbe+ProbeAction]::new()
$Action.type = "input"
$Action.text = "zhongguo"
$Actions += $Action
$Action = [RimeProbe+ProbeAction]::new()
$Action.type = "snapshot"
$Action.label = "page_1"
$Actions += $Action
$Action = [RimeProbe+ProbeAction]::new()
$Action.type = "key"
$Action.keycode = 65366
$Action.mask = 0
$Action.label = "page_2"
$Actions += $Action
$Action = [RimeProbe+ProbeAction]::new()
$Action.type = "key"
$Action.keycode = 65365
$Action.mask = 0
$Action.label = "page_1_again"
$Actions += $Action
$Scenario.actions = [RimeProbe+ProbeAction[]]$Actions
$Snapshots = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "luna_pinyin", $Modules, [RimeProbe+ProbeScenario[]]@($Scenario))
$SnapshotsJson = Join-Path $OracleRoot "m17-luna-pinyin-lattice-snapshots.json"
$Snapshots | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $SnapshotsJson -Encoding UTF8

$Composer = Join-Path $OracleRoot "compose-m17-luna-pinyin-poet-fixtures.js"
@'
const fs = require('fs');
const path = require('path');
const cp = require('child_process');

const root = process.env.ORACLE_ROOT;
const sentenceOutput = process.env.SENTENCE_OUTPUT;
const latticeOutput = process.env.LATTICE_OUTPUT;
const readUtf8 = (file) => fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
const gitHead = (rel) => cp.execFileSync('git', ['-C', path.join(root, rel), 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const writeJson = (file, value) => {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
};
const rowsForExactCodes = (file, codes) => readUtf8(file)
  .split(/\r?\n/)
  .filter(Boolean)
  .filter((line) => {
    const fields = line.split('\t');
    return fields.length >= 2 && codes.has(fields[1].replace(/\s+/g, ''));
  });
const rowsForTerms = (file, terms) => readUtf8(file)
  .split(/\r?\n/)
  .filter(Boolean)
  .filter((line) => terms.has(line.split('\t')[0]));
const candidateTerms = (items) => {
  const terms = new Set();
  for (const item of items) {
    for (const candidate of item.selected_candidates || []) {
      if (candidate.text) terms.add(candidate.text);
    }
    if (item.commit_text) terms.add(item.commit_text);
  }
  return terms;
};
const observedCodes = new Set([
  'zhong', 'guo', 'gu',
  'ni', 'hao',
  'wo', 'shi',
  'tian',
  'ren', 'min'
]);

const oracle = {
  engine: 'rime/librime',
  engine_tag: '1.17.0',
  engine_commit: '33e78140250125871856cdc5b42ddc6a5fcd3cd4',
  release_url: 'https://github.com/rime/librime/releases/tag/1.17.0',
  binary_assets: [
    'rime-33e7814-Windows-msvc-x64.7z',
    'rime-deps-33e7814-Windows-msvc-x64.7z'
  ],
  capture_date: '2026-06-21',
  capture_command: 'powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-m17-poet.ps1 -OracleRoot target/upstream-oracle/1.17.0'
};
const dependencyRepositories = {
  'rime/rime-prelude': gitHead('schema-src/rime-prelude'),
  'rime/rime-essay': gitHead('schema-src/rime-essay'),
  'rime/rime-stroke': gitHead('schema-src/rime-stroke')
};
const commonCapture = {
  schema_data: 'rime/rime-luna-pinyin',
  schema_data_commit: gitHead('schema-src/rime-luna-pinyin'),
  dependency_repositories: dependencyRepositories,
  dictionary: 'luna_pinyin.dict.yaml',
  vocabulary: 'essay.txt',
  source_dictionary_file: 'rime-luna-pinyin/luna_pinyin.dict.yaml',
  essay_vocabulary_file: 'rime-essay/essay.txt',
  grammar_model: null,
  grammar_fallback_penalty: -13.815510557964274
};
const lunaDict = path.join(root, 'schema-src/rime-luna-pinyin/luna_pinyin.dict.yaml');
const essayTxt = path.join(root, 'schema-src/rime-essay/essay.txt');
const cases = JSON.parse(readUtf8(path.join(root, 'm17-luna-pinyin-sentence-cases.json')));
const snapshots = JSON.parse(readUtf8(path.join(root, 'm17-luna-pinyin-lattice-snapshots.json')));
const terms = new Set([...candidateTerms(cases), ...candidateTerms(snapshots)]);
const dictionaryRows = rowsForExactCodes(lunaDict, observedCodes);
const vocabularyRows = rowsForTerms(essayTxt, terms);

writeJson(sentenceOutput, {
  oracle,
  schema: 'luna_pinyin',
  module_list: ['default'],
  input_sequence: cases.map((testCase) => testCase.input),
  capture: {
    ...commonCapture,
    source_row_policy: 'm17_upstream_luna_sentence_language_model',
    tested_codes: Array.from(observedCodes).sort(),
    in_scope_candidate_texts: Array.from(terms).sort(),
    source_row_counts: {
      dictionary: dictionaryRows.length,
      essay: vocabularyRows.length
    },
    source_dictionary_rows_for_tested_codes: dictionaryRows,
    essay_vocabulary_rows_for_candidates: vocabularyRows
  },
  cases
});

writeJson(latticeOutput, {
  oracle,
  schema: 'luna_pinyin',
  module_list: ['default'],
  scenarios: [
    {
      name: 'sentence_lattice_zhongguo',
      actions: [
        { type: 'input', text: 'zhongguo' },
        { type: 'snapshot', label: 'page_1' },
        { type: 'key', keycode: 65366, mask: 0, label: 'page_2' },
        { type: 'key', keycode: 65365, mask: 0, label: 'page_1_again' }
      ]
    }
  ],
  capture: {
    ...commonCapture,
    source_row_policy: 'm17_upstream_luna_sentence_lattice',
    tested_codes: Array.from(observedCodes).filter((code) => ['zhong', 'guo', 'gu'].includes(code)).sort(),
    in_scope_candidate_texts: Array.from(candidateTerms(snapshots)).sort(),
    source_row_counts: {
      dictionary: rowsForExactCodes(lunaDict, new Set(['zhong', 'guo', 'gu'])).length,
      essay: rowsForTerms(essayTxt, candidateTerms(snapshots)).length
    },
    source_dictionary_rows_for_tested_codes: rowsForExactCodes(lunaDict, new Set(['zhong', 'guo', 'gu'])),
    essay_vocabulary_rows_for_candidates: rowsForTerms(essayTxt, candidateTerms(snapshots))
  },
  snapshots
});
'@ | Set-Content -LiteralPath $Composer -Encoding UTF8

$env:ORACLE_ROOT = $OracleRoot
$env:SENTENCE_OUTPUT = $SentenceOutput
$env:LATTICE_OUTPUT = $LatticeOutput
node $Composer
if ($LASTEXITCODE -ne 0) {
    throw "M17 fixture composer failed with exit code $LASTEXITCODE"
}
Write-Host "Wrote $SentenceOutput"
Write-Host "Wrote $LatticeOutput"
