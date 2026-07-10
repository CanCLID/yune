# Capture complete Yune candidate lists through the librime-shaped native ABI.
# The supplied DLL is copied as rime.dll into an isolated work root, deploys a
# copied shared-data tree, then runs the same all-pages RimeProbe.Capture used by
# the upstream oracle scripts. The output embeds the raw cases plus reproducible
# hashes; no DLL or deployed artifact belongs in tracked evidence.
param(
    [Parameter(Mandatory = $true)]
    [string]$YuneDll,
    [Parameter(Mandatory = $true)]
    [string]$SharedDataDir,
    [Parameter(Mandatory = $true)]
    [string]$SchemaId,
    [Parameter(Mandatory = $true)]
    [string]$OracleCapture,
    [Parameter(Mandatory = $true)]
    [string]$Output,
    [string[]]$Inputs,
    [string]$DefaultYamlOverlay,
    [switch]$NarrowSchemaList,
    [string]$WorkRoot,
    [string]$ExpectedYuneDllSha256,
    [switch]$AllowDirty,
    [switch]$KeepWorkRoot
)

$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)
$ProbeSource = Join-Path $RepoRoot "scripts\oracle-rime-probe.cs"

function Full-ExistingPath([string]$Path, [string]$Label) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing $Label`: $Path"
    }
    return [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Path).Path)
}

function Evidence-Path([string]$Path) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = $RepoRoot.TrimEnd("\", "/")
    if ($Full.StartsWith($Root + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Full.Substring($Root.Length + 1).Replace("\", "/")
    }
    return $Full.Replace("\", "/")
}

function Quote-CommandArg([string]$Value) {
    return "'" + $Value.Replace("'", "''") + "'"
}

function File-Sha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Bytes-Sha256([byte[]]$Bytes) {
    $Hasher = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([System.BitConverter]::ToString($Hasher.ComputeHash($Bytes))).Replace("-", "").ToLowerInvariant()
    }
    finally {
        $Hasher.Dispose()
    }
}

function Tree-Sha256([string]$Root) {
    $RootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd("\", "/")
    $Rows = Get-ChildItem -LiteralPath $RootFull -Recurse -File -Force |
        Sort-Object { $_.FullName.Substring($RootFull.Length + 1).Replace("\", "/") } |
        ForEach-Object {
            $Relative = $_.FullName.Substring($RootFull.Length + 1).Replace("\", "/")
            "$Relative`t$(File-Sha256 $_.FullName)"
        }
    $Payload = (($Rows -join "`n") + "`n")
    return Bytes-Sha256 ([System.Text.Encoding]::UTF8.GetBytes($Payload))
}

function Write-Utf8NoBom([string]$Path, [string]$Text) {
    $Parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($Parent)) {
        New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    }
    [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

function Copy-Tree([string]$Source, [string]$Destination) {
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $Destination -Recurse -Force
    }
}

function Inputs-FromOracle([object]$Oracle) {
    if ($null -ne $Oracle.cases -and @($Oracle.cases).Count -gt 0) {
        return @($Oracle.cases | ForEach-Object { [string]$_.input })
    }
    throw "Oracle capture must contain a non-empty cases array."
}

$YuneDll = Full-ExistingPath $YuneDll "Yune DLL"
$SharedDataDir = Full-ExistingPath $SharedDataDir "shared-data directory"
$OracleCapture = Full-ExistingPath $OracleCapture "oracle capture"
$ProbeSource = Full-ExistingPath $ProbeSource "Rime probe"
$Output = [System.IO.Path]::GetFullPath($Output)
if ($SchemaId -notmatch '^[A-Za-z0-9_][A-Za-z0-9_.-]*$') {
    throw "SchemaId must be a logical schema identifier, not a path."
}
if (-not [string]::IsNullOrWhiteSpace($DefaultYamlOverlay)) {
    $DefaultYamlOverlay = Full-ExistingPath $DefaultYamlOverlay "default.yaml overlay"
}
if ($DefaultYamlOverlay -and $NarrowSchemaList.IsPresent) {
    throw "Use either -DefaultYamlOverlay or -NarrowSchemaList, not both."
}

$RepoHead = (& git -C $RepoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $RepoHead.Length -ne 40) {
    throw "Unable to resolve the Yune source commit."
}
$RepoStatus = @(& git -C $RepoRoot status --short)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to read the Yune worktree status."
}
if ($RepoStatus.Count -gt 0 -and -not $AllowDirty.IsPresent) {
    throw "Refusing a release evidence capture from a dirty worktree. Re-run with -AllowDirty only for a diagnostic capture."
}

$YuneDllSha256 = File-Sha256 $YuneDll
if (-not [string]::IsNullOrWhiteSpace($ExpectedYuneDllSha256) -and
    $YuneDllSha256 -ne $ExpectedYuneDllSha256.ToLowerInvariant()) {
    throw "Unexpected Yune DLL SHA-256. Expected $ExpectedYuneDllSha256, got $YuneDllSha256."
}

