param(
    [string]$OracleRoot,
    [Parameter(Mandatory = $true)]
    [string]$SchemaId,
    [string]$SchemaDataRepo,
    [string[]]$DependencyRepo,
    [string[]]$InputSequence,
    [string]$Output,
    [string]$SourceRowPolicy
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}

function Default-SchemaDataRepo($Schema) {
    switch ($Schema) {
        "luna_pinyin" { "rime/rime-luna-pinyin"; break }
        "double_pinyin" { "rime/rime-double-pinyin"; break }
        "cangjie5" { "rime/rime-cangjie"; break }
        "bopomofo" { "rime/rime-bopomofo"; break }
        default { throw "No default schema-data repository for schema '$Schema'." }
    }
}

function Default-Inputs($Schema) {
    switch ($Schema) {
        "luna_pinyin" { @("ni", "hao", "zhong", "guo", "zhongguo"); break }
        "double_pinyin" { @("ni", "hk", "vs", "go"); break }
        "cangjie5" { @("a", "am", "amd"); break }
        "bopomofo" { @("su3", "cl3", "j06", "w/4"); break }
        default { throw "No default input sequence for schema '$Schema'." }
    }
}

function Default-Dependencies($Schema) {
    switch ($Schema) {
        "luna_pinyin" { @("rime/rime-prelude", "rime/rime-essay", "rime/rime-stroke"); break }
        "double_pinyin" { @("rime/rime-prelude", "rime/rime-essay", "rime/rime-luna-pinyin", "rime/rime-stroke"); break }
        "cangjie5" { @("rime/rime-prelude", "rime/rime-essay", "rime/rime-luna-pinyin"); break }
        "bopomofo" { @("rime/rime-prelude", "rime/rime-essay", "rime/rime-terra-pinyin", "rime/rime-stroke"); break }
        default { @() }
    }
}

function Fixture-Name($Schema) {
    switch ($Schema) {
        "luna_pinyin" { "luna-pinyin-basic.json"; break }
        "double_pinyin" { "double-pinyin-basic.json"; break }
        "cangjie5" { "cangjie5-basic.json"; break }
        "bopomofo" { "bopomofo-basic.json"; break }
        default { "$($Schema.Replace('_', '-'))-basic.json" }
    }
}

if ([string]::IsNullOrWhiteSpace($SchemaDataRepo)) {
    $SchemaDataRepo = Default-SchemaDataRepo $SchemaId
}
if ($null -eq $DependencyRepo -or $DependencyRepo.Count -eq 0) {
    $DependencyRepo = Default-Dependencies $SchemaId
}
if ($null -eq $InputSequence -or $InputSequence.Count -eq 0) {
    $InputSequence = Default-Inputs $SchemaId
}
if ($InputSequence.Count -eq 1 -and $InputSequence[0].Contains(",")) {
    $InputSequence = $InputSequence[0].Split(",") |
        ForEach-Object { $_.Trim() } |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot ("crates\yune-core\tests\fixtures\upstream-1.17.0\" + (Fixture-Name $SchemaId))
}
if ([string]::IsNullOrWhiteSpace($SourceRowPolicy)) {
    $SourceRowPolicy = "m19_${SchemaId}_curated_oracle_winners"
}

$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
$Extract = Join-Path $OracleRoot "extract"
$Shared = Join-Path $OracleRoot ("m19-" + $SchemaId.Replace("_", "-") + "-shared")
$User = Join-Path $OracleRoot ("m19-" + $SchemaId.Replace("_", "-") + "-user")
$Build = Join-Path $User "build"
$SchemaRoot = Join-Path $OracleRoot "schema-src"
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"
$RepoFolder = $SchemaDataRepo.Split("/")[-1]
$SchemaRepoPath = Join-Path $SchemaRoot $RepoFolder

$RequiredPaths = @(
    (Join-Path $Extract "dist\lib\rime.dll"),
    (Join-Path $Extract "dist\bin\rime_deployer.exe"),
    (Join-Path $Extract "dist\include\rime_api.h"),
    $ProbeSource,
    $SchemaRepoPath
)
foreach ($Repo in $DependencyRepo) {
    $RequiredPaths += Join-Path $SchemaRoot ($Repo.Split("/")[-1])
}
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

$ReposToCopy = @($SchemaDataRepo) + $DependencyRepo | Select-Object -Unique
foreach ($Repo in $ReposToCopy) {
    $Source = Join-Path $SchemaRoot ($Repo.Split("/")[-1])
    Get-ChildItem -LiteralPath $Source -File |
        Where-Object { $_.Name -like "*.yaml" -or $_.Name -eq "essay.txt" } |
        ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Shared $_.Name) -Force
        }
}

