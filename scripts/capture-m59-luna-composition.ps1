# Captures the librime 1.17.0 oracle provenance for M59 leading-single
# reachability + partial-selection composition on luna_pinyin:
#   - paged candidate lists for the PRIMARY non-lexicon case moboyi -> mo/bo/yi
#     (remainders boyi / yi) plus zhonggao / zhongguo / gao / guo (the
#     reachable-single positions the M59 acceptance rows cite), and
#   - the moboyi -> 莫伯洢 partial-selection composition chain (commit the phrase
#     from selecting the leading singles 莫, 伯, 洢 one at a time).
#
# Runs the real rime.dll via scripts/oracle-rime-probe.cs. Requires the upstream
# oracle root laid out by the capture-upstream-* pipeline (pinned rime.dll,
# rime_deployer.exe, schema source repositories, and upstream OpenCC data). It
# deploys a clean disposable shared/user/build tree for every capture. Emits raw
# UTF-8 snapshot JSON, then curates
# the checked-in fixture via scripts/curate-m59-luna-composition.py (Python reads
# the raw files as UTF-8 files, avoiding PowerShell native-pipe codepage
# corruption of the CJK candidate texts). This .ps1 is intentionally pure-ASCII.
param(
    [string]$OracleRoot,
    [string]$Output,
    [string]$ExpectedRimeDllSha256 = "86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b",
    [string]$ExpectedRimeDeployerSha256 = "3abb72b5bb56fcafcfe925d533ae5f832c68d5a0bc9952fd0eea0682fb1ab071",
    [string]$ExpectedLunaPinyinCommit = "18a80335c37522311f7cff02886cd81cec3b460a",
    [string]$ExpectedPreludeCommit = "082425ea0684bca36474415d4a0e8db9b016487e",
    [string]$ExpectedEssayCommit = "48c7538f0b760fcc8c9d6bf08711f82cfbd2e9ed",
    [string]$ExpectedStrokeCommit = "3a4b0f4013e2b4c14b1e80c92b1d4723eb65f39c"
)
$ErrorActionPreference = "Stop"
$RepoRoot = [System.IO.Path]::GetFullPath((Resolve-Path (Join-Path $PSScriptRoot "..")).Path)
if ([string]::IsNullOrWhiteSpace($OracleRoot)) {
    $OracleRoot = Join-Path $RepoRoot "target/upstream-oracle/1.17.0"
}
if ([string]::IsNullOrWhiteSpace($Output)) {
    $Output = Join-Path $RepoRoot "crates/yune-core/tests/fixtures/upstream-1.17.0/m59-luna-leading-single-composition.json"
}
$OracleRoot = [System.IO.Path]::GetFullPath($OracleRoot)
$Output = [System.IO.Path]::GetFullPath($Output)
$Extract = Join-Path $OracleRoot "extract"
$Probe = Join-Path $RepoRoot "scripts/oracle-rime-probe.cs"
$RimeDll = Join-Path $Extract "dist/lib/rime.dll"
$RimeDeployer = Join-Path $Extract "dist/bin/rime_deployer.exe"
$SchemaRoot = Join-Path $OracleRoot "schema-src"

$PageDown = 65366