$Oracle = Get-Content -LiteralPath $OracleCapture -Raw -Encoding UTF8 | ConvertFrom-Json
$InputsWereProvided = $PSBoundParameters.ContainsKey("Inputs") -and $null -ne $Inputs -and $Inputs.Count -gt 0
$OracleInputs = Inputs-FromOracle $Oracle
$OracleInputSet = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
foreach ($OracleInput in $OracleInputs) {
    if ([string]::IsNullOrWhiteSpace($OracleInput) -or -not $OracleInputSet.Add($OracleInput)) {
        throw "Oracle capture inputs must be non-empty and unique."
    }
}
if ($null -eq $Inputs -or $Inputs.Count -eq 0) {
    $Inputs = $OracleInputs
}
$NormalizedInputs = New-Object System.Collections.Generic.List[string]
foreach ($InputValue in @($Inputs)) {
    foreach ($Part in ([string]$InputValue).Split(",")) {
        $Normalized = $Part.Trim()
        if (-not [string]::IsNullOrWhiteSpace($Normalized)) {
            $NormalizedInputs.Add($Normalized)
        }
    }
}
$Inputs = @($NormalizedInputs)
$InputSet = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
if ($Inputs.Count -eq 0 -or @($Inputs | Where-Object { -not $InputSet.Add($_) }).Count -gt 0) {
    throw "Capture inputs must be non-empty and unique."
}
foreach ($InputValue in $Inputs) {
    if (-not $OracleInputSet.Contains($InputValue)) {
        throw "Capture input '$InputValue' is absent from the oracle capture."
    }
}

$WorkRootWasProvided = -not [string]::IsNullOrWhiteSpace($WorkRoot)
if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    $WorkRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("yune-m59-candidate-capture-" + [guid]::NewGuid().ToString("N"))
}
$WorkRoot = [System.IO.Path]::GetFullPath($WorkRoot)
if (Test-Path -LiteralPath $WorkRoot) {
    throw "Capture work root already exists; refusing to reuse or delete it: $WorkRoot"
}
$OutputUnderWorkRoot = $Output.StartsWith(
    $WorkRoot.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar,
    [System.StringComparison]::OrdinalIgnoreCase
)
if ($OutputUnderWorkRoot -and -not $KeepWorkRoot.IsPresent) {
    throw "Output is inside the disposable work root; pass -KeepWorkRoot or choose an external output path."
}

$EffectiveParameters = [ordered]@{
    yune_dll = Evidence-Path $YuneDll
    shared_data_dir = Evidence-Path $SharedDataDir
    schema_id = $SchemaId
    oracle_capture = Evidence-Path $OracleCapture
    output = Evidence-Path $Output
    inputs = @($Inputs)
    inputs_source = if ($InputsWereProvided) { "explicit" } else { "oracle_cases" }
    default_yaml_overlay = Evidence-Path $DefaultYamlOverlay
    narrow_schema_list = $NarrowSchemaList.IsPresent
    work_root = if ($WorkRootWasProvided) { Evidence-Path $WorkRoot } else { "generated_disposable" }
    expected_yune_dll_sha256 = if ($ExpectedYuneDllSha256) { $ExpectedYuneDllSha256.ToLowerInvariant() } else { $null }
    allow_dirty = $AllowDirty.IsPresent
    keep_work_root = $KeepWorkRoot.IsPresent
}
$Invocation = @(
    "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-yune-candidate-order.ps1",
    "-YuneDll $(Quote-CommandArg $EffectiveParameters.yune_dll)",
    "-SharedDataDir $(Quote-CommandArg $EffectiveParameters.shared_data_dir)",
    "-SchemaId $(Quote-CommandArg $SchemaId)",
    "-OracleCapture $(Quote-CommandArg $EffectiveParameters.oracle_capture)",
    "-Output $(Quote-CommandArg $EffectiveParameters.output)",
    "-Inputs $(Quote-CommandArg ($Inputs -join ','))"
)
if ($DefaultYamlOverlay) { $Invocation += "-DefaultYamlOverlay $(Quote-CommandArg $EffectiveParameters.default_yaml_overlay)" }
if ($NarrowSchemaList.IsPresent) { $Invocation += "-NarrowSchemaList" }
if ($WorkRootWasProvided) { $Invocation += "-WorkRoot $(Quote-CommandArg $EffectiveParameters.work_root)" }
if ($ExpectedYuneDllSha256) { $Invocation += "-ExpectedYuneDllSha256 $(Quote-CommandArg $EffectiveParameters.expected_yune_dll_sha256)" }
if ($AllowDirty.IsPresent) { $Invocation += "-AllowDirty" }
if ($KeepWorkRoot.IsPresent) { $Invocation += "-KeepWorkRoot" }
$ActualInvocation = $Invocation -join " "

$Shared = Join-Path $WorkRoot "shared"
$User = Join-Path $WorkRoot "user"
$Build = Join-Path $User "build"
$Bin = Join-Path $WorkRoot "bin"
$Marker = Join-Path $WorkRoot ".yune-m59-candidate-capture-root"
$MarkerText = "created-by=capture-yune-candidate-order.ps1`n"
$OldPath = $env:PATH

