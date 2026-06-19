param(
    [string]$OracleRoot,
    [string]$Output
)

$ErrorActionPreference = "Stop"

$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot "crates\yune-core\tests\fixtures\upstream-1.17.0\luna-pinyin-basic.json"
}

$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
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
Copy-Item -LiteralPath (Join-Path $Extract "share\opencc\*") -Destination $OpenCcDest -Recurse -Force
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

$Composer = Join-Path $OracleRoot "compose-luna-pinyin-fixture.js"
@'
const fs = require('fs');
const path = require('path');
const cp = require('child_process');

const root = process.env.ORACLE_ROOT;
const output = process.env.OUTPUT;
const readUtf8 = (file) => fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
const gitHead = (rel) => cp.execFileSync('git', ['-C', path.join(root, rel), 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
const rowsForTerms = (file, terms) => readUtf8(file)
  .split(/\r?\n/)
  .filter(Boolean)
  .filter((line) => terms.has(line.split('\t')[0]));

const cases = JSON.parse(readUtf8(path.join(root, 'luna-pinyin-capture-cases.json')));
const terms = new Set();
for (const testCase of cases) {
  for (const candidate of testCase.selected_candidates) {
    terms.add(candidate.text);
  }
}

const fixture = {
  oracle: {
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
  },
  schema: 'luna_pinyin',
  module_list: ['default'],
  input_sequence: ['ni', 'hao', 'zhong', 'guo', 'zhongguo'],
  capture: {
    schema_data: 'rime/rime-luna-pinyin',
    schema_data_commit: gitHead('schema-src/rime-luna-pinyin'),
    dependency_repositories: {
      'rime/rime-prelude': gitHead('schema-src/rime-prelude'),
      'rime/rime-essay': gitHead('schema-src/rime-essay'),
      'rime/rime-stroke': gitHead('schema-src/rime-stroke')
    },
    dictionary: 'luna_pinyin.dict.yaml',
    vocabulary: 'essay.txt',
    source_dictionary_rows: rowsForTerms(path.join(root, 'schema-src/rime-luna-pinyin/luna_pinyin.dict.yaml'), terms),
    source_vocabulary_rows: rowsForTerms(path.join(root, 'schema-src/rime-essay/essay.txt'), terms)
  },
  cases
};

fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, `${JSON.stringify(fixture, null, 2)}\n`, 'utf8');
'@ | Set-Content -LiteralPath $Composer -Encoding UTF8

$env:ORACLE_ROOT = $OracleRoot
$env:OUTPUT = $Output
node $Composer
Write-Host "Wrote $Output"