$OpenCcSource = Join-Path $Extract "share\opencc"
if (Test-Path -LiteralPath $OpenCcSource) {
    $OpenCcDest = Join-Path $Shared "opencc"
    New-Item -ItemType Directory -Force -Path $OpenCcDest | Out-Null
    Get-ChildItem -LiteralPath $OpenCcSource | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDest -Recurse -Force
    }
}

@"
patch:
  schema_list:
    - schema: $SchemaId
"@ | Set-Content -LiteralPath (Join-Path $Shared "default.custom.yaml") -Encoding UTF8

New-Item -ItemType Directory -Force -Path $Build | Out-Null
$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
& (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}

Add-Type -Path $ProbeSource
$Modules = [string[]]@("default")
$Cases = [RimeProbe]::Capture($Shared, $User, $Build, $SchemaId, $Modules, [string[]]$InputSequence)
$CasesJson = Join-Path $OracleRoot ("m19-" + $SchemaId.Replace("_", "-") + "-capture-cases.json")
$Cases | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $CasesJson -Encoding UTF8

function New-ProbeAction($Type, $Keycode, $Label) {
    $Action = [RimeProbe+ProbeAction]::new()
    $Action.type = $Type
    $Action.keycode = $Keycode
    $Action.mask = 0
    $Action.label = $Label
    $Action
}

function New-InputAction($Text) {
    $Action = [RimeProbe+ProbeAction]::new()
    $Action.type = "input"
    $Action.text = $Text
    $Action
}

function New-SnapshotAction($Label) {
    $Action = [RimeProbe+ProbeAction]::new()
    $Action.type = "snapshot"
    $Action.label = $Label
    $Action
}

function New-Scenario($Name, $Actions) {
    $Scenario = [RimeProbe+ProbeScenario]::new()
    $Scenario.name = $Name
    $Scenario.actions = [RimeProbe+ProbeAction[]]$Actions
    $Scenario
}

$ActionInput = [string]$InputSequence[0]
$Scenarios = [RimeProbe+ProbeScenario[]]@(
    (New-Scenario "paging_first_input" @(
        (New-InputAction $ActionInput),
        (New-SnapshotAction "page_1"),
        (New-ProbeAction "key" 65366 "page_2"),
        (New-ProbeAction "key" 65365 "page_1_again")
    )),
    (New-Scenario "select_first_input_second" @(
        (New-InputAction $ActionInput),
        (New-SnapshotAction "before_select"),
        (New-ProbeAction "key" 50 "after_select_2")
    )),
    (New-Scenario "commit_first_input_space" @(
        (New-InputAction $ActionInput),
        (New-SnapshotAction "before_space"),
        (New-ProbeAction "key" 32 "after_space")
    ))
)
$Snapshots = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, $SchemaId, $Modules, $Scenarios)
$SnapshotsJson = Join-Path $OracleRoot ("m19-" + $SchemaId.Replace("_", "-") + "-scenario-snapshots.json")
$Snapshots | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $SnapshotsJson -Encoding UTF8

$Composer = Join-Path $OracleRoot ("compose-m19-" + $SchemaId.Replace("_", "-") + "-fixture.js")
@'
const fs = require('fs');
const path = require('path');
const cp = require('child_process');

const root = process.env.ORACLE_ROOT;
const output = process.env.OUTPUT;
const schemaId = process.env.SCHEMA_ID;
const schemaDataRepo = process.env.SCHEMA_DATA_REPO;
const sourceRowPolicy = process.env.SOURCE_ROW_POLICY;
let inputSequence = JSON.parse(process.env.INPUT_SEQUENCE_JSON);
if (!Array.isArray(inputSequence)) inputSequence = [inputSequence];
const dependencyRepos = JSON.parse(process.env.DEPENDENCY_REPOS_JSON);

