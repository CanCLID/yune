param(
    [string]$OracleRoot,
    [Parameter(Mandatory = $true)]
    [string]$SchemaId,
    [string]$SchemaDataRepo,
    [string[]]$DependencyRepo,
    [string[]]$InputSequence,
    [string]$Output,
    [string]$SourceRowPolicy,
    [ValidateSet("m19-component", "m59-whole-input")]
    [string]$CaptureMode = "m19-component",
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d{4}-\d{2}-\d{2}$')]
    [string]$CaptureDate,
    [string]$ExpectedRimeDllSha256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b",
    [string]$ExpectedRimeDeployerSha256 = "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071",
    [string]$ExpectedSchemaDataCommit,
    [string[]]$ExpectedDependencyCommit
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0

$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}

$PinnedRepositoryCommits = [ordered]@{
    "rime/rime-luna-pinyin" = "18a80335c37522311f7cff02886cd81cec3b460a"
    "rime/rime-double-pinyin" = "01a13287cbd27819be1c34fa1ddc1b3643d5001b"
    "rime/rime-cangjie" = "52d90a1b1312e74042b38c1cbc8142defbc53171"
    "rime/rime-bopomofo" = "6085c9a38a4a728047862b33d67eee18aa86f3b9"
    "rime/rime-prelude" = "082425ea0684bca36474415d4a0e8db9b016487e"
    "rime/rime-essay" = "48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed"
    "rime/rime-stroke" = "3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c"
    "rime/rime-terra-pinyin" = "8ddd679e4485e1d2f411e90f104244eddf580382"
}

function Quote-CommandArg([string]$Value) {
    if ($null -eq $Value) {
        return "''"
    }
    return "'" + $Value.Replace("'", "''") + "'"
}

function File-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-NewUtf8NoBom([string]$Path, [string]$Text) {
    $Parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($Parent)) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    $Bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($Text)
    $Stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::Write,
        [System.IO.FileShare]::None
    )
    try {
        $Stream.Write($Bytes, 0, $Bytes.Length)
        $Stream.Flush()
    }
    finally {
        $Stream.Dispose()
    }
}

function Get-GitRepositoryState([string]$Path) {
    $Head = (& git -C $Path rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $Head -notmatch '^[0-9a-fA-F]{40}$') {
        throw "Unable to resolve Git HEAD for $Path"
    }
    $Tree = (& git -C $Path rev-parse 'HEAD^{tree}').Trim()
    if ($LASTEXITCODE -ne 0 -or $Tree -notmatch '^[0-9a-fA-F]{40}$') {
        throw "Unable to resolve Git tree for $Path"
    }
    $Status = @(& git -C $Path status --porcelain=v1 -uall)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to read Git status for $Path"
    }
    return [pscustomobject]@{
        commit = $Head.ToLowerInvariant()
        tree = $Tree.ToLowerInvariant()
        clean = $Status.Count -eq 0
        status = @($Status)
    }
}

function Assert-PinnedGitRepository(
    [string]$Path,
    [string]$Repository,
    [string]$ExpectedCommit
) {
    $State = Get-GitRepositoryState $Path
    if ($State.commit -ne $ExpectedCommit) {
        throw "Upstream repository commit mismatch for ${Repository}: expected $ExpectedCommit, observed $($State.commit)"
    }
    if (-not $State.clean -or $State.status.Count -ne 0) {
        throw "Upstream repository must be clean for capture: $Repository"
    }
    return $State
}

function Assert-GitRepositoryStateUnchanged(
    [string]$Path,
    [string]$Repository,
    [object]$Before
) {
    $After = Get-GitRepositoryState $Path
    if ($After.commit -ne $Before.commit -or
        $After.tree -ne $Before.tree -or
        $After.clean -ne $Before.clean -or
        ((@($After.status) -join "`n") -cne (@($Before.status) -join "`n"))) {
        throw "Upstream repository state changed during capture: $Repository"
    }
}

