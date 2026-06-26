param(
    [string]$OutputRoot,
    [string]$UpstreamOracleRoot,
    [string]$YuneDll,
    [int]$Iterations = 9,
    [int]$SessionIterations = 60,
    [int]$KeyIterations = 80,
    [string]$TrackAInputs = "ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru",
    [string]$TrackBInputs = "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung",
    [switch]$DeployProductBeforeBenchmark
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

Clear-DirectoryUnder $EvidenceRoot $OutputRoot
Clear-DirectoryUnder (Join-Path $RepoRoot "target\native-inprocess") $WorkRoot

Assert-Path $UpstreamOracleRoot "upstream oracle root"
Assert-Path $SharedSource "upstream shared data"
Assert-Path $BuildSource "upstream prebuilt build data"
Assert-Path $UpstreamDll "upstream rime.dll"
Assert-Path $ProductSchemaRoot "TypeDuck-Web product schema assets"

Push-Location $RepoRoot
try {
    Invoke-Logged "cargo-build-release-yune-rime-api" @("build", "--release", "-p", "yune-rime-api") (Join-Path $OutputRoot "cargo-build-release-yune-rime-api.log")
} finally {
    Pop-Location
}
Assert-Path $YuneDll "Yune release DLL"

$TrackAYuneRun = Prepare-UpstreamRun "track-a-yune" $YuneDll
$TrackALibrimeRun = Prepare-UpstreamRun "track-a-librime-1.17.0" $UpstreamDll
$TrackBProductRun = Prepare-ProductRun "track-b-yune-product" $YuneDll

$Commands = @(
    "cargo build --release -p yune-rime-api",
    "powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot $OutputRoot -Iterations $Iterations -SessionIterations $SessionIterations -KeyIterations $KeyIterations -TrackAInputs $TrackAInputs -TrackBInputs $TrackBInputs$(if ($DeployProductBeforeBenchmark) { ' -DeployProductBeforeBenchmark' } else { '' })"
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
    "track_a_inputs=$TrackAInputs",
    "track_b_inputs=$TrackBInputs",
    "iterations=$Iterations",
    "session_iterations=$SessionIterations",
    "key_iterations=$KeyIterations"
)
$Identity | Set-Content -LiteralPath (Join-Path $OutputRoot "environment.txt") -Encoding UTF8

Run-NativeBench "yune" "track-a-comparison" "luna_pinyin" $TrackAYuneRun $UpstreamDistLib $TrackAInputs "track-a-yune"
Run-NativeBench "librime-1.17.0" "track-a-comparison" "luna_pinyin" $TrackALibrimeRun (($UpstreamDistLib, $UpstreamBin, $UpstreamDistBin) -join ";") $TrackAInputs "track-a-librime-1.17.0"
Run-NativeBench "yune" "track-b-product" "jyut6ping3_mobile" $TrackBProductRun $RepoRoot $TrackBInputs "track-b-yune-product" -DeployBeforeBenchmark:$DeployProductBeforeBenchmark.IsPresent

$CombinedSummary = @()
$CombinedSamples = @()
$CombinedM37Metrics = @()
$CombinedProductPathStatus = @()
$CombinedStartupSessionTrace = @()
$CombinedRawLookupMicrobench = @()
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
$CombinedSummary | Export-Csv -LiteralPath (Join-Path $OutputRoot "summary.csv") -NoTypeInformation -Encoding UTF8
$CombinedSamples | Export-Csv -LiteralPath (Join-Path $OutputRoot "samples.csv") -NoTypeInformation -Encoding UTF8
$CombinedM37Metrics | Export-Csv -LiteralPath (Join-Path $OutputRoot "m37_metrics.csv") -NoTypeInformation -Encoding UTF8
$CombinedProductPathStatus | Export-Csv -LiteralPath (Join-Path $OutputRoot "product_path_status.csv") -NoTypeInformation -Encoding UTF8
$CombinedStartupSessionTrace | Export-Csv -LiteralPath (Join-Path $OutputRoot "startup_session_trace.csv") -NoTypeInformation -Encoding UTF8
$CombinedRawLookupMicrobench | Export-Csv -LiteralPath (Join-Path $OutputRoot "raw_lookup_microbench.csv") -NoTypeInformation -Encoding UTF8

@"
# Native In-Process Benchmark

This run uses the Rust `native_inprocess_benchmark` bench and loads each engine DLL directly in the measured process. It does not use the historical managed `.NET`/PInvoke benchmark host.

- Track A: `luna_pinyin`, Yune versus librime `1.17.0`.
- Track B: `jyut6ping3_mobile`, Yune Cantonese profile/product path.
- Track A inputs: `$TrackAInputs`.
- Track B inputs: `$TrackBInputs`.
"@ | Set-Content -LiteralPath (Join-Path $OutputRoot "README.md") -Encoding UTF8
