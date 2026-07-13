[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$OutputRoot,

    [Parameter(Mandatory = $true)]
    [string]$WorkRoot,

    [string]$RepoRoot,
    [string]$CargoTargetDir,
    [string]$YuneCli,
    [string]$ModelPath,
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[0-9a-fA-F]{40}$')]
    [string]$ExpectedSourceCommit,
    [string]$ExpectedYuneCliSha256,
    [int]$CaseTimeoutSeconds = 120,
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$ModelUrl = 'https://raw.githubusercontent.com/lotem/rime-octagram-data/bb8e1313552f0f27f2f968031dfaf4563e55d982/zh-hant-t-essay-bgw.gram'
$ModelName = 'zh-hant-t-essay-bgw.gram'
$ExpectedModelBytes = 10513408
$ExpectedModelSha256 = '574c99d100f422766c433c601ed6efd642e881d69a30df9fffb6f1695be550e3'
$FixtureRelativePath = 'crates/yune-core/tests/fixtures/upstream-octagram/lotem-luna-pinyin-octagram.json'
$UserOverrideContent = @'
patch:
  schema_list:
    - schema: luna_pinyin
    - schema: luna_pinyin_octagram
  menu:
    page_size: 6
'@

function Resolve-FullPath([string]$Path) {
    return [System.IO.Path]::GetFullPath($Path)
}

function Get-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Write-Utf8NoBom([string]$Path, [string]$Content) {
    $Encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $Encoding)
}

function ConvertTo-WindowsCommandLineArgument([string]$Value) {
    if ($Value.Length -gt 0 -and $Value -notmatch '[\s"]') {
        return $Value
    }

    # CommandLineToArgvW-compatible quoting for .NET Framework's string-only
    # ProcessStartInfo.Arguments surface. Backslashes must be doubled only when
    # they precede a quote or the closing delimiter.
    $Builder = New-Object System.Text.StringBuilder
    [void]$Builder.Append([char]34)
    $Backslashes = 0
    foreach ($Character in $Value.ToCharArray()) {
        if ($Character -eq [char]92) {
            $Backslashes += 1
            continue
        }
        if ($Character -eq [char]34) {
            [void]$Builder.Append([char]92, (2 * $Backslashes) + 1)
            [void]$Builder.Append([char]34)
            $Backslashes = 0
            continue
        }
        if ($Backslashes -gt 0) {
            [void]$Builder.Append([char]92, $Backslashes)
            $Backslashes = 0
        }
        [void]$Builder.Append($Character)
    }
    if ($Backslashes -gt 0) {
        [void]$Builder.Append([char]92, 2 * $Backslashes)
    }
    [void]$Builder.Append([char]34)
    return $Builder.ToString()
}