const readUtf8 = (file) => fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
const gitHead = (rel) => cp.execFileSync('git', ['-C', path.join(root, rel), 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const repoDir = (repo) => `schema-src/${repo.split('/').at(-1)}`;
const repoFile = (repo, file) => path.join(root, repoDir(repo), file);
const bodyRows = (file) => {
  const lines = readUtf8(file).split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === '...');
  return lines.slice(start + 1).filter((line) => line && !line.startsWith('#'));
};
const rowsForTerms = (file, terms) => bodyRows(file)
  .filter((line) => terms.has(line.split('\t')[0]));
const rowsForCodes = (file, codes) => bodyRows(file)
  .filter((line) => {
    const fields = line.split('\t');
    return fields.length >= 2 && codes.has(fields[1]);
  });
const rowsForTermsOrCodes = (file, terms, codes) => {
  const seen = new Set();
  const rows = [];
  for (const row of [...rowsForTerms(file, terms), ...rowsForCodes(file, codes)]) {
    if (!seen.has(row)) {
      seen.add(row);
      rows.push(row);
    }
  }
  return rows;
};
const writeJson = (file, value) => {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
};
const termsFromRecords = (records) => {
  const terms = new Set();
  for (const record of records) {
    for (const candidate of record.selected_candidates || []) {
      if (candidate.text) terms.add(candidate.text);
    }
    if (record.commit_text) terms.add(record.commit_text);
    if (record.commit_text_preview) terms.add(record.commit_text_preview);
  }
  return terms;
};
const termsWithCharacters = (terms) => {
  const expanded = new Set(terms);
  for (const term of terms) {
    for (const ch of Array.from(term)) {
      expanded.add(ch);
    }
  }
  return expanded;
};
const stringListBlock = (file, key) => {
  const lines = readUtf8(file).split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === `${key}:`);
  if (start < 0) return [];
  const indent = lines[start].match(/^\s*/)[0].length;
  const formulas = [];
  for (const line of lines.slice(start + 1)) {
    if (!line.trim()) continue;
    const currentIndent = line.match(/^\s*/)[0].length;
    if (currentIndent <= indent) break;
    const trimmed = line.trim();
    if (trimmed.startsWith('- ')) {
      formulas.push(trimmed.slice(2).replace(/^['"]|['"]$/g, ''));
    }
  }
  return formulas;
};
const zhuyinSection = (file, section) => {
  const lines = readUtf8(file).split(/\r?\n/);
  const start = lines.findIndex((line) => line.trim() === `${section}:`);
  if (start < 0) return [];
  const formulas = [];
  for (const line of lines.slice(start + 1)) {
    if (/^\S/.test(line) && !line.startsWith(`${section}:`)) break;
    const trimmed = line.trim();
    if (trimmed.startsWith('- ')) {
      formulas.push(trimmed.slice(2).replace(/^['"]|['"]$/g, '').replace(/\s+#.*$/, ''));
    }
  }
  return formulas;
};

let cases = JSON.parse(readUtf8(path.join(root, `m19-${schemaId.replaceAll('_', '-')}-capture-cases.json`)));
if (!Array.isArray(cases)) cases = [cases];
if (schemaId === 'luna_pinyin') {
  for (const testCase of cases) {
    delete testCase.rime_get_input;
  }
}
const snapshots = JSON.parse(readUtf8(path.join(root, `m19-${schemaId.replaceAll('_', '-')}-scenario-snapshots.json`)));
const terms = termsFromRecords([...cases, ...snapshots]);
const codes = new Set(inputSequence);
const dependencyRepositories = {};
for (const repo of dependencyRepos) {
  dependencyRepositories[repo] = gitHead(repoDir(repo));
}

const schemaRepoDir = repoDir(schemaDataRepo);
const schemaFile = `${schemaId}.schema.yaml`;
let capture = {
  schema_data: schemaDataRepo,
  schema_data_commit: gitHead(schemaRepoDir),
  dependency_repositories: dependencyRepositories,
  source_row_policy: sourceRowPolicy,
  schema_file: `${schemaDataRepo.split('/').at(-1)}/${schemaFile}`,
};

if (schemaId === 'luna_pinyin' || schemaId === 'double_pinyin') {
  const lunaDict = repoFile('rime/rime-luna-pinyin', 'luna_pinyin.dict.yaml');
  const essayTxt = repoFile('rime/rime-essay', 'essay.txt');
  capture = {
    ...capture,
    dictionary: 'luna_pinyin.dict.yaml',
    vocabulary: 'essay.txt',
    source_dictionary_file: 'rime-luna-pinyin/luna_pinyin.dict.yaml',
    essay_vocabulary_file: 'rime-essay/essay.txt',
    source_dictionary_rows: rowsForTermsOrCodes(lunaDict, terms, codes),
    source_vocabulary_rows: rowsForTerms(essayTxt, terms),
    speller_algebra_rules: schemaId === 'double_pinyin'
      ? stringListBlock(path.join(root, schemaRepoDir, schemaFile), 'algebra')
      : [],
  };
} else if (schemaId === 'cangjie5') {
  const dictFiles = ['cangjie5.base.dict.yaml', 'cangjie5.stem.dict.yaml', 'cangjie5.extended.dict.yaml'];
  const essayTxt = repoFile('rime/rime-essay', 'essay.txt');
  const expandedTerms = termsWithCharacters(terms);
  const sourceRows = {};
  for (const file of dictFiles) {
    sourceRows[file] = rowsForTermsOrCodes(repoFile('rime/rime-cangjie', file), expandedTerms, codes);
  }
  capture = {
    ...capture,
    dictionary: 'cangjie5.dict.yaml',
    vocabulary: 'essay.txt',
    source_dictionary_file: 'rime-cangjie/cangjie5.dict.yaml',
    essay_vocabulary_file: 'rime-essay/essay.txt',
    source_dictionary_import_rows: sourceRows,
    source_vocabulary_rows: rowsForTerms(essayTxt, terms),
    translator_comment_format: stringListBlock(path.join(root, schemaRepoDir, schemaFile), 'comment_format'),
    translator_preedit_format: stringListBlock(path.join(root, schemaRepoDir, schemaFile), 'preedit_format'),
  };
} else if (schemaId === 'bopomofo') {
  const terraDict = repoFile('rime/rime-terra-pinyin', 'terra_pinyin.dict.yaml');
  const essayTxt = repoFile('rime/rime-essay', 'essay.txt');
  const zhuyinYaml = repoFile('rime/rime-bopomofo', 'zhuyin.yaml');
  capture = {
    ...capture,
    dictionary: 'terra_pinyin.dict.yaml',
    vocabulary: 'essay.txt',
    source_dictionary_file: 'rime-terra-pinyin/terra_pinyin.dict.yaml',
    essay_vocabulary_file: 'rime-essay/essay.txt',
    source_dictionary_rows: rowsForTerms(terraDict, terms),
    source_vocabulary_rows: rowsForTerms(essayTxt, terms),
    speller_algebra_rules: [
      ...zhuyinSection(zhuyinYaml, 'pinyin_to_zhuyin'),
      ...zhuyinSection(zhuyinYaml, 'free_order'),
      ...zhuyinSection(zhuyinYaml, 'abbreviation'),
      ...zhuyinSection(zhuyinYaml, 'keymap_bopomofo'),
    ],
  };
}

const oracle = {
  engine: 'rime/librime',
  engine_tag: '1.17.0',
  engine_commit: '33e78140250125871856cdc5b42ddc6a5fcd3cd4',
  release_url: 'https://github.com/rime/librime/releases/tag/1.17.0',
  binary_assets: [
    'rime-33e7814-Windows-msvc-x64.7z',
    'rime-deps-33e7814-Windows-msvc-x64.7z',
  ],
  capture_date: '2026-06-21',
  capture_command: `powershell -ExecutionPolicy Bypass -File scripts/capture-upstream-schema.ps1 -OracleRoot target/upstream-oracle/1.17.0 -SchemaId ${schemaId} -SchemaDataRepo ${schemaDataRepo} -InputSequence ${inputSequence.join(',')}`,
};

writeJson(output, {
  oracle,
  schema: schemaId,
  module_list: ['default'],
  input_sequence: inputSequence,
  capture,
  cases,
  snapshots,
});
'@ | Set-Content -LiteralPath $Composer -Encoding UTF8

$env:ORACLE_ROOT = $OracleRoot
$env:OUTPUT = $Output
$env:SCHEMA_ID = $SchemaId
$env:SCHEMA_DATA_REPO = $SchemaDataRepo
$env:SOURCE_ROW_POLICY = $SourceRowPolicy
$env:INPUT_SEQUENCE_JSON = ($InputSequence | ConvertTo-Json -Compress)
$env:DEPENDENCY_REPOS_JSON = ($DependencyRepo | ConvertTo-Json -Compress)
node $Composer
if ($LASTEXITCODE -ne 0) {
    throw "fixture composer failed with exit code $LASTEXITCODE"
}
Write-Host "Wrote $Output"