try {
    New-Item -ItemType Directory -Force -Path $WorkRoot, $User, $Build, $Bin | Out-Null
    Write-Utf8NoBom $Marker $MarkerText
    Copy-Tree $SharedDataDir $Shared
    if (-not [string]::IsNullOrWhiteSpace($DefaultYamlOverlay)) {
        Copy-Item -LiteralPath $DefaultYamlOverlay -Destination (Join-Path $Shared "default.yaml") -Force
    }
    if ($NarrowSchemaList.IsPresent) {
        $DefaultYaml = Join-Path $Shared "default.yaml"
        if (-not (Test-Path -LiteralPath $DefaultYaml)) {
            throw "-NarrowSchemaList requires shared/default.yaml."
        }
        $DefaultText = [System.IO.File]::ReadAllText($DefaultYaml, [System.Text.Encoding]::UTF8)
        $SchemaListPattern = "(?ms)^schema_list:\s*\r?\n(?:[ \t]+-\s+schema:\s*[^\r\n]+\r?\n)+"
        $SchemaListRegex = [System.Text.RegularExpressions.Regex]::new($SchemaListPattern)
        $Narrowed = $SchemaListRegex.Replace($DefaultText, "schema_list:`n  - schema: $SchemaId`n", 1)
        if ($Narrowed -eq $DefaultText) {
            throw "Unable to narrow schema_list in staged default.yaml."
        }
        Write-Utf8NoBom $DefaultYaml $Narrowed
    }
    Copy-Item -LiteralPath $YuneDll -Destination (Join-Path $Bin "rime.dll") -Force

    $env:PATH = $Bin + ";" + $OldPath
    Add-Type -Path $ProbeSource
    $Modules = [string[]]@("default")
    $DeployResult = [RimeProbe]::DeployWorkspace($Shared, $User, $Build, $Modules)
    if ($DeployResult -eq 0) {
        throw "Yune RimeDeployWorkspace returned false."
    }
    $Cases = [RimeProbe]::Capture($Shared, $User, $Build, $SchemaId, $Modules, [string[]]$Inputs)
    if ($Cases.Count -ne $Inputs.Count) {
        throw "Yune capture returned $($Cases.Count) cases for $($Inputs.Count) inputs."
    }
    foreach ($Case in $Cases) {
        if (-not $Case["captured_all_pages"]) {
            $Reason = if ($Case.ContainsKey("pagination_error")) { $Case["pagination_error"] } else { "unknown" }
            throw "Yune capture for '$($Case["input"])' is incomplete: $Reason"
        }
    }

    $Evidence = [ordered]@{
        capture = [ordered]@{
            engine = "yune"
            source_commit = $RepoHead
            source_dirty = ($RepoStatus.Count -gt 0)
            source_status_short = @($RepoStatus)
            schema_id = $SchemaId
            modules = @($Modules)
            yune_dll_sha256 = $YuneDllSha256
            probe_sha256 = File-Sha256 $ProbeSource
            capture_script_sha256 = File-Sha256 $PSCommandPath
            oracle_capture_sha256 = File-Sha256 $OracleCapture
            source_shared_tree_sha256 = Tree-Sha256 $SharedDataDir
            staged_shared_tree_sha256 = Tree-Sha256 $Shared
            default_yaml_overlay_sha256 = if ($DefaultYamlOverlay) { File-Sha256 $DefaultYamlOverlay } else { $null }
            schema_list_narrowed = $NarrowSchemaList.IsPresent
            page_policy = "RimeProbe.Capture all pages; hard failure on non-advancing or incomplete pagination"
            actual_invocation = $ActualInvocation
            effective_parameters = $EffectiveParameters
        }
        inputs = @($Inputs)
        cases = @($Cases)
    }
    Write-Utf8NoBom $Output (($Evidence | ConvertTo-Json -Depth 100) + "`n")
    Write-Output "captured complete Yune candidate order -> $Output"
}
finally {
    $env:PATH = $OldPath
    if (-not $KeepWorkRoot.IsPresent -and (Test-Path -LiteralPath $WorkRoot)) {
        $ResolvedWorkRoot = [System.IO.Path]::GetFullPath($WorkRoot)
        if (-not (Test-Path -LiteralPath $Marker)) {
            throw "Refusing to remove an unmarked capture work root: $ResolvedWorkRoot"
        }
        if ([System.IO.File]::ReadAllText($Marker, [System.Text.Encoding]::UTF8) -ne $MarkerText) {
            throw "Refusing to remove a capture work root with an invalid marker: $ResolvedWorkRoot"
        }
        if ($ResolvedWorkRoot -eq $RepoRoot -or $ResolvedWorkRoot.Length -lt 8) {
            throw "Refusing to remove unsafe capture work root: $ResolvedWorkRoot"
        }
        try {
            Remove-Item -LiteralPath $ResolvedWorkRoot -Recurse -Force -ErrorAction Stop
        }
        catch {
            # Windows keeps rime.dll loaded until this PowerShell process exits.
            # Do not mask the capture result; the retained path is outside the
            # tracked tree and can be removed by the caller after process exit.
            Write-Warning "Work-root cleanup is deferred until rime.dll unloads: $ResolvedWorkRoot"
        }
    }
}