function Assert-ProcessSuccess([string]$Label) {
    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

function Get-OracleTop([object[]]$Rows, [string]$InputText) {
    $Row = @($Rows | Where-Object { $_.input -eq $InputText })
    if ($Row.Count -ne 1) {
        throw "expected exactly one oracle row for $InputText, found $($Row.Count)"
    }
    $Candidates = @($Row[0].selected_candidates)
    if ($Candidates.Count -eq 0 -or [string]::IsNullOrEmpty([string]$Candidates[0].text)) {
        throw "oracle row for $InputText has no top candidate"
    }
    return [string]$Candidates[0].text
}

if (-not $RepoRoot) {
    $RepoRoot = Split-Path -Parent $PSScriptRoot
}
$RepoRoot = Resolve-FullPath $RepoRoot
$RepoRoot = $RepoRoot.TrimEnd([char[]]@(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
))
$OutputRoot = Resolve-FullPath $OutputRoot
$WorkRoot = Resolve-FullPath $WorkRoot
$RepoPrefix = $RepoRoot + [System.IO.Path]::DirectorySeparatorChar
foreach ($ScratchPath in @($OutputRoot, $WorkRoot)) {
    if ($ScratchPath -ieq $RepoRoot -or $ScratchPath.StartsWith($RepoPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "WEB-04 output and work roots must stay outside the tracked repository: $ScratchPath"
    }
}
$SourceCommit = (& git -C $RepoRoot rev-parse HEAD).Trim()
Assert-ProcessSuccess 'git rev-parse HEAD'
if ($SourceCommit -ne $ExpectedSourceCommit) {
    throw "source commit mismatch: expected $ExpectedSourceCommit, found $SourceCommit"
}
$GitStatus = @(& git -C $RepoRoot status --porcelain=v1)
Assert-ProcessSuccess 'git status'
if ($GitStatus.Count -ne 0) {
    throw "source worktree must be clean at $ExpectedSourceCommit; status: $($GitStatus -join '; ')"
}

$SuppliedYuneCli = -not [string]::IsNullOrWhiteSpace($YuneCli)
$ReusedYuneCli = $SuppliedYuneCli -or $SkipBuild.IsPresent
if ($ReusedYuneCli -and [string]::IsNullOrWhiteSpace($ExpectedYuneCliSha256)) {
    throw '-ExpectedYuneCliSha256 is required when -YuneCli or -SkipBuild reuses an existing executable'
}
if (-not [string]::IsNullOrWhiteSpace($ExpectedYuneCliSha256)) {
    $ExpectedYuneCliSha256 = $ExpectedYuneCliSha256.Trim().ToLowerInvariant()
    if ($ExpectedYuneCliSha256 -notmatch '^[0-9a-f]{64}$') {
        throw '-ExpectedYuneCliSha256 must be exactly 64 hexadecimal characters'
    }
}
$YuneCliProvenanceMode = if ($ReusedYuneCli) { 'reused-expected' } else { 'source-built-release' }
$YuneCliCargoProfile = if ($ReusedYuneCli) { $null } else { 'release' }

New-Item -ItemType Directory -Path $OutputRoot -Force | Out-Null
New-Item -ItemType Directory -Path $WorkRoot -Force | Out-Null
$RunRoot = Join-Path $WorkRoot ("run-{0}" -f ([guid]::NewGuid().ToString('N')))
$SharedRoot = Join-Path $RunRoot 'shared'
$UserRoot = Join-Path $RunRoot 'users'
New-Item -ItemType Directory -Path $SharedRoot -Force | Out-Null
New-Item -ItemType Directory -Path $UserRoot -Force | Out-Null

if (-not $YuneCli) {
    if (-not $CargoTargetDir) {
        throw 'pass -CargoTargetDir when -YuneCli is not supplied; repository-local target output is forbidden'
    }
    $CargoTargetDir = Resolve-FullPath $CargoTargetDir
    if (-not $SkipBuild) {
        $PreviousTarget = $env:CARGO_TARGET_DIR
        try {
            $env:CARGO_TARGET_DIR = $CargoTargetDir
            & cargo build --manifest-path (Join-Path $RepoRoot 'Cargo.toml') --release -p yune-cli
            Assert-ProcessSuccess 'cargo build --release -p yune-cli'
        }
        finally {
            $env:CARGO_TARGET_DIR = $PreviousTarget
        }
    }
    $YuneCli = Join-Path $CargoTargetDir 'release/yune-cli.exe'
}
$YuneCli = Resolve-FullPath $YuneCli
if (-not (Test-Path -LiteralPath $YuneCli -PathType Leaf)) {
    throw "yune-cli executable not found: $YuneCli"
}
$YuneCliSha256 = Get-Sha256 $YuneCli
if ($ExpectedYuneCliSha256 -and $YuneCliSha256 -ne $ExpectedYuneCliSha256) {
    throw "yune-cli sha256 mismatch: expected $ExpectedYuneCliSha256, found $YuneCliSha256"
}

$SchemaRoot = Join-Path $RepoRoot 'apps/yune-web/public/schema'
if (-not (Test-Path -LiteralPath $SchemaRoot -PathType Container)) {
    throw "tracked schema root not found: $SchemaRoot"
}
foreach ($Item in Get-ChildItem -LiteralPath $SchemaRoot -Force) {
    Copy-Item -LiteralPath $Item.FullName -Destination $SharedRoot -Recurse -Force
}

if ($ModelPath) {
    $ModelPath = Resolve-FullPath $ModelPath
    if (-not (Test-Path -LiteralPath $ModelPath -PathType Leaf)) {
        throw "model path not found: $ModelPath"
    }
}
else {
    $ExternalRoot = Join-Path $RunRoot 'external'
    New-Item -ItemType Directory -Path $ExternalRoot -Force | Out-Null
    $ModelPath = Join-Path $ExternalRoot $ModelName
    Invoke-WebRequest -UseBasicParsing -Uri $ModelUrl -OutFile $ModelPath
}
$ModelInfo = Get-Item -LiteralPath $ModelPath
$ModelSha256 = Get-Sha256 $ModelPath
if ($ModelInfo.Length -ne $ExpectedModelBytes -or $ModelSha256 -ne $ExpectedModelSha256) {
    throw "pinned model verification failed: bytes $($ModelInfo.Length)/$ExpectedModelBytes, sha256 $ModelSha256/$ExpectedModelSha256"
}
Copy-Item -LiteralPath $ModelPath -Destination (Join-Path $SharedRoot $ModelName) -Force

$FixturePath = Join-Path $RepoRoot $FixtureRelativePath
$FixtureSha256 = Get-Sha256 $FixturePath
$Fixture = Get-Content -LiteralPath $FixturePath -Raw -Encoding UTF8 | ConvertFrom-Json
$Inputs = @($Fixture.observed_octagram_differences)
if ($Inputs.Count -ne 4) {
    throw "expected four observed WEB-04 differences, found $($Inputs.Count)"
}
if ($CaseTimeoutSeconds -le 0) {
    throw '-CaseTimeoutSeconds must be positive'
}

$Results = New-Object System.Collections.Generic.List[object]
foreach ($InputText in $Inputs) {
    foreach ($Lane in @(
        [pscustomobject]@{ Name = 'plain-null'; SchemaId = 'luna_pinyin'; Expected = (Get-OracleTop @($Fixture.null_grammar_control) $InputText) },
        [pscustomobject]@{ Name = 'octagram'; SchemaId = 'luna_pinyin_octagram'; Expected = (Get-OracleTop @($Fixture.cases) $InputText) }
    )) {
        # One fresh user root per lane pays deployment once, then reuses the
        # exact deployed bytes for every captured input. No candidate is
        # selected, so these read-only sequences do not mutate user history.
        $CaseUserRoot = Join-Path $UserRoot $Lane.Name
        New-Item -ItemType Directory -Path (Join-Path $CaseUserRoot 'build') -Force | Out-Null
        # Rime reads custom overrides from the user root, not shared data. Keep
        # the diagnostic lane intentionally narrow while deploying both the
        # plain/null control and its Octagram sibling.
        Write-Utf8NoBom (Join-Path $CaseUserRoot 'default.custom.yaml') $UserOverrideContent
        $Arguments = @(
            'frontend',
            '--shared-data-dir', $SharedRoot,
            '--user-data-dir', $CaseUserRoot,
            '--schema', $Lane.SchemaId,
            '--sequence', $InputText,
            '--output', 'json'
        )
        $CaseLogRoot = Join-Path $RunRoot ("logs/{0}-{1}" -f $Lane.Name, $InputText)
        New-Item -ItemType Directory -Path $CaseLogRoot -Force | Out-Null
        $StdoutPath = Join-Path $CaseLogRoot 'stdout.json'
        $StderrPath = Join-Path $CaseLogRoot 'stderr.txt'
        Write-Host ("WEB-04 native case start: {0}/{1}" -f $Lane.Name, $InputText)
        $StartInfo = New-Object System.Diagnostics.ProcessStartInfo
        $StartInfo.FileName = $YuneCli
        $StartInfo.Arguments = (($Arguments | ForEach-Object {
            ConvertTo-WindowsCommandLineArgument ([string]$_)
        }) -join ' ')
        $StartInfo.UseShellExecute = $false
        $StartInfo.CreateNoWindow = $true
        $StartInfo.RedirectStandardOutput = $true
        $StartInfo.RedirectStandardError = $true
        $Process = New-Object System.Diagnostics.Process
        $Process.StartInfo = $StartInfo
        if (-not $Process.Start()) {
            throw "native WEB-04 $($Lane.Name) $InputText failed to start"
        }
        $StdoutTask = $Process.StandardOutput.ReadToEndAsync()
        $StderrTask = $Process.StandardError.ReadToEndAsync()
        if (-not $Process.WaitForExit($CaseTimeoutSeconds * 1000)) {
            $Process.Kill()
            $Process.WaitForExit()
            throw "native WEB-04 $($Lane.Name) $InputText exceeded ${CaseTimeoutSeconds}s"
        }
        $Process.WaitForExit()
        $ExitCode = $Process.ExitCode
        $TranscriptText = $StdoutTask.GetAwaiter().GetResult()
        $NativeError = $StderrTask.GetAwaiter().GetResult()
        Write-Utf8NoBom $StdoutPath $TranscriptText
        Write-Utf8NoBom $StderrPath $NativeError
        if ($ExitCode -ne 0) {
            throw "native WEB-04 $($Lane.Name) $InputText failed with exit code ${ExitCode}: $NativeError"
        }
        try {
            $Transcript = $TranscriptText | ConvertFrom-Json
        }
        catch {
            throw "native WEB-04 $($Lane.Name) $InputText emitted invalid JSON: $($_.Exception.Message)"
        }
        if ([string]$Transcript.schema_id -cne $Lane.SchemaId) {
            throw "native WEB-04 $($Lane.Name) $InputText selected schema $($Transcript.schema_id), expected $($Lane.SchemaId)"
        }
        if ([int]$Transcript.context.page_size -le 0) {
            throw "native WEB-04 $($Lane.Name) $InputText returned page_size=$($Transcript.context.page_size); schema was not deployed"
        }
        $Candidates = @($Transcript.context.candidates)
        $Actual = if ($Candidates.Count -gt 0) { [string]$Candidates[0].text } else { '' }
        $Results.Add([pscustomobject]@{
            lane = $Lane.Name
            schema_id = $Lane.SchemaId
            input = $InputText
            expected_top = $Lane.Expected
            actual_top = $Actual
            verdict = if ($Actual -ceq $Lane.Expected) { 'pass' } else { 'fail' }
            candidate_count = $Candidates.Count
        })
        Write-Host ("WEB-04 native case {0}: {1}/{2} expected={3} actual={4}" -f `
            $Results[$Results.Count - 1].verdict, $Lane.Name, $InputText, $Lane.Expected, $Actual)
    }
}

$ResultArray = $Results.ToArray()
$Failures = @($ResultArray | Where-Object { $_.verdict -ne 'pass' })
$Evidence = [ordered]@{
    gate = 'M59 Increment 4e native WEB-04 plain/null and Octagram top-candidate parity'
    verdict = if ($Failures.Count -eq 0) { 'pass' } else { 'fail' }
    source_commit = $SourceCommit
    source_clean = $true
    source_dirty = $false
    source_status = @($GitStatus)
    yune_cli = [ordered]@{
        path = $YuneCli
        sha256 = $YuneCliSha256
        expected_sha256 = if ($ExpectedYuneCliSha256) { $ExpectedYuneCliSha256 } else { $null }
        provenance_mode = $YuneCliProvenanceMode
        cargo_profile = $YuneCliCargoProfile
    }
    oracle_fixture = [ordered]@{
        path = $FixtureRelativePath
        sha256 = $FixtureSha256
        engine_commit = [string]$Fixture.oracle.engine_commit
        octagram_plugin_commit = [string]$Fixture.oracle.octagram_plugin_commit
    }
    grammar_model = [ordered]@{
        source_url = $ModelUrl
        source_commit = 'bb8e1313552f0f27f2f968031dfaf4563e55d982'
        file = $ModelName
        bytes = $ModelInfo.Length
        sha256 = $ModelSha256
        vendored = $false
    }
    tracked_schema_root = 'apps/yune-web/public/schema'
    user_root_policy = 'one fresh root per lane, reused across four non-selecting input sequences'
    user_override = [ordered]@{
        logical_path = 'default.custom.yaml'
        content = $UserOverrideContent
        schema_ids = @('luna_pinyin', 'luna_pinyin_octagram')
    }
    work_root = $RunRoot
    results = $ResultArray
    failure_count = $Failures.Count
}

$JsonPath = Join-Path $OutputRoot 'web04-native-verdict.json'
$CsvPath = Join-Path $OutputRoot 'web04-native-results.csv'
Write-Utf8NoBom $JsonPath (($Evidence | ConvertTo-Json -Depth 10) + [Environment]::NewLine)
$ResultArray | Export-Csv -LiteralPath $CsvPath -NoTypeInformation -Encoding UTF8

Write-Host "WEB-04 native verdict: $($Evidence.verdict); failures=$($Failures.Count); json=$JsonPath"
if ($Failures.Count -ne 0) {
    foreach ($Failure in $Failures) {
        Write-Error ("{0}/{1}: expected {2}, actual {3}" -f $Failure.lane, $Failure.input, $Failure.expected_top, $Failure.actual_top)
    }
    exit 1
}