function Convert-ToEvidencePath([string]$Path, [string]$Role) {
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = $RepoRoot.TrimEnd("\", "/")
    $Prefix = $Root + [System.IO.Path]::DirectorySeparatorChar
    if ($Full.StartsWith($Prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Full.Substring($Prefix.Length).Replace("\", "/")
    }
    return "external/$Role"
}

function Assert-Sha256([string]$Value, [string]$Label) {
    if ($Value -notmatch '^[0-9a-fA-F]{64}$') {
        throw "$Label must be a 64-character hexadecimal SHA-256."
    }
}

function Assert-Commit([string]$Value, [string]$Label) {
    if ($Value -notmatch '^[0-9a-fA-F]{40}$') {
        throw "$Label must be a 40-character hexadecimal Git commit."
    }
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
if ($DependencyRepo.Count -eq 1 -and $DependencyRepo[0].Contains(",")) {
    $DependencyRepo = $DependencyRepo[0].Split(",") |
        ForEach-Object { $_.Trim() } |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
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
    $SourceRowPolicy = if ($CaptureMode -eq "m59-whole-input") {
        "m59_transformed_algebra_whole_input_oracle"
    }
    else {
        "m19_${SchemaId}_curated_oracle_winners"
    }
}
if ($CaptureMode -eq "m59-whole-input" -and $InputSequence.Count -ne 1) {
    throw "m59-whole-input capture requires exactly one InputSequence value."
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
$RimeDll = Join-Path $Extract "dist\lib\rime.dll"
$RimeDeployer = Join-Path $Extract "dist\bin\rime_deployer.exe"
$RimeHeader = Join-Path $Extract "dist\include\rime_api.h"

$ParsedCaptureDate = [datetime]::MinValue
if (-not [datetime]::TryParseExact(
        $CaptureDate,
        "yyyy-MM-dd",
        [System.Globalization.CultureInfo]::InvariantCulture,
        [System.Globalization.DateTimeStyles]::None,
        [ref]$ParsedCaptureDate)) {
    throw "CaptureDate must be a real calendar date in yyyy-MM-dd form."
}
Assert-Sha256 $ExpectedRimeDllSha256 "ExpectedRimeDllSha256"
Assert-Sha256 $ExpectedRimeDeployerSha256 "ExpectedRimeDeployerSha256"
$ExpectedRimeDllSha256 = $ExpectedRimeDllSha256.ToLowerInvariant()
$ExpectedRimeDeployerSha256 = $ExpectedRimeDeployerSha256.ToLowerInvariant()

$ExplicitDependencyCommits = [ordered]@{}
$NormalizedExpectedDependencyCommits = @(
    foreach ($Argument in @($ExpectedDependencyCommit)) {
        if (-not [string]::IsNullOrWhiteSpace($Argument)) {
            $Argument.Split(",") |
                ForEach-Object { $_.Trim() } |
                Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
        }
    }
)
foreach ($Entry in $NormalizedExpectedDependencyCommits) {
    if ([string]::IsNullOrWhiteSpace($Entry)) {
        continue
    }
    $Parts = $Entry.Split([char[]]@('='), 2)
    if ($Parts.Count -ne 2 -or
        [string]::IsNullOrWhiteSpace($Parts[0]) -or
        [string]::IsNullOrWhiteSpace($Parts[1])) {
        throw "ExpectedDependencyCommit entries must use repository=commit form: $Entry"
    }
    $Repository = $Parts[0].Trim()
    $Commit = $Parts[1].Trim().ToLowerInvariant()
    Assert-Commit $Commit "ExpectedDependencyCommit for $Repository"
    if ($ExplicitDependencyCommits.Contains($Repository)) {
        throw "Duplicate ExpectedDependencyCommit entry for $Repository"
    }
    $ExplicitDependencyCommits[$Repository] = $Commit
}

if ([string]::IsNullOrWhiteSpace($ExpectedSchemaDataCommit)) {
    if (-not $PinnedRepositoryCommits.Contains($SchemaDataRepo)) {
        throw "ExpectedSchemaDataCommit is required for unrecognized repository $SchemaDataRepo"
    }
    $ExpectedSchemaDataCommit = $PinnedRepositoryCommits[$SchemaDataRepo]
}
Assert-Commit $ExpectedSchemaDataCommit "ExpectedSchemaDataCommit"
$ExpectedSchemaDataCommit = $ExpectedSchemaDataCommit.ToLowerInvariant()

$ExpectedDependencyCommits = [ordered]@{}
foreach ($Repository in $DependencyRepo) {
    if ($ExpectedDependencyCommits.Contains($Repository)) {
        throw "DependencyRepo contains a duplicate repository: $Repository"
    }
    if ($ExplicitDependencyCommits.Contains($Repository)) {
        $Commit = $ExplicitDependencyCommits[$Repository]
    }
    elseif ($PinnedRepositoryCommits.Contains($Repository)) {
        $Commit = $PinnedRepositoryCommits[$Repository]
    }
    else {
        throw "ExpectedDependencyCommit is required for unrecognized repository $Repository"
    }
    $ExpectedDependencyCommits[$Repository] = $Commit
}
foreach ($Repository in $ExplicitDependencyCommits.Keys) {
    if (-not $ExpectedDependencyCommits.Contains($Repository)) {
        throw "ExpectedDependencyCommit names a repository not present in DependencyRepo: $Repository"
    }
}

if (Test-Path -LiteralPath $Output) {
    throw "Output must not already exist: $Output"
}
$OracleRootPrefix = $OracleRoot.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
if ([string]::Equals($Output, $OracleRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
    $Output.StartsWith($OracleRootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Output must not be inside or equal to OracleRoot: $Output"
}

$RequiredPaths = @(
    $RimeDll,
    $RimeDeployer,
    $RimeHeader,
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

$ObservedRimeDllSha256 = File-Sha256 $RimeDll
$ObservedRimeDeployerSha256 = File-Sha256 $RimeDeployer
if ($ObservedRimeDllSha256 -ne $ExpectedRimeDllSha256) {
    throw "rime.dll SHA-256 mismatch: expected $ExpectedRimeDllSha256, observed $ObservedRimeDllSha256"
}
if ($ObservedRimeDeployerSha256 -ne $ExpectedRimeDeployerSha256) {
    throw "rime_deployer.exe SHA-256 mismatch: expected $ExpectedRimeDeployerSha256, observed $ObservedRimeDeployerSha256"
}

$RepositoryStates = [ordered]@{}
$RepositoryTrees = [ordered]@{}
$RepositoryCommits = [ordered]@{}
$RepositoryClean = [ordered]@{}
$Repositories = @($SchemaDataRepo) + @($DependencyRepo) | Select-Object -Unique
foreach ($Repository in $Repositories) {
    $Path = Join-Path $SchemaRoot ($Repository.Split("/")[-1])
    $ExpectedCommit = if ($Repository -eq $SchemaDataRepo) {
        $ExpectedSchemaDataCommit
    }
    else {
        $ExpectedDependencyCommits[$Repository]
    }
    $State = Assert-PinnedGitRepository $Path $Repository $ExpectedCommit
    $RepositoryStates[$Repository] = $State
    $RepositoryTrees[$Repository] = $State.tree
    $RepositoryCommits[$Repository] = $State.commit
    $RepositoryClean[$Repository] = $true
}

$CaptureScriptSha256 = File-Sha256 $PSCommandPath
$ProbeSha256 = File-Sha256 $ProbeSource
$EvidenceOracleRoot = Convert-ToEvidencePath $OracleRoot "oracle-root"
$EvidenceOutput = Convert-ToEvidencePath $Output "output"
$DependencyArguments = Quote-CommandArg ($DependencyRepo -join ",")
$InputArguments = Quote-CommandArg ($InputSequence -join ",")
$ExpectedDependencyArguments = Quote-CommandArg ((@(
    $DependencyRepo | ForEach-Object {
        "$_=" + $ExpectedDependencyCommits[$_]
    }
) -join ","))
$CaptureCommand = @(
    "powershell.exe -NoProfile -ExecutionPolicy Bypass",
    "-File $(Quote-CommandArg 'scripts/capture-upstream-schema.ps1')",
    "-OracleRoot $(Quote-CommandArg $EvidenceOracleRoot)",
    "-SchemaId $(Quote-CommandArg $SchemaId)",
    "-SchemaDataRepo $(Quote-CommandArg $SchemaDataRepo)",
    "-DependencyRepo $DependencyArguments",
    "-InputSequence $InputArguments",
    "-Output $(Quote-CommandArg $EvidenceOutput)",
    "-SourceRowPolicy $(Quote-CommandArg $SourceRowPolicy)",
    "-CaptureMode $(Quote-CommandArg $CaptureMode)",
    "-CaptureDate $(Quote-CommandArg $CaptureDate)",
    "-ExpectedRimeDllSha256 $(Quote-CommandArg $ExpectedRimeDllSha256)",
    "-ExpectedRimeDeployerSha256 $(Quote-CommandArg $ExpectedRimeDeployerSha256)",
    "-ExpectedSchemaDataCommit $(Quote-CommandArg $ExpectedSchemaDataCommit)",
    "-ExpectedDependencyCommit $ExpectedDependencyArguments"
) -join " "

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

$DefaultCustom = Join-Path $Shared "default.custom.yaml"
Write-NewUtf8NoBom $DefaultCustom "patch:`n  schema_list:`n    - schema: $SchemaId`n"

New-Item -ItemType Directory -Force -Path $Build | Out-Null
$env:PATH = (Join-Path $Extract "dist\lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
& (Join-Path $Extract "dist\bin\rime_deployer.exe") --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}

Add-Type -Path $ProbeSource
$Modules = [string[]]@("default")
$Cases = [RimeProbe]::Capture($Shared, $User, $Build, $SchemaId, $Modules, [string[]]$InputSequence)
$CasesJson = Join-Path $User "capture-cases.json"
$CasesText = (($Cases | ConvertTo-Json -Depth 20).Replace("`r`n", "`n").TrimEnd([char]10)) + "`n"
Write-NewUtf8NoBom $CasesJson $CasesText

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
$ScenarioList = New-Object System.Collections.Generic.List[object]
$ScenarioList.Add((New-Scenario "paging_first_input" @(
    (New-InputAction $ActionInput),
    (New-SnapshotAction "page_1"),
    (New-ProbeAction "key" 65366 "page_2"),
    (New-ProbeAction "key" 65365 "page_1_again")
)))
if ($CaptureMode -eq "m19-component") {
    if ($SchemaId -eq "bopomofo") {
        $ScenarioList.Add((New-Scenario "tone_key_2_after_first_input" @(
            (New-InputAction $ActionInput),
            (New-SnapshotAction "before_tone_key_2"),
            (New-ProbeAction "key" 50 "after_tone_key_2")
        )))
    }
    else {
        $ScenarioList.Add((New-Scenario "select_first_input_second" @(
            (New-InputAction $ActionInput),
            (New-SnapshotAction "before_select"),
            (New-ProbeAction "key" 50 "after_select_2")
        )))
    }
}
$ScenarioList.Add((New-Scenario "commit_first_input_space" @(
    (New-InputAction $ActionInput),
    (New-SnapshotAction "before_space"),
    (New-ProbeAction "key" 32 "after_space")
)))
$Scenarios = [RimeProbe+ProbeScenario[]]$ScenarioList.ToArray()
$EffectiveScenarios = @($Scenarios | ForEach-Object { $_.name })
$Snapshots = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, $SchemaId, $Modules, $Scenarios)
$SnapshotsJson = Join-Path $User "scenario-snapshots.json"
$SnapshotsText = (($Snapshots | ConvertTo-Json -Depth 20).Replace("`r`n", "`n").TrimEnd([char]10)) + "`n"
Write-NewUtf8NoBom $SnapshotsJson $SnapshotsText

$Composer = Join-Path $User "compose-fixture.js"
@'
const fs = require('fs');
const path = require('path');

const root = process.env.ORACLE_ROOT;
const output = process.env.OUTPUT;
const schemaId = process.env.SCHEMA_ID;
const schemaDataRepo = process.env.SCHEMA_DATA_REPO;
const sourceRowPolicy = process.env.SOURCE_ROW_POLICY;
const captureDate = process.env.CAPTURE_DATE;
const captureCommand = process.env.CAPTURE_COMMAND;
const captureMode = process.env.CAPTURE_MODE;
const casesPath = process.env.CASES_PATH;
const snapshotsPath = process.env.SNAPSHOTS_PATH;
const rimeDllSha256 = process.env.RIME_DLL_SHA256;
const rimeDeployerSha256 = process.env.RIME_DEPLOYER_SHA256;
const captureScriptSha256 = process.env.CAPTURE_SCRIPT_SHA256;
const probeSha256 = process.env.PROBE_SHA256;
let inputSequence = JSON.parse(process.env.INPUT_SEQUENCE_JSON);
if (!Array.isArray(inputSequence)) inputSequence = [inputSequence];
const dependencyRepos = JSON.parse(process.env.DEPENDENCY_REPOS_JSON);
const repositoryCommits = JSON.parse(process.env.REPOSITORY_COMMITS_JSON);
const repositoryTrees = JSON.parse(process.env.REPOSITORY_TREES_JSON);
const repositoryClean = JSON.parse(process.env.REPOSITORY_CLEAN_JSON);
const effectiveScenarios = JSON.parse(process.env.EFFECTIVE_SCENARIOS_JSON);

const readUtf8 = (file) => fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
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
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`, { encoding: 'utf8', flag: 'wx' });
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

let cases = JSON.parse(readUtf8(casesPath));
if (!Array.isArray(cases)) cases = [cases];
if (schemaId === 'luna_pinyin') {
  for (const testCase of cases) {
    delete testCase.rime_get_input;
  }
}
const snapshots = JSON.parse(readUtf8(snapshotsPath));
const terms = termsFromRecords([...cases, ...snapshots]);
const sourceRowTerms = captureMode === 'm59-whole-input' ? termsWithCharacters(terms) : terms;
const codes = new Set(inputSequence);
const dependencyRepositories = {};
for (const repo of dependencyRepos) {
  dependencyRepositories[repo] = repositoryCommits[repo];
}

const schemaRepoDir = repoDir(schemaDataRepo);
const schemaFile = `${schemaId}.schema.yaml`;
let capture = {
  schema_data: schemaDataRepo,
  schema_data_commit: repositoryCommits[schemaDataRepo],
  dependency_repositories: dependencyRepositories,
  source_repository_trees: repositoryTrees,
  source_repositories_clean: repositoryClean,
  source_row_policy: sourceRowPolicy,
  schema_file: `${schemaDataRepo.split('/').at(-1)}/${schemaFile}`,
  capture_mode: captureMode,
  effective_scenarios: effectiveScenarios,
  key_event_semantics: 'RimeProbe.Capture sends each input UTF-16 code unit to RimeProcessKey(keycode, 0); digits remain real schema key events (including Bopomofo tone keys), never pre-decoded syllable separators.',
  page_policy: 'RimeProbe.Capture follows Page_Down until last_page; incomplete or non-advancing pagination is fatal.',
  commands: {
    deploy: 'rime_deployer.exe --build disposable/user disposable/shared disposable/user/build',
    capture: `RimeProbe.Capture(schema=${schemaId}, inputs=${JSON.stringify(inputSequence)})`,
  },
  source_row_term_expansion: captureMode === 'm59-whole-input'
    ? 'oracle terms plus Unicode-scalar constituents'
    : 'oracle terms only',
};
let sourceLexiconFile = null;
let sourceVocabularyFile = null;

if (schemaId === 'luna_pinyin' || schemaId === 'double_pinyin') {
  const lunaDict = repoFile('rime/rime-luna-pinyin', 'luna_pinyin.dict.yaml');
  const essayTxt = repoFile('rime/rime-essay', 'essay.txt');
  sourceLexiconFile = lunaDict;
  sourceVocabularyFile = essayTxt;
  capture = {
    ...capture,
    dictionary: 'luna_pinyin.dict.yaml',
    vocabulary: 'essay.txt',
    source_dictionary_file: 'rime-luna-pinyin/luna_pinyin.dict.yaml',
    essay_vocabulary_file: 'rime-essay/essay.txt',
    source_dictionary_rows: rowsForTermsOrCodes(lunaDict, sourceRowTerms, codes),
    source_vocabulary_rows: rowsForTerms(essayTxt, sourceRowTerms),
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
  sourceLexiconFile = terraDict;
  sourceVocabularyFile = essayTxt;
  capture = {
    ...capture,
    dictionary: 'terra_pinyin.dict.yaml',
    vocabulary: 'essay.txt',
    source_dictionary_file: 'rime-terra-pinyin/terra_pinyin.dict.yaml',
    essay_vocabulary_file: 'rime-essay/essay.txt',
    source_dictionary_rows: rowsForTerms(terraDict, sourceRowTerms),
    source_vocabulary_rows: rowsForTerms(essayTxt, sourceRowTerms),
    speller_algebra_rules: [
      ...zhuyinSection(zhuyinYaml, 'pinyin_to_zhuyin'),
      ...zhuyinSection(zhuyinYaml, 'free_order'),
      ...zhuyinSection(zhuyinYaml, 'abbreviation'),
      ...zhuyinSection(zhuyinYaml, 'keymap_bopomofo'),
    ],
  };
}

if (captureMode === 'm59-whole-input') {
  if (cases.length !== 1) {
    throw new Error(`m59-whole-input capture requires exactly one input case, got ${cases.length}`);
  }
  if (!sourceLexiconFile || !sourceVocabularyFile) {
    throw new Error(`m59-whole-input source-lexicon proof is not implemented for ${schemaId}`);
  }
  capture.whole_input_oracle_rows = cases.map((testCase) => {
    const top = (testCase.all_candidates || [])[0];
    if (!top || !top.text) {
      throw new Error(`m59-whole-input capture has no oracle top candidate for ${testCase.input}`);
    }
    const sourceDictionaryExactTermCount = rowsForTerms(sourceLexiconFile, new Set([top.text])).length;
    const sourceVocabularyExactTermCount = rowsForTerms(sourceVocabularyFile, new Set([top.text])).length;
    return {
      input: testCase.input,
      oracle_top: top.text,
      source_dictionary_exact_term_count: sourceDictionaryExactTermCount,
      source_vocabulary_exact_term_count: sourceVocabularyExactTermCount,
      source_lexicon_absent: sourceDictionaryExactTermCount === 0 && sourceVocabularyExactTermCount === 0,
    };
  });
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
  rime_dll_sha256: rimeDllSha256,
  rime_deployer_sha256: rimeDeployerSha256,
  capture_script: 'scripts/capture-upstream-schema.ps1',
  capture_script_sha256: captureScriptSha256,
  probe_source: 'scripts/oracle-rime-probe.cs',
  probe_sha256: probeSha256,
  capture_date: captureDate,
  capture_command: captureCommand,
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

$ComposedOutput = Join-Path $User "composed-fixture.json"
$env:ORACLE_ROOT = $OracleRoot
$env:OUTPUT = $ComposedOutput
$env:SCHEMA_ID = $SchemaId
$env:SCHEMA_DATA_REPO = $SchemaDataRepo
$env:SOURCE_ROW_POLICY = $SourceRowPolicy
$env:CAPTURE_DATE = $CaptureDate
$env:CAPTURE_COMMAND = $CaptureCommand
$env:CAPTURE_MODE = $CaptureMode
$env:CASES_PATH = $CasesJson
$env:SNAPSHOTS_PATH = $SnapshotsJson
$env:RIME_DLL_SHA256 = $ObservedRimeDllSha256
$env:RIME_DEPLOYER_SHA256 = $ObservedRimeDeployerSha256
$env:CAPTURE_SCRIPT_SHA256 = $CaptureScriptSha256
$env:PROBE_SHA256 = $ProbeSha256
$env:INPUT_SEQUENCE_JSON = ($InputSequence | ConvertTo-Json -Compress)
$env:DEPENDENCY_REPOS_JSON = ($DependencyRepo | ConvertTo-Json -Compress)
$env:REPOSITORY_COMMITS_JSON = ($RepositoryCommits | ConvertTo-Json -Compress)
$env:REPOSITORY_TREES_JSON = ($RepositoryTrees | ConvertTo-Json -Compress)
$env:REPOSITORY_CLEAN_JSON = ($RepositoryClean | ConvertTo-Json -Compress)
$env:EFFECTIVE_SCENARIOS_JSON = ($EffectiveScenarios | ConvertTo-Json -Compress)
node $Composer
if ($LASTEXITCODE -ne 0) {
    throw "fixture composer failed with exit code $LASTEXITCODE"
}

foreach ($Repository in $Repositories) {
    $Path = Join-Path $SchemaRoot ($Repository.Split("/")[-1])
    Assert-GitRepositoryStateUnchanged $Path $Repository $RepositoryStates[$Repository]
}
if ((File-Sha256 $RimeDll) -ne $ObservedRimeDllSha256) {
    throw "rime.dll changed during capture."
}
if ((File-Sha256 $RimeDeployer) -ne $ObservedRimeDeployerSha256) {
    throw "rime_deployer.exe changed during capture."
}
if ((File-Sha256 $PSCommandPath) -ne $CaptureScriptSha256) {
    throw "capture-upstream-schema.ps1 changed during capture."
}
if ((File-Sha256 $ProbeSource) -ne $ProbeSha256) {
    throw "oracle-rime-probe.cs changed during capture."
}

$Utf8 = [System.Text.UTF8Encoding]::new($false, $true)
$ComposedText = $Utf8.GetString([System.IO.File]::ReadAllBytes($ComposedOutput))
Write-NewUtf8NoBom $Output $ComposedText
Write-Host "Wrote $Output"