function New-Action($type, $text, $kc, $label) {
    $a = [RimeProbe+ProbeAction]::new()
    $a.type = $type
    if ($text) { $a.text = $text }
    if ($kc) { $a.keycode = $kc; $a.mask = 0 }
    if ($label) { $a.label = $label }
    return $a
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

function Tree-Sha256([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "Cannot hash missing evidence tree: $Path"
    }
    $Root = [System.IO.Path]::GetFullPath((Resolve-Path -LiteralPath $Path).Path).TrimEnd("\", "/")
    $Rows = [System.Collections.Generic.List[string]]::new()
    foreach ($File in (Get-ChildItem -LiteralPath $Root -Recurse -File -Force)) {
        $Relative = $File.FullName.Substring($Root.Length + 1).Replace("\", "/")
        $Rows.Add("$Relative`t$(File-Sha256 $File.FullName)")
    }
    $Ordered = $Rows.ToArray()
    [Array]::Sort($Ordered, [System.StringComparer]::Ordinal)
    $Payload = (($Ordered -join "`n") + "`n")
    return Bytes-Sha256 ([System.Text.Encoding]::UTF8.GetBytes($Payload))
}

function Git-Head([string]$Path) {
    return (& git -C $Path rev-parse HEAD).Trim()
}

function Git-Tree([string]$Path) {
    return (& git -C $Path rev-parse 'HEAD^{tree}').Trim()
}

function Assert-Git-Clean([string]$Path, [string]$Identity) {
    $Status = @(& git -C $Path status --porcelain=v1 --untracked-files=all)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to inspect source cleanliness for $Identity at $Path"
    }
    if ($Status.Count -ne 0) {
        throw "Source repository $Identity is not clean; exact Lane-B byte provenance requires a clean pinned checkout."
    }
}

function Copy-PinnedRimeData([string]$Source, [string]$Destination) {
    foreach ($File in (Get-ChildItem -LiteralPath $Source -File)) {
        if ($File.Name -notlike "*.yaml" -and $File.Name -ne "essay.txt") {
            continue
        }
        & git -C $Source ls-files --error-unmatch -- $File.Name 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "Refusing untracked staged source file: $($File.FullName)"
        }
        Copy-Item -LiteralPath $File.FullName -Destination (Join-Path $Destination $File.Name)
    }
}

function Evidence-Path([string]$Path) {
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

$ActualRimeDllSha256 = File-Sha256 $RimeDll
$ActualRimeDeployerSha256 = File-Sha256 $RimeDeployer
if ($ActualRimeDllSha256 -ne $ExpectedRimeDllSha256.ToLowerInvariant()) {
    throw "Unexpected upstream rime.dll SHA-256. Expected $ExpectedRimeDllSha256, got $ActualRimeDllSha256."
}
if ($ActualRimeDeployerSha256 -ne $ExpectedRimeDeployerSha256.ToLowerInvariant()) {
    throw "Unexpected upstream rime_deployer.exe SHA-256. Expected $ExpectedRimeDeployerSha256, got $ActualRimeDeployerSha256."
}
$SchemaSourceCommit = Git-Head (Join-Path $SchemaRoot "rime-luna-pinyin")
$PreludeCommit = Git-Head (Join-Path $SchemaRoot "rime-prelude")
$EssayCommit = Git-Head (Join-Path $SchemaRoot "rime-essay")
$StrokeCommit = Git-Head (Join-Path $SchemaRoot "rime-stroke")
$PinnedCommits = [ordered]@{
    "rime/rime-luna-pinyin" = [ordered]@{ actual = $SchemaSourceCommit; expected = $ExpectedLunaPinyinCommit }
    "rime/rime-prelude" = [ordered]@{ actual = $PreludeCommit; expected = $ExpectedPreludeCommit }
    "rime/rime-essay" = [ordered]@{ actual = $EssayCommit; expected = $ExpectedEssayCommit }
    "rime/rime-stroke" = [ordered]@{ actual = $StrokeCommit; expected = $ExpectedStrokeCommit }
}
foreach ($Entry in $PinnedCommits.GetEnumerator()) {
    if ($Entry.Value.actual -ne $Entry.Value.expected.ToLowerInvariant()) {
        throw "Unexpected $($Entry.Key) commit. Expected $($Entry.Value.expected), got $($Entry.Value.actual)."
    }
}
$SourceRepoPaths = [ordered]@{
    "rime/rime-luna-pinyin" = Join-Path $SchemaRoot "rime-luna-pinyin"
    "rime/rime-prelude" = Join-Path $SchemaRoot "rime-prelude"
    "rime/rime-essay" = Join-Path $SchemaRoot "rime-essay"
    "rime/rime-stroke" = Join-Path $SchemaRoot "rime-stroke"
}
$SourceGitTrees = [ordered]@{}
foreach ($Entry in $SourceRepoPaths.GetEnumerator()) {
    Assert-Git-Clean $Entry.Value $Entry.Key
    $Tree = Git-Tree $Entry.Value
    if ($Tree -notmatch '^[0-9a-f]{40}$') {
        throw "Unexpected git tree identity for $($Entry.Key): $Tree"
    }
    $SourceGitTrees[$Entry.Key] = $Tree
}
$OldPath = $env:PATH
$env:PATH = (Join-Path $Extract "dist/lib") + ";" + (Join-Path $Extract "bin") + ";" + $env:PATH
Add-Type -Path $Probe

# A composition scenario: type $input, then for each step turn $pd pages and
# press the $digit selection key (49='1' selects page position 0, etc.). Digit
# keys are 1-indexed within the page: global index N sits on page (N / page_size)
# at position (N % page_size), selected by digit (position + 1).
function New-Compose([string]$name, [string]$sequence, [object[]]$steps) {
    $acts = New-Object System.Collections.Generic.List[object]
    $acts.Add((New-Action "input" $sequence 0 $null))
    $acts.Add((New-Action "snapshot" $null 0 ($sequence + "_page0")))
    foreach ($step in $steps) {
        for ($i = 1; $i -le [int]$step.pd; $i++) {
            $acts.Add((New-Action "key" $null $PageDown ($step.label + "_pd$i")))
        }
        $acts.Add((New-Action "key" $null ([int]$step.digit) ("after_select_" + $step.label)))
    }
    $sc = [RimeProbe+ProbeScenario]::new()
    $sc.name = $name
    $sc.actions = [RimeProbe+ProbeAction[]]$acts.ToArray()
    return $sc
}

$WorkRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("m59-luna-capture-" + [guid]::NewGuid().ToString("N"))
if (Test-Path -LiteralPath $WorkRoot) {
    throw "Refusing to reuse an existing Lane-B capture root: $WorkRoot"
}
New-Item -ItemType Directory -Path $WorkRoot | Out-Null
$WorkRootMarker = Join-Path $WorkRoot ".yune-m59-luna-capture-root"
$WorkRootMarkerText = "created-by=capture-m59-luna-composition.ps1`n"
[System.IO.File]::WriteAllText(
    $WorkRootMarker,
    $WorkRootMarkerText,
    [System.Text.UTF8Encoding]::new($false)
)
$Shared = Join-Path $WorkRoot "shared"
$User = Join-Path $WorkRoot "user"
$Build = Join-Path $User "build"
$Raw = Join-Path $WorkRoot "raw"
try {
New-Item -ItemType Directory -Path $Shared, $User, $Build, $Raw | Out-Null
foreach ($Entry in $SourceRepoPaths.GetEnumerator()) {
    Copy-PinnedRimeData $Entry.Value $Shared
}
$OpenCcSource = Join-Path $Extract "share/opencc"
$OpenCcDestination = Join-Path $Shared "opencc"
if (-not (Test-Path -LiteralPath $OpenCcSource -PathType Container)) {
    throw "Missing pinned upstream OpenCC data: $OpenCcSource"
}
New-Item -ItemType Directory -Path $OpenCcDestination | Out-Null
Get-ChildItem -LiteralPath $OpenCcSource -Force | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $OpenCcDestination -Recurse
}
$DefaultCustom = Join-Path $Shared "default.custom.yaml"
[System.IO.File]::WriteAllText(
    $DefaultCustom,
    "patch:`n  schema_list:`n    - schema: luna_pinyin`n",
    [System.Text.UTF8Encoding]::new($false)
)
$StagedTimestampUtc = [DateTimeOffset]::FromUnixTimeSeconds(946684800).AddMilliseconds(500).UtcDateTime
Get-ChildItem -LiteralPath $Shared -Recurse -File -Force | ForEach-Object {
    $_.LastWriteTimeUtc = $StagedTimestampUtc
}
$TimestampDrift = @(
    Get-ChildItem -LiteralPath $Shared -Recurse -File -Force |
        Where-Object { $_.LastWriteTimeUtc.Ticks -ne $StagedTimestampUtc.Ticks }
)
if ($TimestampDrift.Count -ne 0) {
    throw "Staged timestamp readback was not exact for $($TimestampDrift.Count) files."
}
& $RimeDeployer --build $User $Shared $Build
if ($LASTEXITCODE -ne 0) {
    throw "Pinned rime_deployer.exe --build failed with exit code $LASTEXITCODE"
}
$SharedTreeSha256BeforeCapture = Tree-Sha256 $Shared
$BuildTreeSha256BeforeCapture = Tree-Sha256 $Build
$DefaultCustomSha256 = File-Sha256 $DefaultCustom
$OpenCcTreeSha256 = Tree-Sha256 $OpenCcDestination

$modules = [string[]]@("default")

# PRIMARY (and only) case: moboyi -> the non-lexicon phrase 莫伯洢. `boyi`/`yi` are
# its recompose remainders; zhonggao/zhongguo/gao/guo are the class rows.
$pagingInputs = @("moboyi", "boyi", "yi", "zhonggao", "zhongguo", "gao", "guo")
$pageSnaps = [RimeProbe]::Capture($Shared, $User, $Build, "luna_pinyin", $modules, [string[]]$pagingInputs)
foreach ($Case in $pageSnaps) {
    if (-not $Case["captured_all_pages"]) {
        $Reason = if ($Case.ContainsKey("pagination_error")) { $Case["pagination_error"] } else { "unknown" }
        throw "Capture for input '$($Case["input"])' did not capture all pages: $Reason"
    }
}

# moboyi -> 莫伯洢: 莫@2 (page0, digit '3'); boyi 伯@19 (page3 pos4, digit '5' after
# 3 Page_Down); yi 洢@155 (page31 pos0, digit '1' after 31 Page_Down).
$moboyi = New-Compose "moboyi_compose" "moboyi" @(
    @{ pd = 0; digit = 51; label = "mo" },
    @{ pd = 3; digit = 53; label = "bo" },
    @{ pd = 31; digit = 49; label = "yi" }
)
$composeSnaps = [RimeProbe]::CaptureScenarios($Shared, $User, $Build, "luna_pinyin", $modules, [RimeProbe+ProbeScenario[]]@($moboyi))
$SharedTreeSha256AfterCapture = Tree-Sha256 $Shared
$BuildTreeSha256AfterCapture = Tree-Sha256 $Build
if ($SharedTreeSha256AfterCapture -ne $SharedTreeSha256BeforeCapture) {
    throw "Lane-B shared data changed during capture; refusing unbound evidence."
}
if ($BuildTreeSha256AfterCapture -ne $BuildTreeSha256BeforeCapture) {
    throw "Lane-B deployed build changed during capture; refusing unbound evidence."
}
foreach ($Entry in $SourceRepoPaths.GetEnumerator()) {
    Assert-Git-Clean $Entry.Value $Entry.Key
    if ((Git-Head $Entry.Value) -ne $PinnedCommits[$Entry.Key].actual -or
        (Git-Tree $Entry.Value) -ne $SourceGitTrees[$Entry.Key]) {
        throw "Pinned source identity changed during Lane-B capture: $($Entry.Key)"
    }
}

$PagesRaw = Join-Path $Raw "pages.json"
$ComposeRaw = Join-Path $Raw "compose.json"
$MetadataRaw = Join-Path $Raw "metadata.json"
    $pageSnaps | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $PagesRaw -Encoding UTF8
    $composeSnaps | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $ComposeRaw -Encoding UTF8

    $EffectiveParameters = [ordered]@{
        oracle_root = Evidence-Path $OracleRoot
        output = Evidence-Path $Output
        expected_rime_dll_sha256 = $ExpectedRimeDllSha256.ToLowerInvariant()
        expected_rime_deployer_sha256 = $ExpectedRimeDeployerSha256.ToLowerInvariant()
        expected_luna_pinyin_commit = $ExpectedLunaPinyinCommit.ToLowerInvariant()
        expected_prelude_commit = $ExpectedPreludeCommit.ToLowerInvariant()
        expected_essay_commit = $ExpectedEssayCommit.ToLowerInvariant()
        expected_stroke_commit = $ExpectedStrokeCommit.ToLowerInvariant()
    }
    $EffectiveInvocation = @(
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-m59-luna-composition.ps1",
        "-OracleRoot $(Quote-CommandArg $EffectiveParameters.oracle_root)",
        "-Output $(Quote-CommandArg $EffectiveParameters.output)",
        "-ExpectedRimeDllSha256 $(Quote-CommandArg $EffectiveParameters.expected_rime_dll_sha256)",
        "-ExpectedRimeDeployerSha256 $(Quote-CommandArg $EffectiveParameters.expected_rime_deployer_sha256)",
        "-ExpectedLunaPinyinCommit $(Quote-CommandArg $EffectiveParameters.expected_luna_pinyin_commit)",
        "-ExpectedPreludeCommit $(Quote-CommandArg $EffectiveParameters.expected_prelude_commit)",
        "-ExpectedEssayCommit $(Quote-CommandArg $EffectiveParameters.expected_essay_commit)",
        "-ExpectedStrokeCommit $(Quote-CommandArg $EffectiveParameters.expected_stroke_commit)"
    ) -join " "
    $Metadata = [ordered]@{
        rime_dll_sha256 = $ActualRimeDllSha256
        rime_deployer_sha256 = $ActualRimeDeployerSha256
        schema_source_repo = "rime/rime-luna-pinyin"
        schema_source_commit = $SchemaSourceCommit
        dependency_commits = [ordered]@{
            "rime/rime-prelude" = $PreludeCommit
            "rime/rime-essay" = $EssayCommit
            "rime/rime-stroke" = $StrokeCommit
        }
        source_repositories_clean = [ordered]@{
            "rime/rime-luna-pinyin" = $true
            "rime/rime-prelude" = $true
            "rime/rime-essay" = $true
            "rime/rime-stroke" = $true
        }
        source_git_trees = $SourceGitTrees
        queried_data = [ordered]@{
            shared_path = "disposable/shared"
            build_path = "disposable/user/build"
            shared_tree_sha256 = $SharedTreeSha256BeforeCapture
            build_tree_sha256 = $BuildTreeSha256BeforeCapture
            tree_hash_algorithm = "sha256 of ordinal path<TAB>file-sha256 rows joined by LF with final LF"
            mutation_policy = "raw shared/build hashes must remain identical before/after capture"
            deployment_policy = "clean disposable deploy from pinned tracked source files plus pinned upstream OpenCC"
            timestamp_normalization_policy = "all staged files use fixed half-second LastWriteTimeUtc verified by exact FileTimeUtc readback before deployment"
            staged_timestamp_utc = "2000-01-01T00:00:00.500Z"
            default_custom_sha256 = $DefaultCustomSha256
            opencc_tree_sha256 = $OpenCcTreeSha256
        }
        actual_invocation = $EffectiveInvocation
        effective_parameters = $EffectiveParameters
    }
    $Metadata | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $MetadataRaw -Encoding UTF8

    $Curate = Join-Path $RepoRoot "scripts/curate-m59-luna-composition.py"
    & python $Curate $PagesRaw $ComposeRaw $MetadataRaw $Output
    if ($LASTEXITCODE -ne 0) {
        throw "curation failed with exit code $LASTEXITCODE"
    }
}
finally {
    $env:PATH = $OldPath
    $ResolvedWorkRoot = [System.IO.Path]::GetFullPath($WorkRoot)
    if (-not (Test-Path -LiteralPath $WorkRootMarker)) {
        throw "Refusing to remove an unmarked Lane-B capture root: $ResolvedWorkRoot"
    }
    if ([System.IO.File]::ReadAllText($WorkRootMarker, [System.Text.Encoding]::UTF8) -ne $WorkRootMarkerText) {
        throw "Refusing to remove a Lane-B capture root with an invalid marker: $ResolvedWorkRoot"
    }
    $TempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd("\", "/")
    if (-not $ResolvedWorkRoot.StartsWith(
            $TempRoot + [System.IO.Path]::DirectorySeparatorChar,
            [System.StringComparison]::OrdinalIgnoreCase
        ) -or $ResolvedWorkRoot -eq $TempRoot) {
        throw "Refusing to remove unsafe Lane-B capture root: $ResolvedWorkRoot"
    }
    Remove-Item -LiteralPath $ResolvedWorkRoot -Recurse -Force
}
Write-Output "captured M59 luna composition oracle -> $Output"
