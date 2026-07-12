param(
    [string]$OutputRoot,
    [string]$UpstreamOracleRoot,
    [string]$YuneDll,
    [string]$PrebuiltNativeBenchmarkExecutable,
    [string]$PrebuiltNativeBenchmarkReceipt,
    [int]$Iterations = 9,
    [int]$SessionIterations = 60,
    [int]$KeyIterations = 80,
    [string]$TrackAInputs = "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru",
    [string]$TrackBInputs = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung",
    [switch]$DeployProductBeforeBenchmark,
    [switch]$SkipTrackB,
    [string]$TrackAThresholds,
    [switch]$FailOnRegression,
    [string]$ProductSchemaRoot,
    [string]$WorkRoot,
    [switch]$AllowDirty
)

$ErrorActionPreference = "Stop"

function Assert-PlainFileSystemPath([string]$Path, [string]$Label) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "$Label must not be empty"
    }
    $Normalized = $Path.Replace("/", "\")
    if ($Normalized.StartsWith("\\?\", [System.StringComparison]::OrdinalIgnoreCase) -or
        $Normalized.StartsWith("\\.\", [System.StringComparison]::OrdinalIgnoreCase) -or
        $Normalized.StartsWith("\??\", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label must not use a Windows device or extended path: $Path"
    }
}

function Get-FinalPhysicalPath([string]$Path) {
    if ($null -eq ("Yune.BenchmarkNativePath" -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

namespace Yune {
    public static class BenchmarkNativePath {
        private const uint OpenExisting = 3;
        private const uint BackupSemantics = 0x02000000;
        private const uint ShareReadWriteDelete = 0x00000007;

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern SafeFileHandle CreateFileW(
            string path,
            uint desiredAccess,
            uint shareMode,
            IntPtr securityAttributes,
            uint creationDisposition,
            uint flagsAndAttributes,
            IntPtr templateFile
        );

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern uint GetFinalPathNameByHandleW(
            SafeFileHandle file,
            StringBuilder path,
            uint pathLength,
            uint flags
        );

        public static string Resolve(string path) {
            using (SafeFileHandle handle = CreateFileW(
                path,
                0,
                ShareReadWriteDelete,
                IntPtr.Zero,
                OpenExisting,
                BackupSemantics,
                IntPtr.Zero
            )) {
                if (handle.IsInvalid) {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                var buffer = new StringBuilder(512);
                uint length = GetFinalPathNameByHandleW(
                    handle, buffer, (uint)buffer.Capacity, 0
                );
                if (length == 0) {
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                }
                if (length >= buffer.Capacity) {
                    buffer.Capacity = checked((int)length + 1);
                    length = GetFinalPathNameByHandleW(
                        handle, buffer, (uint)buffer.Capacity, 0
                    );
                    if (length == 0 || length >= buffer.Capacity) {
                        throw new Win32Exception(Marshal.GetLastWin32Error());
                    }
                }
                return buffer.ToString();
            }
        }
    }
}
'@
    }
    $Final = [Yune.BenchmarkNativePath]::Resolve($Path)
    if ($Final.StartsWith("\\?\UNC\", [System.StringComparison]::OrdinalIgnoreCase)) {
        return "\\" + $Final.Substring(8)
    }
    if ($Final.StartsWith("\\?\", [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Final.Substring(4)
    }
    if ($Final.StartsWith("\??\", [System.StringComparison]::OrdinalIgnoreCase)) {
        return $Final.Substring(4)
    }
    return $Final
}

function Get-CanonicalSafePath([string]$Path, [string]$Label) {
    Assert-PlainFileSystemPath $Path $Label
    $Full = [System.IO.Path]::GetFullPath($Path)
    $Root = [System.IO.Path]::GetPathRoot($Full)
    if ([string]::IsNullOrWhiteSpace($Root)) {
        throw "$Label has no filesystem root: $Path"
    }
    if ($Full.Substring($Root.Length).Contains(":")) {
        throw "$Label must not use an alternate data stream: $Path"
    }

    $MissingParts = [System.Collections.Generic.Stack[string]]::new()
    $Probe = $Full
    while (-not (Test-Path -LiteralPath $Probe)) {
        $Leaf = Split-Path -Leaf $Probe
        $Parent = Split-Path -Parent $Probe
        if ([string]::IsNullOrWhiteSpace($Leaf) -or
            [string]::IsNullOrWhiteSpace($Parent) -or
            $Parent.Equals($Probe, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "$Label has no existing filesystem ancestor: $Path"
        }
        $MissingParts.Push($Leaf)
        $Probe = $Parent
    }

    $Existing = Get-Item -LiteralPath $Probe -Force
    if ($MissingParts.Count -gt 0 -and -not $Existing.PSIsContainer) {
        throw "$Label descends from a file rather than a directory: $($Existing.FullName)"
    }
    $Ancestor = $Existing
    while ($null -ne $Ancestor) {
        if (($Ancestor.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw "$Label has a reparse-point ancestor: $($Ancestor.FullName)"
        }
        $ParentPath = Split-Path -Parent $Ancestor.FullName
        if ([string]::IsNullOrWhiteSpace($ParentPath) -or
            $ParentPath.Equals($Ancestor.FullName, [System.StringComparison]::OrdinalIgnoreCase)) {
            break
        }
        $Ancestor = Get-Item -LiteralPath $ParentPath -Force
    }

    $Canonical = Get-FinalPhysicalPath $Existing.FullName
    while ($MissingParts.Count -gt 0) {
        $Canonical = Join-Path $Canonical $MissingParts.Pop()
    }
    return [System.IO.Path]::GetFullPath($Canonical)
}

function Assert-BenchmarkSourcePolicy(
    [string[]]$StatusRows,
    [bool]$AllowDirtyRequested,
    [string]$SignedThresholdPath
) {
    if ($AllowDirtyRequested -and -not [string]::IsNullOrWhiteSpace($SignedThresholdPath)) {
        throw "-AllowDirty is diagnostic-only and cannot be combined with signed -TrackAThresholds"
    }
    if ($StatusRows.Count -gt 0 -and -not $AllowDirtyRequested) {
        throw "Benchmark source must be clean; use -AllowDirty only for an unsigned diagnostic run without thresholds. Dirty state: $($StatusRows -join ' | ')"
    }
}

$RepoRoot = Get-CanonicalSafePath (Join-Path $PSScriptRoot "..") "repository root"
$OutputRootWasProvided = -not [string]::IsNullOrWhiteSpace($OutputRoot)
$WorkRootWasProvided = -not [string]::IsNullOrWhiteSpace($WorkRoot)
$YuneDllWasProvided = -not [string]::IsNullOrWhiteSpace($YuneDll)
$NativeBenchmarkExecutableWasProvided = -not [string]::IsNullOrWhiteSpace($PrebuiltNativeBenchmarkExecutable)
$NativeBenchmarkReceiptWasProvided = -not [string]::IsNullOrWhiteSpace($PrebuiltNativeBenchmarkReceipt)
if ($NativeBenchmarkExecutableWasProvided -ne $NativeBenchmarkReceiptWasProvided) {
    throw "PrebuiltNativeBenchmarkExecutable and PrebuiltNativeBenchmarkReceipt must be supplied together"
}
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $RepoRoot "docs\reports\evidence\m36-product-path\phase-0-native-inprocess"
}
if ([string]::IsNullOrWhiteSpace($UpstreamOracleRoot)) {
    $UpstreamOracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ([string]::IsNullOrWhiteSpace($YuneDll)) {
    $YuneDll = Join-Path $RepoRoot "target\release\yune_rime_api.dll"
}
if ([string]::IsNullOrWhiteSpace($ProductSchemaRoot)) {
    $ProductSchemaRoot = Join-Path $RepoRoot "apps\yune-web\public\schema"
}

$OutputRoot = Get-CanonicalSafePath $OutputRoot "OutputRoot"
$EvidenceRoot = Get-CanonicalSafePath (Join-Path $RepoRoot "docs\reports\evidence") "legacy evidence root"
$LegacyWorkRoot = Get-CanonicalSafePath (Join-Path $RepoRoot "target\native-inprocess") "legacy work root"
if ([string]::IsNullOrWhiteSpace($WorkRoot)) {
    $WorkRoot = Join-Path $LegacyWorkRoot (Split-Path -Leaf $OutputRoot)
}
$WorkRoot = Get-CanonicalSafePath $WorkRoot "WorkRoot"
$UpstreamOracleRoot = Get-CanonicalSafePath $UpstreamOracleRoot "UpstreamOracleRoot"
$YuneDll = Get-CanonicalSafePath $YuneDll "YuneDll"
$ProductSchemaRoot = Get-CanonicalSafePath $ProductSchemaRoot "ProductSchemaRoot"
if (-not [string]::IsNullOrWhiteSpace($TrackAThresholds)) {
    $TrackAThresholds = Get-CanonicalSafePath $TrackAThresholds "TrackAThresholds"
}
$SharedSource = Get-CanonicalSafePath (Join-Path $UpstreamOracleRoot "rime-shared") "upstream shared data"
$BuildSource = Get-CanonicalSafePath (Join-Path $UpstreamOracleRoot "rime-user\build") "upstream build data"
$UpstreamDll = Get-CanonicalSafePath (Join-Path $UpstreamOracleRoot "extract\dist\lib\rime.dll") "upstream rime.dll"
$UpstreamDistLib = Get-CanonicalSafePath (Join-Path $UpstreamOracleRoot "extract\dist\lib") "upstream dist lib"
$UpstreamBin = Get-CanonicalSafePath (Join-Path $UpstreamOracleRoot "extract\bin") "upstream bin"
$UpstreamDistBin = Get-CanonicalSafePath (Join-Path $UpstreamOracleRoot "extract\dist\bin") "upstream dist bin"

function Assert-Path($Path, $Label) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing $Label`: $Path"
    }
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

function Command-IdentitySha256([string]$Command, [string[]]$ArgumentList) {
    $Output = @(& $Command @ArgumentList 2>&1 | ForEach-Object { [string]$_ })
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to capture tool identity: $Command $($ArgumentList -join ' ')"
    }
    $Payload = (($Output -join "`n") + "`n")
    return Bytes-Sha256 ([System.Text.Encoding]::UTF8.GetBytes($Payload))
}

function Write-NativeBenchmarkBuildReceipt(
    [string]$Path,
    [System.Collections.IDictionary]$Fields
) {
    if (Test-Path -LiteralPath $Path) {
        throw "Refusing to overwrite native benchmark build receipt: $Path"
    }
    $Lines = foreach ($Entry in $Fields.GetEnumerator()) {
        $Key = [string]$Entry.Key
        $Value = [string]$Entry.Value
        if ([string]::IsNullOrWhiteSpace($Key) -or $Key.Contains("=") -or
            $Key.Contains("`r") -or $Key.Contains("`n") -or
            $Value.Contains("`r") -or $Value.Contains("`n")) {
            throw "Native benchmark build receipt contains an invalid key/value"
        }
        "$Key=$Value"
    }
    $Text = (($Lines -join "`n") + "`n")
    [System.IO.File]::WriteAllText(
        $Path,
        $Text,
        [System.Text.UTF8Encoding]::new($false)
    )
}

function Read-NativeBenchmarkBuildReceipt([string]$Path) {
    $Fields = @{}
    $LineNumber = 0
    foreach ($Line in Get-Content -LiteralPath $Path -Encoding UTF8) {
        $LineNumber += 1
        if ([string]::IsNullOrWhiteSpace($Line) -or $Line.TrimStart().StartsWith("#")) {
            continue
        }
        $Separator = $Line.IndexOf("=")
        if ($Separator -le 0) {
            throw "$Path`:$LineNumber is not key=value"
        }
        $Key = $Line.Substring(0, $Separator).Trim()
        $Value = $Line.Substring($Separator + 1).Trim()
        if ($Fields.ContainsKey($Key)) {
            throw "$Path`:$LineNumber duplicates receipt key $Key"
        }
        $Fields[$Key] = $Value
    }
    return $Fields
}

function Assert-NativeBenchmarkBuildReceipt(
    [System.Collections.IDictionary]$Receipt,
    [System.Collections.IDictionary]$Expected
) {
    foreach ($Key in @(
        "format_version",
        "source_commit",
        "source_tree",
        "source_clean",
        "source_content_binding_sha256",
        "benchmark_script_sha256",
        "benchmark_rust_source_sha256",
        "cargo_lock_sha256",
        "rustc_identity_sha256",
        "cargo_identity_sha256",
        "cargo_command",
        "native_benchmark_build_command",
        "cargo_target_root",
        "native_benchmark_executable_path",
        "native_benchmark_executable_sha256"
    )) {
        if (-not $Receipt.Contains($Key) -or [string]::IsNullOrWhiteSpace([string]$Receipt[$Key])) {
            throw "Native benchmark build receipt is missing $Key"
        }
    }
    foreach ($Entry in $Expected.GetEnumerator()) {
        $Key = [string]$Entry.Key
        $ExpectedValue = [string]$Entry.Value
        $ActualValue = [string]$Receipt[$Key]
        $Matches = if ($Key -eq "native_benchmark_executable_path") {
            $ActualValue.Equals($ExpectedValue, [System.StringComparison]::OrdinalIgnoreCase)
        } else {
            $ActualValue.Equals($ExpectedValue, [System.StringComparison]::Ordinal)
        }
        if (-not $Matches) {
            throw "Native benchmark build receipt mismatch for $Key`: expected [$ExpectedValue], found [$ActualValue]"
        }
    }
}

function Tree-Sha256([string]$Root) {
    $RootFull = (Get-CanonicalSafePath $Root "tree hash root").TrimEnd("\", "/")
    if (-not (Test-Path -LiteralPath $RootFull -PathType Container)) {
        throw "Cannot hash non-directory tree: $Root"
    }
    $Rows = [System.Collections.Generic.List[string]]::new()
    $Pending = [System.Collections.Generic.Stack[string]]::new()
    $Pending.Push($RootFull)
    while ($Pending.Count -gt 0) {
        $Directory = $Pending.Pop()
        foreach ($Item in Get-ChildItem -LiteralPath $Directory -Force) {
            if (($Item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Tree hash input contains a reparse point: $($Item.FullName)"
            }
            if ($Item.PSIsContainer) {
                $Pending.Push($Item.FullName)
                continue
            }
            $Relative = $Item.FullName.Substring($RootFull.Length + 1).Replace("\", "/")
            $Rows.Add("$Relative`t$(File-Sha256 $Item.FullName)")
        }
    }
    $Ordered = $Rows.ToArray()
    [Array]::Sort($Ordered, [System.StringComparer]::Ordinal)
    $Payload = (($Ordered -join "`n") + "`n")
    return Bytes-Sha256 ([System.Text.Encoding]::UTF8.GetBytes($Payload))
}

function Get-RepositorySourceSnapshot {
    $Head = (& git -C $RepoRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw "Unable to inspect repository HEAD" }
    $Tree = (& git -C $RepoRoot rev-parse 'HEAD^{tree}').Trim()
    if ($LASTEXITCODE -ne 0) { throw "Unable to inspect repository source tree" }
    $StatusRows = @(& git -C $RepoRoot status --porcelain=v1 --untracked-files=all)
    if ($LASTEXITCODE -ne 0) { throw "Unable to inspect repository source status" }
    $DiffRows = @(& git -C $RepoRoot diff --no-ext-diff --no-textconv --binary HEAD --)
    if ($LASTEXITCODE -ne 0) { throw "Unable to bind repository source diff" }
    $Untracked = @(& git -C $RepoRoot ls-files --others --exclude-standard)
    if ($LASTEXITCODE -ne 0) { throw "Unable to enumerate untracked source files" }
    $UntrackedRows = foreach ($Relative in $Untracked) {
        $Full = Join-Path $RepoRoot $Relative
        if (-not (Test-Path -LiteralPath $Full -PathType Leaf)) {
            throw "Untracked source entry is not a regular file: $Relative"
        }
        "$($Relative.Replace('\', '/'))`t$(File-Sha256 $Full)"
    }
    $BindingRows = [System.Collections.Generic.List[string]]::new()
    $BindingRows.Add("head=$Head")
    $BindingRows.Add("tree=$Tree")
    $BindingRows.Add("status:")
    foreach ($Row in $StatusRows) { $BindingRows.Add([string]$Row) }
    $BindingRows.Add("diff:")
    foreach ($Row in $DiffRows) { $BindingRows.Add([string]$Row) }
    $BindingRows.Add("untracked:")
    foreach ($Row in $UntrackedRows) { $BindingRows.Add([string]$Row) }
    $BindingPayload = $BindingRows.ToArray() -join "`n"
    return [pscustomobject]@{
        Head = $Head
        Tree = $Tree
        StatusRows = @($StatusRows)
        Clean = $StatusRows.Count -eq 0
        ContentBindingSha256 = Bytes-Sha256 ([System.Text.Encoding]::UTF8.GetBytes($BindingPayload))
    }
}

function Assert-RepositorySourceSnapshot($Expected, [string]$Stage) {
    $Observed = Get-RepositorySourceSnapshot
    $ExpectedStatus = @($Expected.StatusRows)
    $ObservedStatus = @($Observed.StatusRows)
    $StatusMatches = $ExpectedStatus.Count -eq $ObservedStatus.Count
    if ($StatusMatches) {
        for ($Index = 0; $Index -lt $ExpectedStatus.Count; $Index++) {
            if (-not $ExpectedStatus[$Index].Equals(
                $ObservedStatus[$Index],
                [System.StringComparison]::Ordinal
            )) {
                $StatusMatches = $false
                break
            }
        }
    }
    if (-not $Expected.Head.Equals($Observed.Head, [System.StringComparison]::OrdinalIgnoreCase) -or
        -not $Expected.Tree.Equals($Observed.Tree, [System.StringComparison]::OrdinalIgnoreCase) -or
        -not $StatusMatches -or
        -not $Expected.ContentBindingSha256.Equals(
            $Observed.ContentBindingSha256,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
        throw "Repository source drifted at $Stage. Expected HEAD/tree/binding [$($Expected.Head)/$($Expected.Tree)/$($Expected.ContentBindingSha256)] with status [$($ExpectedStatus -join ' | ')]; observed [$($Observed.Head)/$($Observed.Tree)/$($Observed.ContentBindingSha256)] with status [$($ObservedStatus -join ' | ')]"
    }
}

function Quote-CommandArg([string]$Value) {
    return "'" + $Value.Replace("'", "''") + "'"
}

function Test-PathWithinOrEqual([string]$Candidate, [string]$Root) {
    $CandidateFull = [System.IO.Path]::GetFullPath($Candidate).TrimEnd("\", "/")
    $RootFull = [System.IO.Path]::GetFullPath($Root).TrimEnd("\", "/")
    return $CandidateFull.Equals($RootFull, [System.StringComparison]::OrdinalIgnoreCase) -or
        $CandidateFull.StartsWith($RootFull + "\", [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-DirectoryRootsDisjoint([string]$First, [string]$FirstLabel, [string]$Second, [string]$SecondLabel) {
    if ((Test-PathWithinOrEqual $First $Second) -or (Test-PathWithinOrEqual $Second $First)) {
        throw "$FirstLabel and $SecondLabel must be disjoint: [$First] versus [$Second]"
    }
}

function Assert-ExplicitRootOutsideRepo([string]$Path, [bool]$WasProvided, [string]$Label) {
    if ($WasProvided -and
        ((Test-PathWithinOrEqual $Path $RepoRoot) -or (Test-PathWithinOrEqual $RepoRoot $Path))) {
        throw "Explicit $Label must be disjoint from the repository root: $Path"
    }
}

function Assert-FileOutsideRoot([string]$FilePath, [string]$FileLabel, [string]$Root, [string]$RootLabel) {
    if (Test-PathWithinOrEqual $FilePath $Root) {
        throw "$FileLabel must not be inside $RootLabel`: $FilePath"
    }
}

function Resolve-PrebuiltNativeBenchmarkExecutable(
    [string]$Path,
    [string]$OutputRoot,
    [string]$WorkRoot
) {
    $Canonical = Get-CanonicalSafePath $Path "PrebuiltNativeBenchmarkExecutable"
    if (-not (Test-Path -LiteralPath $Canonical -PathType Leaf)) {
        throw "PrebuiltNativeBenchmarkExecutable must be an existing plain file: $Canonical"
    }
    Assert-FileOutsideRoot $Canonical "PrebuiltNativeBenchmarkExecutable" $OutputRoot "OutputRoot"
    Assert-FileOutsideRoot $Canonical "PrebuiltNativeBenchmarkExecutable" $WorkRoot "WorkRoot"
    Assert-FileOutsideRoot $Canonical "PrebuiltNativeBenchmarkExecutable" $RepoRoot "repository root"
    return $Canonical
}

function Resolve-PrebuiltNativeBenchmarkReceipt(
    [string]$Path,
    [string]$OutputRoot,
    [string]$WorkRoot
) {
    $Canonical = Get-CanonicalSafePath $Path "PrebuiltNativeBenchmarkReceipt"
    if (-not (Test-Path -LiteralPath $Canonical -PathType Leaf)) {
        throw "PrebuiltNativeBenchmarkReceipt must be an existing plain file: $Canonical"
    }
    Assert-FileOutsideRoot $Canonical "PrebuiltNativeBenchmarkReceipt" $OutputRoot "OutputRoot"
    Assert-FileOutsideRoot $Canonical "PrebuiltNativeBenchmarkReceipt" $WorkRoot "WorkRoot"
    Assert-FileOutsideRoot $Canonical "PrebuiltNativeBenchmarkReceipt" $RepoRoot "repository root"
    return $Canonical
}

function Initialize-BenchmarkRoot([string]$Path, [string]$LegacyParent, [bool]$WasProvided, [string]$Label) {
    $ResolvedPath = [System.IO.Path]::GetFullPath($Path)
    $FileSystemRoot = [System.IO.Path]::GetPathRoot($ResolvedPath).TrimEnd("\", "/")
    if ($ResolvedPath.TrimEnd("\", "/").Equals($FileSystemRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to use a filesystem root as $Label`: $ResolvedPath"
    }
    if ($WasProvided) {
        if (Test-Path -LiteralPath $ResolvedPath) {
            throw "Explicit $Label must be a new path; refusing to clear or reuse: $ResolvedPath"
        }
        New-Item -ItemType Directory -Path $ResolvedPath | Out-Null
        return
    }
    Clear-DirectoryUnder $LegacyParent $ResolvedPath
}

# M59 provenance guard. Returns the schema-level `leading_syllable_reachability`
# state of a schema file: 'absent(engine-default)' when the flag is not present
# (the row relies on the engine default), or the literal value otherwise. The
# finding-#8 measurement hole was exactly this drifting: the benchmark deployed a
# luna without the flag while the shipped product carried it, so the ratchet
# silently measured the feature OFF while the product shipped it ON.
function Get-ReachabilityFlagState($SchemaPath) {
    if (-not (Test-Path -LiteralPath $SchemaPath)) { return 'schema-missing' }
    $match = Select-String -LiteralPath $SchemaPath `
        -Pattern '^\s*leading_syllable_reachability\s*:\s*(\S+)' | Select-Object -First 1
    if ($null -eq $match) { return 'absent(engine-default)' }
    return $match.Matches[0].Groups[1].Value
}

function Clear-DirectoryUnder($Root, $Path) {
    $ResolvedRoot = [System.IO.Path]::GetFullPath($Root).TrimEnd('\')
    $ResolvedPath = [System.IO.Path]::GetFullPath($Path)
    if (-not $ResolvedPath.StartsWith($ResolvedRoot + "\", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clear directory outside $ResolvedRoot`: $ResolvedPath"
    }
    if (Test-Path -LiteralPath $ResolvedPath) {
        Remove-Item -LiteralPath $ResolvedPath -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $ResolvedPath | Out-Null
}

function Copy-DirectoryContents($Source, $Destination) {
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Get-ChildItem -LiteralPath $Source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $Destination $_.Name) -Recurse -Force
    }
}

function Set-RunStatus([string]$Status, [string]$Detail = "") {
    $SanitizedDetail = $Detail.Replace("`r", " ").Replace("`n", " ")
    @(
        "status=$Status",
        "date_utc=$([DateTime]::UtcNow.ToString('o'))",
        "detail=$SanitizedDetail"
    ) | Set-Content -LiteralPath $RunStatusPath -Encoding UTF8
}

function Invoke-Logged($Description, [string[]]$ArgumentList, $LogPath, $ExtraPath = "") {
    $LogDir = Split-Path -Parent $LogPath
    New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
    $StdOut = Join-Path $LogDir "$Description.stdout.tmp"
    $StdErr = Join-Path $LogDir "$Description.stderr.tmp"
    Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue
    $PreviousPath = $env:PATH
    $PreviousErrorActionPreference = $ErrorActionPreference
    try {
        if (-not [string]::IsNullOrWhiteSpace($ExtraPath)) {
            $env:PATH = ($ExtraPath, $PreviousPath -join ";")
        }
        Push-Location $RepoRoot
        try {
            $ErrorActionPreference = "SilentlyContinue"
            & cargo @ArgumentList 1> $StdOut 2> $StdErr
            $ExitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $PreviousErrorActionPreference
            Pop-Location
        }
        $Output = @()
        if (Test-Path -LiteralPath $StdOut) {
            $Output += Get-Content -LiteralPath $StdOut
        }
        if (Test-Path -LiteralPath $StdErr) {
            $Output += Get-Content -LiteralPath $StdErr
        }
        $Output | Set-Content -LiteralPath $LogPath -Encoding UTF8
        $Output | ForEach-Object { Write-Host $_ }
        if ($ExitCode -ne 0) {
            throw "$Description failed with exit code $ExitCode"
        }
    } finally {
        Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue
        $env:PATH = $PreviousPath
        $ErrorActionPreference = $PreviousErrorActionPreference
    }
}

function Build-NativeBenchmarkExecutable($LogPath) {
    $Description = "cargo-build-native-inprocess-benchmark"
    $LogDir = Split-Path -Parent $LogPath
    New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
    $StdOut = Join-Path $LogDir "$Description.stdout.tmp"
    $StdErr = Join-Path $LogDir "$Description.stderr.tmp"
    Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue
    $PreviousErrorActionPreference = $ErrorActionPreference
    $PreviousCargoTargetDir = $env:CARGO_TARGET_DIR
    try {
        $env:CARGO_TARGET_DIR = $BenchmarkCargoTargetRoot
        Push-Location $RepoRoot
        try {
            $ErrorActionPreference = "SilentlyContinue"
            & cargo bench -p yune-rime-api --bench native_inprocess_benchmark --no-run --message-format=json-render-diagnostics 1> $StdOut 2> $StdErr
            $ExitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $PreviousErrorActionPreference
            Pop-Location
        }
        $StdOutLines = if (Test-Path -LiteralPath $StdOut) { @(Get-Content -LiteralPath $StdOut) } else { @() }
        $StdErrLines = if (Test-Path -LiteralPath $StdErr) { @(Get-Content -LiteralPath $StdErr) } else { @() }
        @($StdOutLines) + @($StdErrLines) | Set-Content -LiteralPath $LogPath -Encoding UTF8
        if ($ExitCode -ne 0) {
            throw "$Description failed with exit code $ExitCode"
        }

        $Executables = [System.Collections.Generic.List[string]]::new()
        foreach ($Line in $StdOutLines) {
            try {
                $Message = $Line | ConvertFrom-Json
            }
            catch {
                continue
            }
            if ($Message.reason -eq "compiler-artifact" -and
                $Message.target.name -eq "native_inprocess_benchmark" -and
                @($Message.target.kind) -contains "bench" -and
                -not [string]::IsNullOrWhiteSpace($Message.executable)) {
                $Executables.Add([string]$Message.executable)
            }
        }
        $UniqueExecutables = @($Executables | Select-Object -Unique)
        if ($UniqueExecutables.Count -ne 1) {
            throw "$Description expected exactly one executable artifact, found $($UniqueExecutables.Count)"
        }
        $Executable = Get-CanonicalSafePath $UniqueExecutables[0] "native benchmark executable"
        Assert-Path $Executable "native benchmark executable"
        return $Executable
    }
    finally {
        Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue
        $env:CARGO_TARGET_DIR = $PreviousCargoTargetDir
        $ErrorActionPreference = $PreviousErrorActionPreference
    }
}

function Select-NativeBenchmarkExecutable(
    [bool]$BuildPerformed,
    [string]$PrebuiltExecutable,
    [string]$BuildLogPath
) {
    if ($BuildPerformed) {
        return Build-NativeBenchmarkExecutable $BuildLogPath
    }
    return $PrebuiltExecutable
}

function Invoke-NativeBenchmarkLogged($Description, [string[]]$ArgumentList, $LogPath, $ExtraPath = "") {
    if ([string]::IsNullOrWhiteSpace($NativeBenchmarkExecutable)) {
        throw "Native benchmark executable has not been built"
    }
    $LogDir = Split-Path -Parent $LogPath
    New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
    $StdOut = Join-Path $LogDir "$Description.stdout.tmp"
    $StdErr = Join-Path $LogDir "$Description.stderr.tmp"
    Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue
    $PreviousPath = $env:PATH
    $PreviousErrorActionPreference = $ErrorActionPreference
    try {
        if (-not [string]::IsNullOrWhiteSpace($ExtraPath)) {
            $env:PATH = ($ExtraPath, $PreviousPath -join ";")
        }
        Push-Location $RepoRoot
        try {
            $ErrorActionPreference = "SilentlyContinue"
            & $NativeBenchmarkExecutable @ArgumentList 1> $StdOut 2> $StdErr
            $ExitCode = $LASTEXITCODE
        }
        finally {
            $ErrorActionPreference = $PreviousErrorActionPreference
            Pop-Location
        }
        $Output = @()
        if (Test-Path -LiteralPath $StdOut) { $Output += Get-Content -LiteralPath $StdOut }
        if (Test-Path -LiteralPath $StdErr) { $Output += Get-Content -LiteralPath $StdErr }
        $Output | Set-Content -LiteralPath $LogPath -Encoding UTF8
        $Output | ForEach-Object { Write-Host $_ }
        if ($ExitCode -ne 0) {
            throw "$Description failed with exit code $ExitCode"
        }
    }
    finally {
        Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue
        $env:PATH = $PreviousPath
        $ErrorActionPreference = $PreviousErrorActionPreference
    }
}

function Prepare-UpstreamRun($EngineName, $DllPath) {
    $RunRoot = Join-Path $WorkRoot $EngineName
    Clear-DirectoryUnder $WorkRoot $RunRoot
    Copy-Item -LiteralPath $DllPath -Destination (Join-Path $RunRoot "rime.dll") -Force
    Copy-DirectoryContents $SharedSource (Join-Path $RunRoot "shared")
    New-Item -ItemType Directory -Force -Path (Join-Path $RunRoot "user") | Out-Null
    Copy-DirectoryContents $BuildSource (Join-Path $RunRoot "user\build")
    return $RunRoot
}

function Prepare-ProductRun($EngineName, $DllPath) {
    $RunRoot = Join-Path $WorkRoot $EngineName
    Clear-DirectoryUnder $WorkRoot $RunRoot
    Copy-Item -LiteralPath $DllPath -Destination (Join-Path $RunRoot "rime.dll") -Force
    Copy-DirectoryContents $ProductSchemaRoot (Join-Path $RunRoot "shared")
    New-Item -ItemType Directory -Force -Path (Join-Path $RunRoot "user") | Out-Null
    Copy-DirectoryContents (Join-Path $ProductSchemaRoot "build") (Join-Path $RunRoot "user\build")
    return $RunRoot
}

function Run-NativeBench(
    $EngineName,
    $Track,
    $Schema,
    $RunRoot,
    $ExtraPath,
    $Inputs,
    $OutputName,
    [switch]$DeployBeforeBenchmark
) {
    $EngineOutput = Join-Path $OutputRoot $OutputName
    Clear-DirectoryUnder $OutputRoot $EngineOutput
    $LogPath = Join-Path $EngineOutput "cargo-bench-native-inprocess.log"
    $BenchArgs = @(
        "--engine", $EngineName,
        "--track", $Track,
        "--schema", $Schema,
        "--dll", (Join-Path $RunRoot "rime.dll"),
        "--shared", (Join-Path $RunRoot "shared"),
        "--user", (Join-Path $RunRoot "user"),
        "--build", (Join-Path $RunRoot "user\build"),
        "--output", $EngineOutput,
        "--inputs", $Inputs,
        "--iterations", "$Iterations",
        "--session-iterations", "$SessionIterations",
        "--key-iterations", "$KeyIterations"
    )
    if ($DeployBeforeBenchmark) {
        $BenchArgs += "--deploy-before-benchmark"
    }
    Invoke-NativeBenchmarkLogged "$OutputName-native-inprocess" $BenchArgs $LogPath (($RunRoot, $ExtraPath) -join ";")
}

function Invoke-DeployPrep(
    $EngineName,
    $Track,
    $Schema,
    $RunRoot,
    $ExtraPath,
    $OutputName
) {
    $PrepOutput = Join-Path $WorkRoot "$OutputName-deploy-prep-output"
    Clear-DirectoryUnder $WorkRoot $PrepOutput
    $LogPath = Join-Path $OutputRoot "$OutputName-deploy-prep.log"
    $BenchArgs = @(
        "--engine", $EngineName,
        "--track", $Track,
        "--schema", $Schema,
        "--dll", (Join-Path $RunRoot "rime.dll"),
        "--shared", (Join-Path $RunRoot "shared"),
        "--user", (Join-Path $RunRoot "user"),
        "--build", (Join-Path $RunRoot "user\build"),
        "--output", $PrepOutput,
        "--inputs", "deploy-prep",
        "--iterations", "1",
        "--session-iterations", "1",
        "--key-iterations", "1",
        "--deploy-before-benchmark",
        "--deploy-only"
    )
    Invoke-NativeBenchmarkLogged "$OutputName-deploy-prep" $BenchArgs $LogPath (($RunRoot, $ExtraPath) -join ";")
}

function Invoke-TrackAPoetDeployPrep(
    $RunRoot,
    $ExtraPath
) {
    # The signed Track A gate measures the default owned sentence model. Its
    # clean deploy nevertheless needs a poet artifact so the wrapper can restore
    # the upstream table/prism/reverse bytes and add only Yune's deterministic
    # poet sidecar. Poet *generation* is guarded by the same opt-in as poet
    # consumption, so scope the opt-in to the separate deploy-prep invocation and
    # restore the benchmark environment before any timing process starts.
    $PreviousPoetByteBacked = [Environment]::GetEnvironmentVariable(
        "YUNE_POET_BYTE_BACKED",
        "Process"
    )
    if ($PreviousPoetByteBacked -eq "1") {
        throw "The signed Track A gate requires default-owned poet measurement; unset YUNE_POET_BYTE_BACKED before running it."
    }
    try {
        [Environment]::SetEnvironmentVariable("YUNE_POET_BYTE_BACKED", "1", "Process")
        Invoke-DeployPrep "yune" "track-a-comparison" "luna_pinyin" $RunRoot $ExtraPath "track-a-yune"
    }
    finally {
        [Environment]::SetEnvironmentVariable(
            "YUNE_POET_BYTE_BACKED",
            $PreviousPoetByteBacked,
            "Process"
        )
    }
    $RestoredPoetByteBacked = [Environment]::GetEnvironmentVariable(
        "YUNE_POET_BYTE_BACKED",
        "Process"
    )
    if ($RestoredPoetByteBacked -ne $PreviousPoetByteBacked) {
        throw "Failed to restore YUNE_POET_BYTE_BACKED after Track A deploy prep."
    }
}

function Write-TrackAComparison($Rows, $DestinationPath) {
    $YuneRows = @{}
    $LibrimeRows = @{}
    foreach ($Row in $Rows) {
        if ($Row.track -ne "track-a-comparison" -or $Row.schema_id -ne "luna_pinyin") {
            continue
        }
        $Key = "$($Row.workload)|$($Row.input)"
        if ($Row.engine -eq "yune") {
            $YuneRows[$Key] = $Row
        } elseif ($Row.engine -eq "librime-1.17.0") {
            $LibrimeRows[$Key] = $Row
        }
    }

    $Comparison = foreach ($Key in ($YuneRows.Keys | Sort-Object)) {
        if (-not $LibrimeRows.ContainsKey($Key)) {
            continue
        }
        $Yune = $YuneRows[$Key]
        $Librime = $LibrimeRows[$Key]
        $YuneMedian = [double]$Yune.median_us
        $LibrimeMedian = [double]$Librime.median_us
        $Ratio = if ($LibrimeMedian -eq 0.0) { [double]::PositiveInfinity } else { $YuneMedian / $LibrimeMedian }
        [pscustomobject]@{
            track = $Yune.track
            schema_id = $Yune.schema_id
            workload = $Yune.workload
            input = $Yune.input
            yune_median_us_raw = $YuneMedian
            librime_median_us_raw = $LibrimeMedian
            yune_librime_median_ratio_raw = $Ratio
            absolute_gap_us_raw = $YuneMedian - $LibrimeMedian
            yune_median_us = "{0:F3}" -f $YuneMedian
            librime_median_us = "{0:F3}" -f $LibrimeMedian
            yune_librime_median_ratio = "{0:F3}" -f $Ratio
            absolute_gap_us = "{0:F3}" -f ($YuneMedian - $LibrimeMedian)
            yune_max_peak_working_set_bytes = $Yune.max_peak_working_set_bytes
            librime_max_peak_working_set_bytes = $Librime.max_peak_working_set_bytes
            yune_median_private_bytes = $Yune.median_private_bytes
            librime_median_private_bytes = $Librime.median_private_bytes
        }
    }

    $Comparison |
        Select-Object track, schema_id, workload, input, yune_median_us, librime_median_us, yune_librime_median_ratio, absolute_gap_us, yune_max_peak_working_set_bytes, librime_max_peak_working_set_bytes, yune_median_private_bytes, librime_median_private_bytes |
        Export-Csv -LiteralPath $DestinationPath -NoTypeInformation -Encoding UTF8
    return @($Comparison)
}

function Invoke-TrackAThresholdCheck($ComparisonRows, $SummaryRows, $MemoryOwnerRows, $ThresholdPath, $DestinationPath, [switch]$Fail) {
    if ([string]::IsNullOrWhiteSpace($ThresholdPath)) {
        if ($Fail) {
            throw "-FailOnRegression requires -TrackAThresholds"
        }
        return
    }
    $ResolvedThresholdPath = [System.IO.Path]::GetFullPath($ThresholdPath)
    Assert-Path $ResolvedThresholdPath "Track A thresholds"

    $ThresholdRows = Import-Csv -LiteralPath $ResolvedThresholdPath
    $YunePeakOwner = $MemoryOwnerRows |
        Where-Object {
            $_.engine -eq "yune" -and
            $_.track -eq "track-a-comparison" -and
            $_.schema_id -eq "luna_pinyin" -and
            $_.owner_id -eq "process.peak_working_set_high_water"
        } |
        Select-Object -First 1
    $SummaryPeak = ($ComparisonRows |
        ForEach-Object { [UInt64]$_.yune_max_peak_working_set_bytes } |
        Measure-Object -Maximum).Maximum
    $ObservedPeak = if ($null -ne $YunePeakOwner) { [UInt64]$YunePeakOwner.retained_estimate_bytes } else { [UInt64]$SummaryPeak }

    $Results = foreach ($Threshold in $ThresholdRows) {
        $Observed = $null
        if ($Threshold.kind -eq "latency_ratio") {
            $Match = $ComparisonRows |
                Where-Object {
                    $_.workload -eq $Threshold.workload -and
                    $_.input -eq $Threshold.input
                } |
                Select-Object -First 1
            if ($null -eq $Match) {
                [pscustomobject]@{
                    kind = $Threshold.kind
                    workload = $Threshold.workload
                    input = $Threshold.input
                    metric = $Threshold.metric
                    observed = ""
                    ceiling = $Threshold.ceiling
                    unit = $Threshold.unit
                    status = "missing"
                    notes = $Threshold.notes
                }
                continue
            }
            $Observed = [double]$Match.yune_librime_median_ratio_raw
        } elseif ($Threshold.kind -eq "latency_absolute_us" -or $Threshold.kind -eq "memory_absolute_bytes") {
            $Workload = $Threshold.workload
            $Track = ""
            if ($Workload -like "*/*") {
                $Parts = $Workload.Split("/", 2)
                $Track = $Parts[0]
                $Workload = $Parts[1]
            }
            $Match = $SummaryRows |
                Where-Object {
                    ($Track -eq "" -or $_.track -eq $Track) -and
                    $_.workload -eq $Workload -and
                    $_.input -eq $Threshold.input -and
                    $_.engine -eq "yune"
                } |
                Select-Object -First 1
            if ($null -eq $Match) {
                [pscustomobject]@{
                    kind = $Threshold.kind
                    workload = $Threshold.workload
                    input = $Threshold.input
                    metric = $Threshold.metric
                    observed = ""
                    ceiling = $Threshold.ceiling
                    unit = $Threshold.unit
                    status = "missing"
                    notes = $Threshold.notes
                }
                continue
            }
            if ($Threshold.metric -eq "median_us") {
                $Observed = [double]$Match.median_us
            } elseif ($Threshold.metric -eq "max_peak_working_set_bytes") {
                $Observed = [double]$Match.max_peak_working_set_bytes
            } elseif ($Threshold.metric -eq "median_working_set_bytes") {
                $Observed = [double]$Match.median_working_set_bytes
            } elseif ($Threshold.metric -eq "median_private_bytes") {
                $Observed = [double]$Match.median_private_bytes
            } else {
                [pscustomobject]@{
                    kind = $Threshold.kind
                    workload = $Threshold.workload
                    input = $Threshold.input
                    metric = $Threshold.metric
                    observed = ""
                    ceiling = $Threshold.ceiling
                    unit = $Threshold.unit
                    status = "unknown-metric"
                    notes = $Threshold.notes
                }
                continue
            }
        } elseif ($Threshold.kind -eq "memory_peak") {
            $Observed = [double]$ObservedPeak
        } else {
            [pscustomobject]@{
                kind = $Threshold.kind
                workload = $Threshold.workload
                input = $Threshold.input
                metric = $Threshold.metric
                observed = ""
                ceiling = $Threshold.ceiling
                unit = $Threshold.unit
                status = "unknown-kind"
                notes = $Threshold.notes
            }
            continue
        }

        $Ceiling = [double]$Threshold.ceiling
        $Status = if ($Observed -le $Ceiling) { "pass" } else { "fail" }
        [pscustomobject]@{
            kind = $Threshold.kind
            workload = $Threshold.workload
            input = $Threshold.input
            metric = $Threshold.metric
            observed = if ($Threshold.unit -eq "bytes") { "{0:F0}" -f $Observed } else { "{0:F3}" -f $Observed }
            ceiling = $Threshold.ceiling
            unit = $Threshold.unit
            status = $Status
            notes = $Threshold.notes
        }
    }

    $Results | Export-Csv -LiteralPath $DestinationPath -NoTypeInformation -Encoding UTF8
    $Failures = @($Results | Where-Object { $_.status -ne "pass" })
    if ($Fail -and $Failures.Count -gt 0) {
        $FailureSummary = ($Failures | ForEach-Object { "$($_.metric)[$($_.input)] observed=$($_.observed) ceiling=$($_.ceiling) status=$($_.status)" }) -join "; "
        throw "Track A threshold regression detected: $FailureSummary"
    }
}

Assert-Path $UpstreamOracleRoot "upstream oracle root"
Assert-Path $SharedSource "upstream shared data"
Assert-Path $BuildSource "upstream prebuilt build data"
Assert-Path $UpstreamDll "upstream rime.dll"
Assert-Path $ProductSchemaRoot "Yune web product schema assets"
if ($YuneDllWasProvided) {
    Assert-Path $YuneDll "supplied Yune DLL"
}
if ($NativeBenchmarkExecutableWasProvided) {
    $PrebuiltNativeBenchmarkExecutable = Resolve-PrebuiltNativeBenchmarkExecutable `
        $PrebuiltNativeBenchmarkExecutable `
        $OutputRoot `
        $WorkRoot
    $PrebuiltNativeBenchmarkReceipt = Resolve-PrebuiltNativeBenchmarkReceipt `
        $PrebuiltNativeBenchmarkReceipt `
        $OutputRoot `
        $WorkRoot
}
if (-not [string]::IsNullOrWhiteSpace($TrackAThresholds)) {
    Assert-Path $TrackAThresholds "Track A thresholds"
}
$InitialSourceSnapshot = Get-RepositorySourceSnapshot
$YuneHead = $InitialSourceSnapshot.Head
$YuneTree = $InitialSourceSnapshot.Tree
$YuneStatusRows = @($InitialSourceSnapshot.StatusRows)
$SourceClean = $InitialSourceSnapshot.Clean
$SourceContentBindingSha256 = $InitialSourceSnapshot.ContentBindingSha256
$YuneStatus = $YuneStatusRows -join " | "
Assert-BenchmarkSourcePolicy $YuneStatusRows $AllowDirty.IsPresent $TrackAThresholds

Assert-DirectoryRootsDisjoint $OutputRoot "OutputRoot" $WorkRoot "WorkRoot"
Assert-DirectoryRootsDisjoint $OutputRoot "OutputRoot" $UpstreamOracleRoot "UpstreamOracleRoot"
Assert-DirectoryRootsDisjoint $WorkRoot "WorkRoot" $UpstreamOracleRoot "UpstreamOracleRoot"
Assert-DirectoryRootsDisjoint $OutputRoot "OutputRoot" $ProductSchemaRoot "ProductSchemaRoot"
Assert-DirectoryRootsDisjoint $WorkRoot "WorkRoot" $ProductSchemaRoot "ProductSchemaRoot"
Assert-ExplicitRootOutsideRepo $OutputRoot $OutputRootWasProvided "OutputRoot"
Assert-ExplicitRootOutsideRepo $WorkRoot $WorkRootWasProvided "WorkRoot"
Assert-FileOutsideRoot $YuneDll "YuneDll" $OutputRoot "OutputRoot"
Assert-FileOutsideRoot $YuneDll "YuneDll" $WorkRoot "WorkRoot"
if (-not [string]::IsNullOrWhiteSpace($TrackAThresholds)) {
    Assert-FileOutsideRoot $TrackAThresholds "TrackAThresholds" $OutputRoot "OutputRoot"
    Assert-FileOutsideRoot $TrackAThresholds "TrackAThresholds" $WorkRoot "WorkRoot"
}

Initialize-BenchmarkRoot $OutputRoot $EvidenceRoot $OutputRootWasProvided "OutputRoot"
$OutputRootAfterCreate = Get-CanonicalSafePath $OutputRoot "created OutputRoot"
if (-not $OutputRootAfterCreate.Equals($OutputRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "OutputRoot physical path changed during creation: [$OutputRoot] -> [$OutputRootAfterCreate]"
}
$RunStatusPath = Join-Path $OutputRoot "run-status.txt"
Set-RunStatus "in-progress"

try {
    Initialize-BenchmarkRoot $WorkRoot $LegacyWorkRoot $WorkRootWasProvided "WorkRoot"
    $WorkRootAfterCreate = Get-CanonicalSafePath $WorkRoot "created WorkRoot"
    if (-not $WorkRootAfterCreate.Equals($WorkRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "WorkRoot physical path changed during creation: [$WorkRoot] -> [$WorkRootAfterCreate]"
    }
    Assert-DirectoryRootsDisjoint $OutputRootAfterCreate "OutputRoot" $WorkRootAfterCreate "WorkRoot"
    Assert-DirectoryRootsDisjoint $OutputRootAfterCreate "OutputRoot" $UpstreamOracleRoot "UpstreamOracleRoot"
    Assert-DirectoryRootsDisjoint $WorkRootAfterCreate "WorkRoot" $UpstreamOracleRoot "UpstreamOracleRoot"
    Assert-DirectoryRootsDisjoint $OutputRootAfterCreate "OutputRoot" $ProductSchemaRoot "ProductSchemaRoot"
    Assert-DirectoryRootsDisjoint $WorkRootAfterCreate "WorkRoot" $ProductSchemaRoot "ProductSchemaRoot"

    $BuildPerformed = -not $YuneDllWasProvided
    if ($BuildPerformed) {
        Push-Location $RepoRoot
        try {
            Invoke-Logged "cargo-build-release-yune-rime-api" @("build", "--release", "-p", "yune-rime-api") (Join-Path $OutputRoot "cargo-build-release-yune-rime-api.log")
        }
        finally {
            Pop-Location
        }
    }
    Assert-Path $YuneDll "Yune release DLL"
    $YuneDllAfterBuild = Get-CanonicalSafePath $YuneDll "Yune release DLL"
    if (-not $YuneDllAfterBuild.Equals($YuneDll, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Yune DLL physical path changed during build: [$YuneDll] -> [$YuneDllAfterBuild]"
    }

    $YuneDllSha256 = File-Sha256 $YuneDll
    $UpstreamDllSha256 = File-Sha256 $UpstreamDll
    $UpstreamSharedTreeSha256 = Tree-Sha256 $SharedSource
    $UpstreamBuildTreeSha256 = Tree-Sha256 $BuildSource
    $ProductSchemaTreeSha256 = Tree-Sha256 $ProductSchemaRoot
    $BenchmarkScriptSha256 = File-Sha256 $PSCommandPath
    $BenchmarkRustSource = Get-CanonicalSafePath `
        (Join-Path $RepoRoot "crates\yune-rime-api\benches\native_inprocess_benchmark.rs") `
        "native benchmark Rust source"
    $CargoLock = Get-CanonicalSafePath (Join-Path $RepoRoot "Cargo.lock") "Cargo.lock"
    $BenchmarkRustSourceSha256 = File-Sha256 $BenchmarkRustSource
    $CargoLockSha256 = File-Sha256 $CargoLock
    $RustcIdentitySha256 = Command-IdentitySha256 "rustc" @("-vV")
    $CargoIdentitySha256 = Command-IdentitySha256 "cargo" @("-V")
    $TrackAThresholdsSha256 = if ([string]::IsNullOrWhiteSpace($TrackAThresholds)) { "" } else { File-Sha256 $TrackAThresholds }

    $NativeBenchmarkBuildPerformed = -not $NativeBenchmarkExecutableWasProvided
    $BenchmarkCargoTargetRoot = if ($NativeBenchmarkBuildPerformed) { Join-Path $WorkRoot "cargo-target" } else { "" }
    $BenchmarkCargoCommand = "cargo bench -p yune-rime-api --bench native_inprocess_benchmark --no-run --message-format=json-render-diagnostics"
    $BenchmarkBuildCommand = if ($NativeBenchmarkBuildPerformed) {
        "`$env:CARGO_TARGET_DIR=$(Quote-CommandArg $BenchmarkCargoTargetRoot); $BenchmarkCargoCommand"
    } else {
        ""
    }
    $NativeBenchmarkExecutable = Select-NativeBenchmarkExecutable `
        $NativeBenchmarkBuildPerformed `
        $PrebuiltNativeBenchmarkExecutable `
        (Join-Path $OutputRoot "cargo-build-native-inprocess-benchmark.log")
    Assert-Path $NativeBenchmarkExecutable "native benchmark executable"
    $NativeBenchmarkExecutableSha256 = File-Sha256 $NativeBenchmarkExecutable
    $ReceiptExpected = [ordered]@{
        format_version = "1"
        source_commit = $YuneHead
        source_tree = $YuneTree
        source_clean = [string]$SourceClean
        source_content_binding_sha256 = $SourceContentBindingSha256
        benchmark_script_sha256 = $BenchmarkScriptSha256
        benchmark_rust_source_sha256 = $BenchmarkRustSourceSha256
        cargo_lock_sha256 = $CargoLockSha256
        rustc_identity_sha256 = $RustcIdentitySha256
        cargo_identity_sha256 = $CargoIdentitySha256
        cargo_command = $BenchmarkCargoCommand
        native_benchmark_executable_path = $NativeBenchmarkExecutable
        native_benchmark_executable_sha256 = $NativeBenchmarkExecutableSha256
    }
    if ($NativeBenchmarkBuildPerformed) {
        $BenchmarkCargoTargetRoot = Get-CanonicalSafePath `
            $BenchmarkCargoTargetRoot `
            "native benchmark cargo target root"
        $NativeBenchmarkReceipt = Join-Path `
            $OutputRoot `
            "native-benchmark-build-receipt.txt"
        $ReceiptFields = [ordered]@{}
        foreach ($Entry in $ReceiptExpected.GetEnumerator()) {
            $ReceiptFields[[string]$Entry.Key] = [string]$Entry.Value
        }
        $ReceiptFields["native_benchmark_build_command"] = $BenchmarkBuildCommand
        $ReceiptFields["cargo_target_root"] = $BenchmarkCargoTargetRoot
        Write-NativeBenchmarkBuildReceipt $NativeBenchmarkReceipt $ReceiptFields
        $NativeBenchmarkReceipt = Get-CanonicalSafePath `
            $NativeBenchmarkReceipt `
            "native benchmark build receipt"
        $NativeBenchmarkReceiptInput = $NativeBenchmarkReceipt
        $NativeBenchmarkReceiptInputSha256 = File-Sha256 `
            $NativeBenchmarkReceiptInput
    } else {
        $NativeBenchmarkReceiptInput = $PrebuiltNativeBenchmarkReceipt
        $NativeBenchmarkReceiptInputSha256 = File-Sha256 `
            $NativeBenchmarkReceiptInput
        $Receipt = Read-NativeBenchmarkBuildReceipt $NativeBenchmarkReceiptInput
        $ReceiptCargoTargetRoot = Get-CanonicalSafePath `
            ([string]$Receipt["cargo_target_root"]) `
            "receipt cargo target root"
        if (-not $ReceiptCargoTargetRoot.Equals(
            [string]$Receipt["cargo_target_root"],
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Native benchmark build receipt cargo target physical path drifted"
        }
        Assert-DirectoryRootsDisjoint `
            $ReceiptCargoTargetRoot `
            "receipt cargo target root" `
            $RepoRoot `
            "repository root"
        Assert-DirectoryRootsDisjoint `
            $ReceiptCargoTargetRoot `
            "receipt cargo target root" `
            $OutputRoot `
            "OutputRoot"
        Assert-DirectoryRootsDisjoint `
            $ReceiptCargoTargetRoot `
            "receipt cargo target root" `
            $WorkRoot `
            "WorkRoot"
        if (-not (Test-PathWithinOrEqual $NativeBenchmarkExecutable $ReceiptCargoTargetRoot)) {
            throw "PrebuiltNativeBenchmarkExecutable is not inside the receipt cargo target root"
        }
        $ReceiptExpected["native_benchmark_build_command"] = `
            "`$env:CARGO_TARGET_DIR=$(Quote-CommandArg $ReceiptCargoTargetRoot); $BenchmarkCargoCommand"
        $ReceiptExpected["cargo_target_root"] = $ReceiptCargoTargetRoot
        Assert-NativeBenchmarkBuildReceipt $Receipt $ReceiptExpected
        if ((File-Sha256 $NativeBenchmarkReceiptInput) -ne
            $NativeBenchmarkReceiptInputSha256) {
            throw "Prebuilt native benchmark build receipt changed during validation"
        }
        $NativeBenchmarkReceipt = Join-Path `
            $OutputRoot `
            "native-benchmark-build-receipt.txt"
        Copy-Item `
            -LiteralPath $NativeBenchmarkReceiptInput `
            -Destination $NativeBenchmarkReceipt
        $NativeBenchmarkReceipt = Get-CanonicalSafePath `
            $NativeBenchmarkReceipt `
            "packet native benchmark build receipt"
    }
    $NativeBenchmarkReceiptSha256 = File-Sha256 $NativeBenchmarkReceipt
    if ($NativeBenchmarkReceiptInputSha256 -ne $NativeBenchmarkReceiptSha256) {
        throw "Packet native benchmark build receipt differs from its validated input"
    }
    Assert-RepositorySourceSnapshot $InitialSourceSnapshot "post-build"

$TrackAYuneRun = Prepare-UpstreamRun "track-a-yune" $YuneDll
$TrackALibrimeRun = Prepare-UpstreamRun "track-a-librime-1.17.0" $UpstreamDll
if (-not $SkipTrackB) {
    $TrackBProductRun = Prepare-ProductRun "track-b-yune-product" $YuneDll
}

# M59 provenance guard: the benchmark's deployed luna and the shipped web-product
# luna must agree on the reachability state, or the ratchet measures a different
# feature than ships. Post-M59 both rely on the engine default (no schema flag);
# this fails loudly the instant either side re-introduces a schema-level flag that
# the other lacks. One assertion, no new inputs.
$DeployedLuna = Join-Path $TrackAYuneRun "shared\luna_pinyin.schema.yaml"
$ShippedLuna = Join-Path $ProductSchemaRoot "luna_pinyin.schema.yaml"
$DeployedFlag = Get-ReachabilityFlagState $DeployedLuna
$ShippedFlag = Get-ReachabilityFlagState $ShippedLuna
if ($DeployedFlag -ne $ShippedFlag) {
    throw "M59 provenance mismatch: benchmark luna leading_syllable_reachability=[$DeployedFlag] but selected product root=[$ShippedFlag]. The ratchet would measure a different feature state than ships (the finding-#8 hole). Reconcile $ProductSchemaRoot with the deployed luna before trusting these numbers."
}

$InvocationParts = @(
    "powershell",
    "-NoProfile",
    "-ExecutionPolicy", "Bypass",
    "-File", (Quote-CommandArg $PSCommandPath),
    "-OutputRoot", (Quote-CommandArg $OutputRoot),
    "-WorkRoot", (Quote-CommandArg $WorkRoot),
    "-UpstreamOracleRoot", (Quote-CommandArg $UpstreamOracleRoot),
    "-ProductSchemaRoot", (Quote-CommandArg $ProductSchemaRoot),
    "-YuneDll", (Quote-CommandArg $YuneDll),
    "-Iterations", "$Iterations",
    "-SessionIterations", "$SessionIterations",
    "-KeyIterations", "$KeyIterations",
    "-TrackAInputs", (Quote-CommandArg $TrackAInputs),
    "-TrackBInputs", (Quote-CommandArg $TrackBInputs)
)
if ($NativeBenchmarkExecutableWasProvided) {
    $InvocationParts += @(
        "-PrebuiltNativeBenchmarkExecutable",
        (Quote-CommandArg $PrebuiltNativeBenchmarkExecutable),
        "-PrebuiltNativeBenchmarkReceipt",
        (Quote-CommandArg $PrebuiltNativeBenchmarkReceipt)
    )
}
if ($DeployProductBeforeBenchmark) { $InvocationParts += "-DeployProductBeforeBenchmark" }
if ($SkipTrackB) { $InvocationParts += "-SkipTrackB" }
if (-not [string]::IsNullOrWhiteSpace($TrackAThresholds)) {
    $InvocationParts += @("-TrackAThresholds", (Quote-CommandArg $TrackAThresholds))
}
if ($FailOnRegression) { $InvocationParts += "-FailOnRegression" }
if ($AllowDirty) { $InvocationParts += "-AllowDirty" }
$ActualInvocation = $InvocationParts -join " "
$Commands = @()
if ($BuildPerformed) {
    $Commands += "cargo build --release -p yune-rime-api  # cwd=$(Quote-CommandArg $RepoRoot)"
}
if ($NativeBenchmarkBuildPerformed) {
    $Commands += "$BenchmarkBuildCommand  # cwd=$(Quote-CommandArg $RepoRoot)"
} else {
    $Commands += "# reused prebuilt native benchmark executable $(Quote-CommandArg $NativeBenchmarkExecutable) sha256=$NativeBenchmarkExecutableSha256; cargo bench build skipped"
}
$Commands += "# native benchmark build receipt $(Quote-CommandArg $NativeBenchmarkReceipt) sha256=$NativeBenchmarkReceiptSha256"
$Commands += $ActualInvocation
$Commands | Set-Content -LiteralPath (Join-Path $OutputRoot "commands.txt") -Encoding UTF8

@(
    "command=$ActualInvocation",
    "build_performed=$BuildPerformed",
    "yune_dll_supplied=$YuneDllWasProvided",
    "native_benchmark_executable_prebuilt=$NativeBenchmarkExecutableWasProvided",
    "native_benchmark_build_performed=$NativeBenchmarkBuildPerformed",
    "native_benchmark_receipt=$NativeBenchmarkReceipt",
    "native_benchmark_receipt_sha256=$NativeBenchmarkReceiptSha256"
) | Set-Content -LiteralPath (Join-Path $OutputRoot "actual-invocation.txt") -Encoding UTF8

$Identity = @(
    "date_utc=$([DateTime]::UtcNow.ToString('o'))",
    "repo_root=$RepoRoot",
    "yune_git_head=$YuneHead",
    "yune_git_tree=$YuneTree",
    "yune_git_status_short=$YuneStatus",
    "source_clean=$SourceClean",
    "source_content_binding_sha256=$SourceContentBindingSha256",
    "allow_dirty=$($AllowDirty.IsPresent)",
    "upstream_oracle_root=$UpstreamOracleRoot",
    "product_schema_root=$ProductSchemaRoot",
    "output_root=$OutputRoot",
    "work_root=$WorkRoot",
    "transient_work_root=$WorkRoot",
    "yune_dll=$YuneDll",
    "yune_dll_supplied=$YuneDllWasProvided",
    "build_performed=$BuildPerformed",
    "native_benchmark_executable=$NativeBenchmarkExecutable",
    "native_benchmark_executable_sha256=$NativeBenchmarkExecutableSha256",
    "native_benchmark_executable_prebuilt=$NativeBenchmarkExecutableWasProvided",
    "native_benchmark_build_performed=$NativeBenchmarkBuildPerformed",
    "native_benchmark_receipt=$NativeBenchmarkReceipt",
    "native_benchmark_receipt_sha256=$NativeBenchmarkReceiptSha256",
    "native_benchmark_cargo_target_root=$BenchmarkCargoTargetRoot",
    "native_benchmark_build_command=$BenchmarkBuildCommand",
    "benchmark_script_sha256=$BenchmarkScriptSha256",
    "track_a_thresholds_sha256=$TrackAThresholdsSha256",
    "actual_invocation=$ActualInvocation",
    "managed_runtime=false",
    "deploy_product_before_benchmark=$($DeployProductBeforeBenchmark.IsPresent)",
    "skip_track_b=$($SkipTrackB.IsPresent)",
    "track_a_thresholds=$TrackAThresholds",
    "fail_on_regression=$($FailOnRegression.IsPresent)",
    "track_a_inputs=$TrackAInputs",
    "track_b_inputs=$TrackBInputs",
    "iterations=$Iterations",
    "session_iterations=$SessionIterations",
    "key_iterations=$KeyIterations"
)
$Identity | Set-Content -LiteralPath (Join-Path $OutputRoot "environment.txt") -Encoding UTF8

@(
    "source_commit=$YuneHead",
    "source_tree=$YuneTree",
    "source_content_binding_sha256=$SourceContentBindingSha256",
    "measured_yune_dll_sha256=$YuneDllSha256",
    "upstream_rime_dll_sha256=$UpstreamDllSha256",
    "upstream_shared_tree_sha256=$UpstreamSharedTreeSha256",
    "upstream_build_tree_sha256=$UpstreamBuildTreeSha256",
    "product_schema_tree_sha256=$ProductSchemaTreeSha256",
    "native_benchmark_executable=$NativeBenchmarkExecutable",
    "native_benchmark_executable_sha256=$NativeBenchmarkExecutableSha256",
    "native_benchmark_executable_prebuilt=$NativeBenchmarkExecutableWasProvided",
    "native_benchmark_build_performed=$NativeBenchmarkBuildPerformed",
    "native_benchmark_receipt=$NativeBenchmarkReceipt",
    "native_benchmark_receipt_sha256=$NativeBenchmarkReceiptSha256",
    "benchmark_script_sha256=$BenchmarkScriptSha256"
) | Set-Content -LiteralPath (Join-Path $OutputRoot "external-provenance.txt") -Encoding UTF8

$TrackAYuneBuild = Join-Path $TrackAYuneRun "user\build"
$TrackAOriginalBuild = Join-Path $WorkRoot "track-a-yune-original-build"
$TrackAGeneratedPoet = Join-Path $WorkRoot "track-a-yune-luna_pinyin.poet.bin"
Clear-DirectoryUnder $WorkRoot $TrackAOriginalBuild
Copy-DirectoryContents $TrackAYuneBuild $TrackAOriginalBuild
Invoke-TrackAPoetDeployPrep $TrackAYuneRun $UpstreamDistLib
Assert-Path (Join-Path $TrackAYuneBuild "luna_pinyin.poet.bin") "Track A Yune poet artifact after deploy prep"
Copy-Item -LiteralPath (Join-Path $TrackAYuneBuild "luna_pinyin.poet.bin") -Destination $TrackAGeneratedPoet -Force
Clear-DirectoryUnder $WorkRoot $TrackAYuneBuild
Copy-DirectoryContents $TrackAOriginalBuild $TrackAYuneBuild
Copy-Item -LiteralPath $TrackAGeneratedPoet -Destination (Join-Path $TrackAYuneBuild "luna_pinyin.poet.bin") -Force
$TrackATable = Get-Item -LiteralPath (Join-Path $TrackAYuneBuild "luna_pinyin.table.bin")
$TrackAPoet = Get-Item -LiteralPath (Join-Path $TrackAYuneBuild "luna_pinyin.poet.bin")
@(
    "track_a_deploy_prep=separate_process",
    "poet_generation_environment=YUNE_POET_BYTE_BACKED=1 (deploy-prep invocation only)",
    "benchmark_poet_environment=$(if ([Environment]::GetEnvironmentVariable('YUNE_POET_BYTE_BACKED', 'Process') -eq '1') { 'invalid-byte-backed' } else { 'default-owned' })",
    "restored_oracle_build_artifacts=true",
    "poet_artifact=$(Join-Path $TrackAYuneBuild "luna_pinyin.poet.bin")",
    "poet_bytes=$($TrackAPoet.Length)",
    "table_artifact=$(Join-Path $TrackAYuneBuild "luna_pinyin.table.bin")",
    "table_bytes=$($TrackATable.Length)"
) | Set-Content -LiteralPath (Join-Path $OutputRoot "track-a-yune-deploy-prep-artifacts.txt") -Encoding UTF8
Run-NativeBench "yune" "track-a-comparison" "luna_pinyin" $TrackAYuneRun $UpstreamDistLib $TrackAInputs "track-a-yune"
Run-NativeBench "librime-1.17.0" "track-a-comparison" "luna_pinyin" $TrackALibrimeRun (($UpstreamDistLib, $UpstreamBin, $UpstreamDistBin) -join ";") $TrackAInputs "track-a-librime-1.17.0"
if (-not $SkipTrackB) {
    Run-NativeBench "yune" "track-b-product" "jyut6ping3_mobile" $TrackBProductRun $RepoRoot $TrackBInputs "track-b-yune-product" -DeployBeforeBenchmark:$DeployProductBeforeBenchmark.IsPresent
}

$CombinedSummary = @()
$CombinedSamples = @()
$CombinedM37Metrics = @()
$CombinedProductPathStatus = @()
$CombinedStartupSessionTrace = @()
$CombinedCandidateSnapshots = @()
$CombinedRawLookupMicrobench = @()
$CombinedMemoryOwnerProfile = @()
foreach ($Summary in Get-ChildItem -LiteralPath $OutputRoot -Recurse -Filter summary.csv) {
    $CombinedSummary += Import-Csv -LiteralPath $Summary.FullName
}
foreach ($Samples in Get-ChildItem -LiteralPath $OutputRoot -Recurse -Filter samples.csv) {
    $CombinedSamples += Import-Csv -LiteralPath $Samples.FullName
}
foreach ($Metrics in Get-ChildItem -LiteralPath $OutputRoot -Recurse -Filter m37_metrics.csv) {
    $CombinedM37Metrics += Import-Csv -LiteralPath $Metrics.FullName
}
foreach ($Status in Get-ChildItem -LiteralPath $OutputRoot -Recurse -Filter product_path_status.csv) {
    $CombinedProductPathStatus += Import-Csv -LiteralPath $Status.FullName
}
foreach ($Trace in Get-ChildItem -LiteralPath $OutputRoot -Recurse -Filter startup_session_trace.csv) {
    $CombinedStartupSessionTrace += Import-Csv -LiteralPath $Trace.FullName
}
foreach ($Snapshot in Get-ChildItem -LiteralPath $OutputRoot -Recurse -Filter candidate_snapshots.csv) {
    $CombinedCandidateSnapshots += Import-Csv -LiteralPath $Snapshot.FullName
}
foreach ($RawLookup in Get-ChildItem -LiteralPath $OutputRoot -Recurse -Filter raw_lookup_microbench.csv) {
    $CombinedRawLookupMicrobench += Import-Csv -LiteralPath $RawLookup.FullName
}
foreach ($MemoryOwner in Get-ChildItem -LiteralPath $OutputRoot -Recurse -Filter memory-owner-profile.csv) {
    $CombinedMemoryOwnerProfile += Import-Csv -LiteralPath $MemoryOwner.FullName
}
$CombinedSummary | Export-Csv -LiteralPath (Join-Path $OutputRoot "summary.csv") -NoTypeInformation -Encoding UTF8
$CombinedSamples | Export-Csv -LiteralPath (Join-Path $OutputRoot "samples.csv") -NoTypeInformation -Encoding UTF8
$CombinedM37Metrics | Export-Csv -LiteralPath (Join-Path $OutputRoot "m37_metrics.csv") -NoTypeInformation -Encoding UTF8
$CombinedProductPathStatus | Export-Csv -LiteralPath (Join-Path $OutputRoot "product_path_status.csv") -NoTypeInformation -Encoding UTF8
$CombinedStartupSessionTrace | Export-Csv -LiteralPath (Join-Path $OutputRoot "startup_session_trace.csv") -NoTypeInformation -Encoding UTF8
$CombinedCandidateSnapshots | Export-Csv -LiteralPath (Join-Path $OutputRoot "candidate_snapshots.csv") -NoTypeInformation -Encoding UTF8
$CombinedRawLookupMicrobench | Export-Csv -LiteralPath (Join-Path $OutputRoot "raw_lookup_microbench.csv") -NoTypeInformation -Encoding UTF8
$CombinedMemoryOwnerProfile | Export-Csv -LiteralPath (Join-Path $OutputRoot "memory-owner-profile.csv") -NoTypeInformation -Encoding UTF8

$TrackAOwnerRows = @($CombinedMemoryOwnerProfile | Where-Object {
    $_.engine -eq "yune" -and
    $_.track -eq "track-a-comparison" -and
    $_.schema_id -eq "luna_pinyin"
})
if ($TrackAOwnerRows.Count -eq 0) {
    throw "The signed Track A gate produced no Yune luna_pinyin memory-owner rows."
}
$ByteBackedPoetOwners = @($TrackAOwnerRows | Where-Object {
    $_.mapping_mode -like "poet_bin:*"
})
if ($ByteBackedPoetOwners.Count -gt 0) {
    throw "The signed Track A gate requires default-owned poet measurement, but memory-owner evidence reports $($ByteBackedPoetOwners.Count) poet_bin owner rows."
}

$TrackAComparison = Write-TrackAComparison $CombinedSummary (Join-Path $OutputRoot "summary-comparison.csv")
Invoke-TrackAThresholdCheck $TrackAComparison $CombinedSummary $CombinedMemoryOwnerProfile $TrackAThresholds (Join-Path $OutputRoot "threshold-check.csv") -Fail:$($FailOnRegression.IsPresent)

$TrackBReadme = if ($SkipTrackB) { "skipped for this run." } else { "jyut6ping3_mobile, Yune Cantonese profile/product path." }
$ThresholdReadme = if ([string]::IsNullOrWhiteSpace($TrackAThresholds)) { "not run." } else { "threshold-check.csv against $TrackAThresholds." }
@"
# Native In-Process Benchmark

This run uses the Rust native_inprocess_benchmark bench and loads each engine DLL directly in the measured process. It does not use the historical managed .NET/PInvoke benchmark host.

- Track A: luna_pinyin, Yune versus librime 1.17.0.
- Track B: $TrackBReadme
- Track A inputs: $TrackAInputs.
- Track B inputs: $TrackBInputs.
- Summary comparison: summary-comparison.csv.
- Threshold gate: $ThresholdReadme
"@ | Set-Content -LiteralPath (Join-Path $OutputRoot "README.md") -Encoding UTF8

$InputDrift = @()
if ((File-Sha256 $YuneDll) -ne $YuneDllSha256) {
    $InputDrift += "Yune DLL changed after its measured hash was recorded: $YuneDll"
}
if ((File-Sha256 $UpstreamDll) -ne $UpstreamDllSha256) {
    $InputDrift += "upstream rime.dll changed during the benchmark: $UpstreamDll"
}
if ((Tree-Sha256 $SharedSource) -ne $UpstreamSharedTreeSha256) {
    $InputDrift += "upstream shared tree changed during the benchmark: $SharedSource"
}
if ((Tree-Sha256 $BuildSource) -ne $UpstreamBuildTreeSha256) {
    $InputDrift += "upstream build tree changed during the benchmark: $BuildSource"
}
if ((Tree-Sha256 $ProductSchemaRoot) -ne $ProductSchemaTreeSha256) {
    $InputDrift += "product schema tree changed during the benchmark: $ProductSchemaRoot"
}
if ((File-Sha256 $NativeBenchmarkExecutable) -ne $NativeBenchmarkExecutableSha256) {
    $InputDrift += "native benchmark executable changed during the benchmark: $NativeBenchmarkExecutable"
}
if ((File-Sha256 $NativeBenchmarkReceipt) -ne $NativeBenchmarkReceiptSha256) {
    $InputDrift += "native benchmark build receipt changed during the benchmark: $NativeBenchmarkReceipt"
}
if ((File-Sha256 $NativeBenchmarkReceiptInput) -ne $NativeBenchmarkReceiptInputSha256) {
    $InputDrift += "native benchmark build receipt input changed during the benchmark: $NativeBenchmarkReceiptInput"
}
if ((File-Sha256 $PSCommandPath) -ne $BenchmarkScriptSha256) {
    $InputDrift += "benchmark script changed during the benchmark: $PSCommandPath"
}
if (-not [string]::IsNullOrWhiteSpace($TrackAThresholds) -and
    (File-Sha256 $TrackAThresholds) -ne $TrackAThresholdsSha256) {
    $InputDrift += "Track A thresholds changed during the benchmark: $TrackAThresholds"
}
if ($InputDrift.Count -gt 0) {
    throw "Benchmark input immutability check failed: $($InputDrift -join '; ')"
}
foreach ($PathRecord in @(
    [pscustomobject]@{ Path = $OutputRoot; Label = "OutputRoot" },
    [pscustomobject]@{ Path = $WorkRoot; Label = "WorkRoot" },
    [pscustomobject]@{ Path = $UpstreamOracleRoot; Label = "UpstreamOracleRoot" },
    [pscustomobject]@{ Path = $ProductSchemaRoot; Label = "ProductSchemaRoot" },
    [pscustomobject]@{ Path = $YuneDll; Label = "YuneDll" },
    [pscustomobject]@{ Path = $NativeBenchmarkExecutable; Label = "native benchmark executable" },
    [pscustomobject]@{ Path = $NativeBenchmarkReceipt; Label = "native benchmark build receipt" }
)) {
    $CanonicalNow = Get-CanonicalSafePath $PathRecord.Path $PathRecord.Label
    if (-not $CanonicalNow.Equals($PathRecord.Path, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "$($PathRecord.Label) physical path changed during the benchmark: [$($PathRecord.Path)] -> [$CanonicalNow]"
    }
}
if ($NativeBenchmarkExecutableWasProvided) {
    $ReceiptInputNow = Get-CanonicalSafePath `
        $NativeBenchmarkReceiptInput `
        "prebuilt native benchmark receipt input"
    if (-not $ReceiptInputNow.Equals(
        $NativeBenchmarkReceiptInput,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Prebuilt native benchmark receipt input physical path changed during the benchmark"
    }
}
Assert-RepositorySourceSnapshot $InitialSourceSnapshot "final"
Set-RunStatus "complete"
}
catch {
    $FailureMessage = $_.Exception.Message
    try {
        Set-RunStatus "failed" $FailureMessage
    }
    catch {
        Write-Warning "Unable to record failed benchmark status: $($_.Exception.Message)"
    }
    throw
}
