param(
    [string]$OutputRoot,
    [string]$UpstreamOracleRoot,
    [string]$YuneDll,
    [int]$Iterations = 9,
    [int]$SessionIterations = 60,
    [int]$KeyIterations = 80,
    [string]$TrackAInputs = "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru",
    [string]$TrackBInputs = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung",
    [switch]$DeployProductBeforeBenchmark,
    [switch]$SkipTrackB,
    [string]$TrackAThresholds,
    [switch]$FailOnRegression
)

$ErrorActionPreference = "Stop"

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $RepoRoot "docs\reports\evidence\m36-product-path\phase-0-native-inprocess"
}
if ([string]::IsNullOrWhiteSpace($UpstreamOracleRoot)) {
    $UpstreamOracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ([string]::IsNullOrWhiteSpace($YuneDll)) {
    $YuneDll = Join-Path $RepoRoot "target\release\yune_rime_api.dll"
}

$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$EvidenceRoot = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot "docs\reports\evidence"))
$WorkRoot = Join-Path $RepoRoot ("target\native-inprocess\" + (Split-Path -Leaf $OutputRoot))
$UpstreamOracleRoot = [System.IO.Path]::GetFullPath($UpstreamOracleRoot)
$YuneDll = [System.IO.Path]::GetFullPath($YuneDll)
$SharedSource = Join-Path $UpstreamOracleRoot "rime-shared"
$BuildSource = Join-Path $UpstreamOracleRoot "rime-user\build"
$UpstreamDll = Join-Path $UpstreamOracleRoot "extract\dist\lib\rime.dll"
$UpstreamDistLib = Join-Path $UpstreamOracleRoot "extract\dist\lib"
$UpstreamBin = Join-Path $UpstreamOracleRoot "extract\bin"
$UpstreamDistBin = Join-Path $UpstreamOracleRoot "extract\dist\bin"
$ProductSchemaRoot = Join-Path $RepoRoot "apps\yune-web\source\public\schema"

function Assert-Path($Path, $Label) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing $Label`: $Path"
    }
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
        "bench", "-p", "yune-rime-api", "--bench", "native_inprocess_benchmark", "--",
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
    Invoke-Logged "$OutputName-native-inprocess" $BenchArgs $LogPath (($RunRoot, $ExtraPath) -join ";")
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

Clear-DirectoryUnder $EvidenceRoot $OutputRoot
Clear-DirectoryUnder (Join-Path $RepoRoot "target\native-inprocess") $WorkRoot

Assert-Path $UpstreamOracleRoot "upstream oracle root"
Assert-Path $SharedSource "upstream shared data"
Assert-Path $BuildSource "upstream prebuilt build data"
Assert-Path $UpstreamDll "upstream rime.dll"
if (-not $SkipTrackB) {
    Assert-Path $ProductSchemaRoot "TypeDuck-Web product schema assets"
}

Push-Location $RepoRoot
try {
    Invoke-Logged "cargo-build-release-yune-rime-api" @("build", "--release", "-p", "yune-rime-api") (Join-Path $OutputRoot "cargo-build-release-yune-rime-api.log")
} finally {
    Pop-Location
}
Assert-Path $YuneDll "Yune release DLL"

$TrackAYuneRun = Prepare-UpstreamRun "track-a-yune" $YuneDll
$TrackALibrimeRun = Prepare-UpstreamRun "track-a-librime-1.17.0" $UpstreamDll
if (-not $SkipTrackB) {
    $TrackBProductRun = Prepare-ProductRun "track-b-yune-product" $YuneDll
}

$BenchmarkCommand = "powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot $OutputRoot -Iterations $Iterations -SessionIterations $SessionIterations -KeyIterations $KeyIterations -TrackAInputs $TrackAInputs -TrackBInputs $TrackBInputs$(if ($DeployProductBeforeBenchmark) { ' -DeployProductBeforeBenchmark' } else { '' })$(if ($SkipTrackB) { ' -SkipTrackB' } else { '' })$(if (-not [string]::IsNullOrWhiteSpace($TrackAThresholds)) { " -TrackAThresholds $TrackAThresholds" } else { '' })$(if ($FailOnRegression) { ' -FailOnRegression' } else { '' })"
$Commands = @(
    "cargo build --release -p yune-rime-api",
    $BenchmarkCommand
)
$Commands | Set-Content -LiteralPath (Join-Path $OutputRoot "commands.txt") -Encoding UTF8

$YuneHead = (& git -C $RepoRoot rev-parse HEAD).Trim()
$YuneStatus = (& git -C $RepoRoot status --short) -join " | "
$Identity = @(
    "date_utc=$([DateTime]::UtcNow.ToString('o'))",
    "repo_root=$RepoRoot",
    "yune_git_head=$YuneHead",
    "yune_git_status_short=$YuneStatus",
    "upstream_oracle_root=$UpstreamOracleRoot",
    "transient_work_root=$WorkRoot",
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
$CombinedRawLookupMicrobench | Export-Csv -LiteralPath (Join-Path $OutputRoot "raw_lookup_microbench.csv") -NoTypeInformation -Encoding UTF8
$CombinedMemoryOwnerProfile | Export-Csv -LiteralPath (Join-Path $OutputRoot "memory-owner-profile.csv") -NoTypeInformation -Encoding UTF8

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
