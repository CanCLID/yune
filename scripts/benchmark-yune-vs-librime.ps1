param(
    [string]$OutputRoot,
    [string]$UpstreamOracleRoot,
    [string]$YuneDll,
    [int]$Iterations = 9,
    [int]$SessionIterations = 60,
    [int]$KeyIterations = 80
)

$ErrorActionPreference = "Stop"

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $RepoRoot "docs\reports\evidence\yune-vs-librime-2026-06-23"
}
if ([string]::IsNullOrWhiteSpace($UpstreamOracleRoot)) {
    $UpstreamOracleRoot = Join-Path $RepoRoot "target\upstream-oracle\1.17.0"
}
if ([string]::IsNullOrWhiteSpace($YuneDll)) {
    $YuneDll = Join-Path $RepoRoot "target\release\yune_rime_api.dll"
}

$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
$UpstreamOracleRoot = [System.IO.Path]::GetFullPath($UpstreamOracleRoot)
$YuneDll = [System.IO.Path]::GetFullPath($YuneDll)
$EvidenceRoot = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot "docs\reports\evidence"))
$WorkRoot = Join-Path $RepoRoot ("target\yune-vs-librime-benchmark\" + (Split-Path -Leaf $OutputRoot))
$RunnerSource = Join-Path $RepoRoot "scripts\yune-vs-librime-benchmark.cs"
$RunnerBuild = Join-Path $WorkRoot "runner"
$RunnerExe = Join-Path $RunnerBuild "YuneVsLibrimeBenchmark.exe"
$SharedSource = Join-Path $UpstreamOracleRoot "rime-shared"
$BuildSource = Join-Path $UpstreamOracleRoot "rime-user\build"
$UpstreamDll = Join-Path $UpstreamOracleRoot "extract\dist\lib\rime.dll"
$UpstreamDistLib = Join-Path $UpstreamOracleRoot "extract\dist\lib"
$UpstreamBin = Join-Path $UpstreamOracleRoot "extract\bin"
$UpstreamDistBin = Join-Path $UpstreamOracleRoot "extract\dist\bin"

function Assert-Path($Path, $Label) {
    if (-not (Test-Path -LiteralPath $Path)) {
        throw "Missing $Label`: $Path"
    }
}

function Clear-ChildDirectory($Path) {
    $ResolvedOutput = $OutputRoot.TrimEnd('\')
    $ResolvedPath = [System.IO.Path]::GetFullPath($Path)
    if (-not $ResolvedPath.StartsWith($ResolvedOutput + "\", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to clear directory outside output root: $ResolvedPath"
    }
    if (Test-Path -LiteralPath $ResolvedPath) {
        Remove-Item -LiteralPath $ResolvedPath -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $ResolvedPath | Out-Null
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

function Invoke-ProcessLogged($Description, $FilePath, [string[]]$ArgumentList, $LogPath, $WorkingDirectory = $null) {
    $LogDir = Split-Path -Parent $LogPath
    New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
    $StdOut = Join-Path $LogDir "$Description.stdout.tmp"
    $StdErr = Join-Path $LogDir "$Description.stderr.tmp"
    Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue
    $StartArgs = @{
        FilePath = $FilePath
        ArgumentList = $ArgumentList
        Wait = $true
        PassThru = $true
        NoNewWindow = $true
        RedirectStandardOutput = $StdOut
        RedirectStandardError = $StdErr
    }
    if (-not [string]::IsNullOrWhiteSpace($WorkingDirectory)) {
        $StartArgs.WorkingDirectory = $WorkingDirectory
    }
    $Process = Start-Process @StartArgs
    $Output = @()
    if (Test-Path -LiteralPath $StdOut) {
        $Output += Get-Content -LiteralPath $StdOut
    }
    if (Test-Path -LiteralPath $StdErr) {
        $Output += Get-Content -LiteralPath $StdErr
    }
    $Output | Set-Content -LiteralPath $LogPath -Encoding UTF8
    $Output | ForEach-Object { Write-Host $_ }
    Remove-Item -LiteralPath $StdOut, $StdErr -Force -ErrorAction SilentlyContinue
    if ($Process.ExitCode -ne 0) {
        throw "$Description failed with exit code $($Process.ExitCode)"
    }
}

function Prepare-EngineRun($EngineName, $DllPath) {
    $RunRoot = Join-Path $WorkRoot $EngineName
    Clear-DirectoryUnder $WorkRoot $RunRoot
    Copy-Item -LiteralPath $RunnerExe -Destination (Join-Path $RunRoot "YuneVsLibrimeBenchmark.exe") -Force
    Copy-Item -LiteralPath $DllPath -Destination (Join-Path $RunRoot "rime.dll") -Force
    Copy-DirectoryContents $SharedSource (Join-Path $RunRoot "shared")
    New-Item -ItemType Directory -Force -Path (Join-Path $RunRoot "user") | Out-Null
    Copy-DirectoryContents $BuildSource (Join-Path $RunRoot "user\build")
    return $RunRoot
}

function Run-EngineBenchmark($EngineName, $DllPath, $ExtraPath) {
    $RunRoot = Prepare-EngineRun $EngineName $DllPath
    $EngineOutput = Join-Path (Join-Path $OutputRoot $EngineName) "results"
    Clear-ChildDirectory $EngineOutput
    New-Item -ItemType Directory -Force -Path $EngineOutput | Out-Null
    $Runner = Join-Path $RunRoot "YuneVsLibrimeBenchmark.exe"
    $PreviousPath = $env:PATH
    try {
        $env:PATH = ($RunRoot, $ExtraPath, $PreviousPath -join ";")
        $LogPath = Join-Path $EngineOutput "raw-run.log"
        Invoke-ProcessLogged "$EngineName-benchmark" $Runner @(
            "--engine", $EngineName,
            "--schema", "luna_pinyin",
            "--shared", (Join-Path $RunRoot "shared"),
            "--user", (Join-Path $RunRoot "user"),
            "--build", (Join-Path $RunRoot "user\build"),
            "--output", $EngineOutput,
            "--iterations", "$Iterations",
            "--session-iterations", "$SessionIterations",
            "--key-iterations", "$KeyIterations"
        ) $LogPath $RunRoot
    } finally {
        $env:PATH = $PreviousPath
    }
}

Clear-DirectoryUnder $EvidenceRoot $OutputRoot
Clear-DirectoryUnder (Join-Path $RepoRoot "target\yune-vs-librime-benchmark") $WorkRoot
Clear-DirectoryUnder $WorkRoot $RunnerBuild

Assert-Path $RunnerSource "benchmark runner source"
Assert-Path $UpstreamOracleRoot "upstream oracle root"
Assert-Path $SharedSource "upstream shared data"
Assert-Path $BuildSource "upstream prebuilt build data"
Assert-Path $UpstreamDll "upstream rime.dll"

$CargoBuildLog = Join-Path $OutputRoot "cargo-build-release-yune-rime-api.log"
Push-Location $RepoRoot
try {
    Invoke-ProcessLogged "cargo-build-release-yune-rime-api" "cargo" @("build", "--release", "-p", "yune-rime-api") $CargoBuildLog $RepoRoot
} finally {
    Pop-Location
}
Assert-Path $YuneDll "Yune release DLL"

Add-Type -Path $RunnerSource -OutputAssembly $RunnerExe -OutputType ConsoleApplication
Assert-Path $RunnerExe "compiled benchmark runner"

$Commands = @(
    "cargo build --release -p yune-rime-api",
    "powershell -ExecutionPolicy Bypass -File scripts\benchmark-yune-vs-librime.ps1 -OutputRoot $OutputRoot -Iterations $Iterations -SessionIterations $SessionIterations -KeyIterations $KeyIterations"
)
$Commands | Set-Content -LiteralPath (Join-Path $OutputRoot "commands.txt") -Encoding UTF8

$YuneHead = (& git -C $RepoRoot rev-parse HEAD).Trim()
$YuneStatus = (& git -C $RepoRoot status --short) -join " | "
$LocalLibrimeRoot = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot "..\librime"))
$LocalLibrimeRemote = if (Test-Path -LiteralPath $LocalLibrimeRoot) { ((& git -C $LocalLibrimeRoot remote get-url origin) -join " ").Trim() } else { "missing" }
$LocalLibrimeHead = if (Test-Path -LiteralPath $LocalLibrimeRoot) { ((& git -C $LocalLibrimeRoot rev-parse HEAD) -join " ").Trim() } else { "missing" }
$LocalLibrimeTagCommit = if (Test-Path -LiteralPath $LocalLibrimeRoot) { ((& git -C $LocalLibrimeRoot rev-list -n 1 1.17.0) -join " ").Trim() } else { "missing" }
$YuneDllSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $YuneDll).Hash
$UpstreamDllSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $UpstreamDll).Hash
$ComputerSystem = Get-CimInstance Win32_ComputerSystem
$Processor = Get-CimInstance Win32_Processor | Select-Object -First 1
$ProcessorName = $Processor.Name.Trim()
$OperatingSystem = Get-CimInstance Win32_OperatingSystem
$Identity = @(
    "date_utc=$([DateTime]::UtcNow.ToString('o'))",
    "repo_root=$RepoRoot",
    "yune_git_head=$YuneHead",
    "yune_git_status_short=$YuneStatus",
    "local_librime_root=$LocalLibrimeRoot",
    "local_librime_origin=$LocalLibrimeRemote",
    "local_librime_head=$LocalLibrimeHead",
    "local_librime_1_17_0_commit=$LocalLibrimeTagCommit",
    "upstream_oracle_root=$UpstreamOracleRoot",
    "transient_work_root=$WorkRoot",
    "upstream_rime_dll=$UpstreamDll",
    "upstream_rime_dll_sha256=$UpstreamDllSha256",
    "yune_dll=$YuneDll",
    "yune_dll_sha256=$YuneDllSha256",
    "shared_source=$SharedSource",
    "build_source=$BuildSource",
    "machine_manufacturer=$($ComputerSystem.Manufacturer)",
    "machine_model=$($ComputerSystem.Model)",
    "machine_total_physical_memory_bytes=$($ComputerSystem.TotalPhysicalMemory)",
    "processor=$ProcessorName",
    "processor_cores=$($Processor.NumberOfCores)",
    "processor_logical_processors=$($Processor.NumberOfLogicalProcessors)",
    "os_caption=$($OperatingSystem.Caption)",
    "os_version=$($OperatingSystem.Version)",
    "os_build_number=$($OperatingSystem.BuildNumber)",
    "iterations=$Iterations",
    "session_iterations=$SessionIterations",
    "key_iterations=$KeyIterations"
)
$Identity | Set-Content -LiteralPath (Join-Path $OutputRoot "environment.txt") -Encoding UTF8

Run-EngineBenchmark "yune" $YuneDll $UpstreamDistLib
Run-EngineBenchmark "librime-1.17.0" $UpstreamDll (($UpstreamDistLib, $UpstreamBin, $UpstreamDistBin) -join ";")

$CombinedSummary = @()
foreach ($Summary in @(
    (Join-Path $OutputRoot "yune\results\summary.csv"),
    (Join-Path $OutputRoot "librime-1.17.0\results\summary.csv")
)) {
    Assert-Path $Summary "engine summary"
    $CombinedSummary += Import-Csv -LiteralPath $Summary
}
$CombinedSummary | Export-Csv -LiteralPath (Join-Path $OutputRoot "summary.csv") -NoTypeInformation -Encoding UTF8

$CombinedSamples = @()
foreach ($Samples in @(
    (Join-Path $OutputRoot "yune\results\samples.csv"),
    (Join-Path $OutputRoot "librime-1.17.0\results\samples.csv")
)) {
    Assert-Path $Samples "engine samples"
    $CombinedSamples += Import-Csv -LiteralPath $Samples
}
$CombinedSamples | Export-Csv -LiteralPath (Join-Path $OutputRoot "samples.csv") -NoTypeInformation -Encoding UTF8

@"
# Yune vs librime Benchmark Evidence

Generated by ``scripts\benchmark-yune-vs-librime.ps1``.

- Summary: ``summary.csv``
- Raw samples: ``samples.csv``
- Yune raw run: ``yune\results\raw-run.log``
- librime raw run: ``librime-1.17.0\results\raw-run.log``
- Yune build log: ``cargo-build-release-yune-rime-api.log``
- Commands: ``commands.txt``
- Environment: ``environment.txt``
"@ | Set-Content -LiteralPath (Join-Path $OutputRoot "README.md") -Encoding UTF8

Write-Host "Wrote benchmark evidence to $OutputRoot"
